//! The clocks: the counter scaled to nanoseconds, the monotonic fallback,
//! and the choice between them; one trait over every time source.

use crate::arch;
use crate::detect::{self, OrderedRead};
use crate::error::Error;
use crate::params::{KIND_COUNTER, KIND_MONOTONIC, PARAMS_VERSION, Params};
use crate::scale::{self, Conversion, Scale};
use crate::wall;

/// The time a clock's nanoseconds count from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Domain {
    /// UTC: nanoseconds since 1970 (the UNIX epoch), from `CLOCK_REALTIME`
    /// at calibration. The default, and the domain of kernel
    /// (`SO_TIMESTAMPING`) and NIC timestamps.
    ///
    /// The front [`Clock`](crate::Clock) stays on UTC while its owner calls
    /// [`Clock::update`](crate::Clock::update) about once a second: the rate
    /// follows `CLOCK_REALTIME`'s within ±500 ppm and the clock never goes
    /// back. A fixed segment ([`advanced::Clock`](crate::advanced::Clock))
    /// keeps its own rate unless its owner disciplines it
    /// ([`Discipline`](crate::advanced::discipline::Discipline)): left alone
    /// it does not follow NTP's steps or slews of the wall clock (see
    /// [`WallOffset`](crate::advanced::WallOffset)).
    #[default]
    Realtime,
    /// Nanoseconds since boot: the platform's monotonic clock
    /// (`CLOCK_MONOTONIC` on Linux, macOS's `CLOCK_UPTIME_RAW`, both
    /// `std::time::Instant`'s) at calibration. Opt-in.
    ///
    /// The counter clock then keeps the rate it was calibrated at,
    /// `CLOCK_MONOTONIC_RAW`'s, not NTP's corrections of `CLOCK_MONOTONIC`,
    /// so on Linux the two part by NTP's frequency correction (the
    /// oscillator's error, a few ppm: milliseconds an hour) and by any slew
    /// in progress (hundreds of ppm while systemd-timesyncd removes a boot's
    /// offset: `clock/BENCHMARKS.md`). Stamps compared
    /// across processes over hours come from one calibration
    /// ([`Params`](crate::advanced::Params)), not from `Instant` in the other.
    Monotonic,
}

impl Domain {
    pub(crate) const fn code(self) -> u32 {
        match self {
            Self::Monotonic => 0,
            Self::Realtime => 1,
        }
    }

    pub(crate) const fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Monotonic),
            1 => Some(Self::Realtime),
            _ => None,
        }
    }

    /// The name the report prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Monotonic => "monotonic",
            Self::Realtime => "realtime",
        }
    }
}

/// Anything that tells the time in nanoseconds.
///
/// The calibrated clocks, and the manual and stepped ones that make tests
/// and replays deterministic. Code that reads the time takes a `T: TimeSource` and is monomorphised
/// for the clock it is given, so a read is never a dynamic call.
pub trait TimeSource {
    /// The time now, ns, unordered: the read may be taken before earlier
    /// instructions finish (event time, pacing, freshness). Never a
    /// measurement's begin.
    fn now_ns(&self) -> u64;

    /// The time now, ns, ordered: taken after every earlier instruction
    /// (a measurement bracket's end). The same as `now_ns` for a source
    /// with no cheaper unordered read.
    #[inline]
    fn now_ns_ordered(&self) -> u64 {
        self.now_ns()
    }

    /// The time now, ns, for a measurement bracket's begin: ordered after
    /// earlier work and before the region's work can start. Defaults to
    /// [`now_ns_ordered`](Self::now_ns_ordered), which is what a source with
    /// no separate begin gives; the calibrated clocks override it.
    #[inline]
    fn now_ns_begin(&self) -> u64 {
        self.now_ns_ordered()
    }

    /// Nanoseconds since `start_ns`, never negative: a start a little
    /// ahead of now (another core's unordered read) is no time, not a wrap.
    #[inline]
    fn since_ns(&self, start_ns: u64) -> u64 {
        elapsed_ns(start_ns, self.now_ns())
    }

    /// The smallest step the source's reads advance by, ns.
    #[inline]
    fn resolution_ns(&self) -> u64 {
        1
    }
}

impl<T: TimeSource + ?Sized> TimeSource for &T {
    #[inline]
    fn now_ns(&self) -> u64 {
        (**self).now_ns()
    }
    #[inline]
    fn now_ns_ordered(&self) -> u64 {
        (**self).now_ns_ordered()
    }
    fn now_ns_begin(&self) -> u64 {
        (**self).now_ns_begin()
    }
    #[inline]
    fn since_ns(&self, start_ns: u64) -> u64 {
        (**self).since_ns(start_ns)
    }
    #[inline]
    fn resolution_ns(&self) -> u64 {
        (**self).resolution_ns()
    }
}

/// `end_ns - start_ns`, or 0 when `start_ns` is later: elapsed time is
/// never negative.
#[inline]
#[must_use]
pub const fn elapsed_ns(start_ns: u64, end_ns: u64) -> u64 {
    end_ns.saturating_sub(start_ns)
}

/// The hardware counter scaled to nanoseconds.
///
/// `rdtsc` on x86-64, `CNTVCT_EL0` on 64-bit Arm, through one [`Scale`]. Made only by
/// [`calibrate`](crate::advanced::calibrate), which checked the counter is usable
/// here, or from another calibration's [`Params`]. `Copy`, with no
/// thread-local or shared state: one calibration, copied to every thread,
/// gives every thread the same timebase. Its ordered read was chosen when
/// it was made ([`OrderedRead`]): an ordered read branches on that field of
/// the clock (Arm only; a predictable branch, measured in
/// `clock/BENCHMARKS.md`), never on anything global.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterClock {
    scale: Scale,
    domain: Domain,
    ordered: OrderedRead,
}

