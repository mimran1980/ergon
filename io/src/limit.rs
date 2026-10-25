//! Send budget: venue rate limits checked before a request goes out.
//!
//! - [`Budget`] is a plain struct the application owns: one per account and
//!   IP, shared by every connection that sends for them (REST and WebSocket).
//!   Several source IPs: one budget with each IP's buckets, picked per
//!   connect by [`SourceIps`].
//! - [`Budget::try_charge`] charges up to [`MAX_CHARGES`] buckets at once,
//!   all or none, and answers "go" ([`Ticket`]) or "not before T"
//!   ([`Limited`]). It never sleeps and never queues: the caller waits, drops
//!   the request or alerts.
//! - No I/O, no clock reads, no locks or atomics, and no allocation after
//!   [`Budget::new`] ([`Budget::add`] allocates; it is for start-up and for
//!   instruments listed later, not the send path).
//! - Every call takes `now_ns` from the caller. Use one clock that never
//!   jumps for every call: `CLOCK_MONOTONIC`, quanta, or replay time.
//!   A clock that goes back is clamped: the budget keeps the latest time seen.
//!
//! # One sender per account and IP
//!
//! The budget sees only this process's sends, plus what the server reports
//! back ([`Budget::observe_used`]). The venue counts every sender on the
//! account or IP: other processes, other hosts behind the same NAT, scripts
//! on the same key, a market-data process fetching depth snapshots. Run one
//! sending process per account and IP. Put bulk REST on another IP, or in
//! the same process and budget. Budgets are not shared between processes.
//!
//! # Models
//!
//! - [`Model::Rolling`] for every window limit (request counts, weighted requests,
//!   WebSocket messages, subscribes and connect attempts). It never lets more than `limit` units through in any span of
//!   `window + guard`, whatever window alignment the server uses.
//! - [`Model::Decay`] (GCRA) only for counters that drain continuously. A GCRA with burst L and rate L per W lets
//!   2L through in one window W, so it is wrong for window limits.
//! - [`Model::Gate`]: no count, only holds (an IP scope with no known limit).
//!
//! # Throttles, bans and the halt
//!
//! - [`Budget::throttled`] (a 429 or a venue code) holds the bucket for the
//!   server's wait plus the guard, or for a doubling backoff, and marks it
//!   full: the local count was wrong, so it is not trusted for one window.
//!   Hold the narrowest bucket the error names.
//! - A hold blocks every [`Class`], including reserved work. Application
//!   policy decides which bucket to hold and how to recover.
//! - [`Budget::banned`] does the same and **halts** the bucket. A halted bucket stays
//!   stopped after the server's wait ends, until a person clears it with
//!   [`Budget::clear_halt`]. [`BucketStats::halted`] and [`Budget::halted`]
//!   show it, so the application can alert.
//!
//! # Re-sending
//!
//! Re-send only a request the server did not run (throttled, clock skew, not
//! run), and only while it is inside its own deadline: [`Budget::try_resend`].
//! Never re-send an order whose outcome is unknown; reconcile it by client
//! order id.
//!
//! # Example
//!
//! ```
//! use ergon_io::limit::{BucketSpec, Budget, Charge, Class};
//!
//! # fn main() -> ergon_io::Result<()> {
//! let mut budget = Budget::new(vec![
//!     // Illustrative service limits, configured by the application.
//!     BucketSpec::rolling("ip-weight", 6_000, 60_000, 60).with_reserve(100),
//!     BucketSpec::rolling("orders-10s", 100, 10_000, 10),
//! ])?;
//! let missing = || ergon_io::Error::Config("no such bucket".into());
//! let weight = budget.id("ip-weight").ok_or_else(missing)?;
//! let orders = budget.id("orders-10s").ok_or_else(missing)?;
//!
//! let now_ns = 1_000_000_000; // the caller's clock, read once per loop pass
//! let new_order = [Charge::new(weight, 1), Charge::new(orders, 1)];
//! match budget.try_charge(&new_order, Class::Normal, now_ns) {
//!     // Send it. If it was never handed to the kernel: budget.refund(&ticket, now_ns)?
//!     Ok(ticket) => assert_eq!(ticket.parts().len(), 2),
//!     // Wait until `limited.ready_at_ns`, drop the request, or alert.
//!     Err(limited) => println!("{limited}"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Several source IPs
//!
//! A venue that limits connects and requests per IP: [`SourceIps`] holds
//! each local address with its own buckets in one [`Budget`], picks an IP
//! with room for each connect ([`SourceIps::try_connect`]), and refuses
//! with the soonest wait when none has one. Each connection binds its pick
//! ([`crate::Target::source`]); its requests charge that IP's bucket.

use std::fmt;
use std::net::IpAddr;
use std::num::NonZeroU64;

use crate::error::{Error, Result};
use crate::outcome::Verdict;

/// The most buckets one charge may name.
///
/// A request may charge an IP count, a weighted count, an account count
/// and a per-resource count; a batch needs one per resource plus the
/// sub-account. Split a batch that needs more.
pub const MAX_CHARGES: usize = 8;

const NS_PER_MS: i64 = 1_000_000;
/// The longest ring of one rolling bucket, in sub-windows.
const MAX_RING: u64 = 65_536;
/// The most buckets in one budget (ids are `u16`).
const MAX_BUCKETS: usize = 65_535;

/// A bucket in one [`Budget`]: from [`Budget::id`] or [`Budget::add`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BucketId(u16);

impl BucketId {
    /// The bucket's index, in the order buckets were added.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A drain rate: `units` per `per_ms` milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// Units drained per period (at least 1).
    pub units: u32,
    /// The period, ms (at least 1).
    pub per_ms: u32,
}

/// How a bucket counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// At most `limit` units in any span of `window_ms` + `guard_ms`.
    ///
    /// Kept as sub-windows of `window_ms / slots`, in a ring of
    /// `slots + 1 + ceil(guard / sub-window)`. The check sums the whole ring,
    /// which covers every sub-window that touches that span. So it never
    /// counts less than the span, and at most one sub-window more (plus the
    /// guard rounded up to whole sub-windows). A ring of exactly `slots`
    /// sub-windows would count less, and let more than `limit` through.
    Rolling {
        /// Units per window (at least 1).
        limit: u32,
        /// The window, ms (at least 1; a multiple of `slots`).
        window_ms: u32,
        /// Sub-windows per window (at least 1).
        slots: u16,
    },
    /// A level that drains at `drain` and may not pass `max` (GCRA). Only for
    /// counters that drain continuously.
    Decay {
        /// The highest level, units (at least 1).
        max: u32,
        /// How fast the level drains.
        drain: Rate,
    },
    /// No count, only holds: a scope with no known limit.
    Gate,
}

