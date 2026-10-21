//! Lab sender: a TLS WebSocket server for the two-host benchmark.
//!
//! ```text
//! replay-server --listen 0.0.0.0:9443 --cert-out cert.pem --clients 2 --mode replay --capture capture.bin --speed 1 --duration 600
//! replay-server --listen 0.0.0.0:9443 --cert-out cert.pem --clients 2 --mode burst --burst 50 --gap-us 1000 --duration 600
//! replay-server --listen 0.0.0.0:9443 --cert-out cert.pem --clients 100 --mode mixed --duration 600 \
//!     [--streams 1024 --trickle-min-us 1000 --trickle-max-us 10000 --burst-every-us 2000 --burst-slots 10 --burst 10 --seed 42]
//! replay-server --listen 0.0.0.0:9443 --cert-out cert.pem --mode echo [--threads one]
//! replay-server --listen 0.0.0.0:9445 --cert-out cert.pem --mode http [--threads one] [--duration 600]
//! ```
//!
//! `replay` and `burst` send **every message to every client**, prefixed with
//! `seq u64 | send_ns i64 | capture_ns i64`, alternating which client is written
//! first on even and odd sequence numbers so that write order cancels out in the
//! paired comparison. `mixed` gives each connection its own stream: a client
//! names its slot in its upgrade path (`/slot/<n>`), and every slot gets a
//! trickle at its own random interval plus, every few milliseconds, bursts to a
//! random subset of slots (`io_bench::mixed`, the same for a seed); a message
//! goes to every client of its slot. With `--streams N` the schedule has N
//! streams sharing the connections as on a combined-stream socket (stream s on
//! connection s mod connections); the stream number is sent in place of the
//! capture time. In `mixed`, a client that names no slot (an order connection)
//! is echoed: between any two sends, and while waiting for the next, the one
//! thread answers its frames, so one order connection and N-1 feeds share a
//! receiving loop as an application's would (with no feed client, it is only
//! echoed). `echo` answers each client frame with the same payload.
//! `http` is a keep-alive HTTP/1.1 server answering every request with a fixed
//! JSON body the size of a Binance order acknowledgement.
//! `echo` and `http` serve each connection on a thread of its own (all on the
//! `--cpu`), or with `--threads one` every connection from one thread: then a
//! pool of many connections measures the client, not the server's threads
//! sharing its CPU. That thread polls one socket per NIC queue (with
//! `--busy-poll`, which polls that queue) and the rest through epoll.
//! The server side is not what is being measured; it busy-spins to keep its own
//! jitter low and identical for every receiver. `--busy-poll US` sets
//! `SO_BUSY_POLL` on every accepted socket, so each read polls the NIC queue
//! itself instead of waiting for an interrupt on another CPU (which may be
//! waking from a deep idle state). With `--stalls-dir DIR` the
//! echo and HTTP loops log their slow iterations (`DIR/server-<mode>-<port>.txt`
//! per connection, per session of connections with `--threads one`, and for
//! the order connection of `mixed` as `server-echo-<port>.txt`), so a slow
//! round trip can be traced to the server.

use std::error::Error;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use io_bench::args::Args;
use io_bench::clock::{monotonic_ns, pin_current_thread, realtime_ns};
use io_bench::mixed::{Schedule, Shape};
use io_bench::record::parse_capture;
use io_bench::stalls::StallLog;
use io_bench::ws;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection};

type BoxResult<T> = Result<T, Box<dyn Error>>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("replay-server: {e}");
            ExitCode::FAILURE
        }
    }
}

struct Client {
    sock: TcpStream,
    tls: ServerConnection,
    /// The slot named in the upgrade path (`/slot/<n>`), if any.
    slot: Option<usize>,
}

