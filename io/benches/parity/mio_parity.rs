//! `epoll-spin` against mio: the same mechanism, gated both ways.
//!
//! On Linux mio (and so tokio) waits with `epoll_wait(timeout = 0)` when
//! polled with a zero timeout, then reads the sockets it reports ready.
//! ergon-io's `epoll-spin` backend does the same. The differences are that
//! mio registers every socket edge-triggered and must read until
//! `WouldBlock` (two reads per event: the data, then the empty read), while
//! `epoll-spin` is level-triggered and reads once per ready socket.
//!
//! Two kinds of test:
//!
//! - **Structural** (deterministic): the `Driver`'s own counters show one
//!   `epoll_wait` per poll, a zero timeout (an idle poll returns at once), and
//!   one `recvmsg` per ready socket; the mio loop's counters show two reads
//!   per event.
//! - **Parity** (timed): a loopback TLS 1.3 WebSocket round trip, 64-byte
//!   payload, 10,000/s, once through the `Driver` with `Backend::EpollSpin`
//!   and once through a minimal mio loop doing the same work: the same TLS
//!   record layer (`ergon_io::tls::RecordLayer`), the same WebSocket framing
//!   (`ergon_io::ws`), `TCP_NODELAY` and a 4 MiB `SO_RCVBUF` on both, the
//!   same echo peer on its own CPU. Interleaved A/B pairs, alternating order,
//!   each echo checked byte for byte; the verdict is `rule::verdict`. Either
//!   side ahead fails: a large `epoll-spin` win is a suspected defect too.
//!
//! Run (release, one thread):
//! `cargo test --release --manifest-path io/benches/Cargo.toml --test mio_parity -- --test-threads=1 --nocapture`.
#![allow(missing_docs)]

mod rule;