/// One bucket's configuration, for [`Budget::new`] or [`Budget::add`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketSpec {
    /// Unique within the budget: for [`Budget::id`], stats and logs.
    pub name: String,
    /// How it counts.
    pub model: Model,
    /// Units only [`Class::Reduce`] may use. Below the limit; 0 for a gate.
    pub reserve: u32,
    /// The worst spread of send-to-server delay (jitter), ms.
    ///
    /// Widens a rolling window, lowers a decaying bucket's usable level by
    /// `drain × guard`, and pads every hold. Defaults: one sub-window for
    /// [`Model::Rolling`]; 0 for [`Model::Decay`] and [`Model::Gate`] (set it
    /// to the measured jitter).
    pub guard_ms: u32,
    /// The hold after a throttle with no server wait, ms: the first and the
    /// longest. It doubles per throttle in a row. Default `[1_000, 64_000]`.
    pub backoff_ms: [u32; 2],
}

impl BucketSpec {
    const BACKOFF_MS: [u32; 2] = [1_000, 64_000];

    /// A [`Model::Rolling`] bucket; guard one sub-window, no reserve.
    #[must_use]
    pub fn rolling(name: impl Into<String>, limit: u32, window_ms: u32, slots: u16) -> Self {
        Self {
            name: name.into(),
            model: Model::Rolling {
                limit,
                window_ms,
                slots,
            },
            reserve: 0,
            guard_ms: window_ms.checked_div(u32::from(slots)).unwrap_or(0),
            backoff_ms: Self::BACKOFF_MS,
        }
    }

    /// A [`Model::Decay`] bucket; no guard, no reserve.
    #[must_use]
    pub fn decay(name: impl Into<String>, max: u32, drain: Rate) -> Self {
        Self {
            name: name.into(),
            model: Model::Decay { max, drain },
            reserve: 0,
            guard_ms: 0,
            backoff_ms: Self::BACKOFF_MS,
        }
    }

    /// A [`Model::Gate`]: holds only.
    #[must_use]
    pub fn gate(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            model: Model::Gate,
            reserve: 0,
            guard_ms: 0,
            backoff_ms: Self::BACKOFF_MS,
        }
    }

    /// Sets [`BucketSpec::reserve`].
    #[must_use]
    pub const fn with_reserve(mut self, units: u32) -> Self {
        self.reserve = units;
        self
    }

    /// Sets [`BucketSpec::guard_ms`].
    #[must_use]
    pub const fn with_guard_ms(mut self, ms: u32) -> Self {
        self.guard_ms = ms;
        self
    }

    /// Sets [`BucketSpec::backoff_ms`].
    #[must_use]
    pub const fn with_backoff_ms(mut self, first: u32, max: u32) -> Self {
        self.backoff_ms = [first, max];
        self
    }
}

/// Who may use a bucket's reserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Everything else: may fill a bucket up to `limit - reserve`.
    Normal,
    /// Cancels and reduce-only orders: may fill a bucket up to `limit`.
    Reduce,
}

impl Class {
    const fn idx(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Reduce => 1,
        }
    }
}

/// One part of a charge: `cost` units from `bucket`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Charge {
    /// The bucket.
    pub bucket: BucketId,
    /// Units (an endpoint's weight, a batch's size).
    pub cost: u32,
}

impl Charge {
    /// `cost` units from `bucket`.
    #[must_use]
    pub const fn new(bucket: BucketId, cost: u32) -> Self {
        Self { bucket, cost }
    }
}

/// What a charge took. Give it back with [`Budget::refund`] when the request
/// was never sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket {
    parts: [Charge; MAX_CHARGES],
    len: u8,
    at_ns: i64,
}

impl Ticket {
    const EMPTY: Self = Self {
        parts: [Charge::new(BucketId(0), 0); MAX_CHARGES],
        len: 0,
        at_ns: 0,
    };

    /// The parts charged.
    #[must_use]
    pub fn parts(&self) -> &[Charge] {
        self.parts.get(..usize::from(self.len)).unwrap_or(&[])
    }

    /// When it was charged (the budget's clock, after clamping).
    #[must_use]
    pub const fn at_ns(&self) -> i64 {
        self.at_ns
    }
}

impl Default for Ticket {
    /// A ticket that took nothing.
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Why a charge was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The count is at the limit (for this class).
    Full,
    /// The bucket is held after a throttle or a ban.
    Held,
    /// The bucket is halted after a ban, until [`Budget::clear_halt`].
    Banned,
    /// It will never fit: the cost is above the limit for this class, more
    /// than [`MAX_CHARGES`] parts, or a bucket this budget does not have.
    /// Fix the caller.
    Never,
}

impl fmt::Display for Why {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Full => "full",
            Self::Held => "held",
            Self::Banned => "banned: halted until cleared",
            Self::Never => "never fits: fix the caller",
        })
    }
}

/// A refused charge: not before `ready_at_ns`.
///
/// `ready_at_ns` assumes no other charge meanwhile; for [`Why::Banned`] and
/// [`Why::Never`] it is `i64::MAX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limited {
    /// The part that frees up last (a `Never` or `Banned` part first).
    pub bucket: BucketId,
    /// The earliest time it can pass, on the caller's clock.
    pub ready_at_ns: i64,
    /// Why it was refused.
    pub why: Why,
}

impl Limited {
    /// Nanoseconds from `now_ns` until `ready_at_ns`; 0 if past.
    #[must_use]
    pub const fn wait_ns(&self, now_ns: i64) -> u64 {
        let d = self.ready_at_ns.saturating_sub(now_ns);
        if d > 0 { d.unsigned_abs() } else { 0 }
    }
}

impl fmt::Display for Limited {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rate limit: bucket {}: {}",
            self.bucket.index(),
            self.why
        )?;
        if self.ready_at_ns < i64::MAX {
            write!(f, " until {} ns", self.ready_at_ns)?;
        }
        Ok(())
    }
}

impl std::error::Error for Limited {}

/// What to do with a request the server did not run: [`Budget::resend`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resend {
    /// Inside its deadline, and the budget took it: send it now.
    Send(Ticket),
    /// Inside its deadline, refused for now, free by the deadline: ask again
    /// at `ready_at_ns`.
    Wait(Limited),
    /// Drop it as stale. `None`: its deadline has passed, or the request must
    /// not be sent again at all (`Rejected`, `Banned`, `Done`). `Some`: the
    /// refusal lasts past the deadline (a halt and `Never` always do).
    Stale(Option<Limited>),
    /// The outcome is unknown (a timeout, a 5xx, a lost connection). Reconcile
    /// by client order id; never send an order again blindly.
    Reconcile,
}

