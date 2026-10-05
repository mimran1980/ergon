//! Each runtime hot-path replacement against what it replaced, on its own:
//!
//! * `histogram`: a [`Histogram`](ergon_runtime::metrics::Histogram) cell
//!   (plain loads and stores) against the locked histogram it replaced,
//!   kept here as the baseline;
//! * `wall`: the cached wall-clock offset (one add) against
//!   [`Clock::wall`] (`SystemTime::now`);
//! * `claim`: an exclusive publication's claim (`Ctx::send`) against the
//!   shared one's it replaced (`Publication::record` on a concurrent
//!   publication, behind the shared bus's atomics), kept here as the
//!   baseline, on IPC with a subscriber.
//!
//! Each sample times `REPS` operations; the arms alternate. The process exits
//! non-zero when a replacement's p50 is slower than what it replaced. Needs a
//! media driver at `AERON_TEST_DIR` for `claim`.
//!
//! `claim` first writes each arm through its whole log, untimed. The test
//! driver's log files are sparse, so the first touch of a log page is a
//! page-in (a major fault on macOS, at times with the thread preempted). A
//! sample's frames fill exactly one 16 KiB page; the runtime's output sends
//! its `Source` frame first, which moves its frames off the page grid.
//! Without the pass, every exclusive sample paid a page-in and no shared one
//! did: the shared arm's first touches fell to the sink's untimed poll. The
//! driver settings that avoid page-ins (non-sparse logs, pre-touched) are a
//! deployment choice.

use std::cell::{Cell, RefCell};
use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::idle::Idle;
use ergon_runtime::metrics::Metrics;
use ergon_runtime::rt::{Agent, Config, Ctx, Expiry, FeedId, Runtime};
use ergon_runtime::subscription::{Delivery, Subscription};
use rusteron_archive::{
    Aeron, AeronBufferClaim, AeronContext, AeronOfferError, AeronPublication, IntoCString,
};

const REPS: usize = 256;
const REPS_F64: f64 = 256.0;
const SAMPLES: usize = 2_000;
const WARMUP: usize = 100;
const IPC: &str = "aeron:ipc?term-length=16m";
const FRAME: usize = 32;
/// Samples that write an arm through its log once: three 16 MiB terms of
/// 64-byte frames (`FRAME` and its header), and one more for the `Source`
/// frame.
const LOG_SAMPLES: usize = 3 * (16 << 20) / (REPS * 64) + 1;

type BenchResult<T> = Result<T, Box<dyn std::error::Error>>;

fn percentile(sorted: &[f64], numer: usize, denom: usize) -> f64 {
    let last = sorted.len() - 1;
    sorted[(last * numer).div_ceil(denom).min(last)]
}

fn report(op: &str, arm: &str, samples: &mut [f64]) -> f64 {
    samples.sort_unstable_by(f64::total_cmp);
    let p50 = percentile(samples, 1, 2);
    println!(
        "PERCENTILES {op} {arm} ns/op n={} min={:.2} p50={p50:.2} p99={:.2} p99.99={:.2} max={:.2}",
        samples.len(),
        samples[0],
        percentile(samples, 99, 100),
        percentile(samples, 9_999, 10_000),
        samples[samples.len() - 1],
    );
    p50
}

/// Time `old` and `new` alternately and gate `new`'s p50 on `old`'s.
fn compare(
    op: &str,
    names: (&str, &str),
    mut old: impl FnMut(),
    mut new: impl FnMut(),
    mut between: impl FnMut(),
) -> bool {
    let (mut a, mut b) = (Vec::with_capacity(SAMPLES), Vec::with_capacity(SAMPLES));
    let time = |f: &mut dyn FnMut()| {
        let start = Instant::now();
        f();
        start.elapsed().as_secs_f64() * 1e9 / REPS_F64
    };
    for sample in 0..WARMUP + SAMPLES {
        let (x, y) = if sample % 2 == 0 {
            let x = time(&mut old);
            between();
            (x, time(&mut new))
        } else {
            let y = time(&mut new);
            between();
            (time(&mut old), y)
        };
        between();
        if sample >= WARMUP {
            a.push(x);
            b.push(y);
        }
    }
    let old_p50 = report(op, names.0, &mut a);
    let new_p50 = report(op, names.1, &mut b);
    let pass = new_p50 <= old_p50;
    println!(
        "GATE {op} {}/{} p50={:.3} {}",
        names.1,
        names.0,
        new_p50 / old_p50,
        if pass { "pass" } else { "fail" }
    );
    pass
}

