//! Ticks to nanoseconds: one 32.32 fixed-point multiply.

const NS_PER_S: u64 = 1_000_000_000;
const MIN_HZ: u64 = 1_000;
const MAX_HZ: u64 = 100_000_000_000;

/// A counter's conversion to nanoseconds:
///
/// `ns = base_ns + ((ticks - base_ticks) * mult) >> 32`
///
/// where `mult` is nanoseconds per tick in 32.32 fixed point and the product
/// is 128 bits wide: the DPDK transport's C clock's formula exactly, so a
/// `Scale` and its parameters are interchangeable with it. The difference
/// `ticks - base_ticks` is taken as signed, so a tick a little behind the
/// base (an unordered read on another core, just after calibration)
/// converts to a little before `base_ns`, never to a wrapped, enormous time;
/// for ticks at or after the base this is the unsigned formula above.
///
/// Pure arithmetic: converting ticks recorded elsewhere (another thread,
/// another process on the same host) needs no counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scale {
    conversion: Conversion,
    /// The rate `mult` stands for, Hz (rounded).
    hz: u64,
}

/// The three words a conversion reads: [`Scale`]'s, and the monotonic
/// fallback's (`MonotonicClock`), so both convert by one function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conversion {
    pub base_ticks: u64,
    pub base_ns: u64,
    /// ns per tick, 32.32; in (0, 2^63) so the product can be signed.
    pub mult: u64,
}

impl Conversion {
    /// The time, ns, at counter value `ticks`: `base_ns + ((ticks -
    /// base_ticks) * mult) >> 32`, the difference signed and the product
    /// 128 bits wide.
    #[inline]
    #[must_use]
    pub const fn ns_at(&self, ticks: u64) -> u64 {
        let delta = ticks.wrapping_sub(self.base_ticks).cast_signed() as i128;
        // mult < 2^63, so both operands are sign-extended i64s: one widening
        // signed multiply (x86 `imul`, Arm `smulh` and `mul`)
        let scaled = (delta * self.mult.cast_signed() as i128) >> 32;
        #[allow(clippy::cast_possible_truncation)] // under 2^63 ns: 292 years from the base
        self.base_ns.wrapping_add_signed(scaled as i64)
    }
}

impl Scale {
    /// The conversion for a counter at `hz` (1 kHz to 100 GHz, else `None`)
    /// that read `base_ticks` at `base_ns`. The factor is rounded to the
    /// nearest 2^-32 ns: for 2.4 GHz a relative error under 2e-10.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // each cast is range-checked or bounded first
    pub const fn from_hz(hz: u64, base_ticks: u64, base_ns: u64) -> Option<Self> {
        if hz < MIN_HZ || hz > MAX_HZ {
            return None;
        }
        let wide = ((NS_PER_S as u128) << 32) + (hz as u128) / 2;
        let mult = (wide / hz as u128) as u64;
        Some(Self {
            conversion: Conversion {
                base_ticks,
                base_ns,
                mult,
            },
            hz,
        })
    }

    /// The conversion with factor `mult` (ns per tick, 32.32; in (0, 2^63)
    /// and standing for 1 kHz to 100 GHz, else `None`), as a calibration
    /// measured it or another process published it.
    #[must_use]
    pub const fn from_mult(mult: u64, base_ticks: u64, base_ns: u64) -> Option<Self> {
        let Some(hz) = hz_of(mult) else {
            return None;
        };
        Some(Self {
            conversion: Conversion {
                base_ticks,
                base_ns,
                mult,
            },
            hz,
        })
    }

    /// The same rate re-based: `base_ticks` read at `base_ns`.
    #[must_use]
    pub const fn rebased(self, base_ticks: u64, base_ns: u64) -> Self {
        Self {
            conversion: Conversion {
                base_ticks,
                base_ns,
                ..self.conversion
            },
            ..self
        }
    }