/// One bucket's counters, from [`Budget::stats`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BucketStats {
    /// Units counted now: in the ring (rolling) or the level rounded up
    /// (decay); 0 for a gate.
    pub used: u64,
    /// The current limit (`max` for decay); 0 for a gate.
    pub limit: u32,
    /// The highest `used` seen.
    pub peak: u64,
    /// Units charged by [`Budget::try_charge`].
    pub charged: u64,
    /// Units charged by [`Budget::force`].
    pub forced: u64,
    /// Refusals this bucket caused.
    pub limited: u64,
    /// Units given back by [`Budget::refund`].
    pub refunded: u64,
    /// Units the server counted that this budget never charged (other
    /// senders, window edges). A lower bound: a rolling count that still
    /// holds the last window hides some.
    pub foreign: u64,
    /// The last server figure ([`Budget::observe_used`]); `None` before any.
    /// Far below `used` for long: the local model over-counts.
    pub server_used: Option<u32>,
    /// [`Budget::throttled`] calls.
    pub throttles: u64,
    /// [`Budget::banned`] calls.
    pub bans: u64,
    /// Total time held, ns.
    pub held_ns: u64,
    /// When the current hold ends; `Some(i64::MAX)` while halted.
    pub held_until_ns: Option<i64>,
    /// Banned and not yet cleared: alert a person.
    pub halted: bool,
}

/// A rolling count: a ring of sub-window counters in `Budget::slots`.
#[derive(Debug, Clone)]
struct Ring {
    slot_ns: i64,
    /// End of the current sub-window: at or past it, `advance` first.
    slot_end: i64,
    /// `now / slot_ns` of the current sub-window; `i64::MIN` before first use.
    head_abs: i64,
    /// The ring's counters are `slots[off..off + len]`.
    off: usize,
    len: usize,
    /// The current sub-window's counter, `0..len`.
    head: usize,
    sum: u64,
    limit: u32,
    /// The highest `sum` a charge may reach, per [`Class`].
    cap: [u64; 2],
    peak: u64,
}

impl Ring {
    /// A ring for `[limit, window_ms, reserve, guard_ms]` in `slots`
    /// sub-windows, at offset 0.
    fn new(
        name: &str,
        [limit, window_ms, reserve, guard_ms]: [u32; 4],
        slots: u16,
    ) -> Result<Self> {
        if limit == 0 || window_ms == 0 || slots == 0 {
            return Err(config(
                name,
                "limit, window_ms and slots must be at least 1",
            ));
        }
        if window_ms % u32::from(slots) != 0 {
            return Err(config(name, "window_ms must be a multiple of slots"));
        }
        if reserve >= limit {
            return Err(config(name, "reserve must be below the limit"));
        }
        let slot_ms = window_ms / u32::from(slots);
        let len = u64::from(slots) + 1 + u64::from(guard_ms).div_ceil(u64::from(slot_ms));
        let len = usize::try_from(len)
            .ok()
            .filter(|&l| l as u64 <= MAX_RING)
            .ok_or_else(|| config(name, "guard_ms makes the ring too long"))?;
        Ok(Self {
            slot_ns: i64::from(slot_ms) * NS_PER_MS,
            slot_end: i64::MIN,
            head_abs: i64::MIN,
            off: 0,
            len,
            head: 0,
            sum: 0,
            limit,
            cap: [u64::from(limit - reserve), u64::from(limit)],
            peak: 0,
        })
    }

    /// Moves the current sub-window to `now`'s; expired counters leave.
    #[cold]
    fn advance(&mut self, now: i64, slots: &mut [u32]) {
        let abs = now.div_euclid(self.slot_ns);
        let steps = abs.saturating_sub(self.head_abs);
        if usize::try_from(steps).map_or(true, |s| s >= self.len) {
            slots.fill(0);
            self.sum = 0;
        } else {
            for _ in 0..steps {
                self.head = if self.head + 1 == self.len {
                    0
                } else {
                    self.head + 1
                };
                if let Some(c) = slots.get_mut(self.head) {
                    self.sum = self.sum.saturating_sub(u64::from(std::mem::take(c)));
                }
            }
        }
        self.head_abs = abs;
        self.slot_end = abs.saturating_add(1).saturating_mul(self.slot_ns);
    }

    /// The counter `k` places after the current one (wrapping).
    const fn at(&self, k: usize) -> usize {
        (self.head + k) % self.len
    }

    /// `sum` at `now`, without moving the ring.
    fn used_at(&self, now: i64, slots: &[u32]) -> u64 {
        if now < self.slot_end {
            return self.sum;
        }
        let steps = now.div_euclid(self.slot_ns).saturating_sub(self.head_abs);
        match usize::try_from(steps) {
            Ok(s) if s < self.len => {
                let gone: u64 = (1..=s)
                    .filter_map(|k| slots.get(self.at(k)))
                    .map(|&c| u64::from(c))
                    .sum();
                self.sum.saturating_sub(gone)
            }
            _ => 0,
        }
    }

    /// When `need` units will have left (the ring is current).
    fn ready_at(&self, need: u64, slots: &[u32]) -> i64 {
        let mut freed = 0u64;
        for k in 0..self.len {
            freed += slots.get(self.at(k + 1)).map_or(0, |&c| u64::from(c));
            if freed >= need {
                let k = i64::try_from(k).unwrap_or(i64::MAX);
                return self.slot_end.saturating_add(self.slot_ns.saturating_mul(k));
            }
        }
        i64::MAX
    }
}

/// A draining level, in units × `per_ns` so the math stays exact.
#[derive(Debug, Clone)]
struct Gcra {
    level: u128,
    at: i64,
    units: u128,
    per_ns: u128,
    max: u32,
    /// `units × guard_ns`: the level the guard takes off.
    guard_cut: u128,
    cap: [u128; 2],
    peak: u128,
}

impl Gcra {
    fn new(name: &str, max: u32, drain: Rate, reserve: u32, guard_ns: i64) -> Result<Self> {
        if max == 0 || drain.units == 0 || drain.per_ms == 0 {
            return Err(config(
                name,
                "max, drain units and per_ms must be at least 1",
            ));
        }
        if reserve >= max {
            return Err(config(name, "reserve must be below max"));
        }
        let units = u128::from(drain.units);
        let per_ns = u128::from(drain.per_ms) * 1_000_000;
        let guard_cut = units * u128::from(guard_ns.unsigned_abs());
        let cap = Self::caps(max, reserve, per_ns, guard_cut)
            .ok_or_else(|| config(name, "guard_ms leaves less than one unit"))?;
        Ok(Self {
            level: 0,
            at: i64::MIN,
            units,
            per_ns,
            max,
            guard_cut,
            cap,
            peak: 0,
        })
    }

    fn drain(&mut self, now: i64) {
        let dt = u128::from(now.saturating_sub(self.at).unsigned_abs());
        self.level = self.level.saturating_sub(self.units.saturating_mul(dt));
        self.at = now;
    }

    /// `level` at `now` in units, rounded up, without draining.
    fn used_at(&self, now: i64) -> u64 {
        let dt = u128::from(now.saturating_sub(self.at).unsigned_abs());
        let level = self.level.saturating_sub(self.units.saturating_mul(dt));
        u64::try_from(level.div_ceil(self.per_ns)).unwrap_or(u64::MAX)
    }

    /// The caps for `max` and `reserve`; `None` if the guard leaves less than one unit.
    fn caps(max: u32, reserve: u32, per_ns: u128, guard_cut: u128) -> Option<[u128; 2]> {
        let normal = u128::from(max.checked_sub(reserve)?) * per_ns;
        let normal = normal.checked_sub(guard_cut).filter(|&c| c >= per_ns)?;
        Some([normal, u128::from(max) * per_ns - guard_cut])
    }
}

