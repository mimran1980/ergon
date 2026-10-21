//! A calibration in C layout.

/// The layout version of [`Params`].
pub const PARAMS_VERSION: u32 = 1;
/// [`Params::kind`]: the monotonic fallback (ticks are its nanoseconds).
pub const KIND_MONOTONIC: u32 = 0;
/// [`Params::kind`]: the hardware counter.
pub const KIND_COUNTER: u32 = 1;

/// A calibration's parameters in C layout (48 bytes, 8-aligned).
///
/// For a process or the DPDK transport's C clock to adopt one calibration
/// instead of running its own. Both kinds convert with the counter's formula:
///
/// ```text
/// ns = base_ns + (int64_t)(((__int128)(int64_t)(ticks - base_ticks) * (int64_t)mult) >> 32)
/// ```
///
/// with `ticks` the counter (`kind` 1) or the platform's monotonic clock in
/// nanoseconds (`kind` 0, whose `mult` is 2^32 and `base_ticks` 0).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// [`PARAMS_VERSION`].
    pub version: u32,
    /// [`KIND_COUNTER`] or [`KIND_MONOTONIC`].
    pub kind: u32,
    /// 0: the monotonic domain; 1: the realtime (epoch) domain.
    pub domain: u32,
    /// Zero.
    pub reserved: u32,
    /// The counter value at the base.
    pub base_ticks: u64,
    /// The time, ns, at the base.
    pub base_ns: u64,
    /// Nanoseconds per tick, 32.32 fixed point, under 2^63.
    pub mult: u64,
    /// Ticks a second (the rate `mult` stands for).
    pub hz: u64,
}
