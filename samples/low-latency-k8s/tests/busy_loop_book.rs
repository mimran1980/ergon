//! The busy-spin loop `book/src/deploy/busy-spin-loop.md` includes,
//! compiled and run here: one pinned thread polls an ergon-io `Driver`
//! (a UDP feed), an Aeron subscription and publication, does the app's
//! work and fires its timers; a housekeeping thread runs ergon-tune's
//! `ThrottleWatch` and `FaultWatch` off the loop's CPU.
//!
//! The test thread plays the venue: it sends quotes over UDP, reads the
//! orders the loop publishes on Aeron and answers each with an ack.
//! An embedded media driver stands in for the pod's driver. Linux only
//! (`Driver` is Linux-only).
#![cfg(target_os = "linux")]

use std::collections::VecDeque;
use std::ffi::CString;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ergon_io::placement::process_cpus;
use ergon_io::{ConnId, Driver, Handler, RxMeta, UdpConn, UdpHandler, UdpOptions};
use ergon_tune::{FaultWatch, Host, Role, Thread, ThrottleWatch};
use rusteron_client::{Aeron, AeronContext, AeronPublication, AeronSubscription, Handlers};
use rusteron_media_driver::testing::EmbeddedDriver;

type R<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const ORDERS: i32 = 3001;
const ACKS: i32 = 3002;
const QUOTES: u64 = 50;

// ANCHOR: app
/// The application's state and logic. The loop hands it every event; it
/// never blocks, never allocates and never reads the system clock itself.
struct App {
    /// Orders decided but not yet in the Aeron log. Reserved up front.
    out: VecDeque<[u8; 16]>,
    quotes: u64,
    acks: u64,
    last_acked: u64,
    ticks: u64,
}

impl App {
    fn new() -> Self {
        Self {
            out: VecDeque::with_capacity(1024),
            quotes: 0,
            acks: 0,
            last_acked: 0,
            ticks: 0,
        }
    }

    /// A quote: `seq`, `price`. This app orders on every even price.
    fn on_quote(&mut self, quote: &[u8]) {
        let field = |at: usize| {
            quote
                .get(at..at + 8)
                .and_then(|b| b.try_into().ok())
                .map(u64::from_le_bytes)
        };
        if let (Some(seq), Some(price)) = (field(0), field(8)) {
            self.quotes += 1;
            if price % 2 == 0 && self.out.len() < self.out.capacity() {
                let mut order = [0_u8; 16];
                order[..8].copy_from_slice(&seq.to_le_bytes());
                order[8..].copy_from_slice(&price.to_le_bytes());
                self.out.push_back(order);
            }
        }
    }

    /// An ack: the order's `seq`.
    fn on_ack(&mut self, ack: &[u8]) {
        if let Some(seq) = ack.get(..8).and_then(|b| b.try_into().ok()) {
            self.last_acked = u64::from_le_bytes(seq);
            self.acks += 1;
        }
    }

    /// A timer: short and bounded, like any other event.
    const fn on_timer(&mut self) {
        self.ticks += 1;
    }
}

impl Handler for App {}

impl UdpHandler for App {
    fn on_datagram(&mut self, _conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        self.on_quote(payload);
    }
}
// ANCHOR_END: app

// ANCHOR: loop
/// Fragments a turn may take from one subscription.
const FRAGMENT_LIMIT: usize = 10;
/// The timer's period.
const TIMER: Duration = Duration::from_millis(1);