#[derive(Debug, Clone)]
enum Count {
    Rolling(Ring),
    Decay(Gcra),
    Gate,
}

#[derive(Debug, Clone, Copy, Default)]
struct Counters {
    charged: u64,
    forced: u64,
    limited: u64,
    refunded: u64,
    foreign: u64,
    server_used: Option<u32>,
    throttles: u64,
    bans: u64,
    held_ns: u64,
}

#[derive(Debug, Clone)]
struct Bucket {
    /// No charge before this: the hold end, or `i64::MAX` while halted.
    block_until: i64,
    count: Count,
    hold_end: i64,
    halted: bool,
    strikes: u32,
    guard_ns: i64,
    backoff_ns: [i64; 2],
    reserve: u32,
    name: String,
    st: Counters,
}

impl Bucket {
    /// Holds until at least `end`.
    fn hold(&mut self, end: i64, now: i64) {
        let from = self.hold_end.max(now);
        if end > from {
            self.st.held_ns = self
                .st
                .held_ns
                .saturating_add(end.saturating_sub(from).unsigned_abs());
            self.hold_end = end;
        }
        self.block_until = if self.halted { i64::MAX } else { self.hold_end };
    }

    fn held_until(&self, now: i64) -> Option<i64> {
        if self.halted {
            Some(i64::MAX)
        } else {
            (now < self.hold_end).then_some(self.hold_end)
        }
    }
}

/// Rate-limit buckets for one account and IP. See the [module docs](self).
#[derive(Debug, Clone)]
pub struct Budget {
    buckets: Vec<Bucket>,
    /// Every rolling bucket's sub-window counters.
    slots: Vec<u32>,
    /// The latest `now_ns` seen: the budget's time never goes back.
    last_ns: i64,
}

fn config(name: &str, what: &str) -> Error {
    Error::Config(format!("limit: bucket {name:?}: {what}"))
}

fn ms_to_ns(ms: u64) -> i64 {
    i64::try_from(ms).map_or(i64::MAX, |ms| ms.saturating_mul(NS_PER_MS))
}

/// `first` doubled per strike after the first, up to `max`.
fn backoff(ns: [i64; 2], strikes: u32) -> i64 {
    let shift = strikes.saturating_sub(1).min(62);
    ns[0].saturating_mul(1_i64 << shift).min(ns[1])
}

/// Of two refusals, the one the caller must wait for.
const fn worse(a: Limited, b: Limited) -> Limited {
    const fn rank(w: Why) -> u8 {
        match w {
            Why::Never => 3,
            Why::Banned => 2,
            Why::Held | Why::Full => 1,
        }
    }
    let (ra, rb) = (rank(a.why), rank(b.why));
    if rb > ra || (rb == ra && b.ready_at_ns > a.ready_at_ns) {
        b
    } else {
        a
    }
}

impl Budget {
    /// A budget with these buckets, ids in this order.
    ///
    /// # Errors
    /// [`Error::Config`]: no buckets or more than 65,535; a duplicate name; a
    /// zero limit, window, slot count, drain or period; a window that is not a
    /// multiple of its slots; a reserve at or above the limit (or any reserve
    /// on a gate); a guard that leaves a decaying bucket less than one unit or
    /// makes a ring longer than 65,536 sub-windows; a zero first backoff, or
    /// one above the longest.
    pub fn new(specs: Vec<BucketSpec>) -> Result<Self> {
        if specs.is_empty() {
            return Err(Error::Config(
                "limit: a budget needs at least one bucket".into(),
            ));
        }
        let mut b = Self {
            buckets: Vec::with_capacity(specs.len()),
            slots: Vec::new(),
            last_ns: i64::MIN,
        };
        for spec in specs {
            b.add(spec)?;
        }
        Ok(b)
    }

    /// The bucket called `name`. A linear scan: look ids up at start-up.
    #[must_use]
    pub fn id(&self, name: &str) -> Option<BucketId> {
        let i = self.buckets.iter().position(|b| b.name == name)?;
        u16::try_from(i).ok().map(BucketId)
    }

    /// A bucket's name; `None` if this budget does not have it.
    #[must_use]
    pub fn name(&self, bucket: BucketId) -> Option<&str> {
        self.buckets.get(bucket.index()).map(|b| b.name.as_str())
    }

    /// Adds a bucket, e.g. for a resource discovered after start.
    ///
    /// Allocates: not for the send path.
    ///
    /// # Errors
    /// As [`Budget::new`].
    pub fn add(&mut self, spec: BucketSpec) -> Result<BucketId> {
        let BucketSpec {
            name,
            model,
            reserve,
            guard_ms,
            backoff_ms,
        } = spec;
        if self.buckets.len() >= MAX_BUCKETS {
            return Err(config(&name, "more than 65,535 buckets"));
        }
        if self.buckets.iter().any(|b| b.name == name) {
            return Err(config(&name, "the name is already taken"));
        }
        if backoff_ms[0] == 0 || backoff_ms[0] > backoff_ms[1] {
            return Err(config(
                &name,
                "backoff_ms: the first must be 1 to the longest",
            ));
        }
        let guard_ns = i64::from(guard_ms) * NS_PER_MS;
        let (count, ring_len) = match model {
            Model::Rolling {
                limit,
                window_ms,
                slots,
            } => {
                let r = Ring::new(&name, [limit, window_ms, reserve, guard_ms], slots)?;
                let len = r.len;
                (
                    Count::Rolling(Ring {
                        off: self.slots.len(),
                        ..r
                    }),
                    len,
                )
            }
            Model::Decay { max, drain } => (
                Count::Decay(Gcra::new(&name, max, drain, reserve, guard_ns)?),
                0,
            ),
            Model::Gate if reserve != 0 => {
                return Err(config(&name, "a gate has no count, so no reserve"));
            }
            Model::Gate => (Count::Gate, 0),
        };
        let id =
            BucketId(u16::try_from(self.buckets.len()).map_err(|_| config(&name, "too many"))?);
        self.slots.resize(self.slots.len() + ring_len, 0);
        self.buckets.push(Bucket {
            block_until: i64::MIN,
            count,
            hold_end: i64::MIN,
            halted: false,
            strikes: 0,
            guard_ns,
            backoff_ns: [
                i64::from(backoff_ms[0]) * NS_PER_MS,
                i64::from(backoff_ms[1]) * NS_PER_MS,
            ],
            reserve,
            name,
            st: Counters::default(),
        });
        Ok(id)
    }

    /// The budget's time: never back, and below `i64::MAX` (a halt's
    /// `block_until`).
    const fn clamp(&mut self, now_ns: i64) -> i64 {
        if now_ns > self.last_ns {
            self.last_ns = if now_ns < i64::MAX {
                now_ns
            } else {
                i64::MAX - 1
            };
        }
        self.last_ns
    }