impl Client {
    fn write_all_tls(&mut self) -> std::io::Result<()> {
        while self.tls.wants_write() {
            match self.tls.write_tls(&mut self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => std::hint::spin_loop(),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

fn run() -> BoxResult<()> {
    let a = Args::from_env();
    if let Some(cpu) = a.list("cpu", "").first() {
        pin_current_thread(cpu.parse()?)?;
    }
    let names = vec![a.str_or("sni", "lab.local")];
    let rcgen::CertifiedKey { cert, signing_key } = rcgen::generate_simple_self_signed(names)?;
    std::fs::write(a.str_or("cert-out", "cert.pem"), cert.pem())?;
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(signing_key.serialize_der()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let cfg = Arc::new(
        ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(vec![CertificateDer::from(cert.der().to_vec())], key)?,
    );
    let listener = TcpListener::bind(a.str_or("listen", "0.0.0.0:9443"))?;
    let mode = a.str_or("mode", "replay");
    eprintln!("listening on {} mode={mode}", listener.local_addr()?);
    let stalls_dir = a
        .list("stalls-dir", "")
        .first()
        .map(std::path::PathBuf::from);
    let stall_us: f64 = a.num_or("stall-us", 20.0)?;
    let busy_poll_us: i32 = a.num_or("busy-poll", 0)?;
    if (mode == "http" || mode == "echo") && a.str_or("threads", "per-conn") == "one" {
        let end = a
            .list("duration", "")
            .first()
            .map(|d| d.parse::<i64>().map(|s| monotonic_ns() + s * 1_000_000_000))
            .transpose()?;
        let opts = OneThread {
            http: mode == "http",
            busy_poll_us,
            stalls_dir: stalls_dir.clone(),
            stall_us,
            end_mono_ns: end,
        };
        eprintln!("serving every connection from one thread");
        return serve_one_thread(listener, &cfg, &opts);
    }
    if mode == "http" || mode == "echo" {
        loop {
            let (sock, peer) = listener.accept()?;
            set_busy_poll(&sock, busy_poll_us)?;
            let cfg = cfg.clone();
            let dir = stalls_dir.clone();
            let mode = mode.clone();
            std::thread::spawn(move || {
                let mut log = StallLog::new(stall_us, 1 << 20);
                let r = if mode == "http" {
                    accept_tls(sock, &cfg).and_then(|c| http(c, &mut log))
                } else {
                    accept(sock, &cfg).and_then(|c| echo(c, &mut log))
                };
                if let Err(e) = r {
                    eprintln!("{mode} {peer}: {e}");
                }
                if let Some(d) = dir {
                    let path = d.join(format!("server-{mode}-{}.txt", peer.port()));
                    if let Err(e) = log.write(&path) {
                        eprintln!("{}: {e}", path.display());
                    }
                }
            });
        }
    }

    let n: usize = a.num_or("clients", 2)?;
    let mut clients = Vec::new();
    while clients.len() < n {
        let (sock, peer) = listener.accept()?;
        set_busy_poll(&sock, busy_poll_us)?;
        clients.push(accept(sock, &cfg)?);
        eprintln!("client {} connected from {peer}", clients.len() - 1);
    }
    for c in &clients {
        c.sock.set_nonblocking(true)?;
    }
    // `mixed`: the clients that name no slot are order connections, echoed.
    let mut echo: Vec<EchoState> = Vec::new();
    if mode == "mixed" {
        let (feeds, orders): (Vec<Client>, Vec<Client>) =
            clients.into_iter().partition(|c| c.slot.is_some());
        clients = feeds;
        echo = orders.into_iter().map(EchoState::new).collect();
    }
    let mut echo_log = StallLog::new(stall_us, 1 << 20);
    echo_log.start(0);
    let duration_ns: i64 = a.num_or::<i64>("duration", 600)? * 1_000_000_000;
    let start = monotonic_ns();
    let end = start + duration_ns;
    let mut frame = Vec::with_capacity(1 << 20);
    let mut seq: u64 = 0;
    // To every client (`only` = None, then pongs are drained), or to the given ones.
    let mut send = |clients: &mut Vec<Client>,
                    only: Option<&[usize]>,
                    payload: &[u8],
                    capture_ns: i64,
                    seq: u64|
     -> BoxResult<()> {
        frame.clear();
        let send_ns = realtime_ns();
        let mut body = Vec::with_capacity(24 + payload.len());
        body.extend_from_slice(&seq.to_le_bytes());
        body.extend_from_slice(&send_ns.to_le_bytes());
        body.extend_from_slice(&capture_ns.to_le_bytes());
        body.extend_from_slice(payload);
        frame.resize(ws::header_len(body.len(), false) + body.len(), 0);
        ws::encode_frame(ws::OP_BINARY, &body, None, &mut frame)?;
        let mut order: Vec<usize> =
            only.map_or_else(|| (0..clients.len()).collect(), <[usize]>::to_vec);
        if !seq.is_multiple_of(2) {
            order.reverse();
        }
        for i in order {
            let c = &mut clients[i];
            c.tls.writer().write_all(&frame)?;
            if let Err(e) = c.write_all_tls() {
                // A receiver finished its run; stop rather than skew the others.
                return Err(format!("client {i} gone after {seq} messages: {e}").into());
            }
        }
        if only.is_none() {
            drain_inputs(clients);
        }
        Ok(())
    };

    match mode.as_str() {
        "replay" => {
            let bytes = std::fs::read(a.req("capture")?)?;
            let msgs = parse_capture(&bytes);
            if msgs.is_empty() {
                return Err("empty capture".into());
            }
            let speed: f64 = a.num_or("speed", 1.0)?;
            'outer: loop {
                let t0 = msgs[0].0;
                let base = monotonic_ns();
                for &(t, payload) in &msgs {
                    let due = base + ((t - t0) as f64 / speed) as i64;
                    while monotonic_ns() < due {
                        std::hint::spin_loop();
                    }
                    if monotonic_ns() >= end {
                        break 'outer;
                    }
                    seq += 1;
                    if let Err(e) = send(&mut clients, None, payload, t, seq) {
                        eprintln!("stopping: {e}");
                        break 'outer;
                    }
                }
            }
        }
        "burst" => {
            // Without a capture: one synthetic trade message of Binance's size.
            let bytes = match a.list("capture", "").first() {
                Some(p) => std::fs::read(p)?,
                None => synthetic_capture(),
            };
            let msgs = parse_capture(&bytes);
            if msgs.is_empty() {
                return Err("empty capture".into());
            }
            let burst: usize = a.num_or("burst", 50)?;
            let gap_ns: i64 = a.num_or::<i64>("gap-us", 1000)? * 1000;
            let mut k = 0usize;
            let mut next = monotonic_ns();
            while monotonic_ns() < end {
                while monotonic_ns() < next {
                    std::hint::spin_loop();
                }
                for _ in 0..burst {
                    let (t, payload) = msgs[k % msgs.len()];
                    k += 1;
                    seq += 1;
                    if let Err(e) = send(&mut clients, None, payload, t, seq) {
                        eprintln!("stopping: {e}");
                        return Ok(());
                    }
                }
                next += gap_ns;
            }
        }
        "mixed" => {
            let slots = clients
                .iter()
                .filter_map(|c| c.slot)
                .max()
                .map_or(0, |m| m + 1);
            if slots == 0 && echo.is_empty() {
                return Err("mixed: no client named a slot (/slot/<n>)".into());
            }
            if slots == 0 {
                // Only order connections: echoed until the end.
                while monotonic_ns() < end {
                    if let Err(e) = serve_echo(&mut echo, &mut echo_log) {
                        eprintln!("stopping: {e}");
                        break;
                    }
                }
            }
            // Streams (one schedule slot each) share the connections like a
            // combined-stream socket: stream s goes to connection s mod slots.
            let streams: usize = a.num_or("streams", slots)?;
            let mut groups: Vec<Vec<usize>> = vec![Vec::new(); slots];
            for (i, c) in clients.iter().enumerate() {
                if let Some(s) = c.slot {
                    groups[s].push(i);
                }
            }
            let shape = Shape {
                slots: streams.max(1),
                trickle_min_us: a.num_or("trickle-min-us", 1_000)?,
                trickle_max_us: a.num_or("trickle-max-us", 10_000)?,
                burst_every_us: a.num_or("burst-every-us", 2_000)?,
                burst_slots: a.num_or("burst-slots", 10)?,
                burst_len: a.num_or("burst", 10)?,
                seed: a.num_or("seed", 42)?,
            };
            let bytes = synthetic_capture();
            let msgs = parse_capture(&bytes);
            let &(_, payload) = msgs.first().ok_or("empty capture")?;
            let mut last_drain = start;
            // How far behind its schedule the sender fell: traffic sent late
            // is not the traffic planned (a saturated sender also slows down
            // with a slow receiver). Measured on the first message of each
            // due time: a burst's later messages wait for its earlier ones
            // by design.
            let (mut batches, mut late_100us, mut late_max_ns, mut last_due) =
                (0u64, 0u64, 0i64, -1i64);
            for m in Schedule::new(shape).take_while(|_| slots > 0) {
                let due = start + m.at_ns;
                if due >= end {
                    break;
                }
                while monotonic_ns() < due {
                    if echo.is_empty() {
                        std::hint::spin_loop();
                    } else if let Err(e) = serve_echo(&mut echo, &mut echo_log) {
                        eprintln!("stopping: {e}");
                        break;
                    }
                }
                if echo.iter().any(|e| e.closed) {
                    break;
                }
                if due != last_due {
                    let late = monotonic_ns() - due;
                    late_max_ns = late_max_ns.max(late);
                    late_100us += u64::from(late > 100_000);
                    batches += 1;
                    last_due = due;
                }
                seq += 1;
                // The stream rides in the capture-time field (`src2_ns`).
                let stream = i64::try_from(m.slot).unwrap_or(0);
                if let Err(e) = send(
                    &mut clients,
                    Some(&groups[m.slot % slots]),
                    payload,
                    stream,
                    seq,
                ) {
                    eprintln!("stopping: {e}");
                    break;
                }
                // The order connection waits for at most one send.
                if !echo.is_empty()
                    && let Err(e) = serve_echo(&mut echo, &mut echo_log)
                {
                    eprintln!("stopping: {e}");
                    break;
                }
                // The clients send nothing but pongs and closes: drain now and then.
                if monotonic_ns() - last_drain > 10_000_000 {
                    drain_inputs(&mut clients);
                    last_drain = monotonic_ns();
                }
            }
            if slots > 0 {
                eprintln!(
                    "schedule: sent={seq} batches={batches} late_100us={late_100us} late_max_us={}",
                    late_max_ns / 1000
                );
            }
        }
        other => return Err(format!("unknown mode {other}").into()),
    }
    eprintln!("sent {seq} messages to {} clients", clients.len());
    if !echo.is_empty() {
        eprintln!(
            "echoed {} frames on {} order connection(s)",
            echo.iter().map(|e| e.echoed).sum::<u64>(),
            echo.len()
        );
        if let (Some(d), Some(e)) = (&stalls_dir, echo.first()) {
            let port = e.c.sock.peer_addr().map_or(0, |p| p.port());
            write_stalls(&echo_log, d, "echo", port);
        }
    }
    for c in clients.iter_mut().chain(echo.iter_mut().map(|e| &mut e.c)) {
        c.tls.send_close_notify();
        let _ = c.write_all_tls();
    }
    Ok(())
}

/// Writes a stall log to `dir/server-<mode>-<port>.txt`, reporting a failure.
fn write_stalls(log: &StallLog, dir: &Path, mode: &str, port: u16) {
    let path = dir.join(format!("server-{mode}-{port}.txt"));
    if let Err(e) = log.write(&path) {
        eprintln!("{}: {e}", path.display());
    }
}

/// Answers every order connection's frames until none has more; ticks `log`
/// once per call, so its gaps are how long an order frame could wait.
fn serve_echo(echo: &mut [EchoState], log: &mut StallLog) -> BoxResult<()> {
    log.tick(echo.iter().map(|e| e.echoed).sum());
    for e in echo {
        loop {
            match e.step() {
                Ok(Step::Progress) => {}
                Ok(Step::Idle) => break,
                Ok(Step::Closed) => {
                    e.closed = true;
                    return Err("an order connection closed".into());
                }
                Err(err) => {
                    e.closed = true;
                    return Err(err);
                }
            }
        }
    }
    Ok(())
}

/// What one non-blocking step of a served connection did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// It read something (there may be more).
    Progress,
    /// Nothing to read.
    Idle,
    /// The peer closed it.
    Closed,
}

/// An echoed WebSocket connection: one step reads once, then answers every
/// complete client frame with the same payload.
struct EchoState {
    c: Client,
    plain: Vec<u8>,
    tmp: Vec<u8>,
    out: Vec<u8>,
    echoed: u64,
    closed: bool,
}

impl EchoState {
    fn new(c: Client) -> Self {
        Self {
            c,
            plain: Vec::with_capacity(1 << 20),
            tmp: vec![0u8; 1 << 16],
            out: vec![0u8; 1 << 16],
            echoed: 0,
            closed: false,
        }
    }

    fn step(&mut self) -> BoxResult<Step> {
        match self.c.tls.read_tls(&mut self.c.sock) {
            Ok(0) => return Ok(Step::Closed),
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(Step::Idle),
            Err(e) => return Err(e.into()),
        }
        self.c.tls.process_new_packets()?;
        loop {
            match self.c.tls.reader().read(&mut self.tmp) {
                Ok(0) => return Ok(Step::Closed),
                Ok(n) => self.plain.extend_from_slice(&self.tmp[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        let mut consumed = 0;
        while let Some(h) = ws::parse_header(&self.plain[consumed..])? {
            if self.plain.len() - consumed < h.frame_len() {
                break;
            }
            let a = consumed + h.header_len;
            let b = consumed + h.frame_len();
            if let Some(m) = h.mask {
                ws::apply_mask(&mut self.plain[a..b], m);
            }
            match h.opcode {
                ws::OP_CLOSE => return Ok(Step::Closed),
                ws::OP_TEXT | ws::OP_BINARY => {
                    let n = ws::encode_frame(h.opcode, &self.plain[a..b], None, &mut self.out)?;
                    self.c.tls.writer().write_all(&self.out[..n])?;
                    self.c.write_all_tls()?;
                    self.echoed += 1;
                }
                _ => {}
            }
            consumed = b;
        }
        self.plain.drain(..consumed);
        Ok(Step::Progress)
    }
}

/// A keep-alive HTTP/1.1 connection: one step answers every complete request
/// already decrypted with [`ORDER_ACK`], then reads once.
struct HttpState {
    c: Client,
    plain: Vec<u8>,
    tmp: Vec<u8>,
    resp: Vec<u8>,
    served: u64,
}

impl HttpState {
    fn new(c: Client) -> Self {
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json;charset=UTF-8\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
            ORDER_ACK.len()
        );
        let mut resp = head.into_bytes();
        resp.extend_from_slice(ORDER_ACK);
        Self {
            c,
            plain: Vec::with_capacity(1 << 20),
            tmp: vec![0u8; 1 << 16],
            resp,
            served: 0,
        }
    }

    fn step(&mut self) -> BoxResult<Step> {
        // Plaintext rustls already holds first (it can arrive with the handshake).
        loop {
            match self.c.tls.reader().read(&mut self.tmp) {
                Ok(0) => return Ok(Step::Closed),
                Ok(n) => self.plain.extend_from_slice(&self.tmp[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        let mut answered = false;
        loop {
            let mut hdrs = [httparse::EMPTY_HEADER; 32];
            let mut req = httparse::Request::new(&mut hdrs);
            let httparse::Status::Complete(n) = req.parse(&self.plain)? else {
                break;
            };
            let len = req
                .headers
                .iter()
                .find(|h| h.name.eq_ignore_ascii_case("content-length"))
                .and_then(|h| {
                    std::str::from_utf8(h.value)
                        .ok()?
                        .trim()
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or(0);
            if self.plain.len() < n + len {
                break;
            }
            self.plain.drain(..n + len);
            self.c.tls.writer().write_all(&self.resp)?;
            answered = true;
            self.served += 1;
        }
        if answered {
            self.c.write_all_tls()?;
        }
        match self.c.tls.read_tls(&mut self.c.sock) {
            Ok(0) => Ok(Step::Closed),
            Ok(_) => {
                self.c.tls.process_new_packets()?;
                Ok(Step::Progress)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => Ok(Step::Idle),
            Err(e) => Err(e.into()),
        }
    }
}

/// One served connection of the one-thread server.
enum Served {
    Echo(EchoState),
    Http(HttpState),
}

impl Served {
    fn step(&mut self) -> BoxResult<Step> {
        match self {
            Self::Echo(e) => e.step(),
            Self::Http(h) => h.step(),
        }
    }
    fn client(&self) -> &Client {
        match self {
            Self::Echo(e) => &e.c,
            Self::Http(h) => &h.c,
        }
    }
    fn count(&self) -> u64 {
        match self {
            Self::Echo(e) => e.echoed,
            Self::Http(h) => h.served,
        }
    }
}

/// `--threads one`: what the server does and for how long.
struct OneThread {
    http: bool,
    busy_poll_us: i32,
    stalls_dir: Option<PathBuf>,
    stall_us: f64,
    end_mono_ns: Option<i64>,
}

/// The NIC queue (NAPI id) a socket's packets last came in on; 0 when
/// unknown (no packet yet, or not Linux).
fn napi_id(sock: &TcpStream) -> u32 {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        ergon_io::sys::incoming_napi_id(sock.as_raw_fd()).unwrap_or(0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = sock;
        0
    }
}

/// Every connection from this one thread: accepts as they come (the TLS
/// handshake blocks, before a client measures), then per pass reads one
/// socket of each NIC queue (with `--busy-poll`, that read polls the queue),
/// then serves every socket epoll reports readable, each until it has
/// nothing more. A session's stall log (connections opened until all are
/// closed) is written when its last connection closes.
fn serve_one_thread(
    listener: TcpListener,
    cfg: &Arc<ServerConfig>,
    o: &OneThread,
) -> BoxResult<()> {
    const LISTENER: mio::Token = mio::Token(usize::MAX);
    listener.set_nonblocking(true)?;
    let mut listener = mio::net::TcpListener::from_std(listener);
    let mut poll = mio::Poll::new()?;
    poll.registry()
        .register(&mut listener, LISTENER, mio::Interest::READABLE)?;
    let mut events = mio::Events::with_capacity(1024);
    let mut conns: Vec<Option<(mio::net::TcpStream, Served, u32)>> = Vec::new();
    let mode = if o.http { "http" } else { "echo" };
    let mut log = StallLog::new(o.stall_us, 1 << 20);
    let mut session_port: Option<u16> = None;
    let mut done: u64 = 0;
    log.start(0);
    loop {
        if o.end_mono_ns.is_some_and(|e| monotonic_ns() >= e) {
            return Ok(());
        }
        let count: u64 = done
            + conns
                .iter()
                .flatten()
                .map(|(_, s, _)| s.count())
                .sum::<u64>();
        log.tick(count);
        // One read per NIC queue: polls the queue for every socket on it.
        let mut closed: Vec<usize> = Vec::new();
        if o.busy_poll_us > 0 {
            let mut seen: Vec<u32> = Vec::new();
            for (i, c) in conns.iter_mut().enumerate() {
                let Some((_, s, napi)) = c else { continue };
                if seen.contains(napi) {
                    continue;
                }
                seen.push(*napi);
                if serve(s)? {
                    closed.push(i);
                }
            }
        }
        poll.poll(&mut events, Some(Duration::ZERO))?;
        for ev in &events {
            if ev.token() == LISTENER {
                accept_all(&listener, &poll, cfg, o, &mut conns)?;
                if session_port.is_none() {
                    session_port = conns
                        .iter()
                        .flatten()
                        .find_map(|(_, s, _)| s.client().sock.peer_addr().ok().map(|p| p.port()));
                }
                continue;
            }
            let i = ev.token().0;
            if let Some(Some((_, s, _))) = conns.get_mut(i)
                && serve(s)?
            {
                closed.push(i);
            }
        }
        closed.sort_unstable();
        closed.dedup();
        for i in closed {
            if let Some((mut sock, s, _)) = conns.get_mut(i).and_then(Option::take) {
                done += s.count();
                let _ = poll.registry().deregister(&mut sock);
            }
        }
        if conns.iter().all(Option::is_none) && !conns.is_empty() {
            // The session is over: its stall log, then a fresh one.
            if let (Some(d), Some(p)) = (&o.stalls_dir, session_port) {
                write_stalls(&log, d, mode, p);
            }
            eprintln!(
                "{mode}: session of {} connection(s) over, {done} answered",
                conns.len()
            );
            conns.clear();
            session_port = None;
            done = 0;
            log = StallLog::new(o.stall_us, 1 << 20);
            log.start(0);
        }
    }
}

/// Serves `s` until it has nothing more to read; true when it closed.
fn serve(s: &mut Served) -> BoxResult<bool> {
    loop {
        match s.step() {
            Ok(Step::Progress) => {}
            Ok(Step::Idle) => return Ok(false),
            Ok(Step::Closed) => return Ok(true),
            // A reset connection ends its session, not the server.
            Err(e) => {
                eprintln!("connection: {e}");
                return Ok(true);
            }
        }
    }
}

/// Accepts every waiting connection: busy poll set, TLS (and for echo the
/// WebSocket upgrade) completed blocking, then non-blocking and registered.
fn accept_all(
    listener: &mio::net::TcpListener,
    poll: &mio::Poll,
    cfg: &Arc<ServerConfig>,
    o: &OneThread,
    conns: &mut Vec<Option<(mio::net::TcpStream, Served, u32)>>,
) -> BoxResult<()> {
    loop {
        let (sock, peer) = match listener.accept() {
            Ok(x) => x,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let sock = TcpStream::from(sock);
        sock.set_nonblocking(false)?;
        set_busy_poll(&sock, o.busy_poll_us)?;
        let client = if o.http {
            accept_tls(sock, cfg)
        } else {
            accept(sock, cfg)
        };
        let client = match client {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{peer}: {e}");
                continue;
            }
        };
        client.sock.set_nonblocking(true)?;
        let napi = napi_id(&client.sock);
        let mut reg = mio::net::TcpStream::from_std(client.sock.try_clone()?);
        let i = conns
            .iter()
            .position(Option::is_none)
            .unwrap_or(conns.len());
        poll.registry()
            .register(&mut reg, mio::Token(i), mio::Interest::READABLE)?;
        let served = if o.http {
            Served::Http(HttpState::new(client))
        } else {
            Served::Echo(EchoState::new(client))
        };
        if i == conns.len() {
            conns.push(Some((reg, served, napi)));
        } else {
            conns[i] = Some((reg, served, napi));
        }
    }
}

/// A one-message capture: a Binance spot trade event (about 200 bytes) in the
/// capture format (`capture_ns i64 | len u32 | payload`).
fn synthetic_capture() -> Vec<u8> {
    let payload: &[u8] = br#"{"stream":"btcusdt@trade","data":{"e":"trade","E":1760400000000000,"s":"BTCUSDT","t":5123456789,"p":"62431.17000000","q":"0.00120000","T":1760400000000000,"m":true,"M":true}}"#;
    let mut out = Vec::with_capacity(12 + payload.len());
    out.extend_from_slice(&0i64.to_le_bytes());
    out.extend_from_slice(&u32::try_from(payload.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Reads and discards whatever the clients sent (pongs), without blocking.
fn drain_inputs(clients: &mut [Client]) {
    for c in clients {
        loop {
            match c.tls.read_tls(&mut c.sock) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let _ = c.tls.process_new_packets();
                    let mut sink = [0u8; 4096];
                    while let Ok(n) = c.tls.reader().read(&mut sink) {
                        if n == 0 {
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// TLS handshake plus WebSocket upgrade (blocking).
/// Sets `SO_BUSY_POLL` to `us` on `sock` (nothing when 0, or off Linux).
fn set_busy_poll(sock: &TcpStream, us: i32) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    if us > 0 {
        use std::os::fd::AsRawFd;
        let len = libc::socklen_t::try_from(std::mem::size_of::<i32>()).unwrap_or(4);
        // SAFETY: a valid descriptor, and an int option of the given length.
        let r = unsafe {
            libc::setsockopt(
                sock.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_BUSY_POLL,
                (&raw const us).cast(),
                len,
            )
        };
        if r != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (sock, us);
    Ok(())
}

fn accept(mut sock: TcpStream, cfg: &Arc<ServerConfig>) -> BoxResult<Client> {
    sock.set_nodelay(true)?;
    let mut tls = ServerConnection::new(cfg.clone())?;
    while tls.is_handshaking() {
        tls.complete_io(&mut sock)?;
    }
    let mut req = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        // Drain plaintext rustls already holds first: the upgrade request often
        // arrives in the same read as the client's Finished, inside complete_io.
        loop {
            match tls.reader().read(&mut buf) {
                Ok(0) => break,
                Ok(n) => req.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        if let Some((_, key, path)) = ws::parse_server_request(&req)? {
            tls.writer().write_all(&ws::server_response(&key))?;
            while tls.wants_write() {
                tls.write_tls(&mut sock)?;
            }
            let slot = path.strip_prefix("/slot/").and_then(|n| n.parse().ok());
            return Ok(Client { sock, tls, slot });
        }
        if tls.read_tls(&mut sock)? == 0 {
            return Err("client closed during upgrade".into());
        }
        tls.process_new_packets()?;
    }
}

/// TLS handshake only (HTTP mode).
fn accept_tls(mut sock: TcpStream, cfg: &Arc<ServerConfig>) -> BoxResult<Client> {
    sock.set_nodelay(true)?;
    let mut tls = ServerConnection::new(cfg.clone())?;
    while tls.is_handshaking() {
        tls.complete_io(&mut sock)?;
    }
    Ok(Client {
        sock,
        tls,
        slot: None,
    })
}

/// The response body: the size and shape of a Binance `newOrderRespType=ACK`
/// answer plus padding to about 300 bytes.
const ORDER_ACK: &[u8] = br#"{"symbol":"BTCUSDT","orderId":28457911234,"orderListId":-1,"clientOrderId":"x-ergon-bench-0000000001","transactTime":1760400000000,"price":"0.00000000","origQty":"0.00100000","executedQty":"0.00000000","status":"NEW","timeInForce":"GTC","type":"LIMIT","side":"BUY","workingTime":1760400000000}"#;

/// Answers every HTTP/1.1 request with [`ORDER_ACK`], busy-spinning.
fn http(c: Client, log: &mut StallLog) -> BoxResult<()> {
    c.sock.set_nonblocking(true)?;
    let mut h = HttpState::new(c);
    log.start(0);
    loop {
        log.tick(h.served);
        match h.step()? {
            Step::Closed => return Ok(()),
            Step::Idle => std::hint::spin_loop(),
            Step::Progress => {}
        }
    }
}

/// Echoes every client frame back as a server frame, busy-spinning.
fn echo(c: Client, log: &mut StallLog) -> BoxResult<()> {
    c.sock.set_nonblocking(true)?;
    let mut e = EchoState::new(c);
    log.start(0);
    loop {
        log.tick(e.echoed);
        match e.step()? {
            Step::Closed => return Ok(()),
            Step::Idle => std::hint::spin_loop(),
            Step::Progress => {}
        }
    }
}