/// Counts begin reads in this crate's tests. Both reads report "now" and
/// differ only in ordering, so no assertion on their values can show which
/// one a caller reached; this can. Not compiled into the library.
#[cfg(test)]
mod begin_reads {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNT: AtomicU64 = AtomicU64::new(0);

    pub(super) fn tick() {
        COUNT.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn get() -> u64 {
        COUNT.load(Ordering::Relaxed)
    }
}

impl OrderedRead {
    /// The read itself. Crate-private: a variant names an instruction a CPU
    /// may lack, so only detection (calibration, `from_params`) picks one.
    #[inline]
    pub(crate) fn read(self) -> u64 {
        #[cfg(target_arch = "aarch64")]
        if self == Self::Cntvctss {
            return arch::ticks_self_synchronised();
        }
        #[cfg(not(target_arch = "aarch64"))]
        let _ = self;
        arch::ticks_ordered()
    }

    /// The read for a measurement's begin: ordered after earlier work and
    /// before later work can start. On x86 `rdtscp` alone does not stop the
    /// region's first instructions from issuing before it, so this is
    /// `rdtscp; lfence`. On Arm the ordered read already fences both ways
    /// (`isb; mrs`, and `CNTVCTSS_EL0` is self-synchronised), so it is the
    /// same read as [`read`](Self::read).
    #[inline]
    pub(crate) fn read_begin(self) -> u64 {
        #[cfg(test)]
        begin_reads::tick();
        #[cfg(target_arch = "x86_64")]
        {
            let _ = self;
            arch::ticks_ordered_fenced()
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            self.read()
        }
    }
}

impl CounterClock {
    pub(crate) const fn new(scale: Scale, domain: Domain, ordered: OrderedRead) -> Self {
        Self {
            scale,
            domain,
            ordered,
        }
    }

    /// The counter, unordered (`rdtsc`, `mrs cntvct_el0`): raw ticks with no
    /// per-process base, so comparable across processes on one host.
    #[inline]
    #[must_use]
    pub fn ticks(&self) -> u64 {
        arch::ticks()
    }

    /// The counter, ordered (`rdtscp`; `mrs cntvctss_el0` where the CPU has
    /// `FEAT_ECV`, else `isb; mrs cntvct_el0`), and a compiler barrier: a
    /// measurement bracket's end.
    #[inline]
    #[must_use]
    pub fn ticks_ordered(&self) -> u64 {
        self.ordered.read()
    }

    /// The counter for a measurement bracket's begin: ordered after earlier
    /// work and before the region's work can start ([`OrderedRead`]).
    #[inline]
    #[must_use]
    pub fn ticks_begin(&self) -> u64 {
        self.ordered.read_begin()
    }

    /// The time now, ns, in the clock's domain, from an unordered read.
    ///
    /// Event stamps, deadlines and ages. Never the start of a measurement:
    /// the read may be taken before earlier work finishes, which makes the
    /// interval too long, and the region's work may start before it, which
    /// makes it too short.
    #[inline]
    #[must_use]
    pub fn now_ns(&self) -> u64 {
        self.scale.to_ns(arch::ticks())
    }

    /// The time now, ns, from an ordered read: a measurement bracket's end.
    #[inline]
    #[must_use]
    pub fn now_ns_ordered(&self) -> u64 {
        self.scale.to_ns(self.ordered.read())
    }

    /// The time now, ns, for a measurement bracket's begin
    /// ([`ticks_begin`](Self::ticks_begin)).
    #[inline]
    #[must_use]
    pub fn now_ns_begin(&self) -> u64 {
        self.scale.to_ns(self.ordered.read_begin())
    }

    /// The time, ns, at counter value `ticks`.
    #[inline]
    #[must_use]
    pub const fn to_ns(&self, ticks: u64) -> u64 {
        self.scale.to_ns(ticks)
    }

    /// The duration, ns, of `ticks` ticks.
    #[inline]
    #[must_use]
    pub const fn ticks_to_ns(&self, ticks: u64) -> u64 {
        self.scale.ticks_to_ns(ticks)
    }

    /// The ticks in `ns` nanoseconds (a division: compute it once).
    #[must_use]
    pub const fn ns_to_ticks(&self, ns: u64) -> u64 {
        self.scale.ns_to_ticks(ns)
    }

    /// The counter's rate, Hz.
    #[must_use]
    pub const fn hz(&self) -> u64 {
        self.scale.hz()
    }

    /// The conversion.
    #[must_use]
    pub const fn scale(&self) -> Scale {
        self.scale
    }

    /// The domain its nanoseconds count from.
    #[must_use]
    pub const fn domain(&self) -> Domain {
        self.domain
    }

    /// The instruction its ordered reads take.
    #[must_use]
    pub const fn ordered_read(&self) -> OrderedRead {
        self.ordered
    }

    /// The calibration as C-layout parameters (another process, or the
    /// DPDK transport's C clock, can adopt it).
    #[must_use]
    pub const fn params(&self) -> Params {
        Params {
            version: PARAMS_VERSION,
            kind: KIND_COUNTER,
            domain: self.domain.code(),
            reserved: 0,
            base_ticks: self.scale.base_ticks(),
            base_ns: self.scale.base_ns(),
            mult: self.scale.mult(),
            hz: self.scale.hz(),
        }
    }

