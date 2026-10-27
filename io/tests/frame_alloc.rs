//! The framed-TCP receive and send paths allocate nothing once warm: fed
//! without a driver (every platform), and through the driver over loopback
//! (Linux). Its own test binary, for the counting allocator.
#![allow(missing_docs, unsafe_code, clippy::cast_possible_truncation)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::net::{TcpListener, TcpStream};

use ergon_io::{ConnId, FrameConn, FrameHandler, FrameOptions, Handler, RxMeta};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Counting;

thread_local! {
    /// This thread's allocations: ergon-io's helper threads, and tests
    /// running in parallel, do not count.
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
}

fn allocs() -> u64 {
    ALLOCS.with(Cell::get)
}

// SAFETY: forwards to the system allocator; only adds a thread-local counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count();
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count();
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Counts and sums what it sees without allocating.
#[derive(Default)]
struct Sum {
    n: u64,
    bytes: u64,
    closed: u64,
}

impl Handler for Sum {
    fn on_closed(&mut self, _: ConnId, _: ergon_io::Error) {
        self.closed += 1;
    }
}

impl FrameHandler for Sum {
    fn on_frame(&mut self, _: ConnId, payload: &[u8], _: &RxMeta) {
        self.n += 1;
        self.bytes += payload.len() as u64;
    }
}

fn wire(payload: &[u8]) -> Vec<u8> {
    let mut out = (payload.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(payload);
    out
}

/// Reads of frames cut at awkward places (inside the length, inside the
/// payload, several frames per read) and `send_frame` of every size:
/// nothing allocates once warm, compaction included.
#[test]
fn the_steady_state_frame_paths_do_not_allocate() -> R {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let client = TcpStream::connect(listener.local_addr()?)?;
    let _peer = listener.accept()?;
    let opts = FrameOptions {
        max_frame: 2_048,
        rx_buf: 2_052,
        ..FrameOptions::default()
    };
    let mut conn = FrameConn::from_stream(client, opts)?;
    // Built first: forging the reads allocates, delivering them must not.
    let stream: Vec<u8> = (0..400usize)
        .flat_map(|i| wire(&vec![i as u8; (i * 37) % 2_049]))
        .collect();
    let mut reads = Vec::new();
    let mut at = 0;
    let mut cut = 0usize;
    while at < stream.len() {
        let len = [1usize, 3, 700, 2_000, 5, 1_500][cut % 6].min(stream.len() - at);
        reads.push(stream[at..at + len].to_vec());
        at += len;
        cut += 1;
    }
    let payload = vec![7u8; 2_048];
    let mut h = Sum::default();
    let id = ConnId::default();
    let run = |conn: &mut FrameConn, read: &[u8], h: &mut Sum| -> R {
        let mut rest = read;
        while !rest.is_empty() {
            let take = rest.len().min(conn.free());
            conn.feed(id, &rest[..take], &RxMeta::default(), h)?;
            rest = &rest[take..];
        }
        conn.send_frame(&payload[..read.len().min(2_048)])?;
        conn.clear_tx();
        Ok(())
    };
    let warm = reads.len() / 4;
    for r in &reads[..warm] {
        run(&mut conn, r, &mut h)?;
    }
    let before = allocs();
    for r in &reads[warm..] {
        run(&mut conn, r, &mut h)?;
    }
    let n = allocs() - before;
    assert_eq!(h.n, 400);
    assert!(conn.stats().compact_bytes > 0, "nothing compacted");
    assert_eq!(n, 0, "{n} allocations over {} reads", reads.len() - warm);
    Ok(())
}

/// Ping-pong through the driver (`epoll-spin`): an accepted socket added,
/// the client dialled; after a warm-up, 5 000 round trips allocate nothing
/// on the polling thread.
#[cfg(target_os = "linux")]
#[test]
fn the_steady_state_driver_round_trip_does_not_allocate() -> R {
    use std::time::{Duration, Instant};

    use ergon_io::{Config, Driver};

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let client = TcpStream::connect(listener.local_addr()?)?;
    let (server, _) = listener.accept()?;
    let mut d: Driver<FrameConn> = Driver::new(Config::parse(
        "epoll-spin,rx_timestamps=true,rx_read_timestamp=true",
    )?)?;
    let first = d.add(FrameConn::from_stream(client, FrameOptions::default())?)?;
    let second = d.add(FrameConn::from_stream(server, FrameOptions::default())?)?;
    let payload = [0x5Au8; 64];
    let mut h = Sum::default();
    let trip = |d: &mut Driver<FrameConn>, h: &mut Sum| -> R {
        for (from, want) in [(first, h.n + 1), (second, h.n + 2)] {
            d.get_mut(from).ok_or("closed")?.send_frame(&payload)?;
            let end = Instant::now() + Duration::from_secs(5);
            while h.n < want {
                d.poll(h)?;
                if Instant::now() > end {
                    return Err("a frame never arrived".into());
                }
            }
        }
        Ok(())
    };
    for _ in 0..200 {
        trip(&mut d, &mut h)?;
    }
    let before = allocs();
    for _ in 0..5_000 {
        trip(&mut d, &mut h)?;
    }
    let n = allocs() - before;
    assert_eq!(h.closed, 0);
    assert_eq!(h.n, 2 * 5_200);
    assert_eq!(n, 0, "{n} allocations over 5000 round trips");
    Ok(())
}