    fn bucket(&self, id: BucketId) -> Result<&Bucket> {
        self.buckets.get(id.index()).ok_or_else(|| self.unknown(id))
    }

    fn unknown(&self, id: BucketId) -> Error {
        Error::Config(format!(
            "limit: bucket id {} is not in this budget ({} buckets)",
            id.index(),
            self.buckets.len()
        ))
    }

    /// Checks every part, then charges every part, or none.
    ///
    /// O(parts), plus one ring catch-up when a rolling bucket's slot has
    /// expired. That catch-up is O(ring) on the first charge after a long
    /// idle (a day bucket, 23 h idle): call [`Budget::tick`] from idle time
    /// so it never lands on the send path.
    ///
    /// A hold blocks every class. An empty `parts` charges nothing and passes.
    ///
    /// # Errors
    /// [`Limited`]: the part the caller must wait for longest, and until when.
    pub fn try_charge(
        &mut self,
        parts: &[Charge],
        class: Class,
        now_ns: i64,
    ) -> std::result::Result<Ticket, Limited> {
        let now = self.clamp(now_ns);
        if let Some(extra) = parts.get(MAX_CHARGES) {
            return Err(self.never(extra.bucket));
        }
        let mut ticket = Ticket {
            at_ns: now,
            ..Ticket::EMPTY
        };
        for (i, (&p, slot)) in parts.iter().zip(ticket.parts.iter_mut()).enumerate() {
            if !self.take(p, class, now) {
                return Err(self.refuse(parts, i, class, now));
            }
            *slot = p;
        }
        for p in parts {
            if let Some(b) = self.buckets.get_mut(p.bucket.index()) {
                b.st.charged += u64::from(p.cost);
                match &mut b.count {
                    Count::Rolling(r) => r.peak = r.peak.max(r.sum),
                    Count::Decay(l) => l.peak = l.peak.max(l.level),
                    Count::Gate => {}
                }
            }
        }
        // At most MAX_CHARGES parts: fits a u8.
        ticket.len = u8::try_from(parts.len()).unwrap_or(0);
        Ok(ticket)
    }

    /// Charges one part if it fits now; the send path.
    #[inline]
    fn take(&mut self, p: Charge, class: Class, now: i64) -> bool {
        let Some(b) = self.buckets.get_mut(p.bucket.index()) else {
            return false;
        };
        if now < b.block_until {
            return false;
        }
        match &mut b.count {
            Count::Rolling(r) => {
                if now >= r.slot_end {
                    let Some(ring) = self.slots.get_mut(r.off..r.off + r.len) else {
                        return false;
                    };
                    r.advance(now, ring);
                }
                let next = r.sum + u64::from(p.cost);
                if next > r.cap[class.idx()] {
                    return false;
                }
                let Some(c) = self.slots.get_mut(r.off + r.head) else {
                    return false;
                };
                // No overflow: c <= sum and next <= cap <= u32::MAX.
                *c += p.cost;
                r.sum = next;
                true
            }
            Count::Decay(l) => {
                l.drain(now);
                let next = l.level.saturating_add(l.per_ns * u128::from(p.cost));
                if next > l.cap[class.idx()] {
                    return false;
                }
                l.level = next;
                true
            }
            Count::Gate => true,
        }
    }

    /// Undoes `take` for a part that passed, at the same `now`.
    fn untake(&mut self, p: Charge) {
        let Some(b) = self.buckets.get_mut(p.bucket.index()) else {
            return;
        };
        match &mut b.count {
            Count::Rolling(r) => {
                if let Some(c) = self.slots.get_mut(r.off + r.head) {
                    *c = c.saturating_sub(p.cost);
                }
                r.sum = r.sum.saturating_sub(u64::from(p.cost));
            }
            Count::Decay(l) => l.level = l.level.saturating_sub(l.per_ns * u128::from(p.cost)),
            Count::Gate => {}
        }
    }

    #[cold]
    fn never(&mut self, bucket: BucketId) -> Limited {
        if let Some(b) = self.buckets.get_mut(bucket.index()) {
            b.st.limited += 1;
        }
        Limited {
            bucket,
            ready_at_ns: i64::MAX,
            why: Why::Never,
        }
    }

    /// Part `failed` did not fit: undo the parts that did, then say why the
    /// whole request is refused and until when.
    ///
    /// The caller retries every part together, so a bucket is judged on the
    /// sum of the costs of every part naming it, against the state the retry
    /// starts from (nothing of this request charged). Judged part by part,
    /// `[6, 6]` on a bucket of capacity 10 returned a finite wait although
    /// the request can never fit.
    #[cold]
    fn refuse(&mut self, parts: &[Charge], failed: usize, class: Class, now: i64) -> Limited {
        for &p in parts.iter().take(failed).rev() {
            self.untake(p);
        }
        // At most MAX_CHARGES parts, so at most MAX_CHARGES distinct buckets.
        let mut costs: [u64; MAX_CHARGES] = [0; MAX_CHARGES];
        let mut buckets: [BucketId; MAX_CHARGES] = [BucketId(0); MAX_CHARGES];
        let mut len = 0;
        for &p in parts {
            match buckets.iter().take(len).position(|&b| b == p.bucket) {
                Some(i) => costs[i] = costs[i].saturating_add(u64::from(p.cost)),
                None if len < MAX_CHARGES => {
                    buckets[len] = p.bucket;
                    costs[len] = u64::from(p.cost);
                    len += 1;
                }
                None => {}
            }
        }
        let mut worst: Option<Limited> = None;
        for (bucket, cost) in buckets.iter().zip(&costs).take(len) {
            let l = self.refusal(*bucket, *cost, class, now).unwrap_or(Limited {
                bucket: *bucket,
                ready_at_ns: now,
                why: Why::Full,
            });
            worst = Some(worst.map_or(l, |w| worse(w, l)));
        }
        worst.unwrap_or_else(|| self.never(BucketId(0)))
    }

    /// Why `cost` units on `bucket` would be refused now, if they would.
    fn refusal(&mut self, bucket: BucketId, cost: u64, class: Class, now: i64) -> Option<Limited> {
        let Some(b) = self.buckets.get_mut(bucket.index()) else {
            return Some(self.never(bucket));
        };
        let count_ready = match &mut b.count {
            Count::Rolling(r) => {
                let ring = self.slots.get_mut(r.off..r.off + r.len)?;
                if now >= r.slot_end {
                    r.advance(now, ring);
                }
                let cap = r.cap[class.idx()];
                if cost > cap {
                    None
                } else {
                    let need = (r.sum + cost).saturating_sub(cap);
                    Some(if need == 0 {
                        now
                    } else {
                        r.ready_at(need, ring)
                    })
                }
            }
            Count::Decay(l) => {
                l.drain(now);
                let cap = l.cap[class.idx()];
                let add = l.per_ns * u128::from(cost);
                if add > cap {
                    None
                } else {
                    let need = (l.level + add).saturating_sub(cap);
                    let wait = i64::try_from(need.div_ceil(l.units)).unwrap_or(i64::MAX);
                    Some(now.saturating_add(wait))
                }
            }
            Count::Gate => Some(now),
        };
        let l = match count_ready {
            None => Limited {
                bucket,
                ready_at_ns: i64::MAX,
                why: Why::Never,
            },
            Some(_) if b.halted => Limited {
                bucket,
                ready_at_ns: i64::MAX,
                why: Why::Banned,
            },
            Some(ready) if now < b.hold_end => Limited {
                bucket,
                ready_at_ns: ready.max(b.hold_end),
                why: Why::Held,
            },
            Some(ready) if ready > now => Limited {
                bucket,
                ready_at_ns: ready,
                why: Why::Full,
            },
            Some(_) => return None,
        };
        b.st.limited += 1;
        Some(l)
    }