/// One summary's count, sum, minimum, and maximum, as the locked histogram
/// kept them.
#[derive(Clone, Copy)]
struct HistSample {
    count: u64,
    sum: u64,
    min: u64,
    max: u64,
}

#[expect(
    clippy::disallowed_types,
    reason = "the locked histogram the client replaced: the gate's baseline"
)]
fn histogram() -> bool {
    use std::sync::{Mutex, PoisonError};

    let locked = Mutex::new(HistSample {
        count: 0,
        sum: 0,
        min: u64::MAX,
        max: 0,
    });
    let record_locked = |locked: &Mutex<HistSample>, value: u64| {
        let mut sample = locked.lock().unwrap_or_else(PoisonError::into_inner);
        sample.count = sample.count.wrapping_add(1);
        sample.sum = sample.sum.wrapping_add(value);
        sample.min = sample.min.min(value);
        sample.max = sample.max.max(value);
    };
    let metrics = Metrics::detached();
    let cell = metrics.histogram("lat", &[]);
    // Each record through an opaque handle: neither arm folds its 256
    // records into one store.
    compare(
        "histogram",
        ("locked", "cell"),
        || {
            for v in 0..REPS as u64 {
                record_locked(black_box(&locked), black_box(v));
            }
        },
        || {
            for v in 0..REPS as u64 {
                black_box(&cell).record(black_box(v));
            }
        },
        || {},
    )
}

fn wall() -> bool {
    let clock = Clock::new();
    let offset = clock.wall_offset();
    let now = clock.now();
    compare(
        "wall",
        ("system_time", "cached_offset"),
        || {
            for _ in 0..REPS {
                black_box(clock.wall());
            }
        },
        || {
            for _ in 0..REPS {
                black_box(Nanos(black_box(now).0 + black_box(offset)));
            }
        },
        || {},
    )
}

/// The runtime's agent here: the claim gate sends from outside a callback.
struct Quiet;

impl Agent for Quiet {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        Ok(())
    }

    fn on_message(&mut self, _ctx: &mut Ctx, _feed: FeedId, _msg: &[u8], _d: Delivery) {}

    fn on_timer(&mut self, _ctx: &mut Ctx, _timer: Expiry) {}
}

/// What `Publication::record` was before the client went single-threaded,
/// kept here as the claim gate's baseline: a concurrent publication (an
/// atomic add on the term tail) behind the shared bus's heartbeat and
/// shutdown flag, on a client with a conductor thread, as that bus had.
#[expect(
    clippy::disallowed_types,
    reason = "the shared claim the client replaced, with the bus's atomics: the gate's baseline"
)]
struct Shared {
    _client: Aeron,
    publication: AeronPublication,
    max_payload: usize,
    /// The bus's heartbeat, and the one this publication last sent `Source`
    /// at: loaded on every record.
    heartbeat: std::sync::atomic::AtomicU64,
    sent: std::sync::atomic::AtomicU64,
    /// The bus's shutdown flag: loaded on every record.
    closed: std::sync::atomic::AtomicBool,
    clock: Clock,
    /// Records it did not publish.
    drops: Cell<u64>,
}

impl Shared {
    #[expect(
        clippy::disallowed_types,
        reason = "the shared claim the client replaced, with the bus's atomics: the gate's baseline"
    )]
    fn connect(aeron_dir: &str, stream_id: i32) -> BenchResult<Self> {
        let ctx = AeronContext::new()?;
        ctx.set_dir(&aeron_dir.into_c_string())?;
        let client = Aeron::new(&ctx)?;
        client.start()?;
        let publication = client
            .async_add_publication(&IPC.into_c_string(), stream_id)?
            .poll_blocking(Duration::from_secs(10))?;
        Ok(Self {
            max_payload: publication.max_payload_length()?,
            _client: client,
            publication,
            heartbeat: std::sync::atomic::AtomicU64::new(0),
            sent: std::sync::atomic::AtomicU64::new(0),
            closed: std::sync::atomic::AtomicBool::new(false),
            clock: Clock::new(),
            drops: Cell::new(0),
        })
    }

    /// One record of `len` bytes that `encode` writes, as it was made.
    #[inline]
    fn record<E>(&self, len: usize, encode: impl FnOnce(&mut [u8]) -> Result<usize, E>) {
        use std::sync::atomic::Ordering::Relaxed;

        let dropped = || self.drops.set(self.drops.get() + 1);
        // `Source` again once the bus's heartbeat moved: never here.
        let beat = self.heartbeat.load(Relaxed);
        if beat != self.sent.load(Relaxed) {
            self.sent.store(beat, Relaxed);
        }
        if len > self.max_payload || self.closed.load(Relaxed) {
            dropped();
            return;
        }
        let claim = AeronBufferClaim::new_zeroed_on_stack();
        let mut rotations = 8;
        let claimed = loop {
            match self.publication.try_claim(len, &claim) {
                Err(AeronOfferError::AdminAction) if rotations > 0 => rotations -= 1,
                claimed => break claimed,
            }
        };
        if claimed.is_err() {
            dropped();
            return;
        }
        claim.frame_header_mut().reserved_value = self.clock.read().0;
        let slot = claim.data();
        let honest = encode(slot).is_ok_and(|written| written == len)
            && slot.get(2..4) == Some(&1u16.to_le_bytes()[..]);
        if !(honest && claim.commit().is_ok()) {
            dropped();
        }
    }
}

