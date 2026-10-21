//! The lab measurements written the way most Rust code would: mio for
//! readiness (epoll on Linux, edge-triggered), rustls's standard buffered
//! `ClientConnection`, and a loop that busy-spins on `Poll::poll` with a zero
//! timeout. It is the baseline for ergon-io's own loops: same server, same TLS
//! configuration, same records.
//!
//! ```text
//! mio_bench lab --server ADDR --ca cert.pem --sni lab.local --conns 100 --cpu 3 --duration 60 --out DIR
//! mio_bench pingpong --server ADDR --ca cert.pem --sni lab.local --cpu 3 --rate 10000 --duration 120 --out DIR [--feeds K]
//! mio_bench http --server ADDR --ca cert.pem --sni lab.local --cpu 3 --pool 20 --rate 10000 --duration 120 --out DIR
//! ```
//!
//! `lab` connects `--conns` WebSockets (connection j asks for `/slot/j`, so
//! `replay_server --mode mixed` gives it its own stream) and records every
//! message: sequence, the sender's send time, decoded time. `pingpong` sends a
//! `--payload`-byte frame (sequence, send time) and waits for the echo, paced
//! at `--rate` on a fixed schedule (0: back to back); with `--feeds K`, K more
//! connections (`/slot/j`) on the same `Poll` carry market data, decoded and
//! counted as `io_bench pingpong --feeds` does. `http` keeps a pool of
//! `--pool` keep-alive HTTP/1.1 connections and sends the lab's order request
//! (`io_bench::order`, the bytes `http_bench` sends) through them in turn, one
//! in flight, paced the same way. Writes `records.bin`, `stalls-0.txt` (loop
//! iterations over `--stall-us`) and `meta.txt`, variant `mio`, as `io_bench`
//! and `http_bench` do.

use std::error::Error;
use std::fmt::Write as _;
use std::io::{ErrorKind, Read, Write as _};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use io_bench::args::Args;
use io_bench::clock::{monotonic_ns, pin_current_thread, realtime_ns};
use io_bench::order::{API_KEY, order_body};
use io_bench::record::{Record, read_i64, write_records};
use io_bench::stalls::StallLog;
use io_bench::{tls, ws};
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection};

type BoxResult<T> = Result<T, Box<dyn Error>>;

/// One TLS WebSocket connection.
struct Conn {
    sock: TcpStream,
    tls: ClientConnection,
    /// Decrypted bytes not yet parsed into frames.
    buf: Vec<u8>,
    closed: bool,
}

