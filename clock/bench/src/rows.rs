//! Every clock read the benchmarks compare, one definition for clockbench
//! and Criterion: ergon-clock's reads, the bare instructions, and for
//! comparison quanta, minstant, std and the kernel's clocks.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::quanta::QuantaClock;
use crate::sys;
use ergon_clock::advanced::{
    Calibrated, Clock, CounterClock, MonotonicClock, OrderedRead, Trust, calibrate, raw,
};
use ergon_clock::{Config, Domain, Report};
use std::hint::black_box;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// What a read returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    /// Counter ticks, this many ns each.
    Ticks(f64),
    /// Nanoseconds.
    Ns,
    /// Nothing comparable (an opaque instant, discarded): its cost only;
    /// another row reads the same clock in ns.
    CostOnly,
}

impl Unit {
    /// ns per unit, where there is one.
    #[must_use]
    pub const fn ns_per_unit(self) -> Option<f64> {
        match self {
            Self::Ticks(ns) => Some(ns),
            Self::Ns => Some(1.0),
            Self::CostOnly => None,
        }
    }

    /// The name in the CSV.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ticks(_) => "ticks",
            Self::Ns => "ns",
            Self::CostOnly => "cost-only",
        }
    }
}

/// A row: its name, units, and what it is checked and measured for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    /// The row's name.
    pub name: &'static str,
    /// What a read returns.
    pub unit: Unit,
    /// Never lower than the read before on one thread (checked before
    /// timing: an ordered read, a kernel monotonic clock).
    pub monotonic: bool,
    /// Measured across two threads for backwards steps.
    pub skew: bool,
    /// Measured for drift against `CLOCK_MONOTONIC_RAW` and `CLOCK_MONOTONIC`.
    pub drift: bool,
    /// What it is, for the table.
    pub what: &'static str,
}

impl Spec {
    const fn new(name: &'static str, unit: Unit, what: &'static str) -> Self {
        Self {
            name,
            unit,
            monotonic: false,
            skew: false,
            drift: false,
            what,
        }
    }
    const fn monotonic(mut self) -> Self {
        self.monotonic = true;
        self
    }
    const fn skew(mut self) -> Self {
        self.skew = true;
        self
    }
    const fn drift(mut self) -> Self {
        self.drift = true;
        self
    }
}

/// What a benchmark does with each row; generic so every read is
/// monomorphised into the timed loop (no dynamic call measured).
pub trait Visitor<'c> {
    /// A clock read.
    fn read<F: Fn() -> u64 + Sync + 'c>(&mut self, spec: Spec, read: F);
    /// A conversion of raw ticks (pre-captured inputs).
    fn convert<F: Fn(u64) -> u64 + 'c>(&mut self, spec: Spec, convert: F);
}

/// Every clock, calibrated or created once, before anything is timed.
pub struct Clocks {
    /// ergon-clock's counter clock, calibrated with `Trust::CrossCheck`
    /// (`None` where even that refuses the counter).
    pub ergon: Option<CounterClock>,
    /// That calibration's report.
    pub ergon_report: Report,
    /// The default calibration's report (`Trust::Kernel`): what a
    /// production process gets on this host.
    pub default_report: Report,
    /// The calibrated clock as the enum (the counter where there is one).
    pub ergon_enum: Clock,
    /// ergon-clock's fallback.
    pub monotonic: MonotonicClock,
    /// quanta, held (its global calibration ran when this was made).
    pub quanta: QuantaClock,
    /// quanta's ns per raw tick.
    pub quanta_ns_per_tick: f64,
    /// minstant's anchor (its calibration ran before `main`).
    pub minstant_anchor: minstant::Anchor,
    /// The instant minstant's elapsed time counts from.
    pub minstant_origin: minstant::Instant,
    /// The instant std's elapsed time counts from.
    pub std_origin: Instant,
    /// `rdtscp`, where the CPU has it.
    #[cfg(target_arch = "x86_64")]
    pub rdtscp: Option<raw::Rdtscp>,
    /// `CNTVCTSS_EL0`, where the CPU has `FEAT_ECV`.
    #[cfg(target_arch = "aarch64")]
    pub ecv: Option<raw::Ecv>,
    /// Raw counter values captured once, the conversions' inputs.
    pub samples: Vec<u64>,
}