    /// The re-send decision for one verdict: [`Budget::resend`], which takes
    /// the deadline and the parts too. Use it instead of calling
    /// [`Budget::try_resend`] on an unknown outcome.
    ///
    /// - `NotRun`, `Throttled`, `ClockSkew`: [`Budget::try_resend`].
    /// - `Unknown`: [`Resend::Reconcile`]. Never re-send; reconcile by client
    ///   order id.
    /// - `Done`, `Rejected`, `Banned`: [`Resend::Stale`], nothing charged.
    pub fn resend(
        &mut self,
        verdict: Verdict,
        parts: &[Charge],
        class: Class,
        deadline_ns: i64,
        now_ns: i64,
    ) -> Resend {
        if verdict.may_resend() {
            self.try_resend(parts, class, deadline_ns, now_ns)
        } else if verdict == Verdict::Unknown {
            Resend::Reconcile
        } else {
            Resend::Stale(None)
        }
    }

    /// Re-sends a request the server did not run (throttled, clock skew, not
    /// run), only while `now_ns` is at or before its own `deadline_ns` (the
    /// application's choice, e.g. a few ms after it was first meant to go).
    ///
    /// Past the deadline it is stale and nothing is charged. Inside it, this
    /// is [`Budget::try_charge`], and a refusal that lasts past the deadline
    /// (a halt, `Never`, a hold or a count that frees later) is stale too.
    ///
    /// Never use it for a request whose outcome is unknown (a timeout, a 5xx,
    /// a connection lost after sending): [`Budget::resend`] refuses those with
    /// [`Resend::Reconcile`].
    pub fn try_resend(
        &mut self,
        parts: &[Charge],
        class: Class,
        deadline_ns: i64,
        now_ns: i64,
    ) -> Resend {
        let now = self.clamp(now_ns);
        if now > deadline_ns {
            return Resend::Stale(None);
        }
        match self.try_charge(parts, class, now) {
            Ok(t) => Resend::Send(t),
            Err(l) if matches!(l.why, Why::Full | Why::Held) && l.ready_at_ns <= deadline_ns => {
                Resend::Wait(l)
            }
            Err(l) => Resend::Stale(Some(l)),
        }
    }