    /// The same time at `at_ticks`, then `mult`: [`Scale::retuned`] (a
    /// re-anchor; `None` out of range). The ordered read is kept.
    #[must_use]
    pub const fn retuned(self, at_ticks: u64, mult: u64, forward_ns: u64) -> Option<Self> {
        match self.scale.retuned(at_ticks, mult, forward_ns) {
            Some(scale) => Some(Self { scale, ..self }),
            None => None,
        }
    }

    /// The counter clock another calibration published, for this process
    /// to share its timebase. Its ordered read is this CPU's
    /// ([`HostFacts::ordered_read`](crate::advanced::HostFacts::ordered_read)): the
    /// parameters do not carry one.
    ///
    /// # Errors
    /// [`Error::InvalidParams`] for parameters of another version, of the
    /// monotonic fallback, of an unknown domain, or whose factor and rate
    /// disagree or are out of range; [`Error::CounterUnavailable`] where this
    /// process cannot read the counter (another architecture, no `rdtscp`).
    pub fn from_params(params: &Params) -> Result<Self, Error> {
        let (scale, domain) = checked(params, KIND_COUNTER)?;
        counter_readable()?;
        Ok(Self::new(scale, domain, detect::this_cpu_ordered_read()))
    }
}

/// `params`' conversion and domain, checked: its version, its kind
/// (`kind`), its domain, its factor's range, and its rate against the
/// factor's.
fn checked(params: &Params, kind: u32) -> Result<(Scale, Domain), Error> {
    if params.version != PARAMS_VERSION {
        return Err(Error::InvalidParams("version"));
    }
    if params.kind != kind {
        return Err(Error::InvalidParams(if kind == KIND_COUNTER {
            "kind: not the counter"
        } else {
            "kind: not the monotonic fallback"
        }));
    }
    let domain = Domain::from_code(params.domain).ok_or(Error::InvalidParams("domain"))?;
    let scale = Scale::from_mult(params.mult, params.base_ticks, params.base_ns)
        .ok_or(Error::InvalidParams("mult"))?;
    // the rate stated must be the one the factor stands for, within its
    // rounding (a few parts in 10^10 at GHz rates)
    if scale.hz().abs_diff(params.hz) > scale.hz() / 1_000_000 + 1 {
        return Err(Error::InvalidParams("hz: not the factor's rate"));
    }
    Ok((scale, domain))
}

/// Whether this process can read the counter at all: the architecture has
/// one and, on x86, `rdtscp` (the ordered read) exists.
#[allow(clippy::missing_const_for_fn)] // const only where the x86 check is compiled out
fn counter_readable() -> Result<(), Error> {
    #[cfg(target_arch = "x86_64")]
    if !arch::rdtscp_supported() {
        return Err(Error::CounterUnavailable(crate::detect::Fallback::NoRdtscp));
    }
    if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        Ok(())
    } else {
        Err(Error::CounterUnavailable(
            crate::detect::Fallback::NoCounter,
        ))
    }
}

impl TimeSource for CounterClock {
    #[inline]
    fn now_ns(&self) -> u64 {
        Self::now_ns(self)
    }
    #[inline]
    fn now_ns_ordered(&self) -> u64 {
        Self::now_ns_ordered(self)
    }
    #[inline]
    fn now_ns_begin(&self) -> u64 {
        Self::now_ns_begin(self)
    }
    #[inline]
    fn resolution_ns(&self) -> u64 {
        self.scale.tick_ns()
    }
}

/// One ns per ns in 32.32: the fallback's factor until it is disciplined.
pub const FACTOR_ONE: u64 = 1 << 32;

/// Marks the branch that calls it as rare: the disciplined fallback's
/// multiply goes after the counter's arm in `Clock::now_ns`, which keeps
/// its straight line (x86-64: no jump into a shared tail).
#[cold]
pub const fn cold() {}

/// The fallback: the platform's monotonic clock through the vDSO.
///
/// `CLOCK_MONOTONIC` on Linux, `CLOCK_UPTIME_RAW` on macOS: what
/// `std::time::Instant` reads, and slower than the counter. Never the wall
/// clock, which can step backwards. Its ticks are nanoseconds, converted by
/// the counter's formula (`base_ns + ((ticks - base_ticks) * mult) >> 32`):
/// until its owner disciplines it ([`retuned`](Self::retuned)) the factor
/// is one (2^32) and the base the domain's offset, which reads bit for bit
/// as the monotonic clock plus the offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonotonicClock {
    /// At the base: the monotonic clock's value (0 until retuned), the time
    /// (0, or the realtime domain's offset, until retuned), and ns per
    /// monotonic ns, 32.32 (2^32 until retuned).
    conversion: Conversion,
    /// Under 4.3 s (a read that advanced by more is not a clock); 32 bits
    /// so the clock is four words and `Clock` stays five.
    resolution_ns: u32,
    domain: Domain,
}

impl MonotonicClock {
    /// The fallback in `domain`: for the realtime domain the offset from the
    /// monotonic clock to `CLOCK_REALTIME`, the tightest of 16 bracketed
    /// reads, taken now and kept (it does not follow NTP afterwards unless
    /// its owner disciplines it).
    ///
    /// # Errors
    /// [`Error::ClockRead`] when a clock cannot be read.
    pub fn new(domain: Domain) -> Result<Self, Error> {
        let resolution_ns = wall::monotonic_res_ns()?;
        let offset_ns = match domain {
            Domain::Monotonic => 0,
            Domain::Realtime => {
                let (mono, real, _) = wall::bracket(
                    || wall::read(wall::REALTIME, "CLOCK_REALTIME"),
                    wall::monotonic,
                )?;
                real.wrapping_sub(mono)
            }
        };
        wall::monotonic()?;
        Ok(Self::unretuned(offset_ns, domain, resolution_ns))
    }