impl Clocks {
    /// Calibrates or creates every clock.
    ///
    /// # Errors
    /// A kernel clock could not be read.
    pub fn new() -> Result<Self, ergon_clock::Error> {
        let Calibrated { clock, report } = calibrate(&Config {
            trust: Trust::CrossCheck,
            domain: Domain::Monotonic,
            ..Config::default()
        })?;
        let default_report = calibrate(&Config {
            domain: Domain::Monotonic,
            ..Config::default()
        })?
        .report;
        let ergon = match clock {
            Clock::Counter(c) => Some(retuned(c)),
            Clock::Monotonic(_) => None,
        };
        let clock = ergon.map_or(clock, Clock::Counter);
        let quanta = QuantaClock::new();
        let quanta_ns_per_tick = quanta.ticks_to_ns(1 << 40) as f64 / (1_u64 << 40) as f64;
        let samples = (0..1_024)
            .map(|i| ergon.map_or_else(|| quanta.ticks(), |e| e.ticks()) + i * 7)
            .collect();
        Ok(Self {
            ergon,
            ergon_report: report,
            default_report,
            ergon_enum: clock,
            monotonic: MonotonicClock::new(Domain::Monotonic)?,
            quanta,
            quanta_ns_per_tick,
            minstant_anchor: minstant::Anchor::new(),
            minstant_origin: minstant::Instant::now(),
            std_origin: Instant::now(),
            #[cfg(target_arch = "x86_64")]
            rdtscp: raw::Rdtscp::detect(),
            #[cfg(target_arch = "aarch64")]
            ecv: raw::Ecv::detect(),
            samples,
        })
    }
}

/// How many times [`Clocks::new`] re-anchors the counter clock.
pub const RETUNES: u32 = 10_000;

/// `c` re-anchored [`RETUNES`] times, the last at its own factor.
///
/// Alternately 100 ppm fast and slow: every row reads the counter clock as
/// a disciplined one is read, after its owner's re-anchors, so the gates
/// hold for it (a re-anchor changes the clock's values, never its code).
#[must_use]
pub fn retuned(c: CounterClock) -> CounterClock {
    let m0 = c.scale().mult();
    let mut clock = c;
    for k in 0..RETUNES {
        let mult = if k + 1 == RETUNES {
            m0
        } else if k % 2 == 0 {
            m0 + m0 / 10_000
        } else {
            m0 - m0 / 10_000
        };
        if let Some(next) = clock.retuned(clock.ticks_ordered(), mult, 0) {
            clock = next;
        }
    }
    clock
}

/// ergon-clock's follower: the counter clock as another thread reads it.
///
/// Through the seqlock its owner publishes to (one acquire load and a
/// compare, then the read). Timed by clockbench outside [`visit`]: a
/// follower is per thread (`&mut`).
pub const FOLLOWER: Spec = Spec::new(
    "follower_now_ns",
    Unit::Ns,
    "ergon-clock ClockFollower::clock().now_ns() (the seqlock's acquire load and compare, then the read)",
);

/// The front API's follower: `ergon_clock::Follower::now_ns`.
///
/// The counter clock as another thread reads it (the owner's sequence
/// loaded and compared through a `Cell`, then the read). Timed by
/// clockbench outside [`visit`], as [`FOLLOWER`].
pub const FRONT_FOLLOWER: Spec = Spec::new(
    "front_follower_now_ns",
    Unit::Ns,
    "ergon-clock Follower::now_ns (the front API: the seqlock's acquire load and compare, then the read)",
);

/// The bare row of the instruction ergon-clock's ordered read takes: the
/// row its ordered rows are gated against.
#[must_use]
pub const fn bare_ordered_row(read: OrderedRead) -> Option<&'static str> {
    match read {
        OrderedRead::Rdtscp => Some("raw_rdtscp"),
        OrderedRead::IsbCntvct => Some("raw_isb_cntvct"),
        OrderedRead::Cntvctss => Some("raw_cntvctss"),
        OrderedRead::None => None,
    }
}

/// The kernel row of the clock ergon-clock's fallback reads (macOS:
/// `CLOCK_UPTIME_RAW`): the row its gate compares it with.
pub const FALLBACK_KERNEL_ROW: &str = if cfg!(target_os = "macos") {
    "clock_gettime_uptime_raw"
} else {
    "clock_gettime_monotonic"
};

