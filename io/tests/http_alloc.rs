//! The HTTP request and response paths allocate nothing per request once
//! warm, and the send queue's pages are resident from connect. Its own test
//! binary, for the counting allocator. Every platform: no driver.
#![allow(missing_docs, unsafe_code, clippy::cast_possible_truncation)]

mod http_rig;
mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use ergon_io::{ConnId, Handler, HttpHandler, HttpOptions, RequestTemplate, Response, RxMeta};
use http_rig::HttpRig;
use support::{Lab, R};

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);

// SAFETY: forwards to the system allocator; only adds a relaxed counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Sums what it sees without allocating.
#[derive(Default)]
struct Sum {
    responses: u64,
    bytes: u64,
}

impl Handler for Sum {}

impl HttpHandler for Sum {
    fn on_response(&mut self, _: ConnId, _: u64, r: &Response<'_>, _: &RxMeta) {
        self.responses += 1;
        self.bytes += r.body.len() as u64 + r.headers.len() as u64;
    }
}

fn chunked(body: &[u8], chunk: usize) -> Vec<u8> {
    let mut out = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    for c in body.chunks(chunk) {
        out.extend_from_slice(format!("{:x}\r\n", c.len()).as_bytes());
        out.extend_from_slice(c);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

/// One round's responses, each with its record cuts: a `Content-Length`
/// response with head and body apart, a single chunk whole, several chunks
/// whole, and a chunked body over many records cut inside size lines.
fn round() -> Vec<(Vec<u8>, Vec<usize>)> {
    let body: Vec<u8> = (0..40_000usize).map(|i| b'a' + (i % 26) as u8).collect();
    let mut cl = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", 4096).into_bytes();
    let head = cl.len();
    cl.extend_from_slice(&body[..4096]);
    let streamed = chunked(&body, 3000);
    let cuts: Vec<usize> = (1..10).map(|i| i * 4001).collect();
    vec![
        (cl, vec![head]),
        (chunked(&body[..5000], 5000), vec![]),
        (chunked(&body[..5000], 1000), vec![]),
        (streamed, cuts),
    ]
}

#[test]
fn the_steady_state_request_and_response_paths_do_not_allocate() -> R {
    let lab = Lab::new()?;
    let mut r = HttpRig::new(&lab, HttpOptions::default())?;
    let target = lab.target("")?;
    let tpl = RequestTemplate::new(&target, "POST", "/operation")?;
    let rounds = 50;
    let shapes = round();
    // Seal everything first: forging records allocates, receiving must not.
    let mut reads = Vec::new();
    for _ in 0..rounds {
        for (resp, cuts) in &shapes {
            reads.push(r.records(resp, cuts)?);
        }
    }
    let mut h = Sum::default();
    let headers = [("Content-Type", "application/json"), ("X-Id", "1")];
    let run = |r: &mut HttpRig, recs: &[Vec<u8>], h: &mut Sum| -> R {
        r.conn.request_with(&tpl, "?n=1", &headers, br#"{"a":1}"#)?;
        r.conn.clear_tx();
        for rec in recs {
            r.feed(rec, h)?;
        }
        Ok(())
    };
    let warm = shapes.len();
    for recs in &reads[..warm] {
        run(&mut r, recs, &mut h)?;
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    for recs in &reads[warm..] {
        run(&mut r, recs, &mut h)?;
    }
    let allocs = ALLOCS.load(Ordering::Relaxed) - before;
    assert_eq!(h.responses, (rounds * shapes.len()) as u64);
    assert_eq!(
        allocs,
        0,
        "{allocs} allocations over {} responses",
        reads.len() - warm
    );
    Ok(())
}

/// Every page of the send queue is resident once the connection exists: the
/// first requests take no page fault on the hot path.
#[test]
fn the_send_queue_is_resident_from_connect() -> R {
    let lab = Lab::new()?;
    let mut r = HttpRig::new(&lab, HttpOptions::default())?;
    let (addr, cap) = r.conn.tx_allocation();
    assert!(cap >= 1 << 16, "{cap}");
    // SAFETY: sysconf has no preconditions.
    let page = usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) })?;
    let start = addr & !(page - 1);
    let end = (addr + cap).div_ceil(page) * page;
    let mut vec = vec![0u8; (end - start) / page];
    // SAFETY: the range covers the live allocation `tx_allocation` names,
    // rounded out to whole pages (mincore only reads the page tables), and
    // `vec` has one byte per page.
    let rc = unsafe { libc::mincore(start as *mut _, end - start, vec.as_mut_ptr().cast()) };
    assert_eq!(rc, 0, "mincore: {}", std::io::Error::last_os_error());
    // The first and last pages may be shared with other allocations; every
    // page wholly inside the queue must be resident.
    let inner = &vec[1..vec.len() - 1];
    let missing = inner.iter().filter(|&&b| b & 1 == 0).count();
    assert_eq!(
        missing,
        0,
        "{missing} of {} send-queue pages not resident",
        inner.len()
    );
    Ok(())
}
