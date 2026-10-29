//! WebSocket micro-benchmarks, one suite per run (`--suite`):
//!
//! | suite | what is timed, per message | paths, interleaved on the same input |
//! |---|---|---|
//! | `rx` (default) | parse, decide, copy, check, deliver plaintext already in the receive buffer | `old` (`Interest::Keep`) against `new` (`Fragments` / `Skip`) |
//! | `tx` | build one masked client frame into the send queue, alone (`build`) and inside a sealed TLS record (`sealed`) | `old`: zeroed resize, copy, separate XOR (`ws::encode_frame`), the path before 0.1.35; `new`: header from the stack, copy then XOR in place, no zero fill (`ws::push_frame`) |
//! | `text` | deliver one unfragmented kept text message | `TextCheck::None`, `Ascii` (trusted, reads nothing), `AsciiChecked`, `Utf8` on pure-ASCII and on mixed text |
//! | `frag` | deliver one message in four fragments | `Interest::Keep`, `Fragments`, `Stream` |
//! | `span` | deliver messages from 16 KiB stand-in records cut mid-frame | `old`: the new record always moved back (`set_span_back_only`); `new`: the smaller side moved |
//!
//! ```text
//! ws_bench [--suite rx|tx|text|frag|span] [--sizes 100,300,4096,65536] [--samples 20000] [--utf8 true] [--cpu N] [--out FILE]
//! ```
//!
//! Every suite checks its payloads before timing (and `tx` that the paths'
//! bytes are equal), uses `black_box` on what it produces, and reports
//! p50 / p99 / max per message from one histogram per path. The rest of
//! this comment describes `rx`.
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
use ergon_io::{
    ConnId, Flow, Fragments, Handler, Interest, NonAscii, RxMeta, TextCheck, TextStatus, WsConn,
    WsHandler, WsOptions,
};
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
    fn on_fragments(&mut self, _: ConnId, _: u8, m: &Fragments<'_>, _: &RxMeta, _: u16) -> Flow {
        self.delivered += 1;
        for s in m {
            self.sink = fold(self.sink, s);
        }
        if self.verify {
            self.got.push(m.iter().flatten().copied().collect());
        }
        Flow::Continue
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
    load_with(shape, (0..k).map(|i| message(i, size)).collect())
}

fn load_with(shape: Shape, msgs: Vec<Vec<u8>>) -> BoxResult<Load> {
    let size = msgs.first().map_or(0, Vec::len);
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

const fn id() -> ConnId {
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
    let (sock, layer) = layer()?;
    Ok(WsConn::from_record_layer(sock, layer, opts)?)
}

/// A record layer (and its socket) from a real TLS 1.3 handshake on loopback.
fn layer() -> BoxResult<(TcpStream, RecordLayer)> {
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
    Ok((sock, layer))
}

fn main() -> BoxResult<()> {
    let a = Args::from_env();
    let suite = a.str_or("suite", "rx");
    let default_sizes = if suite == "rx" {
        "100,300,4096,65536"
    } else {
        "100,300,4096,16384"
    };
    let sizes: Vec<usize> = a
        .list("sizes", default_sizes)
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()?;
    let samples: usize = a.num_or("samples", 20_000)?;
    if let Ok(cpu) = a.req("cpu") {
        pin_current_thread(cpu.parse()?)?;
    }
    let _ = ticks_per_us();
    let mut out = String::new();
    out.push_str(&counter_meta());
    out.push_str(&format!("suite={suite}\nsamples={samples}\n"));
    match suite.as_str() {
        "rx" => rx_suite(&a, &sizes, samples, &mut out)?,
        "tx" => tx_suite(&sizes, samples, &mut out)?,
        "text" => text_suite(&sizes, samples, &mut out)?,
        "frag" => frag_suite(&sizes, samples, &mut out)?,
        "span" => span_suite(&sizes, samples, &mut out)?,
        other => return Err(format!("unknown suite {other}").into()),
    }
    if let Ok(path) = a.req("out") {
        std::fs::write(path, out)?;
    }
    Ok(())
}

fn rx_suite(a: &Args, sizes: &[usize], samples: usize, out: &mut String) -> BoxResult<()> {
    let utf8 = !a.str_or("utf8", "true").eq_ignore_ascii_case("false");
    let opts = WsOptions {
        buf_size: 4 << 20,
        max_message: 1 << 20,
        text_check: if utf8 {
            TextCheck::Utf8
        } else {
            TextCheck::None
        },
        ping_interval_ms: 0,
        ..WsOptions::default()
    };
    let mut conn = connect(&opts)?;
    out.push_str(&format!("text_check={:?}\n\n", opts.text_check));
    out.push_str("| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|---|\n");
    for &size in sizes {
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
                        preempted(r.max_preempted),
                        r.frag_copy,
                        r.span_copy,
                    );
                    print!("{line}");
                    out.push_str(&line);
                }
            }
        }
    }
    Ok(())
}