    /// The same time at `at_ticks`, then a new rate: the segment that
    /// starts where this one reads `at_ticks`, `forward_ns` later, and runs
    /// at `mult` (ns per tick, 32.32, range-checked as
    /// [`from_mult`](Self::from_mult): `None` out of range, or where the
    /// time would overflow).
    ///
    /// A clock disciplined to the wall clock is a chain of these: each
    /// re-anchor starts at the time the old segment reads, never earlier
    /// (`forward_ns` is unsigned), so a reader that crosses one never sees
    /// time go back. `at_ticks` is an ordered read taken after every read
    /// of the old segment on the owner's thread.
    #[must_use]
    pub const fn retuned(self, at_ticks: u64, mult: u64, forward_ns: u64) -> Option<Self> {
        let Some(base_ns) = self.to_ns(at_ticks).checked_add(forward_ns) else {
            return None;
        };
        Self::from_mult(mult, at_ticks, base_ns)
    }

    /// The time, ns, at counter value `ticks`.
    #[inline]
    #[must_use]
    pub const fn to_ns(&self, ticks: u64) -> u64 {
        self.conversion.ns_at(ticks)
    }

    /// The duration, ns, of `ticks` ticks.
    #[inline]
    #[must_use]
    pub const fn ticks_to_ns(&self, ticks: u64) -> u64 {
        ticks_to_ns(ticks, self.conversion.mult)
    }

    /// The ticks, rounded to the nearest, in `ns` nanoseconds: the inverse of
    /// [`ticks_to_ns`](Self::ticks_to_ns) within a tick. A division: compute
    /// thresholds once (a rate gate's interval, a stall limit), off the hot
    /// path.
    #[must_use]
    pub const fn ns_to_ticks(&self, ns: u64) -> u64 {
        ns_to_ticks(ns, self.conversion.mult)
    }

    /// The counter's rate, Hz, rounded.
    #[must_use]
    pub const fn hz(&self) -> u64 {
        self.hz
    }

    /// Nanoseconds per tick in 32.32 fixed point.
    #[must_use]
    pub const fn mult(&self) -> u64 {
        self.conversion.mult
    }

    /// The counter value at the base.
    #[must_use]
    pub const fn base_ticks(&self) -> u64 {
        self.conversion.base_ticks
    }

    /// The time, ns, at the base.
    #[must_use]
    pub const fn base_ns(&self) -> u64 {
        self.conversion.base_ns
    }

    /// One tick, ns, rounded up: the counter's resolution.
    #[must_use]
    pub const fn tick_ns(&self) -> u64 {
        let ns = self.ticks_to_ns(1);
        if ns == 0 {
            1
        } else {
            ns + (self.conversion.mult & 0xffff_ffff != 0) as u64
        }
    }
}

/// The rate, Hz (rounded), that `mult` (ns per tick, 32.32) stands for:
/// `None` unless `mult` is in (0, 2^63) and the rate 1 kHz to 100 GHz.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // range-checked first
pub const fn hz_of(mult: u64) -> Option<u64> {
    if mult == 0 || mult >= 1 << 63 {
        return None;
    }
    let wide = ((NS_PER_S as u128) << 32) + (mult as u128) / 2;
    let hz = wide / mult as u128;
    if hz < MIN_HZ as u128 || hz > MAX_HZ as u128 {
        return None;
    }
    Some(hz as u64)
}

/// The duration, ns, of `ticks` ticks at `mult` (saturating).
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)] // range-checked first
pub const fn ticks_to_ns(ticks: u64, mult: u64) -> u64 {
    let ns = (ticks as u128 * mult as u128) >> 32;
    if ns > u64::MAX as u128 {
        u64::MAX
    } else {
        ns as u64
    }
}

/// The ticks, rounded to the nearest, in `ns` nanoseconds at `mult`
/// (non-zero; saturating).
#[must_use]
#[allow(clippy::cast_possible_truncation)] // range-checked first
pub const fn ns_to_ticks(ns: u64, mult: u64) -> u64 {
    let ticks = (((ns as u128) << 32) + (mult as u128) / 2) / mult as u128;
    if ticks > u64::MAX as u128 {
        u64::MAX
    } else {
        ticks as u64
    }
}
