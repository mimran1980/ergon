//! The live runtime against a real media driver: messages, one-shot and
//! repeating timers, cancel, outputs, stop, and no allocation in a steady
//! duty cycle.
//!
//! Needs an Aeron media driver at `AERON_TEST_DIR` (default
//! `/tmp/persist-test-aeron`); `just test` starts it. Fails, never skips,
//! without it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::error::Error;
use std::time::{Duration, Instant};

use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::Nanos;
use ergon_runtime::idle::Idle;
use ergon_runtime::publication::Publication;
use ergon_runtime::rt::{Agent, Config, Ctx, Expiry, FeedId, Out, Runtime};
use ergon_runtime::streams::Streams;
use ergon_runtime::subscription::{Delivery, Subscription};
use ergon_runtime::timer::TimerId;

type TestResult = Result<(), Box<dyn Error>>;

thread_local! {
    /// Count this thread's allocations only: the Aeron client's own threads
    /// and other tests are not the duty cycle.
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    /// This thread's counted allocations.
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    if COUNTING.with(Cell::get) {
        ALLOCS.with(|n| n.set(n.get() + 1));
    }
}

struct Counting;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the system allocator is the delegate, with the caller's layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: as `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator for `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: `ptr` came from this allocator for `layout`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const IPC: &str = "aeron:ipc?term-length=1m";
const FRAME: usize = 32;
const MS: i64 = 1_000_000;

fn aeron_dir() -> String {
    std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into())
}

/// Streams unique to this test in this run.
fn stream(test: i32, n: i32) -> i32 {
    40_000 + (std::process::id() % 20_000).cast_signed() * 16 + test * 4 + n
}

/// Echoes every message to `out`, and keeps what its timers did.
#[derive(Default)]
struct Probe {
    out: Option<Out>,
    seen: u64,
    one_shot: Vec<(u64, Nanos, Nanos)>,
    repeats: u64,
    repeating: Option<TimerId>,
    cancelled_fired: bool,
    stop_after: u64,
}

impl Probe {
    const fn maybe_stop(&self, ctx: &mut Ctx) {
        if self.stop_after > 0 && self.seen >= self.stop_after && self.repeats >= 3 {
            ctx.stop();
        }
    }
}

impl Agent for Probe {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        let bad = |e: ergon_runtime::timer::TimerError| ergon_runtime::Error::Config(e.to_string());
        ctx.after(2 * MS, 1).map_err(bad)?;
        ctx.after(MS, 2).map_err(bad)?;
        let doomed = ctx.after(MS, 3).map_err(bad)?;
        assert!(ctx.cancel(doomed));
        self.repeating = Some(ctx.every(MS, 4).map_err(bad)?);
        Ok(())
    }

    fn on_message(&mut self, ctx: &mut Ctx, _feed: FeedId, msg: &[u8], _d: Delivery) {
        // The publication's `Source` message comes first; only data counts.
        if msg.get(2..4) != Some(&1u16.to_le_bytes()[..]) {
            return;
        }
        self.seen += 1;
        if let Some(out) = self.out {
            let _ = ctx.send(out, 1, FRAME, |buf| {
                buf.copy_from_slice(&msg[..FRAME]);
                Ok::<_, std::convert::Infallible>(FRAME)
            });
        }
        self.maybe_stop(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry) {
        match timer.token {
            4 => {
                self.repeats += 1;
                if self.repeats == 3
                    && let Some(id) = self.repeating
                {
                    assert!(
                        ctx.cancel(id),
                        "a repeating timer cancels from its own firing"
                    );
                }
            }
            3 => self.cancelled_fired = true,
            token => self.one_shot.push((token, timer.deadline, ctx.now())),
        }
        self.maybe_stop(ctx);
    }
}

struct Rig {
    rt: Runtime,
    feed: Publication,
    echo: Subscription,
}

fn rig(test: i32, invoker: bool) -> Result<Rig, Box<dyn Error>> {
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        aeron_invoker: invoker,
        app: "runtime-live-test".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    let feed = bus.publication(IPC, stream(test, 0))?;
    let echo = bus.subscription(IPC, stream(test, 1));
    let mut rt = Runtime::new(Config {
        idle: Idle::Noop,
        ..Config::new(bus, Streams::parse("services: {}\nkinds: {}\n")?)
    })?;
    rt.ctx().subscribe_channel(IPC, stream(test, 0));
    Ok(Rig { rt, feed, echo })
}