    /// The factor one, the base at monotonic 0.
    pub(crate) const fn unretuned(offset_ns: u64, domain: Domain, resolution_ns: u64) -> Self {
        Self {
            conversion: Conversion {
                base_ticks: 0,
                base_ns: offset_ns,
                mult: FACTOR_ONE,
            },
            resolution_ns: narrow(resolution_ns),
            domain,
        }
    }

    /// The fallback in `domain` whatever happens, and [`new`](Self::new)'s
    /// error beside it where there was one: for a holder that must always
    /// have a clock, with no `Option` (and so no extra compare on every
    /// read) and no `Result` to carry.
    ///
    /// Where a kernel clock could not be read (no supported platform: the
    /// clock ids are constants and the pointer valid, so `clock_gettime`
    /// cannot fail there), the clock has no offset into the realtime domain
    /// and a 1 ns resolution, and reads what the monotonic clock gives
    /// (zero if it still fails): wrong, never a panic, and the error says
    /// why. Its reads are [`now_ns`](Self::now_ns)'s either way: nothing is
    /// added to them.
    #[must_use]
    pub fn always(domain: Domain) -> (Self, Option<Error>) {
        Self::or_unread(Self::new(domain), domain)
    }

    /// [`always`](Self::always) from `new`'s result.
    #[allow(clippy::missing_const_for_fn)] // the error has a destructor
    fn or_unread(made: Result<Self, Error>, domain: Domain) -> (Self, Option<Error>) {
        match made {
            Ok(clock) => (clock, None),
            Err(error) => (Self::unretuned(0, domain, 1), Some(error)),
        }
    }

    /// The fallback another process (or this one's discipline) published:
    /// its base and factor, this process's monotonic clock as the ticks.
    /// Measures the monotonic clock's resolution (cold: up to 64 advances
    /// of the clock).
    ///
    /// # Errors
    /// [`Error::InvalidParams`] for parameters of another version, of the
    /// counter, of an unknown domain, or whose factor and rate disagree or
    /// are out of range; [`Error::ClockRead`] when the monotonic clock
    /// cannot be read.
    pub fn from_params(params: &Params) -> Result<Self, Error> {
        let (scale, domain) = checked(params, KIND_MONOTONIC)?;
        let resolution_ns = wall::monotonic_res_ns()?;
        Ok(Self::from_scale(scale, domain, narrow(resolution_ns)))
    }

    /// The clock of `scale`'s conversion.
    const fn from_scale(scale: Scale, domain: Domain, resolution_ns: u32) -> Self {
        Self {
            conversion: Conversion {
                base_ticks: scale.base_ticks(),
                base_ns: scale.base_ns(),
                mult: scale.mult(),
            },
            resolution_ns,
            domain,
        }
    }

    /// The same time at monotonic `at_ticks`, then `mult`: a re-anchor
    /// ([`Scale::retuned`]; `None` out of range). Resolution and domain are
    /// kept.
    #[must_use]
    pub const fn retuned(self, at_ticks: u64, mult: u64, forward_ns: u64) -> Option<Self> {
        let Some(base_ns) = self.to_ns(at_ticks).checked_add(forward_ns) else {
            return None;
        };
        if scale::hz_of(mult).is_none() {
            return None;
        }
        Some(Self {
            conversion: Conversion {
                base_ticks: at_ticks,
                base_ns,
                mult,
            },
            ..self
        })
    }

    /// The monotonic clock, ns (its ticks).
    #[inline]
    #[must_use]
    pub fn ticks(&self) -> u64 {
        arch::clock_ns_unchecked(wall::MONOTONIC)
    }

    /// The time now, ns, in the clock's domain.
    #[inline]
    #[must_use]
    pub fn now_ns(&self) -> u64 {
        self.read_ns(self.ticks())
    }

    /// [`to_ns`](Self::to_ns) of a read just taken. With a factor of one
    /// (as made, until its owner disciplines it) the formula is an add, bit
    /// for bit, and the read takes it: a predicted branch on the factor
    /// keeps the multiply out of the read. The kernel's next read waits at
    /// its barrier for this one's result, so the multiply cost 1.6 to 2.6 ns
    /// a read (clock/BENCHMARKS.md).
    #[inline]
    const fn read_ns(&self, ticks: u64) -> u64 {
        let c = &self.conversion;
        if c.mult == FACTOR_ONE {
            ticks.wrapping_add(c.base_ns.wrapping_sub(c.base_ticks))
        } else {
            cold();
            c.ns_at(ticks)
        }
    }

    /// The time, ns, at monotonic value `ticks`.
    #[inline]
    #[must_use]
    pub const fn to_ns(&self, ticks: u64) -> u64 {
        self.conversion.ns_at(ticks)
    }

    /// The duration, ns, of `ticks` monotonic nanoseconds at the clock's
    /// factor (they are equal until it is retuned).
    #[inline]
    #[must_use]
    pub const fn ticks_to_ns(&self, ticks: u64) -> u64 {
        scale::ticks_to_ns(ticks, self.conversion.mult)
    }

    /// The monotonic nanoseconds in `ns` of the clock's (a division:
    /// compute it once).
    #[must_use]
    pub const fn ns_to_ticks(&self, ns: u64) -> u64 {
        scale::ns_to_ticks(ns, self.conversion.mult)
    }

    /// The rate its factor stands for, Hz: 10^9 until retuned.
    #[must_use]
    pub const fn hz(&self) -> u64 {
        match scale::hz_of(self.conversion.mult) {
            Some(hz) => hz,
            None => 1_000_000_000,
        }
    }

