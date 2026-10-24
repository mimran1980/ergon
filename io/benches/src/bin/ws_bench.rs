//! WebSocket receive-path micro-benchmark: cost per message of parsing frames
//! that are already plaintext in the receive buffer and delivering them.
//!
//! ```text
//! ws_bench [--sizes 100,300,4096,65536] [--samples 20000] [--utf8 true] [--cpu N] [--out FILE]
//! ```
//!
//! A real TLS 1.3 handshake on loopback builds the connection; after that
//! nothing touches a socket or the AEAD. Each sample loads a window of K text
//! messages (untimed: the kernel's copy), then times one
//! `WsConn::deliver_staged` with the raw counter (`rdtsc` / `cntvct_el0`):
//! parse, decide, copy where the case copies, check UTF-8 where it checks,
//! deliver. The figure is that sample's ticks / K, in ns. The two paths of
//! a row run interleaved on the same window, alternating which goes first.
//!
//! | axis | values |
//! |---|---|
//! | shape | `whole`: unfragmented, the window holds every message; `frag4`: each message in 4 frames, same window; `split`: unfragmented, each message cut at its middle and records at most 16 KiB (stand-in records, no AEAD) |
//! | decision | `kept`: the handler reads every message; `skipped`: the handler wants none |
//! | path | `old`: `Interest::Keep` from the first frame, the pre-0.1.35 path: fragmented messages copied into one buffer, UTF-8 checked before delivery, a message the handler does not want still copied and checked; `new`: `Interest::Fragments` (kept) or `Interest::Skip` (skipped) |
//!
//! Every case is checked before it is timed: delivered payloads equal the
//! messages sent, and the copy counters match the case.

use std::error::Error;
use std::hint::black_box;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use ergon_io::tls::{self, RecordLayer};
use ergon_io::ws::{self, OP_CONT, OP_TEXT};
use ergon_io::{ConnId, Fragments, Handler, Interest, RxMeta, WsConn, WsHandler, WsOptions};
use hdrhistogram::Histogram;
use io_bench::args::Args;
use io_bench::clock::{counter_meta, pin_current_thread, rdtsc, thread_ctx_switches, ticks_per_us};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{RootCertStore, ServerConfig, ServerConnection};

type BoxResult<T> = Result<T, Box<dyn Error>>;

/// Histogram unit: 0.1 ns.
const SCALE: f64 = 10.0;
/// Largest stand-in record plaintext.
const RECORD: usize = 1 << 14;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Whole,
    Frag4,
    Split,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Path {
    Old,
    New,
}

/// Reads every kept message as a consumer would touch it: its length and
/// its first and last bytes, through `black_box`.
struct App {
    path: Path,
    keep: bool,
    verify: bool,
    got: Vec<Vec<u8>>,
    delivered: u64,
    sink: u64,
}

impl Handler for App {}

fn fold(sum: u64, p: &[u8]) -> u64 {
    let p = black_box(p);
    let ends = p.first().map_or(0, |&b| u64::from(b)) + p.last().map_or(0, |&b| u64::from(b));
    sum.wrapping_add(p.len() as u64).wrapping_add(ends)
}

impl WsHandler for App {
    #[inline]
    fn on_message_start(&mut self, _: ConnId, _: u8, _: &[u8], _: bool, _: &RxMeta) -> Interest {
        match (self.path, self.keep) {
            (Path::Old, _) => Interest::Keep,
            (Path::New, true) => Interest::Fragments,
            (Path::New, false) => Interest::Skip,
        }
    }

    #[inline]
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        // The old path's "skipped": delivered, then ignored by the handler.
        if !self.keep {
            return;
        }
        self.delivered += 1;
        self.sink = fold(self.sink, payload);
        if self.verify {
            self.got.push(payload.to_vec());
        }
    }

    #[inline]
    fn on_fragments(&mut self, _: ConnId, _: u8, m: &Fragments<'_>, _: &RxMeta, _: u16) {
        self.delivered += 1;
        for s in m {
            self.sink = fold(self.sink, s);
        }
        if self.verify {
            self.got.push(m.iter().flatten().copied().collect());
        }
    }
}

/// An unmasked server frame.
fn frame(op: u8, fin: bool, payload: &[u8], out: &mut Vec<u8>) -> BoxResult<()> {
    let start = out.len();
    out.resize(
        start + ws::header_len(payload.len(), false) + payload.len(),
        0,
    );
    let n = ws::encode_frame(op, payload, None, &mut out[start..])?;
    out.truncate(start + n);
    if !fin {
        out[start] &= 0x7F;
    }
    Ok(())
}

/// Text that looks like a feed: ASCII, varied per message.
fn message(i: usize, len: usize) -> Vec<u8> {
    let pat = b"{\"e\":\"trade\",\"s\":\"BTCUSDT\",\"p\":\"67012.10\",\"q\":\"0.0031\"}";
    (0..len).map(|j| pat[(j + i) % pat.len()]).collect()
}

/// The window for one sample, and where the split shape cuts records.
struct Load {
    msgs: Vec<Vec<u8>>,
    stream: Vec<u8>,
    records: Vec<(usize, usize)>,
}