fn publish(feed: &Publication, n: u64) {
    for i in 0..n {
        let _ = feed.record(1, FRAME, |buf| {
            buf[2..4].copy_from_slice(&1u16.to_le_bytes());
            buf[8..16].copy_from_slice(&i.to_le_bytes());
            Ok::<_, std::convert::Infallible>(FRAME)
        });
    }
}

/// Drive the runtime until `done`, publishing nothing.
fn drive(
    rig: &mut Rig,
    probe: &mut Probe,
    echoed: &mut u64,
    done: impl Fn(&Probe, u64) -> bool,
) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(probe, *echoed) {
        rig.rt.cycle(probe);
        *echoed += rig.echo.poll(|_, _| {}, 256) as u64;
        if Instant::now() > deadline {
            return Err(format!(
                "not done within 10 s: seen {}, echoed {echoed}, repeats {}",
                probe.seen, probe.repeats
            )
            .into());
        }
    }
    Ok(())
}

fn connect(rig: &mut Rig, probe: &mut Probe) -> TestResult {
    let echo_stream = rig.feed.stream_id() + 1;
    let out = rig.rt.ctx().publish_channel(IPC, echo_stream)?;
    probe.out = Some(out);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(rig.feed.is_connected() && rig.rt.ctx().is_connected(out) && rig.echo.is_connected()) {
        rig.rt.cycle(probe);
        rig.echo.poll(|_, _| {}, 256);
        if Instant::now() > deadline {
            return Err("the IPC streams did not connect within 10 s".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    // The echo stream starts with the runtime's `Source` message.
    while rig.echo.poll(|_, _| {}, 256) > 0 {}
    Ok(())
}

#[test]
fn messages_timers_outputs_and_stop() -> TestResult {
    for invoker in [false, true] {
        let mut rig = rig(i32::from(invoker), invoker)?;
        let mut probe = Probe {
            stop_after: 100,
            ..Probe::default()
        };
        rig.rt.start(&mut probe)?;
        connect(&mut rig, &mut probe)?;
        publish(&rig.feed, 100);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !rig.rt.is_stopping() && Instant::now() < deadline {
            rig.rt.cycle(&mut probe);
        }
        assert!(
            rig.rt.is_stopping(),
            "invoker {invoker}: the agent's stop did not stop the loop"
        );
        assert_eq!(probe.seen, 100);
        assert!(!probe.cancelled_fired, "a cancelled one-shot fired");
        assert_eq!(probe.repeats, 3, "a cancelled repeating timer fired again");
        let tokens: Vec<u64> = probe.one_shot.iter().map(|t| t.0).collect();
        assert_eq!(tokens, [2, 1], "one-shots fire in deadline order");
        for (token, deadline, now) in &probe.one_shot {
            assert!(now >= deadline, "timer {token} fired before its deadline");
        }
    }
    Ok(())
}

#[test]
fn a_steady_duty_cycle_does_not_allocate() -> TestResult {
    let mut rig = rig(2, true)?;
    let mut probe = Probe::default();
    rig.rt.start(&mut probe)?;
    connect(&mut rig, &mut probe)?;
    let mut echoed = 0;
    publish(&rig.feed, 500);
    drive(&mut rig, &mut probe, &mut echoed, |p, e| {
        p.seen >= 500 && e >= 500 && p.repeats >= 3
    })?;
    publish(&rig.feed, 1_000);
    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    let result = drive(&mut rig, &mut probe, &mut echoed, |p, e| {
        p.seen >= 1_500 && e >= 1_500
    });
    COUNTING.with(|c| c.set(false));
    result?;
    assert_eq!(
        ALLOCS.with(Cell::get),
        0,
        "poll -> on_message -> send -> timers allocated"
    );
    Ok(())
}

#[test]
fn the_counter_sees_an_allocation_on_the_counted_thread() {
    COUNTING.with(|c| c.set(true));
    let before = ALLOCS.with(Cell::get);
    let boxed = std::hint::black_box(Box::new([0u8; 64]));
    let after = ALLOCS.with(Cell::get);
    COUNTING.with(|c| c.set(false));
    drop(boxed);
    assert!(after > before, "the allocation gate cannot fail");
}