/// Every row, in order, to `v`.
#[allow(clippy::too_many_lines)] // the row list, in its order
pub fn visit<'c, V: Visitor<'c>>(c: &'c Clocks, v: &mut V) {
    use Unit::{CostOnly, Ns, Ticks};
    if let Some(e) = c.ergon {
        let tick = e.ticks_to_ns(1 << 32) as f64 / (1_u64 << 32) as f64;
        v.read(
            Spec::new(
                "ergon_ticks",
                Ticks(tick),
                "ergon-clock CounterClock::ticks (unordered)",
            )
            .skew(),
            move || e.ticks(),
        );
        v.read(
            Spec::new(
                "ergon_ticks_ordered",
                Ticks(tick),
                "ergon-clock CounterClock::ticks_ordered (its OrderedRead: rdtscp; mrs cntvctss_el0 with FEAT_ECV, else isb; mrs cntvct_el0)",
            )
            .monotonic()
            .skew(),
            move || e.ticks_ordered(),
        );
        v.read(
            Spec::new(
                "ergon_now_ns",
                Ns,
                "ergon-clock CounterClock::now_ns (unordered read + 32.32 multiply)",
            )
            .skew()
            .drift(),
            move || e.now_ns(),
        );
        v.read(
            Spec::new(
                "ergon_now_ns_ordered",
                Ns,
                "ergon-clock CounterClock::now_ns_ordered (its OrderedRead, + 32.32 multiply)",
            )
            .monotonic()
            .skew(),
            move || e.now_ns_ordered(),
        );
        // The enum, and its like for like, each loaded from memory at every
        // read (`black_box(&clock)`), as a clock held in a struct is. Held in
        // registers, the compiler hoists the enum's branch out of the timed
        // loop (loop unswitching), and the row times CounterClock::now_ns.
        v.read(
            Spec::new(
                "ergon_now_ns_loaded",
                Ns,
                "ergon-clock CounterClock::now_ns, the clock loaded at every read",
            ),
            move || black_box(&e).now_ns(),
        );
        let clock = c.ergon_enum;
        v.read(
            Spec::new(
                "ergon_clock_now_ns",
                Ns,
                "ergon-clock Clock::now_ns: the enum loaded, and its branch taken, at every read",
            ),
            move || black_box(&clock).now_ns(),
        );
        bare(c, v, tick);
    }
    let m = c.monotonic;
    v.read(
        Spec::new(
            "ergon_monotonic_now_ns",
            Ns,
            "ergon-clock MonotonicClock::now_ns (the fallback)",
        )
        .monotonic()
        .drift(),
        move || m.now_ns(),
    );

    let q = &c.quanta;
    v.read(
        Spec::new(
            "quanta_raw",
            Ticks(c.quanta_ns_per_tick),
            "quanta Clock::raw (held clock)",
        )
        .skew(),
        move || q.ticks(),
    );
    v.read(
        Spec::new(
            "quanta_now",
            CostOnly,
            "quanta Clock::now (held clock; an opaque Instant)",
        ),
        move || {
            black_box(q.inner().now());
            0
        },
    );
    v.read(
        Spec::new(
            "quanta_now_ns",
            Ns,
            "quanta Clock::delta_as_nanos(origin, raw())",
        )
        .skew()
        .drift(),
        move || q.now_ns(),
    );
    v.read(
        Spec::new(
            "quanta_instant_now",
            CostOnly,
            "quanta::Instant::now (global clock, default features)",
        ),
        || {
            black_box(quanta::Instant::now());
            0
        },
    );

    let (anchor, origin) = (&c.minstant_anchor, c.minstant_origin);
    v.read(
        Spec::new("minstant_now", CostOnly, "minstant::Instant::now"),
        || {
            black_box(minstant::Instant::now());
            0
        },
    );
    v.read(
        Spec::new(
            "minstant_elapsed_ns",
            Ns,
            "minstant Instant::elapsed, as ns",
        )
        .skew()
        .drift(),
        move || origin.elapsed().as_nanos() as u64,
    );
    v.read(
        Spec::new(
            "minstant_unix_ns",
            Ns,
            "minstant Instant::now().as_unix_nanos(anchor)",
        ),
        move || minstant::Instant::now().as_unix_nanos(anchor),
    );

    let std_origin = c.std_origin;
    v.read(
        Spec::new("std_instant_now", CostOnly, "std::time::Instant::now"),
        || {
            black_box(Instant::now());
            0
        },
    );
    v.read(
        Spec::new("std_instant_elapsed_ns", Ns, "std Instant::elapsed, as ns")
            .monotonic()
            .skew()
            .drift(),
        move || std_origin.elapsed().as_nanos() as u64,
    );
    v.read(
        Spec::new(
            "std_systemtime_ns",
            Ns,
            "std SystemTime::now since the epoch, as ns",
        )
        .drift(),
        || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64)
        },
    );
    kernel(v);
    conversions(c, v);
}