fn load_for(shape: Shape, size: usize, k: usize) -> BoxResult<Load> {
    let msgs: Vec<Vec<u8>> = (0..k).map(|i| message(i, size)).collect();
    let mut stream = Vec::new();
    let mut cuts = vec![0];
    for m in &msgs {
        let start = stream.len();
        if shape == Shape::Frag4 {
            let q = size.div_ceil(4);
            for (j, c) in m.chunks(q).enumerate() {
                let op = if j == 0 { OP_TEXT } else { OP_CONT };
                frame(op, j == 3 || (j + 1) * q >= size, c, &mut stream)?;
            }
        } else {
            frame(OP_TEXT, true, m, &mut stream)?;
        }
        cuts.push(start + (stream.len() - start) / 2);
    }
    cuts.push(stream.len());
    // Records end at each message's middle and are at most 16 KiB.
    let mut records = Vec::new();
    for w in cuts.windows(2) {
        let mut at = w[0];
        while at < w[1] {
            let end = (at + RECORD).min(w[1]);
            records.push((at, end));
            at = end;
        }
    }
    Ok(Load {
        msgs,
        stream,
        records,
    })
}

fn put(conn: &mut WsConn, shape: Shape, l: &Load) -> BoxResult<()> {
    if shape == Shape::Split {
        for &(a, b) in &l.records {
            conn.stage_plain_record(&l.stream[a..b])?;
        }
    } else {
        conn.preload_plain(&l.stream)?;
    }
    Ok(())
}

struct Row {
    p50: f64,
    p99: f64,
    max: f64,
    frag_copy: f64,
    span_copy: f64,
    max_preempted: bool,
}

fn id() -> ConnId {
    ConnId::from_parts(0, 0)
}

const fn app(path: Path, keep: bool) -> App {
    App {
        path,
        keep,
        verify: true,
        got: Vec::new(),
        delivered: 0,
        sink: 0,
    }
}

/// Checks one path's delivery of `l` before anything is timed: payloads,
/// counts and the copy counters the case implies.
fn verify(conn: &mut WsConn, shape: Shape, l: &Load, path: Path, keep: bool) -> BoxResult<()> {
    let k = l.msgs.len();
    let size = l.msgs.first().map_or(0, Vec::len);
    let mut a = app(path, keep);
    let s0 = conn.stats();
    put(conn, shape, l)?;
    let n = conn.deliver_staged(id(), &RxMeta::default(), &mut a)?;
    let s1 = conn.stats();
    let delivered_expected = if !keep && path == Path::New { 0 } else { k };
    assert_eq!(
        n, delivered_expected,
        "{shape:?} {size} {keep} {path:?}: delivered"
    );
    assert_eq!(conn.buffered(), 0, "the window is consumed");
    if keep {
        assert_eq!(a.got, l.msgs, "{shape:?} {size} {path:?}: payloads");
    }
    let frag_copy = s1.frag_copy_bytes - s0.frag_copy_bytes;
    match (shape, path) {
        (Shape::Frag4, Path::Old) => assert_eq!(frag_copy, (k * size) as u64),
        _ => assert_eq!(
            frag_copy, 0,
            "{shape:?} {path:?} {keep}: no reassembly copy"
        ),
    }
    if path == Path::New && !keep {
        assert_eq!(s1.skipped - s0.skipped, k as u64);
    }
    assert_eq!(s1.frag_fallbacks, s0.frag_fallbacks, "no fallback");
    Ok(())
}

/// Timing state of one path.
struct Side {
    app: App,
    h: Histogram<u64>,
    worst: u64,
    worst_preempted: bool,
    frag_copy: u64,
    span_copy: u64,
}

fn side(path: Path, keep: bool) -> BoxResult<Side> {
    let mut a = app(path, keep);
    a.verify = false;
    Ok(Side {
        app: a,
        h: Histogram::<u64>::new_with_bounds(1, 100_000_000_000, 3)?,
        worst: 0,
        worst_preempted: false,
        frag_copy: 0,
        span_copy: 0,
    })
}

/// Loads `l`, then times one `deliver_staged` of it for `side`.
#[allow(clippy::cast_precision_loss)]
fn sample(
    conn: &mut WsConn,
    shape: Shape,
    l: &Load,
    side: &mut Side,
    record: bool,
    ns_per_tick_per_msg: f64,
) -> BoxResult<()> {
    let meta = RxMeta::default();
    put(conn, shape, l)?;
    let s0 = conn.stats();
    let (_, iv0) = thread_ctx_switches();
    let t0 = rdtsc();
    let n = conn.deliver_staged(id(), &meta, &mut side.app)?;
    let t1 = rdtsc();
    let (_, iv1) = thread_ctx_switches();
    black_box(n);
    if !record {
        return Ok(());
    }
    let s1 = conn.stats();
    side.frag_copy += s1.frag_copy_bytes - s0.frag_copy_bytes;
    side.span_copy += s1.span_copy_bytes - s0.span_copy_bytes;
    let ns = t1.wrapping_sub(t0) as f64 * ns_per_tick_per_msg;
    let v = ((ns * SCALE) as u64).max(1);
    side.h.record(v)?;
    if v > side.worst {
        side.worst = v;
        side.worst_preempted = iv1 != iv0;
    }
    Ok(())
}