    /// The domain its nanoseconds count from.
    #[must_use]
    pub const fn domain(&self) -> Domain {
        self.domain
    }

    /// The calibration as C-layout parameters: the counter's formula with
    /// the monotonic clock as the ticks (a factor of one, 2^32, and a base
    /// at 0 until retuned).
    #[must_use]
    pub const fn params(&self) -> Params {
        Params {
            version: PARAMS_VERSION,
            kind: KIND_MONOTONIC,
            domain: self.domain.code(),
            reserved: 0,
            base_ticks: self.conversion.base_ticks,
            base_ns: self.conversion.base_ns,
            mult: self.conversion.mult,
            hz: self.hz(),
        }
    }
}

/// A resolution in 32 bits: over 4.3 s is no clock, so saturate.
#[allow(clippy::cast_possible_truncation)] // range-checked first
const fn narrow(resolution_ns: u64) -> u32 {
    if resolution_ns > u32::MAX as u64 {
        u32::MAX
    } else {
        resolution_ns as u32
    }
}

impl TimeSource for MonotonicClock {
    #[inline]
    fn now_ns(&self) -> u64 {
        Self::now_ns(self)
    }
    #[inline]
    fn resolution_ns(&self) -> u64 {
        u64::from(self.resolution_ns)
    }
}

/// The calibrated clock.
///
/// The counter where calibration found it usable, else the monotonic
/// fallback. One predictable branch a read picks between
/// them; code that must not pay even that matches once and runs generic
/// over the [`CounterClock`] or [`MonotonicClock`] inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// The hardware counter.
    Counter(CounterClock),
    /// The monotonic fallback.
    Monotonic(MonotonicClock),
}

// Five words: the counter's four-word conversion and its two bytes (the
// domain and the ordered read, whose spare values are the tag), with the
// fallback's four words beside them. A fifth word on the fallback, or
// anything added to `Scale`, grows every copy of the clock and moves the
// tag: the hot path's code is gated on this layout (clock/BENCHMARKS.md).
const _: () = assert!(size_of::<Clock>() == 40);
const _: () = assert!(size_of::<MonotonicClock>() == 32);

impl Clock {
    /// The monotonic fallback in `domain`, always, and the error beside it
    /// where a kernel clock could not be read ([`MonotonicClock::always`]):
    /// a clock to hold where calibration has not run (or need not), with
    /// no `Option` around it. Its reads are this enum's, one predictable
    /// branch, and a counter clock a calibration hands over later replaces
    /// it at the same cost: the counter's read path is unchanged.
    #[must_use]
    pub fn fallback(domain: Domain) -> (Self, Option<Error>) {
        let (clock, error) = MonotonicClock::always(domain);
        (Self::Monotonic(clock), error)
    }

    /// Whether this is the hardware counter.
    #[must_use]
    pub const fn is_counter(&self) -> bool {
        matches!(self, Self::Counter(_))
    }

    /// The time now, ns, unordered.
    #[inline]
    #[must_use]
    pub fn now_ns(&self) -> u64 {
        match self {
            Self::Counter(c) => c.now_ns(),
            Self::Monotonic(m) => m.now_ns(),
        }
    }

    /// The time now, ns, ordered.
    #[inline]
    #[must_use]
    pub fn now_ns_ordered(&self) -> u64 {
        match self {
            Self::Counter(c) => c.now_ns_ordered(),
            Self::Monotonic(m) => m.now_ns(),
        }
    }

    /// The time now, ns, for a measurement bracket's begin. The monotonic
    /// fallback's read is a vDSO call, which orders both ways already.
    #[inline]
    #[must_use]
    pub fn now_ns_begin(&self) -> u64 {
        match self {
            Self::Counter(c) => c.now_ns_begin(),
            Self::Monotonic(m) => m.now_ns(),
        }
    }

    /// Raw ticks for a measurement bracket's begin.
    #[inline]
    #[must_use]
    pub fn ticks_begin(&self) -> u64 {
        match self {
            Self::Counter(c) => c.ticks_begin(),
            Self::Monotonic(m) => m.ticks(),
        }
    }

    /// Raw ticks, unordered (the monotonic clock's ns for the fallback).
    #[inline]
    #[must_use]
    pub fn ticks(&self) -> u64 {
        match self {
            Self::Counter(c) => c.ticks(),
            Self::Monotonic(m) => m.ticks(),
        }
    }

    /// Raw ticks, ordered.
    #[inline]
    #[must_use]
    pub fn ticks_ordered(&self) -> u64 {
        match self {
            Self::Counter(c) => c.ticks_ordered(),
            Self::Monotonic(m) => m.ticks(),
        }
    }

    /// The time, ns, at tick `ticks`.
    #[inline]
    #[must_use]
    pub const fn to_ns(&self, ticks: u64) -> u64 {
        match self {
            Self::Counter(c) => c.to_ns(ticks),
            Self::Monotonic(m) => m.to_ns(ticks),
        }
    }

    /// The duration, ns, of `ticks` ticks.
    #[inline]
    #[must_use]
    pub const fn ticks_to_ns(&self, ticks: u64) -> u64 {
        match self {
            Self::Counter(c) => c.ticks_to_ns(ticks),
            Self::Monotonic(m) => m.ticks_to_ns(ticks),
        }
    }

    /// The ticks in `ns` nanoseconds (compute it once).
    #[must_use]
    pub const fn ns_to_ticks(&self, ns: u64) -> u64 {
        match self {
            Self::Counter(c) => c.ns_to_ticks(ns),
            Self::Monotonic(m) => m.ns_to_ticks(ns),
        }
    }