/// The bare instructions, uncalibrated (ticks converted by ergon's rate).
#[allow(unused_variables)]
fn bare<'c, V: Visitor<'c>>(c: &'c Clocks, v: &mut V, tick: f64) {
    use Unit::Ticks;
    #[cfg(target_arch = "x86_64")]
    {
        v.read(Spec::new("raw_rdtsc", Ticks(tick), "rdtsc"), raw::rdtsc);
        v.read(
            Spec::new("raw_lfence_rdtsc", Ticks(tick), "lfence; rdtsc").monotonic(),
            raw::lfence_rdtsc,
        );
        if let Some(r) = c.rdtscp {
            v.read(
                Spec::new("raw_rdtscp", Ticks(tick), "rdtscp").monotonic(),
                move || r.read(),
            );
            v.read(
                Spec::new("raw_rdtscp_lfence", Ticks(tick), "rdtscp; lfence").monotonic(),
                move || r.read_fenced(),
            );
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        v.read(
            Spec::new("raw_cntvct", Ticks(tick), "mrs cntvct_el0"),
            raw::cntvct,
        );
        v.read(
            Spec::new("raw_isb_cntvct", Ticks(tick), "isb; mrs cntvct_el0").monotonic(),
            raw::isb_cntvct,
        );
        if let Some(e) = c.ecv {
            v.read(
                Spec::new("raw_cntvctss", Ticks(tick), "mrs cntvctss_el0 (FEAT_ECV)")
                    .monotonic()
                    .skew(),
                move || e.read(),
            );
        }
    }
}

/// The kernel's clocks, by `clock_gettime` (the vDSO).
fn kernel<'c, V: Visitor<'c>>(v: &mut V) {
    use Unit::Ns;
    v.read(
        Spec::new(
            "clock_gettime_monotonic",
            Ns,
            "clock_gettime(CLOCK_MONOTONIC)",
        )
        .monotonic(),
        || sys::clock_ns(libc::CLOCK_MONOTONIC),
    );
    v.read(
        Spec::new(
            "clock_gettime_monotonic_raw",
            Ns,
            "clock_gettime(CLOCK_MONOTONIC_RAW)",
        )
        .monotonic()
        .skew(),
        || sys::clock_ns(libc::CLOCK_MONOTONIC_RAW),
    );
    v.read(
        Spec::new(
            "clock_gettime_realtime",
            Ns,
            "clock_gettime(CLOCK_REALTIME)",
        ),
        || sys::clock_ns(libc::CLOCK_REALTIME),
    );
    #[cfg(target_os = "linux")]
    v.read(
        Spec::new(
            "clock_gettime_monotonic_coarse",
            Ns,
            "clock_gettime(CLOCK_MONOTONIC_COARSE) (a tick's resolution)",
        )
        .monotonic(),
        || sys::clock_ns(libc::CLOCK_MONOTONIC_COARSE),
    );
    // Aeron's epoch clock source on Linux: `aeron_epoch_clock()` is
    // `clock_gettime(CLOCK_REALTIME_COARSE)` truncated to milliseconds
    // (`aeron/aeron-client/src/main/c/util/aeron_clock.c`). The driver calls it
    // once per millisecond into `aeron_clock_cache_t`, so its cost sits off the
    // hot path; it is here because it is the one kernel clock Aeron uses and the
    // table was empty for it. Not monotonic: the wall clock steps.
    #[cfg(target_os = "linux")]
    v.read(
        Spec::new(
            "clock_gettime_realtime_coarse",
            Ns,
            "clock_gettime(CLOCK_REALTIME_COARSE) (a tick's resolution; Aeron's epoch clock)",
        ),
        || sys::clock_ns(libc::CLOCK_REALTIME_COARSE),
    );
    #[cfg(target_os = "macos")]
    v.read(
        Spec::new(
            "clock_gettime_uptime_raw",
            Ns,
            "clock_gettime(CLOCK_UPTIME_RAW) (Instant's on macOS)",
        )
        .monotonic(),
        || sys::clock_ns(libc::CLOCK_UPTIME_RAW),
    );
}

/// The conversions alone, over pre-captured raw ticks: ergon's 32.32
/// multiply against quanta's.
fn conversions<'c, V: Visitor<'c>>(c: &'c Clocks, v: &mut V) {
    use Unit::CostOnly;
    let origin = c.samples.first().copied().unwrap_or(0);
    if let Some(e) = c.ergon {
        v.convert(
            Spec::new(
                "convert_ergon_to_ns",
                CostOnly,
                "ergon Scale::to_ns(ticks) (signed, based)",
            ),
            move |t| e.to_ns(t),
        );
        v.convert(
            Spec::new(
                "convert_ergon_ticks_to_ns",
                CostOnly,
                "ergon ticks_to_ns(ticks - origin)",
            ),
            move |t| e.ticks_to_ns(t.wrapping_sub(origin)),
        );
    }
    let q = &c.quanta;
    v.convert(
        Spec::new(
            "convert_quanta_delta_as_nanos",
            CostOnly,
            "quanta Clock::delta_as_nanos(origin, ticks)",
        ),
        move |t| q.inner().delta_as_nanos(origin, t),
    );
    v.convert(
        Spec::new(
            "convert_quanta_scaled",
            CostOnly,
            "quanta Clock::scaled(ticks) (an opaque Instant)",
        ),
        move |t| {
            black_box(q.inner().scaled(t));
            0
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collects the rows' names, timing nothing.
    struct Names(Vec<&'static str>);

    impl<'c> Visitor<'c> for Names {
        fn read<F: Fn() -> u64 + Sync + 'c>(&mut self, spec: Spec, _: F) {
            self.0.push(spec.name);
        }
        fn convert<F: Fn(u64) -> u64 + 'c>(&mut self, spec: Spec, _: F) {
            self.0.push(spec.name);
        }
    }

    /// The fallback is gated against the kernel read of the clock it wraps
    /// (macOS: `CLOCK_UPTIME_RAW`, not `CLOCK_MONOTONIC`): that row's reads
    /// fall between two of the fallback's ticks.
    #[test]
    fn the_fallback_is_gated_against_the_clock_it_reads() -> Result<(), ergon_clock::Error> {
        struct Between {
            m: MonotonicClock,
            found: usize,
            inside: usize,
        }
        impl<'c> Visitor<'c> for Between {
            fn read<F: Fn() -> u64 + Sync + 'c>(&mut self, spec: Spec, read: F) {
                if spec.name == FALLBACK_KERNEL_ROW {
                    self.found += 1;
                    for _ in 0..100 {
                        let (a, r, b) = (self.m.ticks(), read(), self.m.ticks());
                        self.inside += usize::from(a <= r && r <= b);
                    }
                }
            }
            fn convert<F: Fn(u64) -> u64 + 'c>(&mut self, _: Spec, _: F) {}
        }
        let clocks = Clocks::new()?;
        let mut v = Between {
            m: clocks.monotonic,
            found: 0,
            inside: 0,
        };
        visit(&clocks, &mut v);
        assert_eq!((v.found, v.inside), (1, 100), "{FALLBACK_KERNEL_ROW}");
        Ok(())
    }

    /// Each ordered read is gated against the bare row of its own
    /// instruction, and on this host that row is one clockbench times.
    #[test]
    fn the_ordered_rows_are_gated_against_their_own_instruction() -> Result<(), ergon_clock::Error>
    {
        assert_eq!(bare_ordered_row(OrderedRead::Rdtscp), Some("raw_rdtscp"));
        assert_eq!(
            bare_ordered_row(OrderedRead::IsbCntvct),
            Some("raw_isb_cntvct")
        );
        assert_eq!(
            bare_ordered_row(OrderedRead::Cntvctss),
            Some("raw_cntvctss")
        );
        assert_eq!(bare_ordered_row(OrderedRead::None), None);
        let clocks = Clocks::new()?;
        let mut names = Names(Vec::new());
        visit(&clocks, &mut names);
        match clocks.ergon {
            Some(e) => {
                let bare = bare_ordered_row(e.ordered_read());
                assert!(
                    bare.is_some_and(|b| names.0.contains(&b)),
                    "{bare:?} not among {:?}",
                    names.0
                );
                assert!(names.0.contains(&"ergon_ticks_ordered"));
            }
            None => assert!(!names.0.contains(&"ergon_ticks_ordered")),
        }
        Ok(())
    }
}
