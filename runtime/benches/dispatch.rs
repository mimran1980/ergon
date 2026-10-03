//! Per-message cost of an application loop over one IPC feed.
//!
//! `runtime` is the same body as an [`Agent`] on [`Runtime::cycle`]: the
//! clock read once per message, the wall offset cached, lock-free
//! single-thread histograms, an exclusive order publication, and the
//! once-a-second work on a timer. The gate fails when
//! the runtime is slower than the hand-rolled loop at p50 or p99.
//!
//! `runtime-invoker` is `runtime` with the Aeron client conductor driven from
//! the duty cycle instead of its own thread, gated against `runtime`.
//!
//! `hand-rolled` is today's engine loop (`engine/src/main.rs`): poll the feed,
//! read the clock, check the once-a-second work, `Persist::poll`, idle. Its
//! message body is the engine's market-data path: two remote-time
//! conversions through the wall clock, two latency histograms, a counter, and
//! every 16th message an order on a second IPC publication.
//!
//! Each sample publishes `BATCH` frames outside the clock, then times the loop
//! until it has dispatched all of them, and reports nanoseconds per message
//! as min / p50 / p99 / p99.99 / max.
//!
//! Needs an Aeron media driver at `AERON_TEST_DIR` (default
//! `/tmp/persist-test-aeron`); `just bench-runtime` starts it.

use std::hint::black_box;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::idle::Idle;
use ergon_runtime::metrics::{Counter, Histogram, LocalHistogram};
use ergon_runtime::persist::Persist;
use ergon_runtime::publication::Publication;
use ergon_runtime::rt::{Agent, Config, Ctx, Expiry, FeedId, Out, Runtime};
use ergon_runtime::streams::Streams;
use ergon_runtime::subscription::{Delivery, Subscription};

const SCHEMA: &str = include_str!("../schema/events.xml");
const LIMIT: usize = 64;
const BATCH: usize = 256;
const BATCH_F64: f64 = 256.0;
const SAMPLES: usize = 2_000;
const WARMUP: usize = 200;
const FRAME: usize = 32;
const TEMPLATE: u16 = 1;
const ORDER_EVERY: u64 = 16;
const SECOND: i64 = 1_000_000_000;

type BenchResult<T> = Result<T, Box<dyn std::error::Error>>;

fn aeron_dir() -> String {
    std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into())
}

/// Streams unique to this process, so a concurrent run never shares them.
fn stream(n: i32) -> i32 {
    20_000 + (std::process::id() % 50_000).cast_signed() * 8 + n
}

const IPC: &str = "aeron:ipc?term-length=16m";

/// What every arm starts with: the bus, persist, a feed to publish into and
/// an order stream someone takes.
struct Rig {
    bus: Bus,
    persist: Persist,
    feed: Publication,
    orders_sink: Subscription,
    /// Takes persist's rows and metrics, as the ingester would.
    persist_sink: Subscription,
}

impl Rig {
    fn new(arm: i32, invoker: bool) -> BenchResult<Self> {
        let config =
            std::env::temp_dir().join(format!("dispatch-bench-{}.yaml", std::process::id()));
        std::fs::write(&config, "tables: {}\n")?;
        let settings = Settings {
            aeron_dir: Some(aeron_dir()),
            channel: IPC.into(),
            stream_id: stream(arm * 4),
            subscriber_timeout: Duration::ZERO,
            app: "dispatch-bench".into(),
            aeron_invoker: invoker,
            ..Settings::new(&config)
        };
        let bus = Bus::connect(&settings)?;
        let persist = Persist::connect(SCHEMA, &bus, settings)?;
        let feed = bus.publication(IPC, stream(arm * 4 + 1))?;
        let orders_sink = bus.subscription(IPC, stream(arm * 4 + 2));
        let persist_sink = bus.subscription(IPC, stream(arm * 4));
        Ok(Self {
            bus,
            persist,
            feed,
            orders_sink,
            persist_sink,
        })
    }

