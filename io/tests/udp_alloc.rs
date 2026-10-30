//! The UDP send and receive paths through the driver allocate nothing once
//! warm. Its own test binary, for the counting allocator.
#![cfg(target_os = "linux")]
#![allow(missing_docs, unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use ergon_io::{Config, ConnId, Driver, Handler, RxMeta, UdpConn, UdpHandler, UdpOptions};

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

impl UdpHandler for Sum {
    fn on_datagram(&mut self, _: ConnId, payload: &[u8], meta: &RxMeta) {
        self.n += 1;
        self.bytes += payload.len() as u64 + u64::from(meta.read_seq > 0);
    }
}

/// Ping-pong between two connected sockets in one driver: send, poll until
/// it arrives, send back. After a warm-up, 5 000 round trips allocate
/// nothing on the polling thread, with receive timestamps and ergon-io's
/// own receive stamp on.
#[test]
fn the_steady_state_udp_round_trip_does_not_allocate() -> R {
    let loopback = SocketAddr::from(([127, 0, 0, 1], 0));
    let (sa, sb) = (UdpSocket::bind(loopback)?, UdpSocket::bind(loopback)?);
    let (pa, pb) = (sa.local_addr()?, sb.local_addr()?);
    drop((sa, sb));
    let opts = UdpOptions::default();
    let mut d: Driver<UdpConn> = Driver::new(Config::parse(
        "epoll-spin,rx_timestamps=true,rx_read_timestamp=true",
    )?)?;
    let first = d.add(UdpConn::connect(pa, pb, &opts)?)?;
    let second = d.add(UdpConn::connect(pb, pa, &opts)?)?;
    let payload = [0x5Au8; 64];
    let mut h = Sum::default();
    let trip = |d: &mut Driver<UdpConn>, h: &mut Sum| -> R {
        for (from, want) in [(first, h.n + 1), (second, h.n + 2)] {
            d.get_mut(from).ok_or("closed")?.send(&payload)?;
            let end = Instant::now() + Duration::from_secs(5);
            while h.n < want {
                d.poll(h)?;
                if Instant::now() > end {
                    return Err("a datagram never arrived".into());
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

/// Bursts of 8 datagrams each way, sent in one `sendmmsg` and read in one
/// `recvmmsg`: after a warm-up, 2 000 bursts allocate nothing.
#[test]
fn batched_bursts_do_not_allocate() -> R {
    let loopback = SocketAddr::from(([127, 0, 0, 1], 0));
    let (sa, sb) = (UdpSocket::bind(loopback)?, UdpSocket::bind(loopback)?);
    let (pa, pb) = (sa.local_addr()?, sb.local_addr()?);
    drop((sa, sb));
    let opts = UdpOptions::default();
    let mut d: Driver<UdpConn> = Driver::new(Config::parse(
        "epoll-spin,rx_timestamps=true,rx_read_timestamp=true",
    )?)?;
    let first = d.add(UdpConn::connect(pa, pb, &opts)?)?;
    let second = d.add(UdpConn::connect(pb, pa, &opts)?)?;
    let payload = [0x5Au8; 64];
    let mut h = Sum::default();
    let burst = |d: &mut Driver<UdpConn>, h: &mut Sum| -> R {
        for from in [first, second] {
            let want = h.n + 8;
            for _ in 0..8 {
                d.get_mut(from).ok_or("closed")?.send(&payload)?;
            }
            let end = Instant::now() + Duration::from_secs(5);
            while h.n < want {
                d.poll(h)?;
                if Instant::now() > end {
                    return Err("a burst never arrived".into());
                }
            }
        }
        Ok(())
    };
    for _ in 0..100 {
        burst(&mut d, &mut h)?;
    }
    let calls = d.stats().send_calls;
    let before = allocs();
    for _ in 0..2_000 {
        burst(&mut d, &mut h)?;
    }
    let n = allocs() - before;
    assert_eq!(h.closed, 0);
    assert_eq!(h.n, 16 * 2_100);
    assert_eq!(
        d.stats().send_calls - calls,
        2 * 2_000,
        "one send call a burst"
    );
    assert_eq!(n, 0, "{n} allocations over 2000 bursts");
    Ok(())
}
