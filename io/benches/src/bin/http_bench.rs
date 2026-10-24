//! HTTP/1.1 micro-benchmark: response parse-and-deliver and request build,
//! the previous path against the current one.
//!
//! ```text
//! http_bench [--suite rx|body] [--sizes 100,300,4096,16384] [--samples 20000] [--cpu N] [--out FILE]
//! ```
//!
//! `--suite body` times the body check instead: one `Content-Length`
//! response of each size, pure ASCII and mixed (a two-byte character every
//! 64 bytes), delivered by four connections that differ only in
//! `HttpOptions::body_check` (`None`, `Ascii`, `AsciiChecked`, `Utf8`),
//! interleaved sample by sample, rotating which goes first. Each mode's
//! delivered body and `RxMeta::text` are checked before timing.
//!
//! A real TLS 1.3 handshake on loopback builds the connection; after that
//! nothing touches a socket or the AEAD. Response rows stage each record as
//! an unencrypted stand-in record (untimed: the kernel's copy), then time
//! opening it and parsing with the raw counter (`rdtsc` / `cntvct_el0`),
//! one record at a time as reads would bring them. A sample is K
//! responses; the figure is the sample's ticks / K, in ns. Request rows
//! time K builds of one request's plaintext into a cleared buffer. The two
//! paths of a row run interleaved on the same input, alternating which
//! goes first.
//!
//! | row | input |
//! |---|---|
//! | `chunked N` | a chunked body of N bytes in 4 KiB chunks, in 16 KiB records |
//! | `split 4096` | a `Content-Length` response: head in one record, its 4 KiB body in the next |
//! | `build H` | a POST with H per-request headers and a 64-byte body |
//!
//! | path | what runs |
//! |---|---|
//! | `old` | a copy of the receive path at commit a003bf60 (`HttpConn::parse` and `dechunk`, over the same buffer moves as `TlsStream`: a record joins a non-empty window by moving next to it, compaction when the free tail runs low), or that commit's request builder |
//! | `new` | `HttpConn::deliver_staged`, or `RequestTemplate::write` |
//!
//! Every case is checked before it is timed: delivered bodies and headers
//! equal what was sent, built requests are byte-identical, and the copy
//! counters match the case.