    /// `BATCH` frames carrying the publisher's receive and event times.
    fn publish_batch(&self, clock: &Clock) {
        for _ in 0..BATCH {
            let ts = clock.wall().epoch_ns().cast_unsigned();
            let _ = self.feed.record(TEMPLATE, FRAME, |buf| {
                buf[2..4].copy_from_slice(&TEMPLATE.to_le_bytes());
                buf[8..16].copy_from_slice(&ts.to_le_bytes());
                buf[16..24].copy_from_slice(&(ts - 1_000).to_le_bytes());
                Ok::<_, std::convert::Infallible>(FRAME)
            });
        }
    }

    fn drain_orders(&mut self) {
        while self.orders_sink.poll(|_, _| {}, 1_024) > 0 {}
        while self.persist_sink.poll(|_, _| {}, 1_024) > 0 {}
    }
}

fn read_u64(m: &[u8], at: usize) -> u64 {
    m.get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .map_or(0, u64::from_le_bytes)
}

/// The engine's market-data body on today's types.
struct Core {
    clock: Clock,
    orders: Publication,
    md_latency: Histogram,
    venue_latency: Histogram,
    tick_to_order: Histogram,
    seen: Counter,
    next_second: i64,
    count: u64,
}

impl Core {
    fn on_md(&mut self, m: &[u8], delivery: Delivery) {
        let received = self.clock.now();
        if !delivery.is_live() {
            return;
        }
        let at = self
            .clock
            .from_remote(read_u64(m, 8).cast_signed(), received);
        let event = self
            .clock
            .from_remote(read_u64(m, 16).cast_signed(), received);
        self.md_latency
            .record(received.since(at).max(0).cast_unsigned());
        self.venue_latency
            .record(received.since(event).max(0).cast_unsigned());
        self.seen.inc();
        self.count += 1;
        if self.count.is_multiple_of(ORDER_EVERY) {
            let ts = self.clock.wall().epoch_ns().cast_unsigned();
            let sent = self.orders.record(TEMPLATE, FRAME, |buf| {
                buf[2..4].copy_from_slice(&TEMPLATE.to_le_bytes());
                buf[8..16].copy_from_slice(&ts.to_le_bytes());
                Ok::<_, std::convert::Infallible>(FRAME)
            });
            if sent.is_ok() {
                self.tick_to_order
                    .record(self.clock.now().since(at).max(0).cast_unsigned());
            }
        }
    }

    const fn every_second(&mut self, now: Nanos) -> bool {
        if now.epoch_ns() < self.next_second {
            return false;
        }
        self.next_second = (now.epoch_ns() / SECOND + 1) * SECOND;
        true
    }
}

/// Today's engine loop, run until `target` messages have been dispatched.
fn hand_rolled(
    rig: &Rig,
    feed: &mut Subscription,
    core: &mut Core,
    stop: &AtomicBool,
    target: u64,
) {
    let idle = Idle::Noop;
    while core.count < target {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let work = feed.poll(|m, d| core.on_md(m, d), LIMIT);
        let now = core.clock.now();
        black_box(core.every_second(now));
        rig.persist.poll(now);
        idle.idle(work);
    }
}

fn percentile(sorted: &[f64], numer: usize, denom: usize) -> f64 {
    let last = sorted.len() - 1;
    sorted[(last * numer).div_ceil(denom).min(last)]
}

fn report(arm: &str, samples: &mut [f64]) -> (f64, f64) {
    samples.sort_unstable_by(f64::total_cmp);
    let (p50, p99) = (percentile(samples, 1, 2), percentile(samples, 99, 100));
    println!(
        "PERCENTILES dispatch {arm} ns/msg n={} min={:.1} p50={p50:.1} p99={p99:.1} p99.99={:.1} max={:.1}",
        samples.len(),
        samples[0],
        percentile(samples, 9_999, 10_000),
        samples[samples.len() - 1],
    );
    (p50, p99)
}

