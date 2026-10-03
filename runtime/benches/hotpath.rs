//! Each runtime hot-path replacement against what it replaced, on its own:
//!
//! * `histogram`: [`LocalHistogram`] (plain loads and stores) against the
//!   locked [`Histogram`];
//! * `wall`: the cached wall-clock offset (one add) against
//!   [`Clock::wall`] (`SystemTime::now`);
//! * `claim`: an exclusive publication's claim (`Ctx::send`) against the
//!   shared one's (`Publication::record`), on IPC with a subscriber.
//!
//! Each sample times `REPS` operations; the arms alternate. The process exits
//! non-zero when a replacement's p50 is slower than what it replaced. Needs a
//! media driver at `AERON_TEST_DIR` for `claim`.

use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::idle::Idle;
use ergon_runtime::metrics::Metrics;
use ergon_runtime::rt::{Config, Runtime};
use ergon_runtime::streams::Streams;
use ergon_runtime::subscription::Subscription;

const REPS: usize = 256;
const REPS_F64: f64 = 256.0;
const SAMPLES: usize = 2_000;
const WARMUP: usize = 100;
const IPC: &str = "aeron:ipc?term-length=16m";
const FRAME: usize = 32;

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

fn histogram() -> bool {
    let metrics = Metrics::detached();
    let locked = metrics.histogram("lat", &[]);
    let local = metrics.local_histogram("lat", &[]);
    compare(
        "histogram",
        ("locked", "local"),
        || {
            for v in 0..REPS as u64 {
                locked.record(black_box(v));
            }
        },
        || {
            for v in 0..REPS as u64 {
                local.record(black_box(v));
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

fn claim() -> BenchResult<bool> {
    let aeron_dir =
        std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into());
    let settings = Settings {
        aeron_dir: Some(aeron_dir),
        app: "hotpath-bench".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    let base = 30_000 + (std::process::id() % 50_000).cast_signed() * 4;
    let shared = bus.publication(IPC, base)?;
    let mut rt = Runtime::new(Config {
        persist: None,
        region: "bench".into(),
        idle: Idle::Noop,
        limit: 64,
        ..Config::new(bus.clone(), Streams::parse("services: {}\nkinds: {}\n")?)
    })?;
    let exclusive = rt.ctx().publish_channel(IPC, base + 1)?;
    let mut sinks: [Subscription; 2] =
        [bus.subscription(IPC, base), bus.subscription(IPC, base + 1)];
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(shared.is_connected() && rt.ctx().is_connected(exclusive)) {
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
    let ctx = rt.ctx();
    let pass = compare(
        "claim",
        ("shared", "exclusive"),
        || {
            for _ in 0..REPS {
                let _ = shared.record(1, FRAME, write);
            }
        },
        || {
            for _ in 0..REPS {
                let _ = ctx.send(exclusive, 1, FRAME, write);
            }
        },
        || {
            for s in &mut sinks {
                while s.poll(|_, _| {}, 1_024) > 0 {}
            }
            // The driver moves an IPC publication's limit on its own duty
            // cycle: let it, so neither arm meets back pressure.
            std::thread::sleep(Duration::from_millis(1));
        },
    );
    let drops = bus.dropped();
    if drops > 0 {
        return Err(format!(
            "{drops} claims dropped ({:?}): the arms did not do the same work",
            bus.drops()
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