#[cfg(target_os = "linux")]
mod linux {
    use std::hint::black_box;
    use std::io::{ErrorKind, Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use ergon_io::clock::{monotonic_ns, pin_current_thread};
    use ergon_io::tls::{self, HEADER_LEN, Opened, RecordLayer};
    use ergon_io::{
        Backend, Config, ConnId, Driver, DriverStats, Error, Handler, RxMeta, Target, WsConn,
        WsHandler, WsOptions, ws,
    };
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use rustls::{ClientConfig, RootCertStore, ServerConfig, ServerConnection};

    use super::rule::{
        PAIRS, Pair, Pct, Verdict, paired_diff_ns, percentiles, tolerance_ns, verdict,
    };

    type R<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    /// WebSocket payload, bytes: an order-sized message.
    const PAYLOAD: usize = 64;
    /// One round trip every 100 µs: the fair run's 10,000/s.
    const INTERVAL_NS: i64 = 100_000;
    /// Round trips before timing starts, each checked like the timed ones.
    const WARMUP: usize = 2_000;
    /// Timed round trips per pair and arm.
    const TIMED: usize = 10_000;
    /// Blocks per pair, half per arm: 1,000 round trips (0.1 s) each.
    const BLOCKS: usize = 20;
    /// `SO_RCVBUF` on both sides: the `Driver`'s default (`Config::rcvbuf`).
    const RCVBUF: usize = 4 << 20;
    /// Events per `Poll::poll`: the `Driver`'s `epoll_wait` batch
    /// (`MAX_EVENTS` in `io/src/driver/epoll.rs`). Under amd64 emulation on
    /// arm64 (Rosetta) the cost of `epoll_wait` grows with `maxevents`: with
    /// 1,024 the mio loop was 6-9 µs slower per round trip than with 64
    /// (`io/benches/results/mio-parity-docker-20261024T145651Z/summary.md`).
    const EVENTS: usize = 64;
    /// mio's receive buffer, bytes.
    const RX_BUF: usize = 1 << 16;

    // ── the echo peer ────────────────────────────────────────────────────

    struct Lab {
        listener: TcpListener,
        addr: SocketAddr,
        server: Arc<ServerConfig>,
        client: Arc<ClientConfig>,
    }

    fn lab() -> R<Lab> {
        let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let cert = CertificateDer::from(ck.cert.der().to_vec());
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let server = Arc::new(
            ServerConfig::builder_with_provider(provider)
                .with_protocol_versions(&[&rustls::version::TLS13])?
                .with_no_client_auth()
                .with_single_cert(vec![cert.clone()], key)?,
        );
        let mut roots = RootCertStore::empty();
        roots.add(cert)?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        Ok(Lab {
            addr: listener.local_addr()?,
            listener,
            server,
            client: tls::client_config(roots)?,
        })
    }

    /// The server side of one connection.
    struct Served {
        tls: ServerConnection,
        sock: TcpStream,
        plain: Vec<u8>,
    }

    /// Moves the plaintext rustls holds into `plain`; returns the bytes moved.
    fn drain_plain(tls: &mut ServerConnection, plain: &mut Vec<u8>) -> R<usize> {
        let mut b = [0u8; 4096];
        let mut total = 0;
        loop {
            match tls.reader().read(&mut b) {
                Ok(0) => return Ok(total),
                Ok(n) => {
                    plain.extend_from_slice(&b[..n]);
                    total += n;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(total),
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn write_tls(tls: &mut ServerConnection, sock: &mut TcpStream, data: &[u8]) -> R {
        tls.writer().write_all(data)?;
        while tls.wants_write() {
            match tls.write_tls(sock) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => std::hint::spin_loop(),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Completes the TLS handshake and the WebSocket upgrade, blocking.
    fn serve(cfg: &Arc<ServerConfig>, sock: TcpStream) -> R<Served> {
        sock.set_nonblocking(false)?;
        sock.set_nodelay(true)?;
        sock.set_read_timeout(Some(Duration::from_secs(10)))?;
        let mut s = Served {
            tls: ServerConnection::new(cfg.clone())?,
            sock,
            plain: Vec::new(),
        };
        while s.tls.is_handshaking() {
            s.tls.complete_io(&mut s.sock)?;
        }
        let (n, key) = loop {
            if let Some((n, key, _)) = ws::parse_server_request(&s.plain)? {
                break (n, key);
            }
            if drain_plain(&mut s.tls, &mut s.plain)? > 0 {
                continue;
            }
            if s.tls.read_tls(&mut s.sock)? == 0 {
                return Err("client closed during the upgrade".into());
            }
            s.tls.process_new_packets()?;
        };
        s.plain.drain(..n);
        write_tls(&mut s.tls, &mut s.sock, &ws::server_response(&key))?;
        Ok(s)
    }

    /// Accepts `n` clients, then echoes every binary frame on the connections
    /// at `active` (spinning over their non-blocking sockets) until a client
    /// goes away. The rest stay open and idle. Returns the echoes sent.
    fn echo(lab: &Lab, n: usize, active: &[usize], cpu: Option<usize>) -> R<u64> {
        if let Some(c) = cpu {
            pin_current_thread(c)?;
        }
        lab.listener.set_nonblocking(true)?;
        let end = Instant::now() + Duration::from_secs(30);
        let mut conns = Vec::with_capacity(n);
        while conns.len() < n {
            match lab.listener.accept() {
                Ok((sock, _)) => conns.push(serve(&lab.server, sock)?),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if Instant::now() > end {
                        return Err(format!("only {} of {n} clients connected", conns.len()).into());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => return Err(e.into()),
            }
        }
        for &i in active {
            conns[i].sock.set_nonblocking(true)?;
        }
        let mut out = vec![0u8; ws::header_len(PAYLOAD, false) + PAYLOAD];
        let mut echoes = 0;
        loop {
            for &i in active {
                let Served { tls, sock, plain } = &mut conns[i];
                match tls.read_tls(sock) {
                    Ok(0) => return Ok(echoes),
                    Ok(_) => {
                        if tls.process_new_packets()?.peer_has_closed() {
                            return Ok(echoes);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        std::hint::spin_loop();
                        continue;
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            ErrorKind::ConnectionReset | ErrorKind::BrokenPipe
                        ) =>
                    {
                        return Ok(echoes);
                    }
                    Err(e) => return Err(e.into()),
                }
                drain_plain(tls, plain)?;
                let mut at = 0;
                while let Some(h) = ws::parse_header(&plain[at..])? {
                    if plain.len() - at < h.frame_len() {
                        break;
                    }
                    let body = &mut plain[at + h.header_len..at + h.frame_len()];
                    if let Some(m) = h.mask {
                        ws::apply_mask(body, m);
                    }
                    let op = match h.opcode {
                        ws::OP_BINARY => ws::OP_BINARY,
                        ws::OP_PING => ws::OP_PONG,
                        ws::OP_CLOSE => return Ok(echoes),
                        o => return Err(format!("echo: unexpected opcode {o}").into()),
                    };
                    out.resize(ws::header_len(body.len(), false) + body.len(), 0);
                    let k = ws::encode_frame(op, body, None, &mut out)?;
                    write_tls(tls, sock, &out[..k])?;
                    echoes += 1;
                    at += h.frame_len();
                }
                plain.drain(..at);
            }
        }
    }

    // ── the two client loops ─────────────────────────────────────────────

    /// One client loop: queue or send an order, and make one pass.
    trait Arm {
        const NAME: &'static str;
        /// Sends `payload` on the order connection.
        fn send(&mut self, payload: &[u8]) -> R;
        /// One pass of the loop: `Some(t)` when the echo arrived, decoded
        /// at `t` (`CLOCK_MONOTONIC` ns), now in [`Arm::echo`].
        fn pass(&mut self) -> R<Option<i64>>;
        /// The last echo's payload.
        fn echo(&self) -> &[u8; PAYLOAD];
    }

    /// ergon-io: the `Driver` with `Backend::EpollSpin`, default config.
    struct DriverArm {
        d: Driver<WsConn>,
        h: Got,
    }

    struct Got {
        order: ConnId,
        at: Option<i64>,
        echo: [u8; PAYLOAD],
        err: Option<String>,
    }

    impl Handler for Got {
        fn on_closed(&mut self, id: ConnId, error: Error) {
            self.err
                .get_or_insert_with(|| format!("{id:?} closed: {error}"));
        }
    }

    impl WsHandler for Got {
        fn on_message(&mut self, id: ConnId, opcode: u8, payload: &[u8], _: &RxMeta, _: u16) {
            let t = monotonic_ns();
            if id != self.order
                || opcode != ws::OP_BINARY
                || payload.len() != PAYLOAD
                || self.at.is_some()
            {
                self.err.get_or_insert_with(|| {
                    format!(
                        "unexpected message on {id:?}: opcode {opcode}, {} bytes",
                        payload.len()
                    )
                });
                return;
            }
            self.echo.copy_from_slice(black_box(payload));
            self.at = Some(t);
        }
    }

    impl DriverArm {
        fn new(lab: &Lab, n: usize) -> R<Self> {
            let mut d = Driver::new(Config::new(Backend::EpollSpin))?;
            // No timers: the mio loop has none either.
            let opts = WsOptions {
                ping_interval_ms: 0,
                pong_timeout_ms: 0,
                ..WsOptions::default()
            };
            let target = Target::new("localhost", lab.addr.port(), "/").with_addr(lab.addr);
            let mut order = None;
            for _ in 0..n {
                let id = d.add(WsConn::connect_with(
                    &target,
                    lab.client.clone(),
                    opts.clone(),
                )?)?;
                order.get_or_insert(id);
            }
            Ok(Self {
                d,
                h: Got {
                    order: order.ok_or("no connection")?,
                    at: None,
                    echo: [0; PAYLOAD],
                    err: None,
                },
            })
        }

        const fn stats(&self) -> DriverStats {
            *self.d.stats()
        }
    }

    impl Arm for DriverArm {
        const NAME: &'static str = "epoll-spin";

        #[inline]
        fn send(&mut self, payload: &[u8]) -> R {
            let mut c = self
                .d
                .get_mut(self.h.order)
                .ok_or("order connection gone")?;
            c.send_binary(payload)?;
            Ok(())
        }

        #[inline]
        fn pass(&mut self) -> R<Option<i64>> {
            self.d.poll(&mut self.h)?;
            if let Some(e) = self.h.err.take() {
                return Err(e.into());
            }
            Ok(self.h.at.take())
        }

        fn echo(&self) -> &[u8; PAYLOAD] {
            &self.h.echo
        }
    }

    /// The receive side of one mio connection: ciphertext at `head..tail`,
    /// decrypted in place, plaintext copied out for framing.
    struct Rx {
        buf: Box<[u8]>,
        head: usize,
        tail: usize,
        plain: Vec<u8>,
    }

    impl Rx {
        /// Opens every complete record; a reply the record layer queues (a
        /// key update) lands in `tx`.
        fn open(&mut self, rec: &mut RecordLayer, tx: &mut Vec<u8>) -> R {
            while let Some(len) = RecordLayer::complete_record_len(&self.buf[self.head..self.tail])?
            {
                let r = &mut self.buf[self.head..self.head + len];
                if let Opened::AppData(n) = rec.open_in_place(r, tx)? {
                    self.plain.extend_from_slice(&r[HEADER_LEN..HEADER_LEN + n]);
                }
                self.head += len;
            }
            if self.head == self.tail {
                (self.head, self.tail) = (0, 0);
            } else if self.buf.len() - self.tail < RX_BUF / 2 {
                self.buf.copy_within(self.head..self.tail, 0);
                (self.head, self.tail) = (0, self.tail - self.head);
            }
            Ok(())
        }
    }

    struct MioConn {
        sock: mio::net::TcpStream,
        rec: RecordLayer,
        rx: Rx,
        tx: Vec<u8>,
        frame: Vec<u8>,
        mask: u32,
    }

    fn write_spin(sock: &mut mio::net::TcpStream, mut data: &[u8]) -> R {
        while !data.is_empty() {
            match sock.write(data) {
                Ok(n) => data = &data[n..],
                Err(e) if e.kind() == ErrorKind::WouldBlock => std::hint::spin_loop(),
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    impl MioConn {
        /// Connects, then runs the TLS handshake and the upgrade blocking,
        /// as `WsConn::connect_with` does; then switches to non-blocking.
        fn open(lab: &Lab, idx: u8) -> R<Self> {
            let mut s = TcpStream::connect(lab.addr)?;
            s.set_nodelay(true)?;
            socket2::SockRef::from(&s).set_recv_buffer_size(RCVBUF)?;
            s.set_read_timeout(Some(Duration::from_secs(10)))?;
            let (mut rec, leftover) = tls::handshake(&mut s, lab.client.clone(), "localhost")?;
            let key = ws::client_key([idx; 16]);
            let host = format!("localhost:{}", lab.addr.port());
            let mut tx = Vec::with_capacity(256);
            rec.seal_app(&ws::client_request(&host, "/", &key, &[]), &mut tx)?;
            s.write_all(&tx)?;
            tx.clear();
            let mut rx = Rx {
                buf: vec![0u8; RX_BUF].into_boxed_slice(),
                head: 0,
                tail: leftover.len(),
                plain: Vec::with_capacity(4096),
            };
            rx.buf[..leftover.len()].copy_from_slice(&leftover);
            loop {
                rx.open(&mut rec, &mut tx)?;
                if let Some(n) = ws::parse_client_response(&rx.plain, &key)? {
                    rx.plain.drain(..n);
                    break;
                }
                let k = s.read(&mut rx.buf[rx.tail..])?;
                if k == 0 {
                    return Err("server closed during the upgrade".into());
                }
                rx.tail += k;
            }
            if !tx.is_empty() {
                s.write_all(&tx)?;
                tx.clear();
            }
            s.set_read_timeout(None)?;
            s.set_nonblocking(true)?;
            Ok(Self {
                sock: mio::net::TcpStream::from_std(s),
                rec,
                rx,
                tx,
                frame: vec![0u8; ws::header_len(PAYLOAD, true) + PAYLOAD],
                mask: 0x9E37_79B9 ^ u32::from(idx),
            })
        }

        const fn next_mask(&mut self) -> [u8; 4] {
            // xorshift32: masks need not be secret here, only varied.
            let mut x = self.mask;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.mask = x;
            x.to_le_bytes()
        }
    }

    /// mio: one `Poll`, every socket registered readable (mio registers
    /// edge-triggered on Linux), `Poll::poll` with a zero timeout every pass,
    /// and each ready socket read until `WouldBlock`, as mio requires.
    struct MioArm {
        poll: mio::Poll,
        events: mio::Events,
        conns: Vec<MioConn>,
        echo: [u8; PAYLOAD],
        polls: u64,
        reads: u64,
        would_block: u64,
    }

    impl MioArm {
        fn new(lab: &Lab, n: usize) -> R<Self> {
            let poll = mio::Poll::new()?;
            let mut conns = Vec::with_capacity(n);
            for i in 0..n {
                let mut c = MioConn::open(lab, u8::try_from(i)?)?;
                poll.registry()
                    .register(&mut c.sock, mio::Token(i), mio::Interest::READABLE)?;
                conns.push(c);
            }
            Ok(Self {
                poll,
                events: mio::Events::with_capacity(EVENTS),
                conns,
                echo: [0; PAYLOAD],
                polls: 0,
                reads: 0,
                would_block: 0,
            })
        }
    }

    impl Arm for MioArm {
        const NAME: &'static str = "mio";

        #[inline]
        fn send(&mut self, payload: &[u8]) -> R {
            let c = &mut self.conns[0];
            let mask = c.next_mask();
            let n = ws::encode_frame(ws::OP_BINARY, payload, Some(mask), &mut c.frame)?;
            c.rec.seal_app(&c.frame[..n], &mut c.tx)?;
            write_spin(&mut c.sock, &c.tx)?;
            c.tx.clear();
            Ok(())
        }

        #[inline]
        fn pass(&mut self) -> R<Option<i64>> {
            self.poll.poll(&mut self.events, Some(Duration::ZERO))?;
            self.polls += 1;
            let mut got = None;
            for ev in &self.events {
                let i = ev.token().0;
                let c = &mut self.conns[i];
                loop {
                    if c.rx.tail == c.rx.buf.len() {
                        return Err("mio receive buffer full".into());
                    }
                    match c.sock.read(&mut c.rx.buf[c.rx.tail..]) {
                        Ok(0) => return Err("echo peer closed".into()),
                        Ok(k) => {
                            self.reads += 1;
                            c.rx.tail += k;
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            self.reads += 1;
                            self.would_block += 1;
                            break;
                        }
                        Err(e) if e.kind() == ErrorKind::Interrupted => {}
                        Err(e) => return Err(e.into()),
                    }
                }
                c.rx.open(&mut c.rec, &mut c.tx)?;
                if !c.tx.is_empty() {
                    write_spin(&mut c.sock, &c.tx)?;
                    c.tx.clear();
                }
                let mut at = 0;
                while let Some(h) = ws::parse_header(&c.rx.plain[at..])? {
                    if c.rx.plain.len() - at < h.frame_len() {
                        break;
                    }
                    let t = monotonic_ns();
                    let body = &c.rx.plain[at + h.header_len..at + h.frame_len()];
                    if i != 0 || h.opcode != ws::OP_BINARY || body.len() != PAYLOAD || got.is_some()
                    {
                        return Err(format!(
                            "unexpected message on {i}: opcode {}, {} bytes",
                            h.opcode,
                            body.len()
                        )
                        .into());
                    }
                    self.echo.copy_from_slice(black_box(body));
                    got = Some(t);
                    at += h.frame_len();
                }
                c.rx.plain.drain(..at);
            }
            Ok(got)
        }

        fn echo(&self) -> &[u8; PAYLOAD] {
            &self.echo
        }
    }

    // ── the round trip loop both arms share ──────────────────────────────

    fn fill(payload: &mut [u8; PAYLOAD], seq: u64) {
        payload[..8].copy_from_slice(&seq.to_le_bytes());
        for (i, b) in payload[8..].iter_mut().enumerate() {
            *b = (seq as u8).wrapping_add(i as u8);
        }
    }

    /// `warm + timed` paced round trips on the order connection; every echo
    /// is checked byte for byte, and the timed ones' round trips (send to
    /// decoded, ns) are returned.
    fn run<M: Arm>(arm: &mut M, warm: usize, timed: usize) -> R<Vec<u64>> {
        let mut samples = Vec::with_capacity(timed);
        let mut payload = [0u8; PAYLOAD];
        let mut next = monotonic_ns();
        for i in 0..warm + timed {
            let seq = i as u64;
            fill(&mut payload, seq);
            // The loop spins between round trips, as a live one does.
            loop {
                if arm.pass()?.is_some() {
                    return Err(format!("{}: an echo nobody asked for", M::NAME).into());
                }
                if monotonic_ns() >= next {
                    break;
                }
            }
            let t0 = monotonic_ns();
            arm.send(black_box(&payload))?;
            let give_up = t0 + 1_000_000_000;
            let t1 = loop {
                if let Some(t) = arm.pass()? {
                    break t;
                }
                if monotonic_ns() > give_up {
                    return Err(format!("{}: no echo of {seq} within 1 s", M::NAME).into());
                }
            };
            if black_box(arm.echo()) != &payload {
                return Err(format!("{}: echo {seq} differs from what was sent", M::NAME).into());
            }
            if i >= warm {
                samples.push(u64::try_from(t1 - t0)?);
            }
            next += INTERVAL_NS;
        }
        Ok(samples)
    }

    // ── placement and rounds ─────────────────────────────────────────────

    /// (client loop CPU, echo CPU): the last two CPUs of the cpuset.
    fn cpus() -> R<Option<(usize, usize)>> {
        let status = std::fs::read_to_string("/proc/thread-self/status")?;
        let set = status
            .lines()
            .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
            .map(ergon_io::sys::parse_cpu_list)
            .ok_or("no Cpus_allowed_list")?;
        Ok(match set.as_slice() {
            [.., echo, client] => Some((*client, *echo)),
            _ => None,
        })
    }

    /// Opens `n` connections with `make` against a fresh echo peer, runs
    /// `body`, then closes them and joins the peer.
    fn with_arm<T, M: Arm>(
        n: usize,
        echo_cpu: Option<usize>,
        make: impl FnOnce(&Lab, usize) -> R<M>,
        body: impl FnOnce(&mut M) -> R<T>,
    ) -> R<T> {
        let lab = lab()?;
        std::thread::scope(|s| {
            let peer = s.spawn(|| echo(&lab, n, &[0], echo_cpu));
            let out = make(&lab, n).and_then(|mut arm| body(&mut arm));
            // The arm is gone: the peer sees EOF and returns.
            let echoed = peer.join().map_err(|_| "echo peer panicked")?;
            match (out, echoed) {
                (Ok(v), Ok(_)) => Ok(v),
                (Ok(_), Err(e)) => Err(format!("echo peer: {e}").into()),
                (Err(e), Ok(_)) => Err(e),
                (Err(e), Err(p)) => Err(format!("{e} (echo peer: {p})").into()),
            }
        })
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Kind {
        EpollSpin,
        Mio,
    }

    /// Which arm runs block `b` of a pair: A B B A A B B A ..., starting
    /// with `first`. Short alternating blocks put both arms through the same
    /// host state; a whole round per arm let a VM's CPU frequency or core
    /// type change between them.
    const fn block_kind(first: Kind, b: usize) -> Kind {
        let a = matches!(b % 4, 0 | 3);
        match (first, a) {
            (Kind::EpollSpin, true) | (Kind::Mio, false) => Kind::EpollSpin,
            (Kind::EpollSpin, false) | (Kind::Mio, true) => Kind::Mio,
        }
    }

    /// One A/B pair: both arms open `n` connections each to one echo peer
    /// (the `Driver`'s first, then mio's), warm up, then take turns in
    /// `BLOCKS` blocks of `TIMED / (BLOCKS / 2)` round trips. Returns each
    /// arm's pooled percentiles and the paired difference (`rule`).
    fn pair(n: usize, echo_cpu: usize, first: Kind) -> R<(Pct, Pct, i64)> {
        let lab = lab()?;
        let per_block = TIMED / (BLOCKS / 2);
        std::thread::scope(|s| {
            let peer = s.spawn(|| echo(&lab, 2 * n, &[0, n], Some(echo_cpu)));
            let out = (|| -> R<(Pct, Pct, i64)> {
                let mut d = DriverArm::new(&lab, n)?;
                let mut m = MioArm::new(&lab, n)?;
                run(&mut d, WARMUP, 0)?;
                run(&mut m, WARMUP, 0)?;
                let (mut es, mut ms) = (Vec::with_capacity(TIMED), Vec::with_capacity(TIMED));
                let (mut eb, mut mb) = (
                    Vec::with_capacity(BLOCKS / 2),
                    Vec::with_capacity(BLOCKS / 2),
                );
                for b in 0..BLOCKS {
                    let (mut block, all, p50s) = match block_kind(first, b) {
                        Kind::EpollSpin => (run(&mut d, 0, per_block)?, &mut es, &mut eb),
                        Kind::Mio => (run(&mut m, 0, per_block)?, &mut ms, &mut mb),
                    };
                    all.extend_from_slice(&block);
                    p50s.push(percentiles(&mut block).ok_or("empty block")?.p50);
                }
                Ok((
                    percentiles(&mut es).ok_or("no samples")?,
                    percentiles(&mut ms).ok_or("no samples")?,
                    paired_diff_ns(&eb, &mb).ok_or("no sub-pairs")?,
                ))
            })();
            // Both arms are gone: the peer sees EOF and returns.
            let echoed = peer.join().map_err(|_| "echo peer panicked")?;
            match (out, echoed) {
                (Ok(v), Ok(_)) => Ok(v),
                (Ok(_), Err(e)) => Err(format!("echo peer: {e}").into()),
                (Err(e), Ok(_)) => Err(e),
                (Err(e), Err(p)) => Err(format!("{e} (echo peer: {p})").into()),
            }
        })
    }

    fn us(ns: u64) -> f64 {
        ns as f64 / 1_000.0
    }

    fn parity(n: usize) -> R {
        let (client, echo_cpu) = cpus()?.ok_or(
            "the parity gate needs two CPUs: one for the client loop, one for the echo peer",
        )?;
        pin_current_thread(client)?;
        println!(
            "mio parity, N = {n}: {PAIRS} pairs; per pair and arm {WARMUP} warm-up + {TIMED} timed round trips \
             in {} alternating blocks; {PAYLOAD} B at {}/s, client CPU {client}, echo CPU {echo_cpu}; \
             µs p50 / p99 / max",
            BLOCKS / 2,
            1_000_000_000 / INTERVAL_NS
        );
        let mut pairs = Vec::with_capacity(PAIRS);
        for i in 0..PAIRS {
            let first = if i % 2 == 0 {
                Kind::EpollSpin
            } else {
                Kind::Mio
            };
            let (e, m, diff) = pair(n, echo_cpu, first)?;
            let tol = tolerance_ns(e.p50, m.p50);
            println!(
                "pair {i} ({}): epoll-spin {:.2} / {:.2} / {:.1}  mio {:.2} / {:.2} / {:.1}  \
                 mio - epoll-spin p50, paired {:+.2} (pooled {:+.2}, tolerance {:.2})",
                if i % 2 == 0 {
                    "epoll-spin first"
                } else {
                    "mio first"
                },
                us(e.p50),
                us(e.p99),
                us(e.max),
                us(m.p50),
                us(m.p99),
                us(m.max),
                diff as f64 / 1_000.0,
                (m.p50 as f64 - e.p50 as f64) / 1_000.0,
                us(tol),
            );
            pairs.push(Pair {
                mio_minus_epoll: diff,
                epoll_p50: e.p50,
                mio_p50: m.p50,
            });
        }
        let v = verdict(&pairs);
        println!("N = {n}: {v:?}");
        match v {
            Verdict::Level { .. } => Ok(()),
            Verdict::EpollAhead(k) => Err(format!(
                "N = {n}: epoll-spin ahead of mio beyond the tolerance in {k} of {PAIRS} pairs: \
                 a suspected defect (the mio loop doing extra work, or the Driver timing less)"
            )
            .into()),
            Verdict::MioAhead(k) => Err(format!(
                "N = {n}: mio ahead of epoll-spin beyond the tolerance in {k} of {PAIRS} pairs: \
                 the Driver has drifted from mio's mechanism or added per-poll cost"
            )
            .into()),
        }
    }

    #[test]
    fn round_trip_parity_one_socket() -> R {
        parity(1)
    }

    #[test]
    fn round_trip_parity_sixteen_sockets() -> R {
        parity(16)
    }

    // ── structure ────────────────────────────────────────────────────────

    /// Round trips counted after the warm-up (which takes the server's
    /// session tickets).
    const COUNTED: u64 = 2_000;
    /// Idle polls timed: at a 1 ms timeout, the shortest `epoll_wait` can
    /// block, they would take 20 s.
    const IDLE_POLLS: u64 = 20_000;

    #[test]
    fn epoll_spin_waits_with_a_zero_timeout_and_reads_once_per_ready_socket() -> R {
        let placement = cpus()?;
        if let Some((client, _)) = placement {
            pin_current_thread(client)?;
        }
        let echo_cpu = placement.map(|(_, e)| e);
        for n in [1, 16] {
            with_arm(n, echo_cpu, DriverArm::new, |a| {
                run(a, 500, 0)?;
                let s0 = a.stats();
                run(a, 0, usize::try_from(COUNTED)?)?;
                let s1 = a.stats();
                let polls = s1.polls - s0.polls;
                let recvmsg = s1.recvmsg_calls - s0.recvmsg_calls;
                println!(
                    "epoll-spin N = {n}: {COUNTED} round trips, {polls} polls, {} epoll_wait, \
                     {recvmsg} recvmsg ({} EAGAIN)",
                    s1.epoll_wait_calls - s0.epoll_wait_calls,
                    s1.recvmsg_eagain - s0.recvmsg_eagain
                );
                assert_eq!(
                    s1.epoll_wait_calls - s0.epoll_wait_calls,
                    polls,
                    "one epoll_wait per poll"
                );
                assert_eq!(
                    recvmsg, COUNTED,
                    "one recvmsg per ready socket, one ready socket per echo"
                );
                assert_eq!(
                    s1.recvmsg_eagain, s0.recvmsg_eagain,
                    "level-triggered: no read to EAGAIN"
                );

                let t = Instant::now();
                for _ in 0..IDLE_POLLS {
                    if a.pass()?.is_some() {
                        return Err("an echo nobody asked for".into());
                    }
                }
                let took = t.elapsed();
                let s2 = a.stats();
                println!(
                    "epoll-spin N = {n}: {IDLE_POLLS} idle polls in {took:?}, {} epoll_wait, {} recvmsg",
                    s2.epoll_wait_calls - s1.epoll_wait_calls,
                    s2.recvmsg_calls - s1.recvmsg_calls
                );
                assert_eq!(s2.epoll_wait_calls - s1.epoll_wait_calls, IDLE_POLLS);
                assert_eq!(
                    s2.recvmsg_calls, s1.recvmsg_calls,
                    "nothing ready, nothing read"
                );
                assert!(
                    took < Duration::from_secs(2),
                    "{IDLE_POLLS} idle polls took {took:?}: epoll_wait is blocking (timeout above 0)"
                );
                Ok(())
            })?;
        }
        Ok(())
    }

    #[test]
    fn mio_reads_twice_per_event_data_then_would_block() -> R {
        let placement = cpus()?;
        if let Some((client, _)) = placement {
            pin_current_thread(client)?;
        }
        let echo_cpu = placement.map(|(_, e)| e);
        for n in [1, 16] {
            with_arm(n, echo_cpu, MioArm::new, |a| {
                run(a, 500, 0)?;
                let (r0, w0) = (a.reads, a.would_block);
                run(a, 0, usize::try_from(COUNTED)?)?;
                let (reads, wb) = (a.reads - r0, a.would_block - w0);
                println!(
                    "mio N = {n}: {COUNTED} round trips, {reads} reads ({wb} WouldBlock): {:.1}x epoll-spin's",
                    reads as f64 / COUNTED as f64
                );
                assert_eq!(
                    reads,
                    2 * COUNTED,
                    "edge-triggered: the data, then the read to WouldBlock"
                );
                assert_eq!(wb, COUNTED);
                Ok(())
            })?;
        }
        Ok(())
    }
}