fn claim() -> BenchResult<bool> {
    let aeron_dir =
        std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into());
    let settings = Settings {
        aeron_dir: Some(aeron_dir.clone()),
        app: "hotpath-bench".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    let base = 30_000 + (std::process::id() % 50_000).cast_signed() * 4;
    let mut sinks: [Subscription; 2] =
        [bus.subscription(IPC, base), bus.subscription(IPC, base + 1)];
    // The runtime owns the bus and runs its conductor; both closures reach it.
    let rt = RefCell::new(Runtime::new(Config {
        persist: None,
        region: "bench".into(),
        idle: Idle::Noop,
        limit: 64,
        ..Config::new(bus)
    })?);
    rt.borrow_mut().start(&mut Quiet)?;
    let exclusive = rt.borrow_mut().ctx().publish_channel(IPC, base + 1)?;
    let shared = Shared::connect(&aeron_dir, base)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(shared.publication.is_connected() && rt.borrow().ctx_ref().is_connected(exclusive)) {
        rt.borrow_mut().cycle(&mut Quiet);
        for s in &mut sinks {
            s.poll(|_, _| {}, 1);
        }
        if Instant::now() > deadline {
            return Err("the IPC streams did not connect within 10 s".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let write = |buf: &mut [u8]| {
        buf[2..4].copy_from_slice(&1u16.to_le_bytes());
        Ok::<_, std::convert::Infallible>(FRAME)
    };
    let shared_arm = || {
        for _ in 0..REPS {
            shared.record(FRAME, write);
        }
    };
    let exclusive_arm = || {
        let rt = rt.borrow();
        let ctx = rt.ctx_ref();
        for _ in 0..REPS {
            let _ = ctx.send(exclusive, 1, FRAME, write);
        }
    };
    // The runtime's conductor and both sinks, outside the timing.
    let mut drain = || {
        rt.borrow_mut().cycle(&mut Quiet);
        for s in &mut sinks {
            while s.poll(|_, _| {}, 1_024) > 0 {}
        }
    };
    // Every page of both logs touched before any is timed: see the module
    // docs.
    for _ in 0..LOG_SAMPLES {
        shared_arm();
        exclusive_arm();
        drain();
    }
    let pass = compare(
        "claim",
        ("shared", "exclusive"),
        shared_arm,
        exclusive_arm,
        || {
            drain();
            // The driver moves an IPC publication's limit on its own duty
            // cycle: let it, so neither arm meets back pressure. Spinning,
            // as a client's loop does on its core: after a sleep, the wake
            // takes microseconds and swamps the claims being timed.
            let until = Instant::now() + Duration::from_millis(1);
            while Instant::now() < until {
                std::hint::spin_loop();
            }
        },
    );
    let exclusive_drops = rt.borrow().ctx_ref().drops();
    let drops = exclusive_drops.total() + shared.drops.get();
    if drops > 0 {
        return Err(format!(
            "{drops} claims dropped (exclusive {exclusive_drops:?}, shared {}): the arms did not do the same work",
            shared.drops.get()
        )
        .into());
    }
    Ok(pass)
}

fn main() -> ExitCode {
    let mut pass = [histogram(), wall()].iter().all(|p| *p);
    match claim() {
        Ok(p) => pass &= p,
        Err(err) => {
            eprintln!("claim: {err}");
            pass = false;
        }
    }
    if pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