impl Conn {
    /// Opens a TLS WebSocket to `server`, blocking until upgraded, then
    /// switches the socket to non-blocking.
    fn open(server: SocketAddr, sni: &str, path: &str, cfg: &Arc<ClientConfig>) -> BoxResult<Self> {
        let mut sock = std::net::TcpStream::connect(server)?;
        sock.set_nodelay(true)?;
        let name = ServerName::try_from(sni.to_owned())?;
        let mut tls = ClientConnection::new(cfg.clone(), name)?;
        while tls.is_handshaking() {
            tls.complete_io(&mut sock)?;
        }
        let key = ws::client_key(rand_nonce());
        let host = format!("{sni}:{}", server.port());
        tls.writer()
            .write_all(&ws::client_request(&host, path, &key, &[]))?;
        while tls.wants_write() {
            tls.write_tls(&mut sock)?;
        }
        let mut buf = Vec::with_capacity(1 << 16);
        let mut chunk = [0u8; 4096];
        let header = loop {
            loop {
                match tls.reader().read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) => return Err(e.into()),
                }
            }
            if let Some(n) = ws::parse_client_response(&buf, &key)? {
                break n;
            }
            if tls.read_tls(&mut sock)? == 0 {
                return Err("server closed during the upgrade".into());
            }
            tls.process_new_packets()?;
        };
        // Frames that arrived with the response stay in the buffer.
        buf.drain(..header);
        sock.set_nonblocking(true)?;
        Ok(Self {
            sock: TcpStream::from_std(sock),
            tls,
            buf,
            closed: false,
        })
    }

    /// Reads everything the socket has (edge-triggered: until `WouldBlock`)
    /// and decrypts it into `buf`. Returns the reads made.
    fn read_all(&mut self) -> BoxResult<u64> {
        let mut reads = 0;
        let mut chunk = [0u8; 16 * 1024];
        loop {
            reads += 1;
            match self.tls.read_tls(&mut self.sock) {
                Ok(0) => {
                    self.closed = true;
                    return Ok(reads);
                }
                Ok(_) => {
                    self.tls.process_new_packets()?;
                    loop {
                        match self.tls.reader().read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                            Err(e) => return Err(e.into()),
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(reads),
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Calls `f(opcode, payload)` for every complete frame in `buf`.
    fn frames(&mut self, mut f: impl FnMut(u8, &[u8])) -> BoxResult<()> {
        let mut at = 0;
        while let Some(h) = ws::parse_header(&self.buf[at..])? {
            if self.buf.len() - at < h.frame_len() {
                break;
            }
            let p = at + h.header_len..at + h.frame_len();
            f(h.opcode, &self.buf[p]);
            at += h.frame_len();
        }
        self.buf.drain(..at);
        Ok(())
    }

    /// Sends one masked binary frame.
    fn send(&mut self, payload: &[u8], frame: &mut Vec<u8>) -> BoxResult<()> {
        frame.resize(ws::header_len(payload.len(), true) + payload.len(), 0);
        let n = ws::encode_frame(ws::OP_BINARY, payload, Some(rand_mask()), frame)?;
        self.tls.writer().write_all(&frame[..n])?;
        while self.tls.wants_write() {
            match self.tls.write_tls(&mut self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => std::hint::spin_loop(),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}

fn rand_nonce() -> [u8; 16] {
    let mut n = [0u8; 16];
    let t = realtime_ns().to_le_bytes();
    n[..8].copy_from_slice(&t);
    n[8..].copy_from_slice(&monotonic_ns().to_le_bytes());
    n
}

fn rand_mask() -> [u8; 4] {
    let t = monotonic_ns().to_le_bytes();
    [t[0], t[1], t[2], t[3]]
}

/// The server's address, its name and the client TLS configuration (the
/// same as ergon-io's: TLS 1.3, AES-128-GCM).
fn lab_tls(a: &Args) -> BoxResult<(SocketAddr, String, Arc<ClientConfig>)> {
    let server: SocketAddr = a.req("server")?.parse()?;
    let sni = a.str_or("sni", "lab.local");
    let ca = CertificateDer::from_pem_file(a.req("ca")?).map_err(|e| format!("ca: {e}"))?;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca)?;
    Ok((server, sni, tls::client_config(roots)?))
}

/// Opens a WebSocket per path (connection j is `Token(j)`) and registers
/// them with a new `Poll`.
fn connect_paths(a: &Args, paths: &[String]) -> BoxResult<(Poll, Vec<Conn>)> {
    let (server, sni, cfg) = lab_tls(a)?;
    let poll = Poll::new()?;
    let mut out = Vec::with_capacity(paths.len());
    for (j, path) in paths.iter().enumerate() {
        let mut c = Conn::open(server, &sni, path, &cfg)?;
        poll.registry()
            .register(&mut c.sock, Token(j), Interest::READABLE)?;
        out.push(c);
    }
    Ok((poll, out))
}

/// Opens `conns` connections (`/slot/<j>`) and registers them with a new `Poll`.
fn connect(a: &Args, conns: usize) -> BoxResult<(Poll, Vec<Conn>)> {
    let paths: Vec<String> = (0..conns).map(|j| format!("/slot/{j}")).collect();
    connect_paths(a, &paths)
}

/// The loop's own counters, written as `loop.0.*`.
struct LoopCounters {
    spins: u64,
    reads: u64,
    polls: u64,
}

fn meta(
    mode: &str,
    duration_s: i64,
    extra: &str,
    loop_stats: &LoopCounters,
    elapsed_s: i64,
    end_reason: &str,
    end_real_ns: i64,
) -> String {
    let mut m = String::new();
    let _ = writeln!(m, "mode={mode}");
    // The planned run length and how long the run actually lasted. A run whose
    // peer went away early used to be written out as if it had lasted the full
    // `duration_s`, so its samples were compared against complete runs.
    let _ = writeln!(m, "duration_s={duration_s}");
    let _ = writeln!(m, "elapsed_s={elapsed_s}");
    let _ = writeln!(m, "end_reason={end_reason}");
    let _ = writeln!(m, "end_realtime_ns={end_real_ns}");
    let _ = writeln!(
        m,
        "kernel={}",
        std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_owned())
            .unwrap_or_default()
    );
    let _ = writeln!(m, "variants=mio");
    let _ = writeln!(m, "loop.0.variant=mio");
    let _ = writeln!(m, "loop.0.spins={}", loop_stats.spins);
    let _ = writeln!(m, "loop.0.epoll_wait_calls={}", loop_stats.polls);
    let _ = writeln!(m, "loop.0.recvmsg_calls={}", loop_stats.reads);
    let _ = writeln!(m, "loop.0.syscalls={}", loop_stats.polls + loop_stats.reads);
    m.push_str(extra);
    m
}

/// The common end-of-run tolerance: the native runner accepts a peer close
/// within 2 s of the planned end as the orderly shutdown, and anything sooner
/// as a failed run (`io/benches/src/runner.rs`). An early peer close used to
/// be written out here as a successful full-duration run.
const END_TOLERANCE_NS: i64 = 2_000_000_000;

/// Why the run ended. A peer close inside the tolerance is the orderly
/// shutdown at the end of a completed run; one before it is an error, not a
/// short successful one.
fn end_reason(now_ns: i64, end_mono_ns: i64) -> BoxResult<&'static str> {
    if now_ns >= end_mono_ns - END_TOLERANCE_NS {
        Ok("peer-close-at-deadline")
    } else {
        Err("the peer closed before the planned end of the run; not a completed run".into())
    }
}

fn lab(a: &Args) -> BoxResult<()> {
    let conns: usize = a.num_or("conns", 1)?;
    let duration_s: i64 = a.num_or("duration", 60)?;
    let capacity: usize = a.num_or("capacity", 4_000_000)?;
    let out = PathBuf::from(a.str_or("out", "results"));
    std::fs::create_dir_all(&out)?;
    let (mut poll, mut conns_v) = connect(a, conns)?;
    let mut events = Events::with_capacity(1024);
    let mut records: Vec<Record> = Vec::with_capacity(capacity);
    let mut dropped = 0u64;
    let mut stalls = StallLog::new(a.num_or("stall-us", 20.0)?, 1 << 20);
    let (mut spins, mut reads, mut polls) = (0u64, 0u64, 0u64);
    // Frames that came with the upgrade responses.
    for c in &mut conns_v {
        c.frames(|_, _| {})?;
    }
    let start = monotonic_ns();
    let end = start + duration_s * 1_000_000_000;
    let mut ready: Vec<usize> = Vec::with_capacity(conns);
    while monotonic_ns() < end {
        spins += 1;
        polls += 1;
        poll.poll(&mut events, Some(Duration::ZERO))?;
        ready.clear();
        ready.extend(events.iter().map(|e| e.token().0));
        for &j in &ready {
            let c = &mut conns_v[j];
            reads += c.read_all()?;
            let conn = u8::try_from(j).unwrap_or(u8::MAX);
            c.frames(|op, p| {
                if op != ws::OP_BINARY || p.len() < 24 {
                    return;
                }
                let r = Record {
                    conn,
                    id: read_i64(p, 0),
                    src_ns: read_i64(p, 8),
                    src2_ns: read_i64(p, 16),
                    user_ns: realtime_ns(),
                    ..Record::default()
                };
                if records.len() < records.capacity() {
                    records.push(r);
                } else {
                    dropped += 1;
                }
            })?;
            if c.closed {
                // An early peer close fails the run; it is not a short
                // successful one whose samples sit beside full-duration runs.
                let end_mono = monotonic_ns();
                let end_real = realtime_ns();
                let reason = end_reason(end_mono, end)?;
                return finish_lab(
                    &out, duration_s, &records, &stalls, spins, reads, polls, dropped, start,
                    end_mono, end_real, reason,
                );
            }
        }
        stalls.tick(records.len() as u64);
    }
    let end_mono = monotonic_ns();
    let end_real = realtime_ns();
    finish_lab(
        &out, duration_s, &records, &stalls, spins, reads, polls, dropped, start, end_mono,
        end_real, "deadline",
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_lab(
    out: &std::path::Path,
    duration_s: i64,
    records: &[Record],
    stalls: &StallLog,
    spins: u64,
    reads: u64,
    polls: u64,
    dropped: u64,
    start_mono_ns: i64,
    end_mono_ns: i64,
    end_real_ns: i64,
    end_reason: &str,
) -> BoxResult<()> {
    stalls.write(&out.join("stalls-0.txt"))?;
    write_records(&out.join("records.bin"), records)?;
    // Beside the percentiles: what arrived and what produced no sample. A
    // receive-only run does not see the sender's schedule.
    let mut extra = format!(
        "loop.0.messages={}\nloop.0.dropped={dropped}\n",
        records.len()
    );
    io_bench::stats::Work::received(records.len() as u64, dropped).write_meta(&mut extra);
    std::fs::write(
        out.join("meta.txt"),
        meta(
            "lab",
            duration_s,
            &extra,
            &LoopCounters {
                spins,
                reads,
                polls,
            },
            (end_mono_ns - start_mono_ns).max(0) / 1_000_000_000,
            end_reason,
            end_real_ns,
        ),
    )?;
    println!(
        "mio lab: {} messages, {spins} spins, {} syscalls",
        records.len(),
        reads + polls
    );
    Ok(())
}

fn pingpong(a: &Args) -> BoxResult<()> {
    let rate: i64 = a.num_or("rate", 1000)?;
    let duration_s: i64 = a.num_or("duration", 30)?;
    let payload_len: usize = a.num_or::<usize>("payload", 64)?.max(16);
    let out = PathBuf::from(a.str_or("out", "results"));
    std::fs::create_dir_all(&out)?;
    // The order connection (Token(0)) and the feeds on the same `Poll`.
    let feeds: usize = a.num_or("feeds", 0)?;
    let mut paths = vec!["/".to_owned()];
    paths.extend((0..feeds).map(|j| format!("/slot/{j}")));
    let (mut poll, mut conns_v) = connect_paths(a, &paths)?;
    let mut events = Events::with_capacity(1024);
    let (mut feed_messages, mut feed_last) = (0u64, (0i64, 0i64));
    let interval = if rate <= 0 { 0 } else { 1_000_000_000 / rate };
    let per_s = if rate > 0 { rate } else { 100_000 };
    let cap = usize::try_from(per_s * duration_s + 1_000).unwrap_or(1 << 20);
    let mut records: Vec<Record> = Vec::with_capacity(cap);
    let mut stalls = StallLog::new(a.num_or("stall-us", 20.0)?, 1 << 20);
    let (mut spins, mut reads, mut polls) = (0u64, 0u64, 0u64);
    let mut payload = vec![0u8; payload_len];
    let mut frame = Vec::with_capacity(payload_len + 16);
    let (mut seq, mut next_send, mut due_ns, mut outstanding) = (0i64, 0i64, 0i64, false);
    // Round trips not recorded because the vector was full, and sends that went
    // out after their slot: the work counts shown beside the percentiles.
    let (mut dropped, mut late, mut first_due) = (0u64, 0u64, 0i64);
    let start = monotonic_ns();
    let end = start + duration_s * 1_000_000_000;
    let mut reason = "deadline";
    while monotonic_ns() < end {
        spins += 1;
        polls += 1;
        poll.poll(&mut events, Some(Duration::ZERO))?;
        let mut closed = false;
        for ev in &events {
            let j = ev.token().0;
            let c = &mut conns_v[j];
            reads += c.read_all()?;
            if j == 0 {
                let now_rx = &mut outstanding;
                let due = due_ns;
                c.frames(|op, p| {
                    if op != ws::OP_BINARY || p.len() < 16 {
                        return;
                    }
                    if records.len() < records.capacity() {
                        records.push(Record {
                            id: read_i64(p, 0),
                            src_ns: read_i64(p, 8),
                            src2_ns: due,
                            user_ns: realtime_ns(),
                            ..Record::default()
                        });
                    } else {
                        // Counted, not silent: the percentiles are over the
                        // recorded round trips alone, and this is the work they
                        // leave out.
                        dropped += 1;
                    }
                    *now_rx = false;
                })?;
            } else {
                // A feed: decoded, not timed.
                c.frames(|op, p| {
                    if op == ws::OP_BINARY && p.len() >= 16 {
                        feed_last = (read_i64(p, 0), read_i64(p, 8));
                        feed_messages += 1;
                    }
                })?;
            }
            closed |= c.closed;
        }
        if closed {
            // An early peer close fails the run; it is not a short successful
            // one whose samples sit beside full-duration runs.
            reason = end_reason(monotonic_ns(), end)?;
            break;
        }
        // The next round trip: on a fixed schedule that never slips.
        if !outstanding {
            let now = realtime_ns();
            if interval == 0 || now >= next_send {
                // Past its slot, not merely at it: the schedule never slips.
                if next_send != 0 && io_bench::stats::Work::is_late(now, next_send, interval) {
                    late += 1;
                }
                seq += 1;
                payload[..8].copy_from_slice(&seq.to_le_bytes());
                payload[8..16].copy_from_slice(&now.to_le_bytes());
                conns_v[0].send(&payload, &mut frame)?;
                outstanding = true;
                if interval == 0 {
                    due_ns = 0;
                } else {
                    due_ns = if next_send == 0 {
                        first_due = now;
                        now
                    } else {
                        next_send
                    };
                    next_send = due_ns + interval;
                }
            }
        }
        stalls.tick(records.len() as u64);
    }
    // Both endpoints sampled here, before any of the run's own file I/O.
    // Writing stalls and records first folded the disk time into the
    // schedule's span and inflated `scheduled` past the slots the loop
    // actually owed; the reported run length came out long for the same reason.
    let end_mono = monotonic_ns();
    let end_real = realtime_ns();
    stalls.write(&out.join("stalls-0.txt"))?;
    write_records(&out.join("records.bin"), &records)?;
    // Beside the percentiles: what the run planned and what became of it.
    let mut extra = format!(
        "rate={rate}\npayload={payload_len}\nconns_per_variant={}\nfeeds={feeds}\nfeed_messages={feed_messages}\nfeed_last_seq={}\n",
        feeds + 1,
        feed_last.0
    );
    io_bench::stats::Work::paced(
        end_real,
        first_due,
        interval,
        u64::try_from(seq).unwrap_or(0),
        records.len() as u64,
        late,
    )
    .write_meta(&mut extra);
    std::fs::write(
        out.join("meta.txt"),
        meta(
            "pingpong",
            duration_s,
            &extra,
            &LoopCounters {
                spins,
                reads,
                polls,
            },
            (end_mono - start).max(0) / 1_000_000_000,
            reason,
            end_real,
        ),
    )?;
    println!(
        "mio pingpong: {} round trips, {feed_messages} feed messages",
        records.len()
    );
    Ok(())
}

/// One keep-alive HTTP/1.1 connection of the `http` pool.
struct HttpConn {
    sock: TcpStream,
    tls: ClientConnection,
    /// Decrypted bytes not yet parsed into a response.
    buf: Vec<u8>,
    closed: bool,
}

impl HttpConn {
    fn open(server: SocketAddr, sni: &str, cfg: &Arc<ClientConfig>) -> BoxResult<Self> {
        let mut sock = std::net::TcpStream::connect(server)?;
        sock.set_nodelay(true)?;
        let name = ServerName::try_from(sni.to_owned())?;
        let mut tls = ClientConnection::new(cfg.clone(), name)?;
        while tls.is_handshaking() {
            tls.complete_io(&mut sock)?;
        }
        sock.set_nonblocking(true)?;
        Ok(Self {
            sock: TcpStream::from_std(sock),
            tls,
            buf: Vec::with_capacity(1 << 16),
            closed: false,
        })
    }

    /// Reads and decrypts everything the socket has (edge-triggered: until
    /// `WouldBlock`). Returns the reads made.
    fn read_all(&mut self) -> BoxResult<u64> {
        let mut reads = 0;
        let mut chunk = [0u8; 16 * 1024];
        loop {
            reads += 1;
            match self.tls.read_tls(&mut self.sock) {
                Ok(0) => {
                    self.closed = true;
                    return Ok(reads);
                }
                Ok(_) => {
                    self.tls.process_new_packets()?;
                    loop {
                        match self.tls.reader().read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                            Err(e) => return Err(e.into()),
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(reads),
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// One complete response taken from the buffer: its status.
    fn response(&mut self) -> BoxResult<Option<u16>> {
        let mut hdrs = [httparse::EMPTY_HEADER; 32];
        let mut r = httparse::Response::new(&mut hdrs);
        let httparse::Status::Complete(n) = r.parse(&self.buf)? else {
            return Ok(None);
        };
        let len = r
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
            .ok_or("response without Content-Length")?;
        let status = r.code.unwrap_or(0);
        if self.buf.len() < n + len {
            return Ok(None);
        }
        self.buf.drain(..n + len);
        Ok(Some(status))
    }

    fn send(&mut self, req: &[u8]) -> BoxResult<()> {
        self.tls.writer().write_all(req)?;
        while self.tls.wants_write() {
            match self.tls.write_tls(&mut self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => std::hint::spin_loop(),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}

/// The HTTP order round trip through a pool of mio connections (see the
/// module docs). Records as `http_bench` does: `src_ns` when the request is
/// handed over, `src2_ns` when it was due (0 unpaced), `user_ns` when the
/// whole response is in.
fn http(a: &Args) -> BoxResult<()> {
    let pool: usize = a.num_or::<usize>("pool", 1)?.max(1);
    let rate: i64 = a.num_or("rate", 1000)?;
    let duration_s: i64 = a.num_or("duration", 30)?;
    let out = PathBuf::from(a.str_or("out", "results"));
    std::fs::create_dir_all(&out)?;
    let (server, sni, cfg) = lab_tls(a)?;
    let poll_t0 = monotonic_ns();
    let mut poll = Poll::new()?;
    let mut conns_v = Vec::with_capacity(pool);
    for j in 0..pool {
        let mut c = HttpConn::open(server, &sni, &cfg)?;
        poll.registry()
            .register(&mut c.sock, Token(j), Interest::READABLE)?;
        conns_v.push(c);
    }
    let connect_ns = monotonic_ns() - poll_t0;
    let host = format!("{sni}:{}", server.port());
    let mut events = Events::with_capacity(1024);
    let interval = if rate <= 0 { 0 } else { 1_000_000_000 / rate };
    let per_s = if rate > 0 { rate } else { 100_000 };
    let cap = usize::try_from(per_s * duration_s + 1_000).unwrap_or(1 << 20);
    let mut records: Vec<Record> = Vec::with_capacity(cap);
    let mut stalls = StallLog::new(a.num_or("stall-us", 20.0)?, 1 << 20);
    let (mut spins, mut reads, mut polls) = (0u64, 0u64, 0u64);
    let mut req = Vec::with_capacity(1024);
    let mut seq: u64 = 0;
    let start = monotonic_ns();
    let end = start + duration_s * 1_000_000_000;
    let mut next = monotonic_ns();
    stalls.start(0);
    while monotonic_ns() < end {
        // The next slot on a schedule that never slips (busy-waiting).
        let slot = if interval == 0 {
            0
        } else {
            while monotonic_ns() < next {
                stalls.tick(seq);
            }
            next += interval;
            next - interval
        };
        seq += 1;
        let body = order_body(seq);
        req.clear();
        let _ = write!(
            req,
            "POST /api/v3/order HTTP/1.1\r\nHost: {host}\r\nUser-Agent: ergon-io-bench\r\nX-MBX-APIKEY: {API_KEY}\r\n\
             Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stalls.tick(seq);
        let j = usize::try_from(seq).unwrap_or(0) % pool;
        let mono = monotonic_ns();
        let sent = realtime_ns();
        let due = if slot == 0 { 0 } else { sent - (mono - slot) };
        conns_v[j].send(&req)?;
        let status = loop {
            spins += 1;
            polls += 1;
            stalls.tick(seq - 1);
            poll.poll(&mut events, Some(Duration::ZERO))?;
            for ev in &events {
                reads += conns_v[ev.token().0].read_all()?;
            }
            if let Some(s) = conns_v[j].response()? {
                break s;
            }
            if conns_v.iter().any(|c| c.closed) {
                return Err("a pool connection closed".into());
            }
        };
        let now = realtime_ns();
        if status != 200 {
            return Err(format!("status {status}").into());
        }
        if records.len() < records.capacity() {
            records.push(Record {
                id: i64::try_from(seq).unwrap_or(i64::MAX),
                src_ns: sent,
                src2_ns: due,
                user_ns: now,
                ..Record::default()
            });
        }
    }
    let end_mono = monotonic_ns();
    let end_real = realtime_ns();
    stalls.write(&out.join("stalls-0.txt"))?;
    write_records(&out.join("records.bin"), &records)?;
    let extra = format!(
        "rate={rate}\npool={pool}\nconns_per_variant={pool}\nrequests={}\nconnect_us={:.1}\n",
        records.len(),
        connect_ns as f64 / 1000.0
    );
    std::fs::write(
        out.join("meta.txt"),
        meta(
            "pingpong",
            duration_s,
            &extra,
            &LoopCounters {
                spins,
                reads,
                polls,
            },
            (end_mono - start).max(0) / 1_000_000_000,
            "deadline",
            end_real,
        ),
    )?;
    println!(
        "mio http: {} requests over {pool} connection(s)",
        records.len()
    );
    Ok(())
}

fn main() -> BoxResult<()> {
    let a = Args::from_env();
    // As io_bench: memory locked (needs root), the counter calibrated, the
    // thread pinned.
    #[cfg(target_os = "linux")]
    // SAFETY: plain syscall with valid flags.
    let _ = unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) };
    let _ = io_bench::clock::ticks_per_us();
    if let Some(c) = a.list("cpu", "").first() {
        pin_current_thread(c.parse()?)?;
    }
    match a.command.as_str() {
        "lab" => lab(&a),
        "pingpong" => pingpong(&a),
        "http" => http(&a),
        other => Err(format!("unknown command {other:?}: lab, pingpong or http").into()),
    }
}