const fn preempted(p: bool) -> &'static str {
    if !cfg!(target_os = "linux") {
        "n/a"
    } else if p {
        "yes"
    } else {
        "no"
    }
}

/// One path's histogram (0.1 ns units) and whether its max sample saw an
/// involuntary context switch.
struct Hist {
    h: Histogram<u64>,
    worst: u64,
    worst_preempted: bool,
}

impl Hist {
    fn new() -> BoxResult<Self> {
        Ok(Self {
            h: Histogram::<u64>::new_with_bounds(1, 100_000_000_000, 3)?,
            worst: 0,
            worst_preempted: false,
        })
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn record(&mut self, ticks: u64, per: f64, preempted: bool) -> BoxResult<()> {
        let v = ((ticks as f64 * per * SCALE) as u64).max(1);
        self.h.record(v)?;
        if v > self.worst {
            self.worst = v;
            self.worst_preempted = preempted;
        }
        Ok(())
    }

    #[allow(clippy::cast_precision_loss)]
    fn cells(&self) -> String {
        format!(
            "{:.1} | {:.1} | {:.1} | {}",
            self.h.value_at_quantile(0.50) as f64 / SCALE,
            self.h.value_at_quantile(0.99) as f64 / SCALE,
            self.h.max() as f64 / SCALE,
            preempted(self.worst_preempted)
        )
    }
}

/// Times `f` once: ticks, and whether an involuntary switch happened.
#[inline]
fn timed<T>(f: impl FnOnce() -> T) -> (u64, bool, T) {
    let (_, iv0) = thread_ctx_switches();
    let t0 = rdtsc();
    let r = f();
    let t1 = rdtsc();
    let (_, iv1) = thread_ctx_switches();
    (t1.wrapping_sub(t0), iv1 != iv0, r)
}

/// Runs `paths` interleaved, sample by sample, rotating which goes first;
/// `run(path)` returns one sample's ticks and its preemption flag.
fn interleave(
    paths: usize,
    samples: usize,
    per: f64,
    mut run: impl FnMut(usize) -> BoxResult<(u64, bool)>,
) -> BoxResult<Vec<Hist>> {
    let mut h: Vec<Hist> = (0..paths).map(|_| Hist::new()).collect::<BoxResult<_>>()?;
    let warm = samples / 10;
    for i in 0..warm + samples {
        for j in 0..paths {
            let p = (i + j) % paths;
            let (t, pre) = run(p)?;
            if i >= warm {
                h[p].record(t, per, pre)?;
            }
        }
    }
    Ok(h)
}

const fn k_for(size: usize) -> usize {
    let k = 256 * 1024 / size;
    if k < 1 {
        1
    } else if k > 64 {
        64
    } else {
        k
    }
}

// ---- tx ---------------------------------------------------------------------

/// Largest payload one record carries with its frame header (as `WsConn`).
const ONE_RECORD: usize = (1 << 14) - 8;

/// The frame body as built before 0.1.35: one record holds the whole frame.
fn tx_old_one(out: &mut Vec<u8>, op: u8, payload: &[u8], mask: [u8; 4]) -> ergon_io::Result<()> {
    let start = out.len();
    let hl = ws::header_len(payload.len(), true);
    out.resize(start + hl + payload.len(), 0);
    ws::encode_frame(op, payload, Some(mask), &mut out[start..])?;
    Ok(())
}

/// The next piece after `at` as `WsConn` splits a frame over records: the
/// header and the first part, then 16 KiB each.
const fn next_piece(len: usize, at: usize) -> (usize, usize) {
    let first = (1 << 14) - ws::header_len(len, true);
    if at == 0 {
        (0, if first < len { first } else { len })
    } else {
        let end = at + (1 << 14);
        (at, if end < len { end } else { len })
    }
}

/// One message onto `q` as `path` builds it, optionally sealed.
fn tx_one(
    q: &mut Vec<u8>,
    layer: Option<&mut RecordLayer>,
    path: Path,
    payload: &[u8],
    mask: [u8; 4],
) -> ergon_io::Result<()> {
    let op = OP_BINARY_BENCH;
    let one = payload.len() <= ONE_RECORD;
    let build = |out: &mut Vec<u8>, part: (usize, usize)| -> ergon_io::Result<()> {
        match (path, one, part.0 == 0) {
            (Path::Old, true, _) => tx_old_one(out, op, payload, mask),
            (Path::New, true, _) => {
                ws::push_frame(out, op, payload, mask);
                Ok(())
            }
            (Path::Old, false, true) => {
                let start = out.len();
                let hl = ws::header_len(payload.len(), true);
                out.resize(start + hl, 0);
                ws::encode_header(op, payload.len(), Some(mask), &mut out[start..])?;
                let body = out.len();
                out.extend_from_slice(&payload[part.0..part.1]);
                ws::apply_mask(&mut out[body..], mask);
                Ok(())
            }
            (Path::Old, false, false) => {
                let start = out.len();
                out.extend_from_slice(&payload[part.0..part.1]);
                ws::apply_mask_at(&mut out[start..], mask, part.0);
                Ok(())
            }
            (Path::New, false, true) => {
                ws::push_header(out, op, payload.len(), Some(mask));
                ws::push_masked(out, &payload[part.0..part.1], mask, 0);
                Ok(())
            }
            (Path::New, false, false) => {
                ws::push_masked(out, &payload[part.0..part.1], mask, part.0);
                Ok(())
            }
        }
    };
    // No allocation per message: pieces are computed as they go.
    let mut at = 0;
    let mut l = layer;
    loop {
        let part = if one {
            (0, payload.len())
        } else {
            next_piece(payload.len(), at)
        };
        match l.as_deref_mut() {
            None => build(q, part)?,
            Some(l) => l.seal_build(q, |out| build(out, part))?,
        }
        at = part.1;
        if one || at >= payload.len() {
            return Ok(());
        }
    }
}

const OP_BINARY_BENCH: u8 = ws::OP_BINARY;

fn tx_suite(sizes: &[usize], samples: usize, out: &mut String) -> BoxResult<()> {
    let (_sock, mut layer) = layer()?;
    out.push_str("\n| size B | stage | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for &size in sizes {
        let k = k_for(size);
        let msgs: Vec<Vec<u8>> = (0..k).map(|i| message(i, size)).collect();
        let masks: Vec<[u8; 4]> = (0..k)
            .map(|i| (i as u32).wrapping_mul(0x9E37_79B9).to_le_bytes())
            .collect();
        // A queue big enough for every sample, written once (no faults).
        let cap = k * (size + 64) + 2 * (1 << 15);
        let mut q: Vec<u8> = Vec::with_capacity(cap);
        q.resize(cap, 1);
        q.clear();
        // Correctness: both paths build the same bytes.
        let mut want = Vec::new();
        for (m, &mask) in msgs.iter().zip(&masks) {
            tx_one(&mut want, None, Path::Old, m, mask)?;
        }
        for (m, &mask) in msgs.iter().zip(&masks) {
            tx_one(&mut q, None, Path::New, m, mask)?;
        }
        assert!(q == want, "{size} B: old and new frames differ");
        let per = 1000.0 / ticks_per_us() / k as f64;
        for sealed in [false, true] {
            let paths = [Path::Old, Path::New];
            let h = interleave(2, samples, per, |p| {
                q.clear();
                let path = paths[p];
                let l = &mut layer;
                let (t, pre, r) = timed(|| -> ergon_io::Result<()> {
                    for (m, &mask) in msgs.iter().zip(&masks) {
                        tx_one(&mut q, sealed.then_some(&mut *l), path, black_box(m), mask)?;
                    }
                    Ok(())
                });
                r?;
                black_box(q.as_slice());
                Ok((t, pre))
            })?;
            for (path, hist) in paths.iter().zip(&h) {
                let line = format!(
                    "| {size} | {} | {} | {k} | {} |\n",
                    if sealed { "sealed" } else { "build" },
                    format!("{path:?}").to_lowercase(),
                    hist.cells()
                );
                print!("{line}");
                out.push_str(&line);
            }
        }
    }
    Ok(())
}

// ---- text ---------------------------------------------------------------------

/// Reads every delivered message (length and end bytes) and remembers the
/// text status of the last one.
struct Sink {
    interest: Interest,
    sink: u64,
    delivered: u64,
    last: TextStatus,
    verify: bool,
    got: Vec<Vec<u8>>,
    open: Vec<u8>,
}

impl Sink {
    const fn new(interest: Interest) -> Self {
        Self {
            interest,
            sink: 0,
            delivered: 0,
            last: TextStatus::Unchecked,
            verify: false,
            got: Vec::new(),
            open: Vec::new(),
        }
    }
}

impl Handler for Sink {}

impl WsHandler for Sink {
    #[inline]
    fn on_message_start(&mut self, _: ConnId, _: u8, _: &[u8], _: bool, _: &RxMeta) -> Interest {
        self.interest
    }