/// The duty cycle: every turn does each step once, in this order, and
/// nothing else. No step waits: a retryable Aeron error leaves the order
/// queued for the next turn.
fn run(
    driver: &mut Driver<UdpConn>,
    publication: &AeronPublication,
    subscription: &AeronSubscription,
    app: &mut App,
    stop: &AtomicBool,
) -> R {
    let clock = quanta::Clock::new(); // the TSC, calibrated once, here
    let mut next_timer = clock.now() + TIMER;
    while !stop.load(Ordering::Relaxed) {
        // 1. Network I/O: receive, run the handler's callbacks, flush sends.
        driver.poll(app)?;
        // 2. Aeron in: up to FRAGMENT_LIMIT fragments.
        subscription.poll_fn(|ack, _header| app.on_ack(ack), FRAGMENT_LIMIT)?;
        // 3. Aeron out: what the callbacks decided, oldest first.
        while let Some(order) = app.out.front() {
            match publication.offer(order) {
                Ok(_) => {
                    app.out.pop_front();
                }
                Err(e) if e.is_retryable() => break, // back-pressure: next turn
                Err(e) => return Err(e.into()),
            }
        }
        // 4. Timers: one TSC read and a compare.
        let now = clock.now();
        if now >= next_timer {
            app.on_timer();
            next_timer = now + TIMER;
        }
    }
    Ok(())
}
// ANCHOR_END: loop

// ANCHOR: housekeeping
/// Housekeeping: the host checks that must run again while the loop runs,
/// on a thread that never shares the loop's CPU. Each check reads one
/// already-open file; a finding is printed here, never by the loop.
fn spawn_housekeeping(loop_tid: u32, stop: Arc<AtomicBool>) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("housekeeping".to_owned())
        .spawn(move || {
            let host = Host::system();
            // T9: the container's CPU quota throttled it since the last check.
            let mut throttle = ThrottleWatch::open(&host).ok();
            // T19: the loop thread took a page fault since the last check.
            let mut faults = FaultWatch::open(&host, Thread::Task(loop_tid), Role::Loop).ok();
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100)); // a few seconds in production
                let throttled = throttle.as_mut().and_then(ThrottleWatch::check);
                let faulted = faults.as_mut().and_then(FaultWatch::check);
                for finding in throttled.iter().chain(faulted.iter()) {
                    eprintln!("housekeeping: {finding}");
                }
            }
        })
}
// ANCHOR_END: housekeeping

// ANCHOR: start
/// The loop thread, from start to stop. A thread inherits its creator's
/// CPU mask, so the loop thread first moves to the helper CPU, creates
/// every other thread there, and only then pins itself to its own CPU.
fn loop_thread(
    aeron_dir: &str,
    (local, feed): (SocketAddr, SocketAddr),
    ready: &AtomicBool,
    stop: &Arc<AtomicBool>,
) -> R<App> {
    // 1. The CPUs: the loop's, and one for everything else. In a pod these
    //    come from its cpuset; with the shared-CPU plugin, from ERGON_*.
    let (cpus, _) = process_cpus(&Host::system());
    let loop_cpu = cpus.iter().last().ok_or("no CPU for the loop")?;
    if let Some(helper) = cpus.iter().find(|&c| c != loop_cpu) {
        ergon_io::clock::pin_current_thread(usize::try_from(helper)?)?;
    } // else a `cpu: 1` pod: the helpers share the loop's CPU
    // 2. Housekeeping and the Aeron client conductor, on the helper CPU.
    let tid = u32::try_from(rustix::thread::gettid().as_raw_nonzero().get())?;
    let housekeeping = spawn_housekeeping(tid, Arc::clone(stop))?;
    let context = AeronContext::new()?;
    context.set_dir(&CString::new(aeron_dir)?)?;
    let aeron = Aeron::new(&context)?;
    aeron.start()?;
    let publication = aeron
        .async_add_publication(c"aeron:ipc", ORDERS)?
        .poll_blocking(Duration::from_secs(5))?;
    let subscription = aeron
        .async_add_subscription(c"aeron:ipc", ACKS, Handlers::NONE, Handlers::NONE)?
        .poll_blocking(Duration::from_secs(5))?;
    // 3. Pin to the loop's CPU, then create the Driver: it reads that CPU
    //    for its host checks and keeps its own helpers off it.
    ergon_io::clock::pin_current_thread(usize::try_from(loop_cpu)?)?;
    let mut driver: Driver<UdpConn> = Driver::new(ergon_io::Config::parse("epoll-spin")?)?;
    driver.add(UdpConn::connect(local, feed, &UdpOptions::default())?)?;
    ready.store(true, Ordering::Release);
    // 4. Run until told to stop; then close in order.
    let mut app = App::new();
    let ran = run(&mut driver, &publication, &subscription, &mut app, stop);
    stop.store(true, Ordering::Relaxed);
    let _ = housekeeping.join();
    drop(driver);
    publication.close()?;
    subscription.close()?;
    aeron.close()?;
    ran.map(|()| app)
}
// ANCHOR_END: start