    /// Charges without a check: frames the transport sent on its own (a
    /// WebSocket Pong). May take a bucket past its limit.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn force(&mut self, part: Charge, now_ns: i64) -> Result<()> {
        let now = self.clamp(now_ns);
        let Some(b) = self.buckets.get_mut(part.bucket.index()) else {
            return Err(self.unknown(part.bucket));
        };
        b.st.forced += u64::from(part.cost);
        match &mut b.count {
            Count::Rolling(r) => {
                Self::add_now(r, &mut self.slots, now, u64::from(part.cost));
            }
            Count::Decay(l) => {
                l.drain(now);
                l.level = l.level.saturating_add(l.per_ns * u128::from(part.cost));
                l.peak = l.peak.max(l.level);
            }
            Count::Gate => {}
        }
        Ok(())
    }

    /// Adds up to `units` to the current sub-window; returns what was added.
    fn add_now(r: &mut Ring, slots: &mut [u32], now: i64, units: u64) -> u64 {
        let Some(ring) = slots.get_mut(r.off..r.off + r.len) else {
            return 0;
        };
        if now >= r.slot_end {
            r.advance(now, ring);
        }
        let Some(c) = ring.get_mut(r.head) else {
            return 0;
        };
        let add = u32::try_from(units).unwrap_or(u32::MAX).min(u32::MAX - *c);
        *c += add;
        r.sum += u64::from(add);
        r.peak = r.peak.max(r.sum);
        u64::from(add)
    }

    /// Gives a charge back: the request was never sent (`Sent::No`, or
    /// `Busy` from the client).
    ///
    /// A rolling part whose sub-window has left the ring gives nothing back;
    /// a decaying part gives back only what has not drained since.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have (nothing is
    /// refunded).
    pub fn refund(&mut self, ticket: &Ticket, now_ns: i64) -> Result<()> {
        let now = self.clamp(now_ns);
        for p in ticket.parts() {
            self.bucket(p.bucket)?;
        }
        for p in ticket.parts() {
            let Some(b) = self.buckets.get_mut(p.bucket.index()) else {
                continue;
            };
            let back = match &mut b.count {
                Count::Rolling(r) => {
                    let Some(ring) = self.slots.get_mut(r.off..r.off + r.len) else {
                        continue;
                    };
                    if now >= r.slot_end {
                        r.advance(now, ring);
                    }
                    let age = r
                        .head_abs
                        .saturating_sub(ticket.at_ns.div_euclid(r.slot_ns));
                    match usize::try_from(age) {
                        Ok(age) if age < r.len => {
                            let i = r.at(r.len - age);
                            ring.get_mut(i).map_or(0, |c| {
                                let give = (*c).min(p.cost);
                                *c -= give;
                                r.sum = r.sum.saturating_sub(u64::from(give));
                                u64::from(give)
                            })
                        }
                        _ => 0,
                    }
                }
                Count::Decay(l) => {
                    l.drain(now);
                    let since = u128::from(now.saturating_sub(ticket.at_ns).unsigned_abs());
                    let back = (l.per_ns * u128::from(p.cost))
                        .saturating_sub(l.units.saturating_mul(since))
                        .min(l.level);
                    l.level -= back;
                    u64::try_from(back / l.per_ns).unwrap_or(u64::MAX)
                }
                Count::Gate => u64::from(p.cost),
            };
            b.st.refunded += back;
        }
        Ok(())
    }

    /// Server truth: `used` units in the server's window (a usage header, a
    /// WebSocket `rateLimits` entry). Raises the local count to it; the
    /// difference counts as `foreign`. Never lowers it: a missing header is
    /// "unknown", not zero. For a venue that reports what remains, pass
    /// `limit - remaining`.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn observe_used(&mut self, bucket: BucketId, used: u32, now_ns: i64) -> Result<()> {
        let now = self.clamp(now_ns);
        let Some(b) = self.buckets.get_mut(bucket.index()) else {
            return Err(self.unknown(bucket));
        };
        b.st.server_used = Some(used);
        let foreign = match &mut b.count {
            Count::Rolling(r) => {
                if let Some(ring) = self.slots.get_mut(r.off..r.off + r.len)
                    && now >= r.slot_end
                {
                    r.advance(now, ring);
                }
                let gap = u64::from(used).saturating_sub(r.sum);
                if gap > 0 {
                    Self::add_now(r, &mut self.slots, now, gap)
                } else {
                    0
                }
            }
            Count::Decay(l) => {
                l.drain(now);
                let want = l.per_ns * u128::from(used);
                if want > l.level {
                    let had = u64::try_from(l.level.div_ceil(l.per_ns)).unwrap_or(u64::MAX);
                    l.level = want;
                    l.peak = l.peak.max(want);
                    u64::from(used).saturating_sub(had)
                } else {
                    0
                }
            }
            Count::Gate => 0,
        };
        b.st.foreign += foreign;
        Ok(())
    }

    /// The server's current limit, supplied by the application adapter. The reserve and guard stay.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have, a gate, a zero
    /// limit, a limit at or below the reserve, or one the guard leaves less
    /// than one unit of.
    pub fn set_limit(&mut self, bucket: BucketId, limit: u32) -> Result<()> {
        let Some(b) = self.buckets.get_mut(bucket.index()) else {
            return Err(self.unknown(bucket));
        };
        if limit == 0 || limit <= b.reserve {
            return Err(config(&b.name, "the limit must be above 0 and the reserve"));
        }
        match &mut b.count {
            Count::Rolling(r) => {
                r.limit = limit;
                r.cap = [u64::from(limit - b.reserve), u64::from(limit)];
            }
            Count::Decay(l) => {
                l.cap = Gcra::caps(limit, b.reserve, l.per_ns, l.guard_cut)
                    .ok_or_else(|| config(&b.name, "guard_ms leaves less than one unit"))?;
                l.max = limit;
            }
            Count::Gate => return Err(config(&b.name, "a gate has no limit")),
        }
        Ok(())
    }

    /// A throttle (a 429 or a venue code): the server thinks `bucket` is full.
    ///
    /// - Holds it for `wait_ms` (the server's wait) plus the guard. With no
    ///   server wait, for the backoff: the first, doubled per throttle in a
    ///   row, up to the longest. A throttle during a hold is the same episode
    ///   (no doubling); one that comes the longest backoff or more after the
    ///   last hold ended starts again from the first.
    /// - Marks the bucket full **only when the server gave no wait**: nothing
    ///   then passes for one whole window (rolling) or until it drains
    ///   (decay), even when the hold is shorter. With a server wait the hold
    ///   is the server's own answer, so the count is left alone and sends
    ///   come back at the hold's end as before. Filling on a short wait would
    ///   keep a day bucket empty for a day; `Verdict::Throttled.until_ns` is
    ///   the end of the hold, and it must stay true.
    ///
    /// Returns when the hold ends.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn throttled(
        &mut self,
        bucket: BucketId,
        wait_ms: Option<u64>,
        now_ns: i64,
    ) -> Result<i64> {
        let now = self.clamp(now_ns);
        let Some(b) = self.buckets.get_mut(bucket.index()) else {
            return Err(self.unknown(bucket));
        };
        if now >= b.hold_end || b.strikes == 0 {
            let in_a_row = b.strikes > 0 && now.saturating_sub(b.hold_end) < b.backoff_ns[1];
            b.strikes = if in_a_row {
                b.strikes.saturating_add(1)
            } else {
                1
            };
        }
        let wait = wait_ms.map_or_else(|| backoff(b.backoff_ns, b.strikes), ms_to_ns);
        b.hold(now.saturating_add(wait).saturating_add(b.guard_ns), now);
        b.st.throttles += 1;
        if wait_ms.is_none() {
            Self::mark_full(b, &mut self.slots, now);
        }
        Ok(b.hold_end)
    }

    /// An application-identified ban:
    /// as [`Budget::throttled`], and the bucket is **halted**.
    ///
    /// A halted bucket refuses every charge ([`Why::Banned`]) after the
    /// server's wait too, until [`Budget::clear_halt`]: a person decides when
    /// sending resumes. Pass the server's wait, or the venue's documented
    /// minimum.
    ///
    /// Returns when the server's wait (plus the guard) ends.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn banned(&mut self, bucket: BucketId, wait_ms: NonZeroU64, now_ns: i64) -> Result<i64> {
        let now = self.clamp(now_ns);
        let Some(b) = self.buckets.get_mut(bucket.index()) else {
            return Err(self.unknown(bucket));
        };
        b.halted = true;
        b.hold(
            now.saturating_add(ms_to_ns(wait_ms.get()))
                .saturating_add(b.guard_ns),
            now,
        );
        b.st.bans += 1;
        Self::mark_full(b, &mut self.slots, now);
        Ok(b.hold_end)
    }

    fn mark_full(b: &mut Bucket, slots: &mut [u32], now: i64) {
        match &mut b.count {
            Count::Rolling(r) => {
                let gap = u64::from(r.limit).saturating_sub(r.sum);
                Self::add_now(r, slots, now, gap);
            }
            Count::Decay(l) => {
                l.drain(now);
                l.level = l.level.max(u128::from(l.max) * l.per_ns);
                l.peak = l.peak.max(l.level);
            }
            Count::Gate => {}
        }
    }

    /// Lifts a halt: a person's decision after a ban. The server's wait, if
    /// it has not ended, still holds. Returns whether it was halted.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn clear_halt(&mut self, bucket: BucketId) -> Result<bool> {
        let Some(b) = self.buckets.get_mut(bucket.index()) else {
            return Err(self.unknown(bucket));
        };
        let was = b.halted;
        b.halted = false;
        b.block_until = b.hold_end;
        Ok(was)
    }

    /// Every halted bucket: alert while this is not empty. O(buckets).
    pub fn halted(&self) -> impl Iterator<Item = BucketId> + '_ {
        self.buckets
            .iter()
            .enumerate()
            .filter(|(_, b)| b.halted)
            .filter_map(|(i, _)| u16::try_from(i).ok().map(BucketId))
    }

    /// Catches every bucket up to `now_ns` from idle time: each rolling ring
    /// is advanced and each decay drained, so the next [`Budget::try_charge`]
    /// is O(parts) with no ring catch-up.
    ///
    /// Call it when the send path has nothing to do (once per loop pass is
    /// enough). Pure code: no allocation, no lock, no syscall.
    pub fn tick(&mut self, now_ns: i64) {
        let now = self.clamp(now_ns);
        for b in &mut self.buckets {
            match &mut b.count {
                Count::Rolling(r) => {
                    if now >= r.slot_end
                        && let Some(ring) = self.slots.get_mut(r.off..r.off + r.len)
                    {
                        r.advance(now, ring);
                    }
                }
                Count::Decay(l) => l.drain(now),
                Count::Gate => {}
            }
        }
    }

    /// Clears the count (a new connection, for a per-connection bucket).
    /// Holds and halts stay.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn reset(&mut self, bucket: BucketId, now_ns: i64) -> Result<()> {
        let now = self.clamp(now_ns);
        let Some(b) = self.buckets.get_mut(bucket.index()) else {
            return Err(self.unknown(bucket));
        };
        match &mut b.count {
            Count::Rolling(r) => {
                if let Some(ring) = self.slots.get_mut(r.off..r.off + r.len) {
                    if now >= r.slot_end {
                        r.advance(now, ring);
                    }
                    ring.fill(0);
                }
                r.sum = 0;
            }
            Count::Decay(l) => {
                l.level = 0;
                l.at = now;
            }
            Count::Gate => {}
        }
        Ok(())
    }

    /// When `bucket`'s hold ends: `None` if it is not held, `Some(i64::MAX)`
    /// while it is halted.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn held_until(&self, bucket: BucketId, now_ns: i64) -> Result<Option<i64>> {
        Ok(self.bucket(bucket)?.held_until(now_ns.max(self.last_ns)))
    }

    /// `bucket`'s counters at `now_ns`.
    ///
    /// # Errors
    /// [`Error::Config`]: a bucket this budget does not have.
    pub fn stats(&self, bucket: BucketId, now_ns: i64) -> Result<BucketStats> {
        let b = self.bucket(bucket)?;
        let now = now_ns.max(self.last_ns);
        let (used, limit, peak) = match &b.count {
            Count::Rolling(r) => {
                let ring = self.slots.get(r.off..r.off + r.len).unwrap_or(&[]);
                (r.used_at(now, ring), r.limit, r.peak)
            }
            Count::Decay(l) => (
                l.used_at(now),
                l.max,
                u64::try_from(l.peak.div_ceil(l.per_ns)).unwrap_or(u64::MAX),
            ),
            Count::Gate => (0, 0, 0),
        };
        let s = b.st;
        Ok(BucketStats {
            used,
            limit,
            peak,
            charged: s.charged,
            forced: s.forced,
            limited: s.limited,
            refunded: s.refunded,
            foreign: s.foreign,
            server_used: s.server_used,
            throttles: s.throttles,
            bans: s.bans,
            held_ns: s.held_ns,
            held_until_ns: b.held_until(now),
            halted: b.halted,
        })
    }
}