    #[inline]
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], meta: &RxMeta, _: u16) {
        self.delivered += 1;
        self.last = meta.text;
        self.sink = fold(self.sink, payload);
        if self.verify {
            self.got.push(payload.to_vec());
        }
    }

    #[inline]
    fn on_fragments(&mut self, _: ConnId, _: u8, m: &Fragments<'_>, meta: &RxMeta, _: u16) -> Flow {
        self.delivered += 1;
        self.last = meta.text;
        for s in m {
            self.sink = fold(self.sink, s);
        }
        if self.verify {
            self.got.push(m.iter().flatten().copied().collect());
        }
        Flow::Continue
    }

    #[inline]
    fn on_fragment(
        &mut self,
        _: ConnId,
        _: u8,
        part: &[u8],
        _: u32,
        fin: bool,
        meta: &RxMeta,
    ) -> Flow {
        self.last = meta.text;
        self.sink = fold(self.sink, part);
        if self.verify {
            self.open.extend_from_slice(part);
        }
        if fin {
            self.delivered += 1;
            if self.verify {
                self.got.push(std::mem::take(&mut self.open));
            }
        }
        Flow::Continue
    }
}

/// Feed-like text with a two-byte character every 64 bytes.
fn mixed(i: usize, len: usize) -> Vec<u8> {
    let mut m = message(i, len);
    let mut at = 31;
    while at + 1 < len {
        m[at..at + 2].copy_from_slice("é".as_bytes());
        at += 64;
    }
    m
}