/// Call `step` until it reports everything connected.
fn wait(mut step: impl FnMut() -> bool) -> BenchResult<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !step() {
        if Instant::now() > deadline {
            return Err("the IPC feed did not connect within 10 s; is the media driver up?".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

struct HandArm {
    rig: Rig,
    feed: Subscription,
    core: Core,
    stop: Arc<AtomicBool>,
    target: u64,
}

impl HandArm {
    fn new() -> BenchResult<Self> {
        let rig = Rig::new(0, false)?;
        let metrics = rig.persist.metrics();
        let feed = rig.bus.subscription(IPC, stream(1));
        let core = Core {
            clock: Clock::new(),
            orders: rig.bus.publication(IPC, stream(2))?,
            md_latency: metrics.histogram("md_to_engine_ns", &[]),
            venue_latency: metrics.histogram("venue_to_engine_ns", &[]),
            tick_to_order: metrics.histogram("tick_to_order_ns", &[]),
            seen: metrics.counter("seen", &[]),
            next_second: 0,
            count: 0,
        };
        let mut arm = Self {
            rig,
            feed,
            core,
            stop: Arc::new(AtomicBool::new(false)),
            target: 0,
        };
        let (feed, rig) = (&mut arm.feed, &mut arm.rig);
        wait(|| {
            feed.poll(|_, _| {}, 1);
            rig.drain_orders();
            rig.feed.is_connected() && rig.orders_sink.is_connected() && rig.persist.is_connected()
        })?;
        Ok(arm)
    }

    /// One batch: publish untimed, then time the loop until it is dispatched.
    fn sample(&mut self) -> f64 {
        self.rig.publish_batch(&self.core.clock);
        self.target += BATCH as u64;
        let start = Instant::now();
        hand_rolled(
            &self.rig,
            &mut self.feed,
            &mut self.core,
            &self.stop,
            self.target,
        );
        let elapsed = start.elapsed().as_secs_f64() * 1e9;
        self.rig.drain_orders();
        elapsed / BATCH_F64
    }
}

/// The same body as an [`Agent`].
struct BenchAgent {
    orders: Out,
    md_latency: LocalHistogram,
    venue_latency: LocalHistogram,
    tick_to_order: LocalHistogram,
    seen: Counter,
    count: u64,
}

impl Agent for BenchAgent {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        ctx.every_aligned(SECOND, 1)
            .map_err(|e| ergon_runtime::Error::Config(e.to_string()))?;
        Ok(())
    }

    #[inline]
    fn on_message(&mut self, ctx: &mut Ctx, _feed: FeedId, m: &[u8], delivery: Delivery) {
        let received = ctx.now();
        if !delivery.is_live() {
            return;
        }
        let at = ctx.from_remote(read_u64(m, 8).cast_signed());
        let event = ctx.from_remote(read_u64(m, 16).cast_signed());
        self.md_latency
            .record(received.since(at).max(0).cast_unsigned());
        self.venue_latency
            .record(received.since(event).max(0).cast_unsigned());
        self.seen.inc();
        self.count += 1;
        if self.count.is_multiple_of(ORDER_EVERY) {
            let ts = ctx.wall_ns().epoch_ns().cast_unsigned();
            let sent = ctx.send(self.orders, TEMPLATE, FRAME, |buf| {
                buf[2..4].copy_from_slice(&TEMPLATE.to_le_bytes());
                buf[8..16].copy_from_slice(&ts.to_le_bytes());
                Ok::<_, std::convert::Infallible>(FRAME)
            });
            if sent.is_ok() {
                self.tick_to_order
                    .record(ctx.read().since(at).max(0).cast_unsigned());
            }
        }
    }

    fn on_timer(&mut self, _ctx: &mut Ctx, timer: Expiry) {
        black_box(timer);
    }
}

struct RuntimeArm {
    rig: Rig,
    rt: Runtime,
    agent: BenchAgent,
    clock: Clock,
    target: u64,
}

impl RuntimeArm {
    fn new(arm: i32, invoker: bool) -> BenchResult<Self> {
        let rig = Rig::new(arm, invoker)?;
        let metrics = rig.persist.metrics();
        let mut rt = Runtime::new(Config {
            persist: Some(rig.persist.clone()),
            region: "bench".into(),
            idle: Idle::Noop,
            limit: LIMIT,
            ..Config::new(
                rig.bus.clone(),
                Streams::parse("services: {}\nkinds: {}\n")?,
            )
        })?;
        rt.ctx().subscribe_channel(IPC, stream(arm * 4 + 1));
        let orders = rt.ctx().publish_channel(IPC, stream(arm * 4 + 2))?;
        let mut agent = BenchAgent {
            orders,
            md_latency: metrics.local_histogram("md_to_engine_ns", &[]),
            venue_latency: metrics.local_histogram("venue_to_engine_ns", &[]),
            tick_to_order: metrics.local_histogram("tick_to_order_ns", &[]),
            seen: metrics.counter("seen", &[]),
            count: 0,
        };
        rt.start(&mut agent)?;
        let mut arm = Self {
            rig,
            rt,
            agent,
            clock: Clock::new(),
            target: 0,
        };
        let (rt, agent, rig) = (&mut arm.rt, &mut arm.agent, &mut arm.rig);
        wait(|| {
            rt.cycle(agent);
            rig.drain_orders();
            rig.feed.is_connected()
                && rig.orders_sink.is_connected()
                && rt.ctx().is_connected(orders)
        })?;
        Ok(arm)
    }

    fn sample(&mut self) -> f64 {
        self.rig.publish_batch(&self.clock);
        self.target += BATCH as u64;
        let idle = Idle::Noop;
        let start = Instant::now();
        while self.agent.count < self.target {
            let work = self.rt.cycle(&mut self.agent);
            idle.idle(work);
        }
        let elapsed = start.elapsed().as_secs_f64() * 1e9;
        self.rig.drain_orders();
        elapsed / BATCH_F64
    }
}

fn gate(name: &str, new: (f64, f64), old: (f64, f64)) -> bool {
    let pass = new.0 <= old.0 && new.1 <= old.1;
    println!(
        "GATE dispatch {name} p50={:.3} p99={:.3} {}",
        new.0 / old.0,
        new.1 / old.1,
        if pass { "pass" } else { "fail" }
    );
    pass
}

/// Every arm built first, then sampled in a rotating order so none pays the
/// first-arm position penalty or a different machine state.
fn run() -> BenchResult<bool> {
    let mut hand = HandArm::new()?;
    let mut runtime = RuntimeArm::new(1, false)?;
    let mut invoker = RuntimeArm::new(2, true)?;
    let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::with_capacity(SAMPLES));
    for sample in 0..WARMUP + SAMPLES {
        let mut taken = [0.0; 3];
        for k in 0..3 {
            let arm = (sample + k) % 3;
            taken[arm] = match arm {
                0 => hand.sample(),
                1 => runtime.sample(),
                _ => invoker.sample(),
            };
        }
        if sample >= WARMUP {
            for (all, one) in samples.iter_mut().zip(taken) {
                all.push(one);
            }
        }
    }
    for (arm, bus) in [
        ("hand-rolled", &hand.rig.bus),
        ("runtime", &runtime.rig.bus),
        ("runtime-invoker", &invoker.rig.bus),
    ] {
        if bus.dropped() > 0 {
            return Err(format!(
                "{arm} dropped {:?}: the arms did not do the same work",
                bus.drops()
            )
            .into());
        }
    }
    let [h, r, i] = &mut samples;
    let h = report("hand-rolled", h);
    let r = report("runtime", r);
    let i = report("runtime-invoker", i);
    Ok(gate("runtime/hand", r, h) & gate("invoker/conductor-thread", i, r))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("dispatch bench: {err}");
            ExitCode::FAILURE
        }
    }
}