/// Both paths over the same window, interleaved sample by sample (the
/// order alternates), on one connection.
#[allow(clippy::cast_precision_loss)]
fn run_pair(
    conn: &mut WsConn,
    shape: Shape,
    size: usize,
    keep: bool,
    samples: usize,
) -> BoxResult<[Row; 2]> {
    let k = (512 * 1024 / size).clamp(1, 64);
    let l = load_for(shape, size, k)?;
    // Correctness first.
    for path in [Path::Old, Path::New] {
        verify(conn, shape, &l, path, keep)?;
    }
    let mut old = side(Path::Old, keep)?;
    let mut new = side(Path::New, keep)?;
    let per = 1000.0 / ticks_per_us() / k as f64;
    let warm = samples / 10;
    for i in 0..warm + samples {
        let record = i >= warm;
        if i % 2 == 0 {
            sample(conn, shape, &l, &mut old, record, per)?;
            sample(conn, shape, &l, &mut new, record, per)?;
        } else {
            sample(conn, shape, &l, &mut new, record, per)?;
            sample(conn, shape, &l, &mut old, record, per)?;
        }
    }
    let msgs = (samples * k) as f64;
    Ok([old, new].map(|s| {
        black_box(s.app.sink);
        Row {
            p50: s.h.value_at_quantile(0.50) as f64 / SCALE,
            p99: s.h.value_at_quantile(0.99) as f64 / SCALE,
            max: s.h.max() as f64 / SCALE,
            frag_copy: s.frag_copy as f64 / msgs,
            span_copy: s.span_copy as f64 / msgs,
            max_preempted: s.worst_preempted,
        }
    }))
}

/// A connection whose keys come from a real handshake on loopback.
fn connect(opts: &WsOptions) -> BoxResult<WsConn> {
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let cert = CertificateDer::from(ck.cert.der().to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
    let scfg = Arc::new(
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(vec![cert.clone()], key)?,
    );
    let l = TcpListener::bind("127.0.0.1:0")?;
    let addr = l.local_addr()?;
    let srv = std::thread::spawn(move || -> Result<(), String> {
        let (mut sock, _) = l.accept().map_err(|e| e.to_string())?;
        let mut c = ServerConnection::new(scfg).map_err(|e| e.to_string())?;
        while c.is_handshaking() {
            c.complete_io(&mut sock).map_err(|e| e.to_string())?;
        }
        Ok(())
    });
    let mut roots = RootCertStore::empty();
    roots.add(cert)?;
    let mut sock = TcpStream::connect(addr)?;
    let (secrets, kconn, _) =
        tls::handshake_raw(&mut sock, tls::client_config(roots)?, "localhost")?;
    let layer = RecordLayer::from_parts(&secrets, kconn)?;
    srv.join().map_err(|_| "server panicked")??;
    Ok(WsConn::from_record_layer(sock, layer, opts)?)
}

fn main() -> BoxResult<()> {
    let a = Args::from_env();
    let sizes: Vec<usize> = a
        .list("sizes", "100,300,4096,65536")
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()?;
    let samples: usize = a.num_or("samples", 20_000)?;
    let utf8 = !a.str_or("utf8", "true").eq_ignore_ascii_case("false");
    if let Ok(cpu) = a.req("cpu") {
        pin_current_thread(cpu.parse()?)?;
    }
    let opts = WsOptions {
        buf_size: 4 << 20,
        max_message: 1 << 20,
        validate_utf8: utf8,
        ping_interval_ms: 0,
        ..WsOptions::default()
    };
    let mut conn = connect(&opts)?;
    let _ = ticks_per_us();
    let mut out = String::new();
    out.push_str(&counter_meta());
    out.push_str(&format!("samples={samples}\nvalidate_utf8={utf8}\n\n"));
    out.push_str("| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|---|\n");
    for &size in &sizes {
        for shape in [Shape::Whole, Shape::Frag4, Shape::Split] {
            for keep in [true, false] {
                let rows = run_pair(&mut conn, shape, size, keep, samples)?;
                for (path, r) in [Path::Old, Path::New].into_iter().zip(rows) {
                    let k = (512 * 1024 / size).clamp(1, 64);
                    let line = format!(
                        "| {size} | {} | {} | {} | {k} | {:.1} | {:.1} | {:.1} | {} | {:.0} | {:.0} |\n",
                        format!("{shape:?}").to_lowercase(),
                        if keep { "kept" } else { "skipped" },
                        format!("{path:?}").to_lowercase(),
                        r.p50,
                        r.p99,
                        r.max,
                        if !cfg!(target_os = "linux") {
                            "n/a"
                        } else if r.max_preempted {
                            "yes"
                        } else {
                            "no"
                        },
                        r.frag_copy,
                        r.span_copy,
                    );
                    print!("{line}");
                    out.push_str(&line);
                }
            }
        }
    }
    if let Ok(path) = a.req("out") {
        std::fs::write(path, out)?;
    }
    Ok(())
}