/// Times delivering `l` (whole shape) on each connection in turn.
fn deliver_paths(
    conns: &mut [WsConn],
    sinks: &mut [Sink],
    shape: Shape,
    l: &Load,
    samples: usize,
) -> BoxResult<Vec<Hist>> {
    let k = l.msgs.len();
    let meta = RxMeta::default();
    // Correctness first: every path delivers every message intact.
    for (c, s) in conns.iter_mut().zip(sinks.iter_mut()) {
        s.verify = true;
        s.got.clear();
        put(c, shape, l)?;
        c.deliver_staged(id(), &meta, s)?;
        assert_eq!(s.got, l.msgs, "payloads");
        assert_eq!(c.buffered(), 0);
        s.verify = false;
    }
    let per = 1000.0 / ticks_per_us() / k as f64;
    interleave(conns.len(), samples, per, |p| {
        put(&mut conns[p], shape, l)?;
        let (t, pre, r) = timed(|| conns[p].deliver_staged(id(), &meta, &mut sinks[p]));
        black_box(r?);
        Ok((t, pre))
    })
}

fn text_suite(sizes: &[usize], samples: usize, out: &mut String) -> BoxResult<()> {
    let checks = [
        TextCheck::None,
        TextCheck::Ascii,
        TextCheck::AsciiChecked(NonAscii::DeliverUnchecked),
        TextCheck::Utf8,
    ];
    let mut conns = Vec::new();
    for c in checks {
        conns.push(connect(&WsOptions {
            buf_size: 4 << 20,
            max_message: 1 << 20,
            text_check: c,
            ping_interval_ms: 0,
            ..WsOptions::default()
        })?);
    }
    let mut sinks: Vec<Sink> = checks.iter().map(|_| Sink::new(Interest::Keep)).collect();
    out.push_str("\n| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|\n");
    for &size in sizes {
        let k = k_for(size);
        for is_mixed in [false, true] {
            let mut l = load_for(Shape::Whole, size, k)?;
            if is_mixed {
                l = load_with(Shape::Whole, (0..k).map(|i| mixed(i, size)).collect())?;
            }
            let h = deliver_paths(&mut conns, &mut sinks, Shape::Whole, &l, samples)?;
            // Each mode flagged what it found (checked after timing too).
            for (c, s) in checks.iter().zip(&sinks) {
                let want = match c {
                    TextCheck::None => TextStatus::Unchecked,
                    TextCheck::Ascii => TextStatus::AssumedAscii,
                    TextCheck::AsciiChecked(_) if is_mixed => TextStatus::NotAscii,
                    TextCheck::AsciiChecked(_) => TextStatus::Ascii,
                    TextCheck::Utf8 => TextStatus::Utf8,
                };
                if s.last != want {
                    return Err(format!("{c:?}: delivered as {:?}", s.last).into());
                }
            }
            for ((c, s), hist) in checks.iter().zip(&sinks).zip(&h) {
                let line = format!(
                    "| {size} | {} | {} | {:?} | {k} | {} |\n",
                    if is_mixed { "mixed" } else { "ascii" },
                    match c {
                        TextCheck::None => "none",
                        TextCheck::Utf8 => "utf8",
                        TextCheck::Ascii => "ascii",
                        TextCheck::AsciiChecked(_) => "ascii-checked",
                    },
                    s.last,
                    hist.cells()
                );
                print!("{line}");
                out.push_str(&line);
            }
        }
    }
    Ok(())
}