/// One local address connections may come from ([`crate::Target::source`]),
/// and its buckets in the application's [`Budget`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceIp {
    /// The address: one of this host's, never unspecified.
    pub ip: IpAddr,
    /// Charged one unit per connect attempt from `ip`: the venue's limit
    /// on connects per IP. Its own bucket: two IPs never share one.
    pub connects: BucketId,
    /// The IP's request budget (a weight per IP), for the application to
    /// charge with the requests it sends on connections from `ip`; `None`
    /// when the venue has none.
    pub sends: Option<BucketId>,
}

/// The IP [`SourceIps::try_connect`] picked, and what it charged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picked {
    /// Connect from this address ([`crate::Target::with_source`]).
    pub ip: IpAddr,
    /// Its index in [`SourceIps::ips`].
    pub index: usize,
    /// The connect unit charged: give it back with [`Budget::refund`] if
    /// the connect never went out.
    pub ticket: Ticket,
}

/// Several source IPs, each with its own budget: the connect gate for a
/// venue that limits connects and requests per IP.
///
/// - [`SourceIps::try_connect`] picks the next IP whose connect bucket
///   takes one more connect, round-robin from after the last one picked,
///   and charges it. When none does it refuses with the [`Limited`] that
///   frees first: the caller waits (queues) or drops the connect. It never
///   sleeps.
/// - Bind the connection to the pick: [`crate::Target::with_source`], or
///   [`crate::reconnect::Reconnect::poll_sourced`], which does both for
///   every link.
/// - Charge requests sent on that connection to the IP's
///   [`SourceIp::sends`] together with the account's buckets.
/// - The buckets are the application's [`Budget`]'s: throttles, bans and
///   server usage apply to them as to any bucket. A held IP is passed over.
///
/// Like [`Budget`]: no I/O, no clock reads, no allocation after
/// [`SourceIps::new`].
#[derive(Debug, Clone)]
pub struct SourceIps {
    ips: Vec<SourceIp>,
    next: usize,
}

impl SourceIps {
    /// A pool of `ips`, tried in this order.
    ///
    /// # Errors
    /// [`Error::Config`]: no source IPs; an unspecified address; an IP
    /// listed twice; two IPs that share a `connects` bucket.
    pub fn new(ips: Vec<SourceIp>) -> Result<Self> {
        if ips.is_empty() {
            return Err(Error::Config("no source IPs".into()));
        }
        for (i, a) in ips.iter().enumerate() {
            if a.ip.is_unspecified() {
                return Err(Error::Config(format!(
                    "source IP {} is unspecified: name one of this host's addresses",
                    a.ip
                )));
            }
            for b in &ips[..i] {
                if a.ip == b.ip {
                    return Err(Error::Config(format!("source IP {} is listed twice", a.ip)));
                }
                if a.connects == b.connects {
                    return Err(Error::Config(format!(
                        "source IPs {} and {} share a connects bucket: each needs its own",
                        b.ip, a.ip
                    )));
                }
            }
        }
        Ok(Self { ips, next: 0 })
    }

    /// The IPs, in the order given.
    #[must_use]
    pub fn ips(&self) -> &[SourceIp] {
        &self.ips
    }

    /// The entry for `ip`.
    #[must_use]
    pub fn get(&self, ip: IpAddr) -> Option<&SourceIp> {
        self.ips.iter().find(|s| s.ip == ip)
    }

    /// Picks an IP for one connect and charges its `connects` bucket one
    /// unit at `now_ns` (the budget's clock).
    ///
    /// Tries each IP once, starting after the last one picked, so connects
    /// spread over the IPs: at most one charge attempt per IP.
    ///
    /// # Errors
    /// [`Limited`]: no IP takes a connect now; of the refusals, the one
    /// that frees first (`ready_at_ns`). Nothing was charged.
    pub fn try_connect(
        &mut self,
        budget: &mut Budget,
        now_ns: i64,
    ) -> std::result::Result<Picked, Limited> {
        let n = self.ips.len();
        let mut soonest = Limited {
            bucket: BucketId(0),
            ready_at_ns: i64::MAX,
            why: Why::Never,
        };
        for step in 0..n {
            let index = (self.next + step) % n;
            let src = self.ips[index];
            match budget.try_charge(&[Charge::new(src.connects, 1)], Class::Normal, now_ns) {
                Ok(ticket) => {
                    self.next = (index + 1) % n;
                    return Ok(Picked {
                        ip: src.ip,
                        index,
                        ticket,
                    });
                }
                Err(l) => {
                    if step == 0 || l.ready_at_ns < soonest.ready_at_ns {
                        soonest = l;
                    }
                }
            }
        }
        Err(soonest)
    }
}