use std::error::Error;
use std::hint::black_box;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use ergon_io::tls::{self, RecordLayer};
use ergon_io::{
    ConnId, Handler, HttpConn, HttpHandler, HttpOptions, NonAscii, RequestTemplate, Response,
    RxMeta, Target, TextCheck, TextStatus,
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
/// The receive buffer of both paths.
const BUF: usize = 4 << 20;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Path {
    Old,
    New,
}

/// Folds what a consumer would touch: body length and ends, header count.
#[derive(Default)]
struct App {
    responses: u64,
    sink: u64,
    last_body: Vec<u8>,
    last_headers: Vec<(String, Vec<u8>)>,
    /// What the body check found for the last response.
    last_text: TextStatus,
    verify: bool,
}

impl Handler for App {}

impl App {
    fn see(&mut self, headers: &[httparse::Header<'_>], body: &[u8]) {
        self.responses += 1;
        let b = black_box(body);
        let ends = b.first().map_or(0, |&x| u64::from(x)) + b.last().map_or(0, |&x| u64::from(x));
        self.sink = self
            .sink
            .wrapping_add(b.len() as u64)
            .wrapping_add(ends)
            .wrapping_add(headers.len() as u64);
        if self.verify {
            self.last_body = body.to_vec();
            self.last_headers = headers
                .iter()
                .map(|h| (h.name.to_owned(), h.value.to_vec()))
                .collect();
        }
    }
}

impl HttpHandler for App {
    #[inline]
    fn on_response(&mut self, _: ConnId, _: u64, r: &Response<'_>, meta: &RxMeta) {
        self.last_text = meta.text;
        self.see(r.headers, r.body);
    }
}

// ---------------------------------------------------------------------------
// The previous receive path: `HttpConn::parse` and `dechunk` at a003bf60,
// verbatim but for the receiver, over a copy of `TlsStream`'s placement,
// consume and compaction (no hold: HTTP never held).
// ---------------------------------------------------------------------------

const MAX_HEADERS: usize = 64;
const HEADER_LEN: usize = 5;
const TAG_LEN: usize = 16;
const MAX_RECORD: usize = (1 << 14) + 256 + HEADER_LEN;

#[derive(Default)]
struct OldStats {
    span_copy_bytes: u64,
    compact_bytes: u64,
    chunk_copy_bytes: u64,
    head_parses: u64,
}

struct OldConn {
    buf: Box<[u8]>,
    head: usize,
    tail: usize,
    p_start: usize,
    p_end: usize,
    body: Vec<u8>,
    need: usize,
    in_flight: usize,
    limit: usize,
    stats: OldStats,
}

enum Framing {
    Length(usize),
    Chunked,
}

impl OldConn {
    fn new() -> Self {
        Self {
            buf: vec![1u8; BUF].into_boxed_slice(),
            head: 0,
            tail: 0,
            p_start: 0,
            p_end: 0,
            body: Vec::with_capacity(1 << 16),
            need: 0,
            in_flight: 0,
            limit: BUF - 2 * MAX_RECORD,
            stats: OldStats::default(),
        }
    }

    fn stage_plain_record(&mut self, plain: &[u8]) -> BoxResult<()> {
        let body = plain.len() + 1 + TAG_LEN;
        let total = HEADER_LEN + body;
        if self.buf.len() - self.tail < total {
            return Err("stand-in record does not fit".into());
        }
        let at = self.tail;
        let [hi, lo] = u16::try_from(body)?.to_be_bytes();
        self.buf[at..at + HEADER_LEN].copy_from_slice(&[23, 3, 3, hi, lo]);
        let p = at + HEADER_LEN;
        self.buf[p..p + plain.len()].copy_from_slice(plain);
        self.buf[p + plain.len()..at + total].fill(0);
        self.tail += total;
        Ok(())
    }

    fn open_next_plain(&mut self) -> bool {
        let b = &self.buf[self.head..self.tail];
        if b.len() < HEADER_LEN {
            return false;
        }
        let len = HEADER_LEN + usize::from(u16::from_be_bytes([b[3], b[4]]));
        if b.len() < len {
            return false;
        }
        let rec_start = self.head;
        self.head += len;
        self.place(rec_start + HEADER_LEN, len - HEADER_LEN - 1 - TAG_LEN);
        true
    }

    fn place(&mut self, src: usize, plen: usize) {
        if self.p_start == self.p_end {
            self.p_start = src;
            self.p_end = src + plen;
        } else {
            self.buf.copy_within(src..src + plen, self.p_end);
            self.p_end += plen;
            self.stats.span_copy_bytes += plen as u64;
        }
    }

    const fn consume(&mut self, n: usize) {
        self.p_start += n;
        if self.p_start >= self.p_end {
            self.p_start = 0;
            self.p_end = 0;
        }
    }

    fn compact(&mut self) {
        let pending_plain = self.p_start != self.p_end;
        if self.head == self.tail && !pending_plain {
            self.head = 0;
            self.tail = 0;
            return;
        }
        if self.buf.len() - self.tail >= 2 * MAX_RECORD {
            return;
        }
        if pending_plain {
            let live = self.p_start;
            let plain = self.p_end - live;
            let cipher = self.tail - self.head;
            self.buf.copy_within(live..self.p_end, 0);
            self.buf.copy_within(self.head..self.tail, plain);
            self.p_start -= live;
            self.p_end = plain;
            self.head = plain;
            self.tail = plain + cipher;
            self.stats.compact_bytes += (plain + cipher) as u64;
        } else {
            let moved = self.tail - self.head;
            self.buf.copy_within(self.head..self.tail, 0);
            self.head = 0;
            self.tail = moved;
            self.stats.compact_bytes += moved as u64;
        }
    }

    fn deliver_staged(&mut self, app: &mut App) -> BoxResult<usize> {
        let mut delivered = 0;
        while self.open_next_plain() {
            delivered += self.parse(app)?;
        }
        self.compact();
        Ok(delivered)
    }

    fn parse(&mut self, app: &mut App) -> BoxResult<usize> {
        let mut delivered = 0;
        loop {
            let window = &self.buf[self.p_start..self.p_end];
            if window.is_empty() || window.len() < self.need {
                return Ok(delivered);
            }
            self.need = 0;
            if self.in_flight == 0 {
                return Err("response without a request".into());
            }
            let mut hdrs = [httparse::EMPTY_HEADER; MAX_HEADERS];
            let mut resp = httparse::Response::new(&mut hdrs);
            self.stats.head_parses += 1;
            let hl = match resp.parse(window)? {
                httparse::Status::Partial => return Ok(delivered),
                httparse::Status::Complete(n) => n,
            };
            let status = resp.code.unwrap_or(0);
            if (100..200).contains(&status) {
                self.consume(hl);
                continue;
            }
            let framing = framing(resp.headers, status)?;
            let limit = self.limit;
            let (body, total) = match framing {
                Framing::Length(len) => {
                    let total = hl
                        .checked_add(len)
                        .filter(|&t| t <= limit)
                        .ok_or("response larger than the buffer")?;
                    if window.len() < total {
                        self.need = total;
                        return Ok(delivered);
                    }
                    (&window[hl..total], total)
                }
                Framing::Chunked => {
                    let found = dechunk(&window[hl..], &mut self.body, limit)?;
                    // The bytes this call moved (the previous code counted
                    // only the last call's).
                    self.stats.chunk_copy_bytes += self.body.len() as u64;
                    let Some(end) = found else {
                        if window.len() > limit {
                            return Err("response larger than the buffer".into());
                        }
                        return Ok(delivered);
                    };
                    (self.body.as_slice(), hl + end)
                }
            };
            app.see(resp.headers, body);
            self.in_flight -= 1;
            self.consume(total);
            delivered += 1;
        }
    }
}

fn framing(headers: &[httparse::Header<'_>], status: u16) -> BoxResult<Framing> {
    if status == 204 || status == 304 {
        return Ok(Framing::Length(0));
    }
    let mut length = None;
    for h in headers {
        if h.name.eq_ignore_ascii_case("transfer-encoding") {
            if h.value.eq_ignore_ascii_case(b"chunked") {
                return Ok(Framing::Chunked);
            }
            return Err("unsupported transfer encoding".into());
        }
        if h.name.eq_ignore_ascii_case("content-length") {
            let v = std::str::from_utf8(h.value)
                .ok()
                .and_then(|s| s.trim().parse::<usize>().ok())
                .ok_or("bad Content-Length")?;
            if length.is_some_and(|l| l != v) {
                return Err("conflicting Content-Length".into());
            }
            length = Some(v);
        }
    }
    Ok(Framing::Length(
        length.ok_or("response without Content-Length")?,
    ))
}

fn dechunk(data: &[u8], out: &mut Vec<u8>, limit: usize) -> BoxResult<Option<usize>> {
    out.clear();
    let mut pos = 0;
    loop {
        let (hdr, size) =
            match httparse::parse_chunk_size(&data[pos..]).map_err(|_| "bad chunk size")? {
                httparse::Status::Partial => return Ok(None),
                httparse::Status::Complete(v) => v,
            };
        let size = usize::try_from(size)
            .ok()
            .filter(|&s| s <= limit)
            .ok_or("chunk too large")?;
        pos += hdr;
        if size == 0 {
            let rest = &data[pos..];
            let Some(end) = rest.windows(2).position(|w| w == b"\r\n") else {
                return Ok(None);
            };
            if end == 0 {
                return Ok(Some(pos + 2));
            }
            let Some(blank) = rest.windows(4).position(|w| w == b"\r\n\r\n") else {
                return Ok(None);
            };
            return Ok(Some(pos + blank + 4));
        }
        let end = pos
            .checked_add(size)
            .and_then(|e| e.checked_add(2))
            .ok_or("chunk too large")?;
        if data.len() < end {
            return Ok(None);
        }
        out.extend_from_slice(&data[pos..end - 2]);
        if &data[end - 2..end] != b"\r\n" {
            return Err("chunk without CRLF".into());
        }
        pos = end;
    }
}

// The previous request builder (`HttpConn::queue` at a003bf60), with its
// validation.

const RESERVED: [&str; 8] = [
    "content-length",
    "transfer-encoding",
    "host",
    "connection",
    "te",
    "upgrade",
    "trailer",
    "expect",
];

const fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn old_validate(method: &str, path: &str, headers: &[(&str, &str)]) -> BoxResult<()> {
    if method.is_empty() || !method.bytes().all(is_tchar) {
        return Err("invalid method".into());
    }
    if path.is_empty() || path.bytes().any(|b| b <= b' ' || b == 0x7F) {
        return Err("invalid request target".into());
    }
    for (name, value) in headers {
        if name.is_empty() || !name.bytes().all(is_tchar) {
            return Err("invalid header name".into());
        }
        if RESERVED.iter().any(|r| name.eq_ignore_ascii_case(r)) {
            return Err("reserved header".into());
        }
        if value.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
            return Err("invalid header value".into());
        }
    }
    Ok(())
}

fn push_decimal(out: &mut Vec<u8>, mut v: usize) {
    let mut digits = [0u8; 20];
    let mut i = digits.len();
    loop {
        i -= 1;
        #[allow(clippy::cast_possible_truncation)]
        {
            digits[i] = b'0' + (v % 10) as u8;
        }
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[i..]);
}

fn old_build(
    out: &mut Vec<u8>,
    fixed: &[u8],
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> BoxResult<()> {
    old_validate(method, path, headers)?;
    let with_length = !body.is_empty() || matches!(method, "POST" | "PUT" | "PATCH");
    out.extend_from_slice(method.as_bytes());
    out.push(b' ');
    out.extend_from_slice(path.as_bytes());
    out.extend_from_slice(b" HTTP/1.1\r\n");
    out.extend_from_slice(fixed);
    for (k, v) in headers {
        out.extend_from_slice(k.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(v.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    if with_length {
        out.extend_from_slice(b"Content-Length: ");
        push_decimal(out, body.len());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    Ok(())
}

// ---------------------------------------------------------------------------
// Inputs and timing.
// ---------------------------------------------------------------------------

fn body_of(len: usize) -> Vec<u8> {
    (0..len).map(|i| b'a' + (i % 26) as u8).collect()
}

/// A chunked response of `len` body bytes in 4 KiB chunks.
fn chunked(len: usize) -> Vec<u8> {
    let body = body_of(len);
    let mut out =
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n"
            .to_vec();
    for c in body.chunks(4096) {
        out.extend_from_slice(format!("{:x}\r\n", c.len()).as_bytes());
        out.extend_from_slice(c);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

#[derive(Clone)]
struct Load {
    name: String,
    /// The records of one response, each at most 16 KiB.
    records: Vec<Vec<u8>>,
    body: Vec<u8>,
    headers: Vec<(String, Vec<u8>)>,
    /// Responses per sample.
    k: usize,
}

fn load(name: String, resp: &[u8], cuts: &[usize], k: usize) -> BoxResult<Load> {
    let mut at: Vec<usize> = cuts.to_vec();
    at.push(0);
    at.push(resp.len());
    at.sort_unstable();
    at.dedup();
    let mut records = Vec::new();
    for w in at.windows(2) {
        for piece in resp[w[0]..w[1]].chunks(RECORD) {
            records.push(piece.to_vec());
        }
    }
    // What the response decodes to, from the reference parser.
    let mut c = OldConn::new();
    let mut app = App {
        verify: true,
        ..App::default()
    };
    c.in_flight = 1;
    for r in &records {
        c.stage_plain_record(r)?;
        c.deliver_staged(&mut app)?;
    }
    if app.responses != 1 {
        return Err(format!("{name}: reference parser delivered {}", app.responses).into());
    }
    Ok(Load {
        name,
        records,
        body: app.last_body,
        headers: app.last_headers,
        k,
    })
}

struct Side {
    h: Histogram<u64>,
    worst: u64,
    worst_preempted: bool,
    chunk_copy: u64,
    span_copy: u64,
    compact: u64,
    parses: u64,
}

fn side() -> BoxResult<Side> {
    Ok(Side {
        h: Histogram::<u64>::new_with_bounds(1, 100_000_000_000, 3)?,
        worst: 0,
        worst_preempted: false,
        chunk_copy: 0,
        span_copy: 0,
        compact: 0,
        parses: 0,
    })
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn record_sample(
    side: &mut Side,
    ticks: u64,
    preempted: bool,
    ns_per_tick_per: f64,
) -> BoxResult<()> {
    let ns = ticks as f64 * ns_per_tick_per;
    let v = ((ns * SCALE) as u64).max(1);
    side.h.record(v)?;
    if v > side.worst {
        side.worst = v;
        side.worst_preempted = preempted;
    }
    Ok(())
}

/// The two receivers of a response row.
struct Rx {
    new: HttpConn,
    old: OldConn,
    app: App,
}

fn rx_sample(
    rx: &mut Rx,
    path: Path,
    l: &Load,
    side: &mut Side,
    record: bool,
    per: f64,
) -> BoxResult<()> {
    let meta = RxMeta::default();
    let mut ticks = 0u64;
    let (_, iv0) = thread_ctx_switches();
    let before = rx.app.responses;
    match path {
        Path::New => {
            for _ in 0..l.k {
                rx.new.request("GET", "/x", &[], &[])?;
                rx.new.clear_tx();
            }
            let s0 = rx.new.stats();
            for _ in 0..l.k {
                for r in &l.records {
                    rx.new.stage_plain_record(r)?;
                    let t0 = rdtsc();
                    let n = rx
                        .new
                        .deliver_staged(ConnId::from_parts(0, 0), &meta, &mut rx.app)?;
                    ticks += rdtsc().wrapping_sub(t0);
                    black_box(n);
                }
            }
            let s1 = rx.new.stats();
            if record {
                side.chunk_copy += s1.chunk_copy_bytes - s0.chunk_copy_bytes;
                side.span_copy += s1.span_copy_bytes - s0.span_copy_bytes;
                side.compact += s1.compact_bytes - s0.compact_bytes;
                side.parses += s1.head_parses - s0.head_parses;
            }
        }
        Path::Old => {
            rx.old.in_flight += l.k;
            let s0 = (
                rx.old.stats.chunk_copy_bytes,
                rx.old.stats.span_copy_bytes,
                rx.old.stats.compact_bytes,
                rx.old.stats.head_parses,
            );
            for _ in 0..l.k {
                for r in &l.records {
                    rx.old.stage_plain_record(r)?;
                    let t0 = rdtsc();
                    let n = rx.old.deliver_staged(&mut rx.app)?;
                    ticks += rdtsc().wrapping_sub(t0);
                    black_box(n);
                }
            }
            if record {
                side.chunk_copy += rx.old.stats.chunk_copy_bytes - s0.0;
                side.span_copy += rx.old.stats.span_copy_bytes - s0.1;
                side.compact += rx.old.stats.compact_bytes - s0.2;
                side.parses += rx.old.stats.head_parses - s0.3;
            }
        }
    }
    let (_, iv1) = thread_ctx_switches();
    if rx.app.responses - before != l.k as u64 {
        return Err(format!(
            "{}: {path:?} delivered {}",
            l.name,
            rx.app.responses - before
        )
        .into());
    }
    if record {
        record_sample(side, ticks, iv1 != iv0, per)?;
    }
    Ok(())
}

struct Row {
    name: String,
    path: Path,
    k: usize,
    p50: f64,
    p99: f64,
    max: f64,
    max_preempted: bool,
    chunk_copy: f64,
    span_copy: f64,
    compact: f64,
    parses: f64,
}

#[allow(clippy::cast_precision_loss)]
fn finish(name: &str, path: Path, k: usize, samples: usize, s: &Side) -> Row {
    let n = (samples * k) as f64;
    Row {
        name: name.to_owned(),
        path,
        k,
        p50: s.h.value_at_quantile(0.50) as f64 / SCALE,
        p99: s.h.value_at_quantile(0.99) as f64 / SCALE,
        max: s.h.max() as f64 / SCALE,
        max_preempted: s.worst_preempted,
        chunk_copy: s.chunk_copy as f64 / n,
        span_copy: s.span_copy as f64 / n,
        compact: s.compact as f64 / n,
        parses: s.parses as f64 / n,
    }
}

/// Correctness first: both paths deliver the reference body and headers.
fn verify_rx(rx: &mut Rx, l: &Load) -> BoxResult<()> {
    for path in [Path::Old, Path::New] {
        rx.app.verify = true;
        let one = Load { k: 1, ..l.clone() };
        let mut s = side()?;
        rx_sample(rx, path, &one, &mut s, true, 1.0)?;
        rx.app.verify = false;
        if rx.app.last_body != l.body || rx.app.last_headers != l.headers {
            return Err(format!("{}: {path:?} delivered a different response", l.name).into());
        }
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn run_rx(rx: &mut Rx, l: &Load, samples: usize) -> BoxResult<[Row; 2]> {
    verify_rx(rx, l)?;
    let mut old = side()?;
    let mut new = side()?;
    let per = 1000.0 / ticks_per_us() / l.k as f64;
    let warm = samples / 10;
    for i in 0..warm + samples {
        let record = i >= warm;
        if i % 2 == 0 {
            rx_sample(rx, Path::Old, l, &mut old, record, per)?;
            rx_sample(rx, Path::New, l, &mut new, record, per)?;
        } else {
            rx_sample(rx, Path::New, l, &mut new, record, per)?;
            rx_sample(rx, Path::Old, l, &mut old, record, per)?;
        }
    }
    Ok([
        finish(&l.name, Path::Old, l.k, samples, &old),
        finish(&l.name, Path::New, l.k, samples, &new),
    ])
}

/// One request kind for the build rows.
struct Build {
    name: String,
    fixed: Vec<u8>,
    tpl: RequestTemplate,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    k: usize,
}

fn build_sample(
    b: &Build,
    hdrs: &[(&str, &str)],
    path: Path,
    out: &mut Vec<u8>,
    side: &mut Side,
    record: bool,
    per: f64,
) -> BoxResult<()> {
    let (_, iv0) = thread_ctx_switches();
    let t0 = rdtsc();
    for _ in 0..b.k {
        out.clear();
        match path {
            Path::Old => old_build(
                black_box(&mut *out),
                black_box(&b.fixed),
                black_box("POST"),
                black_box("/api/order"),
                black_box(hdrs),
                black_box(&b.body),
            )?,
            Path::New => b.tpl.write(
                black_box(&mut *out),
                black_box(""),
                black_box(hdrs),
                black_box(&b.body),
            )?,
        }
        black_box(out.len());
    }
    let ticks = rdtsc().wrapping_sub(t0);
    let (_, iv1) = thread_ctx_switches();
    if record {
        record_sample(side, ticks, iv1 != iv0, per)?;
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn run_build(b: &Build, samples: usize) -> BoxResult<[Row; 2]> {
    let hdrs: Vec<(&str, &str)> = b
        .headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    // Correctness first: the same bytes.
    let mut want = Vec::new();
    old_build(&mut want, &b.fixed, "POST", "/api/order", &hdrs, &b.body)?;
    let mut got = Vec::new();
    b.tpl.write(&mut got, "", &hdrs, &b.body)?;
    if got != want {
        return Err(format!("{}: template output differs", b.name).into());
    }
    let mut out = Vec::with_capacity(1 << 16);
    let mut old = side()?;
    let mut new = side()?;
    let per = 1000.0 / ticks_per_us() / b.k as f64;
    let warm = samples / 10;
    for i in 0..warm + samples {
        let record = i >= warm;
        let order = if i % 2 == 0 {
            [Path::Old, Path::New]
        } else {
            [Path::New, Path::Old]
        };
        for p in order {
            let s = if p == Path::Old { &mut old } else { &mut new };
            build_sample(b, &hdrs, p, &mut out, s, record, per)?;
        }
    }
    Ok([
        finish(&b.name, Path::Old, b.k, samples, &old),
        finish(&b.name, Path::New, b.k, samples, &new),
    ])
}

/// A connection whose keys come from a real handshake on loopback.
fn connect(target: &Target, opts: HttpOptions) -> BoxResult<HttpConn> {
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
    Ok(HttpConn::from_record_layer(sock, layer, target, opts)?)
}

fn fixed_of(t: &Target) -> Vec<u8> {
    let mut f = format!("Host: {}\r\n", t.host_header()).into_bytes();
    for (k, v) in &t.headers {
        f.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    f
}

// ---------------------------------------------------------------------------
// The body check suite.
// ---------------------------------------------------------------------------

/// Feed-like text with a two-byte character every 64 bytes.
fn mixed_body(len: usize) -> Vec<u8> {
    let mut b = body_of(len);
    let mut at = 31;
    while at + 1 < len {
        b[at..at + 2].copy_from_slice("\u{e9}".as_bytes());
        at += 64;
    }
    b
}

/// Responses per sample: about 256 KiB of body, 1 to 64 responses.
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

/// One sample: `l.k` responses through `conn`. Ticks and whether an
/// involuntary switch happened.
fn body_sample(conn: &mut HttpConn, app: &mut App, l: &Load) -> BoxResult<(u64, bool)> {
    let meta = RxMeta::default();
    for _ in 0..l.k {
        conn.request("GET", "/x", &[], &[])?;
        conn.clear_tx();
    }
    let before = app.responses;
    let (_, iv0) = thread_ctx_switches();
    let mut ticks = 0u64;
    for _ in 0..l.k {
        for r in &l.records {
            conn.stage_plain_record(r)?;
            let t0 = rdtsc();
            let n = conn.deliver_staged(ConnId::from_parts(0, 0), &meta, app)?;
            ticks += rdtsc().wrapping_sub(t0);
            black_box(n);
        }
    }
    let (_, iv1) = thread_ctx_switches();
    if app.responses - before != l.k as u64 {
        return Err(format!("{}: delivered {}", l.name, app.responses - before).into());
    }
    Ok((ticks, iv1 != iv0))
}

/// What `c` flags a pure-ASCII or a mixed body as.
const fn expected(c: TextCheck, is_mixed: bool) -> TextStatus {
    match c {
        TextCheck::None => TextStatus::Unchecked,
        TextCheck::Ascii => TextStatus::AssumedAscii,
        TextCheck::AsciiChecked(_) if is_mixed => TextStatus::NotAscii,
        TextCheck::AsciiChecked(_) => TextStatus::Ascii,
        TextCheck::Utf8 => TextStatus::Utf8,
    }
}

const fn check_name(c: TextCheck) -> &'static str {
    match c {
        TextCheck::None => "none",
        TextCheck::Ascii => "ascii",
        TextCheck::AsciiChecked(_) => "ascii-checked",
        TextCheck::Utf8 => "utf8",
    }
}

#[allow(clippy::cast_precision_loss)]
fn body_suite(target: &Target, sizes: &[usize], samples: usize, out: &mut String) -> BoxResult<()> {
    let checks = [
        TextCheck::None,
        TextCheck::Ascii,
        TextCheck::AsciiChecked(NonAscii::DeliverUnchecked),
        TextCheck::Utf8,
    ];
    let mut conns = Vec::new();
    for c in checks {
        conns.push(connect(
            target,
            HttpOptions {
                buf_size: BUF,
                max_in_flight: 64,
                body_check: c,
                ..HttpOptions::default()
            },
        )?);
    }
    let mut app = App::default();
    out.push_str("| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|\n");
    for &size in sizes {
        for is_mixed in [false, true] {
            let body = if is_mixed {
                mixed_body(size)
            } else {
                body_of(size)
            };
            let mut resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {size}\r\n\r\n"
            )
            .into_bytes();
            resp.extend_from_slice(&body);
            let text = if is_mixed { "mixed" } else { "ascii" };
            let l = load(format!("body {size} {text}"), &resp, &[], k_for(size))?;
            // Correctness first: every mode delivers the body intact and
            // flags it as it should.
            for (c, conn) in checks.iter().zip(conns.iter_mut()) {
                app.verify = true;
                let one = Load { k: 1, ..l.clone() };
                body_sample(conn, &mut app, &one)?;
                app.verify = false;
                let want = expected(*c, is_mixed);
                if app.last_body != body || app.last_text != want {
                    return Err(format!("{}: {c:?} delivered {:?}", l.name, app.last_text).into());
                }
            }
            let mut sides: Vec<Side> = checks.iter().map(|_| side()).collect::<BoxResult<_>>()?;
            let per = 1000.0 / ticks_per_us() / l.k as f64;
            let warm = samples / 10;
            for i in 0..warm + samples {
                for j in 0..checks.len() {
                    let p = (i + j) % checks.len();
                    let (t, pre) = body_sample(&mut conns[p], &mut app, &l)?;
                    if i >= warm {
                        record_sample(&mut sides[p], t, pre, per)?;
                    }
                }
            }
            for (c, s) in checks.iter().zip(&sides) {
                let r = finish(&l.name, Path::New, l.k, samples, s);
                let want = expected(*c, is_mixed);
                let line = format!(
                    "| {size} | {text} | {} | {want:?} | {} | {:.1} | {:.1} | {:.1} | {} |\n",
                    check_name(*c),
                    r.k,
                    r.p50,
                    r.p99,
                    r.max,
                    preempted(r.max_preempted),
                );
                print!("{line}");
                out.push_str(&line);
            }
        }
    }
    black_box(app.sink);
    Ok(())
}

fn main() -> BoxResult<()> {
    let a = Args::from_env();
    let samples: usize = a.num_or("samples", 20_000)?;
    if let Ok(cpu) = a.req("cpu") {
        pin_current_thread(cpu.parse()?)?;
    }
    let target =
        Target::new("api.example.com", 443, "").with_header("X-API-Key", "0123456789abcdef");
    if a.str_or("suite", "rx") == "body" {
        let sizes = a
            .list("sizes", "100,300,4096,16384")
            .iter()
            .map(|s| s.parse())
            .collect::<Result<Vec<usize>, _>>()?;
        let _ = ticks_per_us();
        let mut out = String::new();
        out.push_str(&counter_meta());
        out.push_str(&format!("suite=body\nsamples={samples}\n\n"));
        body_suite(&target, &sizes, samples, &mut out)?;
        if let Ok(path) = a.req("out") {
            std::fs::write(path, out)?;
        }
        return Ok(());
    }
    let opts = HttpOptions {
        buf_size: BUF,
        max_in_flight: 64,
        ..HttpOptions::default()
    };
    let mut rx = Rx {
        new: connect(&target, opts)?,
        old: OldConn::new(),
        app: App::default(),
    };
    let _ = ticks_per_us();
    let mut out = String::new();
    out.push_str(&counter_meta());
    out.push_str(&format!("samples={samples}\n\n"));
    out.push_str("| row | path | K | ns/response p50 | p99 | max | max sample preempted (Linux) | de-chunk copy B/resp | span copy B/resp | compaction B/resp | head parses/resp |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|---|\n");
    let mut loads = Vec::new();
    for (len, k) in [(4096usize, 32usize), (64 << 10, 8), (1 << 20, 1)] {
        loads.push(load(format!("chunked {len}"), &chunked(len), &[], k)?);
    }
    let body = body_of(4096);
    let mut split =
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 4096\r\n\r\n"
            .to_vec();
    let head = split.len();
    split.extend_from_slice(&body);
    loads.push(load("split 4096".into(), &split, &[head], 32)?);
    let mut lines = Vec::new();
    for l in &loads {
        lines.extend(run_rx(&mut rx, l, samples)?);
    }
    black_box(rx.app.sink);
    for r in &lines {
        let line = format!(
            "| {} | {} | {} | {:.1} | {:.1} | {:.1} | {} | {:.0} | {:.0} | {:.0} | {:.2} |\n",
            r.name,
            format!("{:?}", r.path).to_lowercase(),
            r.k,
            r.p50,
            r.p99,
            r.max,
            preempted(r.max_preempted),
            r.chunk_copy,
            r.span_copy,
            r.compact,
            r.parses,
        );
        print!("{line}");
        out.push_str(&line);
    }
    out.push_str(
        "\n| row | path | K | ns/request p50 | p99 | max | max sample preempted (Linux) |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|\n");
    for n in [3usize, 20] {
        let headers: Vec<(String, String)> = (0..n)
            .map(|i| match i {
                0 => ("Content-Type".to_owned(), "application/json".to_owned()),
                1 => ("Accept".to_owned(), "application/json".to_owned()),
                _ => (format!("X-Field-{i}"), format!("value-{i:04}")),
            })
            .collect();
        let b = Build {
            name: format!("build {n}"),
            fixed: fixed_of(&target),
            tpl: RequestTemplate::new(&target, "POST", "/api/order")?,
            headers,
            body: body_of(64),
            k: 64,
        };
        for r in run_build(&b, samples)? {
            let line = format!(
                "| {} | {} | {} | {:.1} | {:.1} | {:.1} | {} |\n",
                r.name,
                format!("{:?}", r.path).to_lowercase(),
                r.k,
                r.p50,
                r.p99,
                r.max,
                preempted(r.max_preempted),
            );
            print!("{line}");
            out.push_str(&line);
        }
    }
    if let Ok(path) = a.req("out") {
        std::fs::write(path, out)?;
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