/// The venue side, on the test thread: quotes over UDP, an ack on Aeron
/// for every order.
#[test]
fn one_loop_feed_aeron_timers_and_housekeeping() -> R {
    let media_driver = EmbeddedDriver::launch()?;
    let dir = media_driver.dir().to_owned();
    let venue_udp = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    let feed = venue_udp.local_addr()?;
    // The loop's own address: a free port, taken by the loop's socket.
    let local = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))?.local_addr()?;
    venue_udp.connect(local)?;
    let stop = Arc::new(AtomicBool::new(false));
    let ready = Arc::new(AtomicBool::new(false));
    let (loop_stop, loop_ready) = (Arc::clone(&stop), Arc::clone(&ready));
    let hot = std::thread::Builder::new()
        .name("loop".to_owned())
        .spawn(move || loop_thread(&dir, (local, feed), &loop_ready, &loop_stop))?;

    let aeron = {
        let context = AeronContext::new()?;
        context.set_dir(&CString::new(media_driver.dir())?)?;
        let aeron = Aeron::new(&context)?;
        aeron.start()?;
        aeron
    };
    let orders = aeron
        .async_add_subscription(c"aeron:ipc", ORDERS, Handlers::NONE, Handlers::NONE)?
        .poll_blocking(Duration::from_secs(5))?;
    let acks = aeron
        .async_add_publication(c"aeron:ipc", ACKS)?
        .poll_blocking(Duration::from_secs(5))?;

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut sent = 0_u64;
    let mut ordered = 0_u64;
    let mut acked = 0_u64;
    let wanted = QUOTES / 2;
    while acked < wanted {
        if Instant::now() > deadline || hot.is_finished() {
            stop.store(true, Ordering::Relaxed);
            let why = hot.join().map_err(|_| "loop panicked")?.err();
            return Err(format!("sent {sent}, ordered {ordered}, acked {acked}: {why:?}").into());
        }
        // Quotes go once the loop's socket exists and Aeron is connected.
        if sent < QUOTES
            && ready.load(Ordering::Acquire)
            && acks.is_connected()
            && orders.is_connected()
        {
            let mut quote = [0_u8; 16];
            quote[..8].copy_from_slice(&sent.to_le_bytes());
            quote[8..].copy_from_slice(&sent.to_le_bytes());
            if venue_udp.send(&quote).is_ok() {
                sent += 1;
            }
        }
        orders.poll_fn(|_, _| ordered += 1, 10)?;
        while acked < ordered {
            // Order `n` is the quote with seq `2n` (even prices only).
            if acks.offer(&(acked * 2).to_le_bytes()).is_err() {
                break;
            }
            acked += 1;
        }
    }
    // Long enough for the 1 ms timer to fire a few times.
    std::thread::sleep(Duration::from_millis(20));
    stop.store(true, Ordering::Relaxed);
    let app = hot.join().map_err(|_| "loop panicked")??;
    assert_eq!(app.quotes, QUOTES);
    assert_eq!(app.acks, wanted);
    assert_eq!(app.last_acked, QUOTES - 2);
    assert!(app.ticks > 0, "the timer never fired");
    Ok(())
}