// ---- frag ---------------------------------------------------------------------

fn frag_suite(sizes: &[usize], samples: usize, out: &mut String) -> BoxResult<()> {
    let interests = [Interest::Keep, Interest::Fragments, Interest::Stream];
    let mut conns = Vec::new();
    for _ in interests {
        conns.push(connect(&WsOptions {
            buf_size: 4 << 20,
            max_message: 1 << 20,
            ping_interval_ms: 0,
            ..WsOptions::default()
        })?);
    }
    let mut sinks: Vec<Sink> = interests.iter().map(|&i| Sink::new(i)).collect();
    out.push_str("text_check=None\n\n| size B | interest | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for &size in sizes {
        let k = k_for(size);
        let l = load_for(Shape::Frag4, size, k)?;
        let before: Vec<u64> = conns.iter().map(|c| c.stats().frag_copy_bytes).collect();
        let h = deliver_paths(&mut conns, &mut sinks, Shape::Frag4, &l, samples)?;
        for (((i, c), b), hist) in interests.iter().zip(&conns).zip(before).zip(&h) {
            let runs = (samples + samples / 10 + 1) * k;
            #[allow(clippy::cast_precision_loss)]
            let copy = (c.stats().frag_copy_bytes - b) as f64 / runs as f64;
            let line = format!(
                "| {size} | {} | {k} | {} | {copy:.0} |\n",
                format!("{i:?}").to_lowercase(),
                hist.cells()
            );
            print!("{line}");
            out.push_str(&line);
        }
    }
    Ok(())
}

// ---- span ---------------------------------------------------------------------

/// `k` messages of `size` as one stream, cut into 16 KiB records at fixed
/// positions: almost every record boundary falls inside a frame.
fn span_load(size: usize, k: usize) -> BoxResult<Load> {
    let msgs: Vec<Vec<u8>> = (0..k).map(|i| message(i, size)).collect();
    let mut stream = Vec::new();
    for m in &msgs {
        frame(OP_TEXT, true, m, &mut stream)?;
    }
    let mut records = Vec::new();
    let mut at = 0;
    while at < stream.len() {
        let end = (at + RECORD).min(stream.len());
        records.push((at, end));
        at = end;
    }
    Ok(Load {
        msgs,
        stream,
        records,
    })
}

fn span_suite(sizes: &[usize], samples: usize, out: &mut String) -> BoxResult<()> {
    let mut conns = Vec::new();
    for back in [true, false] {
        let mut c = connect(&WsOptions {
            buf_size: 4 << 20,
            max_message: 1 << 20,
            ping_interval_ms: 0,
            ..WsOptions::default()
        })?;
        c.set_span_back_only(back);
        conns.push(c);
    }
    let mut sinks = vec![Sink::new(Interest::Keep), Sink::new(Interest::Keep)];
    out.push_str("text_check=None\n\n| size B | path | K | records | ns/message p50 | p99 | max | max sample preempted (Linux) | span copy B/msg |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|\n");
    for &size in sizes {
        // 256 KiB of messages in 16 KiB records.
        let k = (256 * 1024 / (size + 4)).max(1);
        let l = span_load(size, k)?;
        let before: Vec<u64> = conns.iter().map(|c| c.stats().span_copy_bytes).collect();
        let h = deliver_paths(&mut conns, &mut sinks, Shape::Split, &l, samples)?;
        for ((path, (c, b)), hist) in ["old", "new"].iter().zip(conns.iter().zip(before)).zip(&h) {
            let runs = (samples + samples / 10 + 1) * k;
            #[allow(clippy::cast_precision_loss)]
            let copy = (c.stats().span_copy_bytes - b) as f64 / runs as f64;
            let line = format!(
                "| {size} | {path} | {k} | {} | {} | {copy:.0} |\n",
                l.records.len(),
                hist.cells()
            );
            print!("{line}");
            out.push_str(&line);
        }
    }
    Ok(())
}