    /// Ticks a second (the rate the factor stands for: 10^9 for the
    /// fallback until it is retuned).
    #[must_use]
    pub const fn hz(&self) -> u64 {
        match self {
            Self::Counter(c) => c.hz(),
            Self::Monotonic(m) => m.hz(),
        }
    }

    /// The same time at `at_ticks` (this clock's ticks), `forward_ns`
    /// later, then `mult`: a re-anchor, the one primitive a disciplined
    /// clock is made of ([`Scale::retuned`]). `None` where `mult` is out of
    /// range or the time would overflow. Pure: no read, no CPUID.
    ///
    /// With `at_ticks` an ordered read taken after every read of this clock
    /// on its owner's thread, and `forward_ns` unsigned, no read of the new
    /// clock on that thread is earlier than a read of the old one.
    #[must_use]
    pub const fn retuned(&self, at_ticks: u64, mult: u64, forward_ns: u64) -> Option<Self> {
        match self {
            Self::Counter(c) => match c.retuned(at_ticks, mult, forward_ns) {
                Some(c) => Some(Self::Counter(c)),
                None => None,
            },
            Self::Monotonic(m) => match m.retuned(at_ticks, mult, forward_ns) {
                Some(m) => Some(Self::Monotonic(m)),
                None => None,
            },
        }
    }

    /// The clock `p` describes, from this one: how a copy follows its
    /// owner. Of this clock's kind, `p`'s conversion with this copy's
    /// ordered read or resolution kept (no CPUID, no measurement: a check
    /// and a division). Of the other kind, that kind's `from_params` (cold:
    /// detection, or a resolution measured).
    ///
    /// # Errors
    /// As the kind's `from_params`: [`Error::InvalidParams`] for parameters
    /// of another version, an unknown kind or domain, or a factor and rate
    /// that disagree or are out of range.
    pub fn adopt(&self, p: &Params) -> Result<Self, Error> {
        match (self, p.kind) {
            (Self::Counter(c), KIND_COUNTER) => {
                let (scale, domain) = checked(p, KIND_COUNTER)?;
                Ok(Self::Counter(CounterClock::new(scale, domain, c.ordered)))
            }
            (Self::Monotonic(m), KIND_MONOTONIC) => {
                let (scale, domain) = checked(p, KIND_MONOTONIC)?;
                Ok(Self::Monotonic(MonotonicClock::from_scale(
                    scale,
                    domain,
                    m.resolution_ns,
                )))
            }
            (_, KIND_COUNTER) => CounterClock::from_params(p).map(Self::Counter),
            (_, KIND_MONOTONIC) => MonotonicClock::from_params(p).map(Self::Monotonic),
            _ => Err(Error::InvalidParams("kind")),
        }
    }

    /// The domain its nanoseconds count from.
    #[must_use]
    pub const fn domain(&self) -> Domain {
        match self {
            Self::Counter(c) => c.domain(),
            Self::Monotonic(m) => m.domain(),
        }
    }

    /// The calibration as C-layout parameters.
    #[must_use]
    pub const fn params(&self) -> Params {
        match self {
            Self::Counter(c) => c.params(),
            Self::Monotonic(m) => m.params(),
        }
    }
}

impl TimeSource for Clock {
    #[inline]
    fn now_ns(&self) -> u64 {
        Self::now_ns(self)
    }
    #[inline]
    fn now_ns_ordered(&self) -> u64 {
        Self::now_ns_ordered(self)
    }
    // Without this a generic caller gets the trait's default, which is
    // `now_ns_ordered`, and so never reaches the counter clock's begin read.
    #[inline]
    fn now_ns_begin(&self) -> u64 {
        Self::now_ns_begin(self)
    }
    #[inline]
    fn resolution_ns(&self) -> u64 {
        match self {
            Self::Counter(c) => c.resolution_ns(),
            Self::Monotonic(m) => m.resolution_ns(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where `new` failed, `always` still gives a clock (no offset, a 1 ns
    /// resolution, the domain asked for) and the error beside it; where it
    /// did not, `new`'s clock and no error.
    #[test]
    fn always_keeps_the_error_beside_a_clock() -> Result<(), Error> {
        let failed = Error::ClockRead {
            clock: "CLOCK_REALTIME",
            errno: 22,
        };
        let (clock, error) = MonotonicClock::or_unread(Err(failed.clone()), Domain::Realtime);
        assert_eq!(error, Some(failed));
        assert_eq!(clock.domain(), Domain::Realtime);
        assert_eq!(clock.params().base_ns, 0);
        assert_eq!(TimeSource::resolution_ns(&clock), 1);
        let made = MonotonicClock::new(Domain::Realtime)?;
        assert_eq!(
            MonotonicClock::or_unread(Ok(made), Domain::Realtime),
            (made, None)
        );
        Ok(())
    }

    /// xorshift64*: deterministic, no dependency.
    fn xorshift(x: &mut u64) -> u64 {
        *x ^= *x >> 12;
        *x ^= *x << 25;
        *x ^= *x >> 27;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// P11: until retuned, the fallback's conversion (now the counter's
    /// formula with a factor of one and a base at 0) reads bit for bit what
    /// it read before (the monotonic clock plus the offset, wrapping), over
    /// the whole range of ticks and offsets, and its parameters, duration
    /// conversions and rate are unchanged.
    #[test]
    fn an_unretuned_fallback_reads_bit_for_bit_as_before() {
        let mut x = 0x9e37_79b9_7f4a_7c15_u64;
        for i in 0..100_000 {
            let offset = match i % 4 {
                0 => 0,
                1 => 1_760_000_000_000_000_000 + xorshift(&mut x) % 1_000_000_000_000_000,
                _ => xorshift(&mut x),
            };
            let m = MonotonicClock::unretuned(offset, Domain::Realtime, 42);
            let ticks = match i % 3 {
                0 => xorshift(&mut x) >> 10,
                1 => xorshift(&mut x),
                _ => u64::MAX - xorshift(&mut x) % 1_000,
            };
            assert_eq!(
                m.to_ns(ticks),
                ticks.wrapping_add(offset),
                "{ticks} + {offset}"
            );
            assert_eq!(m.ticks_to_ns(ticks), ticks);
            assert_eq!(m.ns_to_ticks(ticks), ticks);
            let p = m.params();
            assert_eq!(
                (p.kind, p.base_ticks, p.base_ns, p.mult, p.hz),
                (KIND_MONOTONIC, 0, offset, 1 << 32, 1_000_000_000)
            );
            assert_eq!(Clock::Monotonic(m).hz(), 1_000_000_000);
        }
    }

    /// The fallback's read converts as its formula, bit for bit: with a
    /// factor of one (as made, or re-anchored at a rate of exactly zero, at
    /// any base) and with any other factor, over the whole range of ticks.
    #[test]
    fn the_fallback_read_converts_as_its_formula() -> Result<(), Error> {
        let mut x = 0x2545_f491_4f6c_dd1d_u64;
        for i in 0..100_000 {
            let mult = match i % 4 {
                0 | 1 => 1 << 32,
                2 => (1 << 32) - (1 << 21) + xorshift(&mut x) % (1 << 22),
                _ => (1 << 26) + xorshift(&mut x) % ((1 << 40) - (1 << 26)),
            };
            let (base_ticks, base_ns) = if i % 8 == 0 {
                (0, xorshift(&mut x))
            } else {
                (xorshift(&mut x), xorshift(&mut x))
            };
            let m = MonotonicClock::from_scale(
                Scale::from_mult(mult, base_ticks, base_ns).ok_or(Error::InvalidParams("mult"))?,
                Domain::Realtime,
                1,
            );
            let ticks = match i % 3 {
                0 => base_ticks.wrapping_add(xorshift(&mut x) >> 20),
                1 => xorshift(&mut x),
                _ => base_ticks.wrapping_sub(xorshift(&mut x) % 1_000),
            };
            assert_eq!(
                m.read_ns(ticks),
                m.to_ns(ticks),
                "{ticks} at {base_ticks}, {base_ns}, {mult}"
            );
        }
        Ok(())
    }

    /// P13: the clock is five words: a fifth word on the fallback, or
    /// anything added to `Scale`, would grow every copy and move the tag.
    #[test]
    fn the_clock_is_five_words() {
        assert_eq!(size_of::<Clock>(), 40);
        assert_eq!(size_of::<MonotonicClock>(), 32);
        assert_eq!(size_of::<CounterClock>(), 40);
        assert_eq!(size_of::<Scale>(), 32);
    }

    /// A re-anchor of either kind reads at its anchor what the old one read
    /// there, plus the forward step, keeps the domain, the ordered read and
    /// the resolution, and refuses a factor out of range; a fallback's
    /// durations follow its factor.
    #[test]
    fn either_kind_reanchors_where_it_reads() -> Result<(), Error> {
        let scale = Scale::from_hz(2_400_000_000, 1_000, 1_760_000_000_000_000_000)
            .ok_or(Error::InvalidParams("rate"))?;
        let counter = Clock::Counter(CounterClock::new(
            scale,
            Domain::Realtime,
            OrderedRead::Cntvctss,
        ));
        let fallback = Clock::Monotonic(MonotonicClock::unretuned(77, Domain::Realtime, 41));
        for clock in [counter, fallback] {
            let m0 = clock.params().mult;
            for (at, mult, forward) in [
                (5_000, m0 + m0 / 2_000, 0),
                (9_999_999, m0 - m0 / 2_000, 123),
            ] {
                let r = clock
                    .retuned(at, mult, forward)
                    .ok_or(Error::InvalidParams("mult"))?;
                assert_eq!(r.to_ns(at), clock.to_ns(at) + forward);
                assert_eq!(r.params().mult, mult);
                assert_eq!(r.domain(), clock.domain());
                assert_eq!(r.resolution_ns(), clock.resolution_ns());
                assert_eq!(
                    r.to_ns(at + 1_000_000) - r.to_ns(at),
                    r.ticks_to_ns(1_000_000)
                );
                // within the ticks of one nanosecond (2.4 at 2.4 GHz)
                assert!(r.ns_to_ticks(r.ticks_to_ns(1_000_000)).abs_diff(1_000_000) <= 3);
                if let (Clock::Counter(a), Clock::Counter(b)) = (clock, r) {
                    assert_eq!(a.ordered_read(), b.ordered_read());
                }
            }
            assert!(clock.retuned(5_000, 0, 0).is_none());
            assert!(clock.retuned(5_000, 1 << 63, 0).is_none());
        }
        Ok(())
    }

    /// A generic `TimeSource` caller reaches the same three reads as a
    /// concrete one. Without the override on `impl TimeSource for Clock` the
    /// begin read silently becomes the ordered one for every generic caller,
    /// and the measured region can start before its timestamp completes.
    ///
    /// Both reads report "now" and differ only in ordering (and on Arm they
    /// are the same instruction), so the test pins *which read ran*, through
    /// [`begin_reads`], and not how the two values compare: consecutive
    /// reads are never equal, and no value shows a fence.
    #[test]
    fn a_generic_caller_sees_the_same_reads_as_a_concrete_one() -> Result<(), Error> {
        let scale = Scale::from_hz(2_400_000_000, 1_000, 1_760_000_000_000_000_000)
            .ok_or(Error::InvalidParams("rate"))?;
        // the read this CPU implements: `Cntvctss` traps where ECV is absent,
        // and a test must not fault on the hardware it runs on
        let clock = Clock::Counter(CounterClock::new(
            scale,
            Domain::Realtime,
            crate::detect::this_cpu_ordered_read(),
        ));

        // the generic path takes the begin read, and only for `now_ns_begin`
        let before = begin_reads::get();
        let generic: fn(&Clock) -> u64 = TimeSource::now_ns_begin;
        let b = generic(&clock);
        assert_eq!(begin_reads::get(), before + 1, "the begin read is taken");
        assert!(b > 0, "the time is read");

        let before = begin_reads::get();
        let o = TimeSource::now_ns_ordered(&clock);
        let n = TimeSource::now_ns(&clock);
        assert_eq!(
            begin_reads::get(),
            before,
            "the other two take no begin read"
        );
        assert!(o > 0 && n > 0, "the times are read");

        // the concrete path is the same read, one each
        let before = begin_reads::get();
        let _ = clock.now_ns_begin();
        assert_eq!(begin_reads::get(), before + 1, "the inherent method agrees");
        // through a reference, which forwards every method
        let before = begin_reads::get();
        let _ = TimeSource::now_ns_begin(&&clock);
        assert_eq!(begin_reads::get(), before + 1, "through a reference");

        // a bracket taken generically does not run backwards
        let (start, end) = (
            TimeSource::now_ns_begin(&clock),
            TimeSource::now_ns_ordered(&clock),
        );
        assert!(end >= start || start - end < 1_000_000_000, "{start} {end}");

        // the fallback's begin read is its only read, so the override is
        // indistinguishable there; a generic caller still gets a bracket
        let fallback = Clock::Monotonic(MonotonicClock::unretuned(77, Domain::Realtime, 41));
        let (start, end) = (
            TimeSource::now_ns_begin(&fallback),
            TimeSource::now_ns_ordered(&fallback),
        );
        assert!(end >= start || start - end < 1_000_000_000, "{start} {end}");
        Ok(())
    }

    /// A copy adopts its owner's parameters: of its own kind keeping its
    /// ordered read and resolution (no detection, no measurement); of the
    /// other kind through that kind's `from_params`; and refuses
    /// parameters it cannot use.
    #[test]
    fn a_copy_adopts_its_owners_parameters() -> Result<(), Error> {
        // an ordered read this CPU would not pick: kept, not detected again
        let foreign = if cfg!(target_arch = "x86_64") {
            OrderedRead::IsbCntvct
        } else {
            OrderedRead::Rdtscp
        };
        let scale = Scale::from_hz(1_000_000_000, 10, 1_760_000_000_000_000_000)
            .ok_or(Error::InvalidParams("rate"))?;
        let counter = Clock::Counter(CounterClock::new(scale, Domain::Realtime, foreign));
        let owner = counter
            .retuned(1_000, (1 << 32) + 2_000, 5)
            .ok_or(Error::InvalidParams("mult"))?;
        let copy = counter.adopt(&owner.params())?;
        assert_eq!(copy, owner, "the same conversion, the same ordered read");
        let fallback = Clock::Monotonic(MonotonicClock::unretuned(77, Domain::Realtime, 41));
        let moved = fallback
            .retuned(1_000, (1 << 32) - 3_000, 9)
            .ok_or(Error::InvalidParams("mult"))?;
        let copy = fallback.adopt(&moved.params())?;
        assert_eq!(copy, moved, "the resolution kept, not measured");
        // the other kind: the fallback's own from_params (a measurement)
        let other = counter.adopt(&moved.params())?;
        assert!(!other.is_counter());
        assert_eq!(other.params(), moved.params());
        // parameters it cannot use
        let mut bad = owner.params();
        bad.hz += bad.hz / 100;
        assert_eq!(
            counter.adopt(&bad),
            Err(Error::InvalidParams("hz: not the factor's rate"))
        );
        let mut bad = owner.params();
        bad.kind = 7;
        assert_eq!(counter.adopt(&bad), Err(Error::InvalidParams("kind")));
        let mut bad = owner.params();
        bad.version = 2;
        assert_eq!(counter.adopt(&bad), Err(Error::InvalidParams("version")));
        let mut bad = moved.params();
        bad.domain = 9;
        assert_eq!(fallback.adopt(&bad), Err(Error::InvalidParams("domain")));
        Ok(())
    }

    /// The fallback from parameters: kind 0 only, its base and factor
    /// taken, its resolution measured.
    #[test]
    fn the_fallback_from_parameters() -> Result<(), Error> {
        let made = MonotonicClock::new(Domain::Realtime)?;
        let back = MonotonicClock::from_params(&made.params())?;
        assert_eq!(back.params(), made.params());
        assert!(TimeSource::resolution_ns(&back) >= 1);
        let scale = Scale::from_hz(1_000_000_000, 0, 0).ok_or(Error::InvalidParams("rate"))?;
        let counter = CounterClock::new(scale, Domain::Realtime, OrderedRead::Rdtscp);
        assert_eq!(
            MonotonicClock::from_params(&counter.params()),
            Err(Error::InvalidParams("kind: not the monotonic fallback"))
        );
        Ok(())
    }
}
