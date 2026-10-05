//! A single-level deadline wheel on UNIX-epoch nanoseconds.
//!
//! Live and sim poll the same [`TimerWheel`]. The clock that supplies `now`
//! is the only difference. Schedule and cancel are O(1). An idle
//! [`TimerWheel::poll`] is one compare.
//!
//! A timer is one 32-byte record, indexed by its handle: deadline, token,
//! sequence, the links of its spoke's list, a repeating flag and a stale
//! flag. A repeating timer's period sits in a side column that only
//! repeating schedules write. Each spoke is a doubly linked list threaded
//! through the records, so a schedule pops a free handle, writes one record
//! and pushes it onto its spoke: an append, as a heap's push is, without the
//! sift. Cancel unlinks in place and frees the handle at once.
//!
//! A spoke's head holds its earliest deadline: a schedule later than the head
//! goes second, out of line. So the search for the next deadline, which the
//! simulation makes after every firing, reads one record per occupied spoke
//! while each head is current. When the earliest leaves its spoke, or a
//! re-arm moves it later, the head is marked stale, and the next search to
//! reach that spoke walks its list once and moves the earliest back to the
//! head.
//!
//! Spokes keep no occupancy bits. The cold walks, a poll after a jump and the
//! next-deadline search, read the spoke heads instead, sixteen at a time past
//! the first few. A byte per spoke would cut what they read to a quarter,
//! but its store cost an ascending schedule 7 to 11%.
//!
//! [`TimerWheel::new`] sizes the slab at `ticks_per_wheel * timers_per_spoke`
//! records, an average: a spoke's list has no capacity of its own, so a
//! crowded tick never grows the wheel. The slab doubles, out of line, only
//! when every record is in use. A re-arm relinks its own record, so it never
//! needs room and a repeating timer is never dropped. Otherwise schedule,
//! cancel and poll do not allocate. A default wheel takes 2,277,376 bytes
//! from `new`: the records, the period column, the heads, and the free list
//! and batch columns, each sized to the slab.
//!
//! [`TimerId`] is `(seq << 32) | handle`. A repeating re-arm relinks the same
//! record into its next spoke, so it keeps the same id.

use std::num::NonZeroU64;

use crate::clock::Nanos;

/// Free-slot sentinel. A deadline of this value is rejected.
const NIL: i64 = i64::MIN;
/// The end of a spoke's list, and an empty spoke's head. Never a handle: the
/// slab holds at most `u32::MAX` records.
const END: u32 = u32::MAX;
/// Forces the next poll to scan. Not a scheduled deadline: [`NIL`] is rejected
/// and this value is only written as the idle cursor.
const SCAN: i64 = i64::MIN + 1;

/// Construction knobs. The defaults are the live HFT wheel: a 1.024 µs tick,
/// 4096 spokes and 8 timers per spoke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Width of one spoke, a power of two. A timer fires at most this late
    /// when the caller polls every tick.
    pub tick_ns: i64,
    /// Spokes in the wheel, a power of two.
    pub ticks_per_wheel: u32,
    /// Timers per spoke on average: the slab starts with `ticks_per_wheel`
    /// times this many records. One spoke may hold any number of them, and
    /// the slab doubles only when every record is in use. A re-arm reuses
    /// its own record, so it never needs room and a repeating timer is never
    /// dropped.
    pub timers_per_spoke: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tick_ns: 1 << 10,
            ticks_per_wheel: 4096,
            timers_per_spoke: 8,
        }
    }
}

impl Settings {
    /// Accept powers of two and a non-empty spoke.
    ///
    /// # Errors
    ///
    /// [`TimerError::Settings`] when `tick_ns` or `ticks_per_wheel` is not a
    /// positive power of two, or `timers_per_spoke` is zero.
    pub fn checked(self) -> Result<Self, TimerError> {
        let Ok(tick) = u64::try_from(self.tick_ns) else {
            return Err(TimerError::Settings);
        };
        if !tick.is_power_of_two() {
            return Err(TimerError::Settings);
        }
        if self.ticks_per_wheel == 0 || !self.ticks_per_wheel.is_power_of_two() {
            return Err(TimerError::Settings);
        }
        if self.timers_per_spoke == 0 {
            return Err(TimerError::Settings);
        }
        Ok(self)
    }
}

/// Why a schedule or a wheel could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerError {
    /// The deadline is the free-slot sentinel.
    NilDeadline,
    /// A repeating period was not greater than zero.
    Period,
    /// [`Settings`] was not a pair of powers of two with a non-empty spoke.
    Settings,
    /// The slab cannot grow without overflowing the index width.
    Capacity,
}

impl std::fmt::Display for TimerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NilDeadline => write!(f, "deadline uses the free-slot sentinel"),
            Self::Period => write!(f, "period must be greater than zero"),
            Self::Settings => write!(f, "timer wheel settings are not powers of two"),
            Self::Capacity => write!(f, "timer wheel cannot grow"),
        }
    }
}

impl std::error::Error for TimerError {}

/// A caller-addressed timer. Repeating re-arms keep this value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerId(u64);

impl TimerId {
    fn pack(seq: u32, handle: u32) -> Self {
        Self((u64::from(seq) << 32) | u64::from(handle))
    }

    /// Sequence assigned by [`TimerWheel::schedule`]. Stable across re-arms.
    #[inline]
    #[must_use]
    pub fn seq(self) -> u32 {
        u32::try_from(self.0 >> 32).unwrap_or(0)
    }

    fn handle(self) -> u32 {
        u32::try_from(self.0 & 0xffff_ffff).unwrap_or(0)
    }
}

/// One timer a [`TimerWheel::poll`] found due, in `(deadline, seq)` order.
///
/// The slot of a one-shot is already free. A repeating timer is already
/// re-armed under the same [`Fired::id`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fired {
    /// Id to pass to [`TimerWheel::cancel`].
    pub id: TimerId,
    /// Token the caller passed to schedule.
    pub token: u64,
    /// Deadline that came due, not the time of the poll.
    pub deadline: Nanos,
    /// Whole periods skipped. Zero for a one-shot, and zero when `poll` is
    /// called at the deadline.
    pub missed: u64,
    /// Zero for a one-shot. Otherwise the repeating period.
    pub period: i64,
}

/// Total order of one sim input. Events are rank 0 and timers rank 1, so at
/// an equal timestamp every event precedes every timer.
///
/// An event orders by feed index (`a`), recording id (`b`) and position (`c`).
/// A timer orders by sequence (`a`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimKey {
    /// Event or deadline time.
    pub ts: i64,
    /// `0` for a feed event, `1` for a timer.
    pub rank: u8,
    /// Feed index, or the timer sequence.
    pub a: u64,
    /// Recording id. Zero for a timer.
    pub b: i64,
    /// Position in the recording. Zero for a timer.
    pub c: i64,
}

impl SimKey {
    /// A feed event. Rank 0.
    #[must_use]
    pub const fn event(ts: i64, feed: u64, recording: i64, position: i64) -> Self {
        Self {
            ts,
            rank: 0,
            a: feed,
            b: recording,
            c: position,
        }
    }

    /// A timer firing. Rank 1, ordered by `seq`.
    #[must_use]
    pub const fn timer(ts: i64, seq: u64) -> Self {
        Self {
            ts,
            rank: 1,
            a: seq,
            b: 0,
            c: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct Due {
    handle: u32,
    deadline: i64,
    seq: u32,
}

/// One timer, indexed by handle, in half a cache line. A free record has
/// `deadline == NIL` and `seq == 0`. `next` and `prev` link the spoke's list:
/// a re-arm relinks the record, never moves it, so the id survives. A
/// repeating timer's period lives in [`TimerWheel::periods`], so a one-shot
/// never touches that column.
#[repr(C)]
#[derive(Clone, Copy)]
struct Rec {
    deadline: i64,
    token: u64,
    seq: u32,
    next: u32,
    prev: u32,
    /// Non-zero for a repeating timer.
    repeating: u16,
    /// Read on a spoke's head only: non-zero when the head may not hold the
    /// spoke's earliest deadline, after that timer left or moved later.
    stale: u16,
}

const FREE: Rec = Rec {
    deadline: NIL,
    token: 0,
    seq: 0,
    next: END,
    prev: END,
    repeating: 0,
    stale: 0,
};

/// Every record, by handle.
///
/// The field is named `deadline`, and [`Records`] indexes to a deadline
/// ([`NIL`] for a free handle), only so the unit tests, written against the
/// earlier deadline column, read `slab.deadline[i]` unchanged. The wheel
/// itself goes through the records.
struct Slab {
    deadline: Records,
}

/// The records. See [`Slab`] for why indexing yields a deadline.
struct Records(Box<[Rec]>);

impl std::ops::Index<usize> for Records {
    type Output = i64;

    fn index(&self, handle: usize) -> &i64 {
        &self.0[handle].deadline
    }
}

/// Agrona-style deadline wheel.
///
/// The agent is not called from [`TimerWheel::poll`]. The runtime walks
/// [`Fired`] afterwards, so a callback can schedule and cancel freely.
#[repr(C)]
pub struct TimerWheel {
    slab: Slab,
    /// A repeating timer's period, by handle.
    periods: Box<[i64]>,
    /// Each spoke's first record, [`END`] when the spoke is empty. Unless it
    /// is stale, it holds the spoke's earliest deadline.
    head: Box<[u32]>,
    /// Beside `head`, whose length a schedule reads with it.
    tick_shift: u64,
    spokes: usize,
    /// The slab holds `spokes * stride` records. An average, not a limit on
    /// any one spoke.
    stride: usize,
    /// The earliest live deadline, `i64::MAX` when empty, or [`SCAN`] after
    /// the timer that held it was cancelled.
    next_tick_start: i64,
    /// No live deadline has a tick below this, so the next-deadline search
    /// walks forward from here instead of scanning the slab.
    floor_tick: i64,
    last_visits: u64,
    // A schedule stores `next_seq` and the free list's length, and the next
    // schedule loads them again. The layout is `repr(C)` so neither shares a
    // paired load with a field read beside it: such a load overlaps the
    // pending narrower store, which cannot forward to it, and every schedule
    // waits for that store to drain (measured: it doubled a schedule). Only
    // the benchmark's ascending-schedule gate guards this order: rerun it
    // after adding or moving a field.
    next_seq: u32,
    /// Free handles, lowest on top, so reuse is deterministic.
    free: Vec<u32>,
    due: Vec<Due>,
    inflight: Vec<TimerId>,
    dead: Vec<u8>,
}

#[inline]
const fn ix(i: u32) -> usize {
    i as usize
}

impl TimerWheel {
    /// Size every column. The returned wheel does not allocate on the
    /// steady-state path until its handles run out.
    ///
    /// # Errors
    ///
    /// [`TimerError::Settings`] for a bad [`Settings`]. [`TimerError::Capacity`]
    /// when the initial slab does not fit in a `u32` index.
    #[must_use = "a wheel that is dropped does not run timers"]
    pub fn new(settings: Settings) -> Result<Self, TimerError> {
        let settings = settings.checked()?;
        let spokes = ix(settings.ticks_per_wheel);
        let stride = ix(settings.timers_per_spoke);
        let slots = spokes.checked_mul(stride).ok_or(TimerError::Capacity)?;
        let top = u32::try_from(slots).map_err(|_| TimerError::Capacity)?;
        Ok(Self {
            tick_shift: u64::from(settings.tick_ns.trailing_zeros()),
            spokes,
            stride,
            next_seq: 1,
            next_tick_start: i64::MAX,
            floor_tick: i64::MAX,
            last_visits: 0,
            slab: Slab {
                deadline: Records(vec![FREE; slots].into_boxed_slice()),
            },
            periods: vec![0; slots].into_boxed_slice(),
            head: vec![END; spokes].into_boxed_slice(),
            free: (0..top).rev().collect(),
            due: Vec::with_capacity(slots),
            inflight: Vec::with_capacity(slots),
            dead: Vec::with_capacity(slots),
        })
    }

    /// One-shot. The id dies when the timer fires, before the caller sees it.
    /// A deadline at or before the next `now` waits for that poll.
    ///
    /// # Errors
    ///
    /// [`TimerError::NilDeadline`] or [`TimerError::Capacity`].
    #[expect(
        clippy::inline_always,
        reason = "out of line, the call and a Result returned through memory add 30% to a schedule"
    )]
    #[inline(always)]
    pub fn schedule(&mut self, deadline: Nanos, token: u64) -> Result<TimerId, TimerError> {
        self.insert(deadline.0, 0, token)
    }

    /// Repeating. The first fire is `first`, then `first + k * period`.
    /// The same [`TimerId`] is kept across re-arms.
    ///
    /// # Errors
    ///
    /// [`TimerError::Period`] when `period` is not positive, plus the errors
    /// of [`TimerWheel::schedule`].
    #[inline]
    pub fn schedule_repeating(
        &mut self,
        first: Nanos,
        period: i64,
        token: u64,
    ) -> Result<TimerId, TimerError> {
        if period <= 0 {
            return Err(TimerError::Period);
        }
        self.insert(first.0, period, token)
    }

    /// Repeating, first deadline the next epoch multiple of `period`.
    /// An `now` that is already aligned is due on the next poll, not inside
    /// this call.
    ///
    /// # Errors
    ///
    /// [`TimerError::Period`] when `period` is not positive, plus the errors
    /// of [`TimerWheel::schedule`].
    pub fn schedule_aligned(
        &mut self,
        now: Nanos,
        period: i64,
        token: u64,
    ) -> Result<TimerId, TimerError> {
        let deadline = aligned_deadline(now.0, period)?;
        self.schedule_repeating(Nanos(deadline), period, token)
    }

    /// `false` when `id` is not a live timer. Cancelling an id whose handle
    /// was reused is a no-op. An id still in the latest [`TimerWheel::poll`]
    /// batch is suppressed either way: [`TimerWheel::is_suppressed`] reports it.
    #[inline]
    pub fn cancel(&mut self, id: TimerId) -> bool {
        if !self.inflight.is_empty() {
            self.suppress(id);
        }
        let h = ix(id.handle());
        let seq = id.seq();
        if seq == 0 || self.slab.deadline.0.get(h).map(|r| r.seq) != Some(seq) {
            return false;
        }
        let deadline = self.release(h);
        self.note_cancelled(deadline);
        true
    }

    /// Write the timers due at `now` into `fired`, earliest deadline first
    /// and sequence order for a tie. Returns how many were written.
    ///
    /// Idle (`now` before the cached next deadline; an empty wheel stores
    /// [`i64::MAX`]) returns 0 and does not write `fired`. Otherwise `fired`
    /// is replaced with this batch. `limit` keeps the rest scheduled, without
    /// moving the idle cursor past them. A jump visits each occupied spoke at
    /// most once.
    #[inline]
    pub fn poll(&mut self, now: Nanos, fired: &mut Vec<Fired>, limit: usize) -> usize {
        if now.0 < self.next_tick_start {
            return 0;
        }
        self.poll_cold(now.0, fired, limit)
    }

    /// Apply one journalled firing instead of polling computed deadlines.
    /// Matches a live token and original deadline, preserving a repeating
    /// timer's id and its recorded skipped-period count. Missing or cancelled
    /// timers reject the replay rather than inventing a timer handle.
    pub fn journal_fire(
        &mut self,
        token: u64,
        deadline: Nanos,
        _at: Nanos,
        missed: u64,
    ) -> Option<Fired> {
        let (h, rec) = self
            .slab
            .deadline
            .0
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, rec)| rec.seq != 0 && rec.token == token && rec.deadline == deadline.0)
            .min_by_key(|(_, rec)| rec.seq)?;
        let period = self.period(h, rec);
        if period == 0 && missed != 0 {
            return None;
        }
        let id = TimerId::pack(rec.seq, u32::try_from(h).ok()?);
        if period == 0 {
            self.release(h);
        } else {
            let periods = i128::from(missed) + 1;
            let next = i128::from(deadline.0) + periods * i128::from(period);
            let next = i64::try_from(next).ok()?;
            if next == NIL {
                return None;
            }
            self.rearm(h, deadline.0, next);
        }
        self.inflight.clear();
        self.dead.clear();
        self.inflight.push(id);
        self.dead.push(0);
        self.recompute_next();
        Some(Fired {
            id,
            token,
            deadline,
            missed,
            period,
        })
    }

    /// Prepare all due journal timers before dispatch, preserving live batch
    /// cancellation semantics: one-shots are already dead and repeats rearmed.
    /// Returns their original expiries in journal order.
    pub fn journal_batch(
        &mut self,
        entries: &[(u64, Nanos, u64)],
        at: Nanos,
    ) -> Option<Vec<Fired>> {
        let mut fired = Vec::with_capacity(entries.len());
        for &(token, deadline, missed) in entries {
            fired.push(self.journal_fire(token, deadline, at, missed)?);
        }
        self.inflight.clear();
        self.dead.clear();
        for f in &fired {
            self.inflight.push(f.id);
            self.dead.push(0);
        }
        Some(fired)
    }

    /// Earliest live deadline. `None` when the wheel is empty. Cached until
    /// a schedule, cancel or fire changes it. The sim driver polls at this
    /// time, so a timer fires at its deadline rather than up to a tick late.
    #[must_use]
    pub fn next_deadline(&mut self) -> Option<Nanos> {
        if self.is_empty() {
            return None;
        }
        if self.next_tick_start == SCAN {
            self.recompute_next();
        }
        Some(Nanos(self.next_tick_start))
    }

    /// `true` when `id` was in the latest non-idle poll batch and
    /// [`TimerWheel::cancel`] has since named it.
    #[must_use]
    pub fn is_suppressed(&self, id: TimerId) -> bool {
        self.inflight
            .iter()
            .zip(self.dead.iter())
            .any(|(inflight, dead)| *inflight == id && *dead == 1)
    }

    /// Occupied spokes the last non-idle poll inspected. A jump of many
    /// revolutions stays at or below the spoke count.
    #[must_use]
    pub const fn spokes_visited(&self) -> u64 {
        self.last_visits
    }

    /// Live timers.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.spokes * self.stride - self.free.len()
    }

    /// Write every column once, so zero-initialised pages are faulted in
    /// now rather than by the first live timer.
    pub fn prefault(&mut self) {
        let free = self.slab.deadline.0.len();
        self.slab.deadline.0.fill(FREE);
        self.periods.fill(0);
        self.head.fill(END);
        debug_assert!(self.is_empty(), "prefault on a wheel with timers");
        debug_assert_eq!(free, self.free.len(), "every handle is free");
    }

    /// `true` when no timer is scheduled.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[expect(
        clippy::inline_always,
        reason = "out of line, the call and a Result returned through memory add 30% to a schedule"
    )]
    #[inline(always)]
    fn insert(&mut self, deadline: i64, period: i64, token: u64) -> Result<TimerId, TimerError> {
        if deadline == NIL {
            return Err(TimerError::NilDeadline);
        }
        let Some(handle) = self.free.pop() else {
            return self
                .insert_grown(deadline, period, token)
                .map(|id| TimerId(id.get()))
                .ok_or(TimerError::Capacity);
        };
        Ok(self.place(handle, deadline, period, token))
    }

    /// Every handle is live: double the slab, then place. Out of line so the
    /// hot insert stays a leaf the caller can inline. The id comes back in a
    /// register: a returned `Result` goes through memory, and the hot path
    /// would then pay a store and a load to meet it.
    #[cold]
    #[inline(never)]
    fn insert_grown(&mut self, deadline: i64, period: i64, token: u64) -> Option<NonZeroU64> {
        self.grow().ok()?;
        let handle = self.free.pop()?;
        NonZeroU64::new(self.place(handle, deadline, period, token).0)
    }

    #[expect(
        clippy::inline_always,
        reason = "schedule's hot path: with plain #[inline], ascending schedule lost to the heap"
    )]
    #[inline(always)]
    fn place(&mut self, handle: u32, deadline: i64, period: i64, token: u64) -> TimerId {
        let seq = self.take_seq();
        if period != 0 {
            self.periods[ix(handle)] = period;
        }
        self.link(
            handle,
            Rec {
                deadline,
                token,
                seq,
                next: END,
                prev: END,
                repeating: u16::from(period != 0),
                stale: 0,
            },
        );
        let next = self.next_tick_start;
        if deadline < next || next == SCAN {
            // `SCAN` stays `SCAN`: no deadline is below it.
            self.next_tick_start = next.min(deadline);
            self.floor_tick = self.floor_tick.min(deadline >> self.tick_shift);
        }
        TimerId::pack(seq, handle)
    }

    /// Push `rec` onto the head of its deadline's spoke, as `handle`. On a
    /// spoke that already had a head, [`Self::settle`] then keeps the
    /// spoke's earliest deadline there.
    #[expect(
        clippy::inline_always,
        reason = "schedule's hot path: with plain #[inline], ascending schedule lost to the heap"
    )]
    #[inline(always)]
    fn link(&mut self, handle: u32, rec: Rec) {
        let spoke = self.spoke_of(rec.deadline);
        let recs = &mut self.slab.deadline.0;
        debug_assert!(spoke < self.head.len() && ix(handle) < recs.len());
        // `get_mut`, not indexing: both are in bounds by construction, and a
        // panic branch apiece put a schedule at the inliner's limit (cost 520
        // of 525 at the benchmark's call site, 430 without them).
        let next = self
            .head
            .get_mut(spoke)
            .map_or(END, |head| std::mem::replace(head, handle));
        if let Some(slot) = recs.get_mut(ix(handle)) {
            *slot = Rec {
                next,
                prev: END,
                stale: 0,
                ..rec
            };
        }
        if next != END {
            self.settle(spoke, handle, next);
        }
    }

    /// `handle` was just pushed onto `spoke` in front of `old`. It stays the
    /// head when its deadline is not later, and takes `old`'s staleness;
    /// otherwise it moves behind `old`. Either way a head that is not stale
    /// holds the spoke's earliest deadline. Out of line so a schedule stays
    /// small enough to inline into its caller.
    #[inline(never)]
    fn settle(&mut self, spoke: usize, handle: u32, old: u32) {
        let recs = &mut self.slab.deadline.0;
        let head = recs[ix(old)];
        if recs[ix(handle)].deadline <= head.deadline {
            recs[ix(old)].prev = handle;
            recs[ix(handle)].stale = head.stale;
            return;
        }
        self.head[spoke] = old;
        recs[ix(old)].next = handle;
        if head.next != END {
            recs[ix(head.next)].prev = handle;
        }
        let rec = &mut recs[ix(handle)];
        rec.next = head.next;
        rec.prev = old;
    }

    /// Take `h` out of its spoke's list. The record keeps its fields.
    #[inline]
    fn unlink(&mut self, h: usize) {
        let Rec {
            deadline,
            next,
            prev,
            ..
        } = self.slab.deadline.0[h];
        if prev == END {
            let spoke = self.spoke_of(deadline);
            self.head[spoke] = next;
        } else {
            self.slab.deadline.0[ix(prev)].next = next;
        }
        if next != END {
            let rec = &mut self.slab.deadline.0[ix(next)];
            rec.prev = prev;
            // When `h` was the head the earliest left, and `next` heads the
            // spoke without being known as its earliest. Otherwise unread.
            rec.stale = 1;
        }
    }

    /// Free a live handle. Returns the deadline it held.
    #[inline]
    fn release(&mut self, h: usize) -> i64 {
        let deadline = self.slab.deadline.0[h].deadline;
        self.unlink(h);
        let rec = &mut self.slab.deadline.0[h];
        rec.deadline = NIL;
        rec.seq = 0;
        // `h` came from a `u32` handle.
        self.free.push(u32::try_from(h).unwrap_or(u32::MAX));
        deadline
    }

    /// Zero for a one-shot. Otherwise the repeating period.
    fn period(&self, h: usize, rec: Rec) -> i64 {
        if rec.repeating == 0 {
            0
        } else {
            self.periods[h]
        }
    }

    #[inline(never)]
    #[cold]
    fn poll_cold(&mut self, now: i64, fired: &mut Vec<Fired>, limit: usize) -> usize {
        self.last_visits = 0;
        if self.next_tick_start == SCAN {
            self.recompute_next();
            if self.is_empty() || now < self.next_tick_start {
                return 0;
            }
        }
        fired.clear();
        self.inflight.clear();
        self.dead.clear();
        self.due.clear();
        self.collect(now);
        self.due
            .sort_unstable_by(|a, b| a.deadline.cmp(&b.deadline).then_with(|| a.seq.cmp(&b.seq)));
        let n = limit.min(self.due.len());
        for i in 0..n {
            let handle = self.due[i].handle;
            self.fire_one(handle, now, fired);
        }
        if n < self.due.len() {
            // The rest are already due and sorted; the first is the minimum.
            self.next_tick_start = self.due[n].deadline;
        } else {
            // Everything at or before `now` fired or re-armed past it.
            self.floor_tick = now >> self.tick_shift;
            self.recompute_next();
        }
        fired.len()
    }

    fn fire_one(&mut self, handle: u32, now: i64, fired: &mut Vec<Fired>) {
        let h = ix(handle);
        let rec = self.slab.deadline.0[h];
        let deadline = rec.deadline;
        let period = self.period(h, rec);
        let id = TimerId::pack(rec.seq, handle);
        let missed = if period == 0 {
            self.release(h);
            0
        } else {
            let (missed, next) = next_grid(deadline, period, now);
            self.rearm(h, deadline, next);
            missed
        };
        fired.push(Fired {
            id,
            token: rec.token,
            deadline: Nanos(deadline),
            missed,
            period,
        });
        self.inflight.push(id);
        self.dead.push(0);
    }

    /// Move a live repeating timer to `next`, under the same handle.
    fn rearm(&mut self, h: usize, old_deadline: i64, next: i64) {
        if self.spoke_of(next) == self.spoke_of(old_deadline) {
            let rec = &mut self.slab.deadline.0[h];
            rec.deadline = next;
            // Later than before, so a head may no longer be the earliest.
            rec.stale = 1;
            return;
        }
        let rec = self.slab.deadline.0[h];
        self.unlink(h);
        // `h` came from a `u32` handle.
        let handle = u32::try_from(h).unwrap_or(u32::MAX);
        self.link(
            handle,
            Rec {
                deadline: next,
                ..rec
            },
        );
    }

    fn collect(&mut self, now: i64) {
        let start_tick = self.next_tick_start >> self.tick_shift;
        let end_tick = now >> self.tick_shift;
        let span = end_tick.saturating_sub(start_tick);
        if span < 0 || span >= i64::try_from(self.spokes).unwrap_or(i64::MAX) {
            self.visit_occupied(now);
            return;
        }
        let mut tick = start_tick;
        loop {
            let spoke = self.spoke_of_tick(tick);
            if self.head[spoke] != END {
                self.visit_spoke(spoke, now);
            }
            // Stop before the step: `end_tick` is `i64::MAX` for a 1 ns tick.
            if tick >= end_tick {
                break;
            }
            tick += 1;
        }
    }

    fn visit_occupied(&mut self, now: i64) {
        let mut at = 0;
        while let Some(spoke) = self.next_occupied(at, self.spokes) {
            self.visit_spoke(spoke, now);
            at = spoke + 1;
        }
    }

    /// The first non-empty spoke in `from..to`. Spokes keep no occupancy bits:
    /// any upkeep on schedule costs more than the gap to a heap's push, so the
    /// cold walks read the heads instead.
    #[inline]
    fn next_occupied(&self, from: usize, to: usize) -> Option<usize> {
        first_live(&self.head[from..to]).map(|i| from + i)
    }

    fn visit_spoke(&mut self, spoke: usize, now: i64) {
        self.last_visits += 1;
        let mut h = self.head[spoke];
        while h != END {
            let rec = self.slab.deadline.0[ix(h)];
            if rec.deadline <= now {
                self.due.push(Due {
                    handle: h,
                    deadline: rec.deadline,
                    seq: rec.seq,
                });
            }
            h = rec.next;
        }
    }

    /// The earliest deadline on a non-empty spoke: its head's, unless the
    /// head is stale. Then one walk of the list finds the earliest and moves
    /// it to the head, so the next search reads the head alone.
    fn spoke_min(&mut self, spoke: usize) -> i64 {
        let first = self.head[spoke];
        let head = self.slab.deadline.0[ix(first)];
        if head.stale == 0 {
            return head.deadline;
        }
        let (mut best, mut min) = (first, head.deadline);
        let mut h = head.next;
        while h != END {
            let rec = self.slab.deadline.0[ix(h)];
            if rec.deadline < min {
                (best, min) = (h, rec.deadline);
            }
            h = rec.next;
        }
        if best != first {
            // Not the head, so this unlink leaves the head alone.
            self.unlink(ix(best));
            self.slab.deadline.0[ix(first)].prev = best;
            let rec = &mut self.slab.deadline.0[ix(best)];
            rec.next = first;
            rec.prev = END;
            self.head[spoke] = best;
        }
        self.slab.deadline.0[ix(best)].stale = 0;
        min
    }

    fn recompute_next(&mut self) {
        if self.is_empty() {
            self.next_tick_start = i64::MAX;
            self.floor_tick = i64::MAX;
            return;
        }
        let min = self.earliest();
        // The minimum is a lower bound too; without this a floor that only
        // full polls advance drifts a revolution behind and every recompute
        // walks a whole revolution.
        self.next_tick_start = min;
        self.floor_tick = min >> self.tick_shift;
    }

    /// The earliest live deadline, from one walk of the occupied spokes
    /// forward from `floor_tick`. The first spoke holding a deadline on its
    /// tick in this revolution holds the minimum: later spokes are later
    /// ticks, and anything off-revolution is at least a revolution later.
    /// When no spoke does, every timer is a revolution or more out, and the
    /// minimum over all the walk saw is the answer.
    ///
    /// No deadline is below the floor, so a spoke's deadlines sit on its
    /// tick in this revolution or a later one, and its earliest is on the
    /// tick whenever any is: each spoke costs one read of its minimum.
    fn earliest(&mut self) -> i64 {
        let floor = self.floor_tick;
        let start = self.spoke_of_tick(floor);
        let mut min = i64::MAX;
        for (from, to) in [(start, self.spokes), (0, start)] {
            let mut at = from;
            while let Some(spoke) = self.next_occupied(at, to) {
                let off = spoke.wrapping_sub(start) & (self.spokes - 1);
                let tick = floor.saturating_add(i64::try_from(off).unwrap_or(i64::MAX));
                let first = self.spoke_min(spoke);
                if first >> self.tick_shift == tick {
                    return first;
                }
                min = min.min(first);
                at = spoke + 1;
            }
        }
        min
    }

    const fn note_cancelled(&mut self, deadline: i64) {
        if self.is_empty() {
            self.next_tick_start = i64::MAX;
            self.floor_tick = i64::MAX;
        } else if deadline == self.next_tick_start {
            self.next_tick_start = SCAN;
        }
    }

    fn suppress(&mut self, id: TimerId) {
        for (inflight, dead) in self.inflight.iter().zip(self.dead.iter_mut()) {
            if *inflight == id {
                *dead = 1;
            }
        }
    }

    /// Double the slab. Handles index records and links hold handles, so no
    /// list moves: only the new records join the free list.
    #[cold]
    fn grow(&mut self) -> Result<(), TimerError> {
        let new_stride = self.stride.checked_mul(2).ok_or(TimerError::Capacity)?;
        let slots = self
            .spokes
            .checked_mul(new_stride)
            .ok_or(TimerError::Capacity)?;
        let top = u32::try_from(slots).map_err(|_| TimerError::Capacity)?;
        let old = self.slab.deadline.0.len();
        let first_new = u32::try_from(old).map_err(|_| TimerError::Capacity)?;
        let mut recs = vec![FREE; slots];
        recs[..old].copy_from_slice(&self.slab.deadline.0);
        self.slab.deadline.0 = recs.into_boxed_slice();
        let mut periods = vec![0; slots];
        periods[..old].copy_from_slice(&self.periods);
        self.periods = periods.into_boxed_slice();
        // Room for every handle now, so the first release or batch after a
        // doubling does not allocate. `reserve` counts from the length.
        self.free.reserve(slots.saturating_sub(self.free.len()));
        self.free.extend((first_new..top).rev());
        self.stride = new_stride;
        self.due.reserve(slots.saturating_sub(self.due.len()));
        self.inflight
            .reserve(slots.saturating_sub(self.inflight.len()));
        self.dead.reserve(slots.saturating_sub(self.dead.len()));
        log::warn!("timer wheel doubled from {old} to {slots} timers");
        Ok(())
    }

    /// Zero marks a free record, so a wrapped sequence skips it.
    const fn take_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq = if seq == u32::MAX { 1 } else { seq + 1 };
        seq
    }

    #[inline]
    fn spoke_of(&self, deadline: i64) -> usize {
        self.spoke_of_tick(deadline >> self.tick_shift)
    }

    /// Masked by the head array's own length, so indexing it with the result
    /// needs no bounds compare.
    #[inline]
    fn spoke_of_tick(&self, tick: i64) -> usize {
        usize::try_from(tick.cast_unsigned()).unwrap_or(0) & self.head.len().wrapping_sub(1)
    }
}

/// The index of the first non-empty head. After a fire the next timer is
/// usually a spoke or two on, so a few heads are read singly; past those,
/// sixteen at a time. An empty head is the largest value, so a chunk's
/// minimum is [`END`] only when every head in it is empty, and that minimum
/// is the form the compiler turns into vector loads.
fn first_live(heads: &[u32]) -> Option<usize> {
    let near = heads.len().min(4);
    if let Some(i) = heads[..near].iter().position(|&h| h != END) {
        return Some(i);
    }
    let mut base = near;
    let mut chunks = heads[near..].chunks_exact(16);
    for chunk in chunks.by_ref() {
        if chunk.iter().copied().fold(END, u32::min) != END {
            return chunk.iter().position(|&h| h != END).map(|i| base + i);
        }
        base += 16;
    }
    chunks
        .remainder()
        .iter()
        .position(|&h| h != END)
        .map(|i| base + i)
}

/// First epoch multiple of `period` at or after `now`. An aligned `now` is
/// returned unchanged so the timer is due on the next poll.
///
/// # Errors
///
/// [`TimerError::Period`] when `period` is not positive.
#[must_use = "the aligned deadline is the value to schedule"]
pub const fn aligned_deadline(now: i64, period: i64) -> Result<i64, TimerError> {
    if period <= 0 {
        return Err(TimerError::Period);
    }
    let rem = now.rem_euclid(period);
    if rem == 0 {
        return Ok(now);
    }
    match now.checked_add(period - rem) {
        Some(deadline) if deadline != NIL => Ok(deadline),
        _ => Ok(i64::MAX),
    }
}

fn next_grid(deadline: i64, period: i64, now: i64) -> (u64, i64) {
    let missed = if now > deadline {
        let late = now.saturating_sub(deadline).cast_unsigned();
        late.checked_div(period.cast_unsigned()).unwrap_or(0)
    } else {
        0
    };
    (
        missed,
        add_steps(deadline, period, missed.saturating_add(1)),
    )
}

fn add_steps(deadline: i64, period: i64, steps: u64) -> i64 {
    let Ok(steps) = i64::try_from(steps) else {
        return i64::MAX;
    };
    let Some(delta) = steps.checked_mul(period) else {
        return i64::MAX;
    };
    match deadline.checked_add(delta) {
        Some(sum) if sum != NIL => sum,
        _ => i64::MAX,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use super::*;

    fn wheel(tick_ns: i64, spokes: u32, per: u32) -> Result<TimerWheel, TimerError> {
        TimerWheel::new(Settings {
            tick_ns,
            ticks_per_wheel: spokes,
            timers_per_spoke: per,
        })
    }

    fn tiny() -> Result<TimerWheel, TimerError> {
        wheel(1024, 8, 4)
    }

    #[test]
    fn journal_batch_frees_later_one_shot_before_first_callback() -> Result<(), TimerError> {
        let mut wheel = TimerWheel::new(Settings::default())?;
        let a = wheel.schedule(Nanos(10), 1)?;
        let b = wheel.schedule(Nanos(10), 2)?;
        let batch = wheel
            .journal_batch(&[(1, Nanos(10), 0), (2, Nanos(10), 0)], Nanos(10))
            .ok_or(TimerError::Capacity)?;
        assert_eq!(batch.iter().map(|f| f.id).collect::<Vec<_>>(), [a, b]);
        assert!(
            !wheel.cancel(b),
            "live poll already freed B before A's callback"
        );
        assert!(wheel.is_suppressed(b));
        Ok(())
    }

    #[test]
    fn journal_preserves_repeating_id_and_coalesced_grid() -> Result<(), TimerError> {
        let mut wheel = TimerWheel::new(Settings::default())?;
        let id = wheel.schedule_repeating(Nanos(10), 10, 17)?;
        let first = wheel
            .journal_fire(17, Nanos(10), Nanos(85), 7)
            .ok_or(TimerError::Capacity)?;
        assert_eq!(first.id, id);
        assert_eq!(first.missed, 7);
        assert_eq!(wheel.next_deadline(), Some(Nanos(90)));
        let second = wheel
            .journal_fire(17, Nanos(90), Nanos(90), 0)
            .ok_or(TimerError::Capacity)?;
        assert_eq!(second.id, id);
        assert!(wheel.cancel(id));
        assert!(wheel.journal_fire(17, Nanos(100), Nanos(100), 0).is_none());
        Ok(())
    }

    #[test]
    fn spoke_order_is_deadline_then_seq() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        // All three sit in spoke 0. Slot order is not deadline order.
        let late = w.schedule(Nanos(2_000), 1)?;
        let mid = w.schedule(Nanos(1_500), 2)?;
        let early = w.schedule(Nanos(1_100), 3)?;
        assert_eq!(w.poll(Nanos(3_000), &mut fired, 8), 3);
        assert_eq!(fired[0].id, early);
        assert_eq!(fired[1].id, mid);
        assert_eq!(fired[2].id, late);
        Ok(())
    }

    #[test]
    fn equal_deadlines_fire_in_schedule_order() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let first = w.schedule(Nanos(5_000), 10)?;
        let second = w.schedule(Nanos(5_000), 11)?;
        assert!(first.seq() < second.seq());
        w.poll(Nanos(5_000), &mut fired, 8);
        assert_eq!(fired[0].token, 10);
        assert_eq!(fired[1].token, 11);
        Ok(())
    }

    #[test]
    fn cancel_removes_a_timer_and_a_reused_slot_is_a_different_id() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let id = w.schedule(Nanos(4_000), 1)?;
        assert!(w.cancel(id));
        assert!(!w.cancel(id));
        assert_eq!(w.poll(Nanos(4_000), &mut fired, 8), 0);
        let again = w.schedule(Nanos(4_000), 2)?;
        assert!(!w.cancel(id));
        assert_eq!(w.poll(Nanos(4_000), &mut fired, 8), 1);
        assert_eq!(fired[0].id, again);
        assert_eq!(fired[0].token, 2);
        Ok(())
    }

    #[test]
    fn a_one_shot_is_free_before_the_caller_sees_it() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let id = w.schedule(Nanos(8_000), 7)?;
        assert_eq!(w.poll(Nanos(8_000), &mut fired, 8), 1);
        assert!(!w.cancel(id), "the id is already dead");
        let again = w.schedule(Nanos(9_000), 7)?;
        assert_ne!(again, id);
        assert_eq!(w.poll(Nanos(9_000), &mut fired, 8), 1);
        assert_eq!(fired[0].id, again);
        Ok(())
    }

    #[test]
    fn a_deadline_in_the_past_waits_for_the_next_poll() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        w.schedule(Nanos(100), 1)?;
        assert_eq!(w.poll(Nanos(50), &mut fired, 8), 0);
        assert_eq!(w.poll(Nanos(100), &mut fired, 8), 1);
        Ok(())
    }

    #[test]
    fn repeating_keeps_its_id_and_does_not_drift() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let period = 10_000_000;
        let first = 1_700_000_000_000_000_000;
        let id = w.schedule_repeating(Nanos(first), period, 4)?;
        let mut due = first;
        for step in 0..100 {
            assert_eq!(w.poll(Nanos(due), &mut fired, 8), 1);
            assert_eq!(fired[0].id, id);
            assert_eq!(fired[0].missed, 0);
            assert_eq!(fired[0].deadline.0, first + step * period);
            due = fired[0].deadline.0 + period;
        }
        assert_eq!(due, first + 100 * period);
        assert_eq!(due - first, 1_000_000_000);
        Ok(())
    }

    #[test]
    fn a_repeating_timer_can_be_cancelled_from_its_own_firing() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let id = w.schedule_repeating(Nanos(20_000), 5_000, 1)?;
        assert_eq!(w.poll(Nanos(20_000), &mut fired, 8), 1);
        assert!(w.cancel(id));
        assert!(w.is_suppressed(id));
        assert_eq!(w.poll(Nanos(25_000), &mut fired, 8), 0);
        Ok(())
    }

    #[test]
    fn a_stall_fires_once_and_lands_on_the_grid() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let period = 10_000_000;
        let first = 5_000_000_000_000;
        let id = w.schedule_repeating(Nanos(first), period, 9)?;
        let now = first + (period / 2) * 15;
        assert_eq!(w.poll(Nanos(now), &mut fired, 8), 1);
        assert_eq!(fired[0].id, id);
        assert_eq!(fired[0].missed, 7);
        assert_eq!(fired[0].deadline.0, first);
        let next = w.next_deadline().ok_or(TimerError::Capacity)?;
        assert_eq!(next.0, first + 8 * period);
        assert!(next.0 > now);
        assert_eq!(w.poll(Nanos(now), &mut fired, 8), 0);
        Ok(())
    }

    #[test]
    fn aligned_deadlines_are_epoch_multiples_in_live_and_sim() -> Result<(), TimerError> {
        let period = 1_000_000_000;
        let mid = 1_700_000_000_500_000_000;
        assert_eq!(aligned_deadline(mid, period)?, mid + 500_000_000);
        assert_eq!(
            aligned_deadline(mid + 500_000_000, period)?,
            mid + 500_000_000
        );
        let mut live = tiny()?;
        let mut sim = tiny()?;
        let mut fired = Vec::new();
        live.schedule_aligned(Nanos(mid), period, 1)?;
        sim.schedule_aligned(Nanos(mid), period, 1)?;
        let due = mid + 500_000_000;
        assert_eq!(live.poll(Nanos(due), &mut fired, 4), 1);
        assert_eq!(fired[0].deadline.0 % period, 0);
        assert_eq!(sim.poll(Nanos(due), &mut fired, 4), 1);
        assert_eq!(fired[0].deadline.0, due);
        Ok(())
    }

    #[test]
    fn a_period_below_the_tick_coalesces_to_one_firing() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let first = 10_000;
        w.schedule_repeating(Nanos(first), 100, 3)?;
        let now = first + 5_000;
        assert_eq!(w.poll(Nanos(now), &mut fired, 8), 1);
        assert_eq!(fired[0].missed, 50);
        assert_eq!(fired[0].deadline.0, first);
        let next = w.next_deadline().ok_or(TimerError::Capacity)?;
        assert_eq!(next.0, first + 51 * 100);
        assert_eq!(w.poll(Nanos(now), &mut fired, 8), 0);
        assert_eq!(w.poll(next, &mut fired, 8), 1);
        Ok(())
    }

    #[test]
    fn cancelling_a_later_timer_in_the_batch_suppresses_it() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let a = w.schedule(Nanos(30_000), 1)?;
        let b = w.schedule(Nanos(30_000), 2)?;
        assert_eq!(w.poll(Nanos(30_000), &mut fired, 8), 2);
        assert!(!w.cancel(b));
        assert!(w.is_suppressed(b));
        assert!(!w.is_suppressed(a));
        let seen: Vec<u64> = fired
            .iter()
            .filter(|item| !w.is_suppressed(item.id))
            .map(|item| item.token)
            .collect();
        assert_eq!(seen, vec![1]);
        Ok(())
    }

    #[test]
    fn cancelling_a_rearmed_repeating_timer_in_the_batch_suppresses_it() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let a = w.schedule_repeating(Nanos(40_000), 1_000, 1)?;
        let b = w.schedule_repeating(Nanos(40_000), 1_000, 2)?;
        assert_eq!(w.poll(Nanos(40_000), &mut fired, 8), 2);
        assert!(w.cancel(b));
        assert!(w.is_suppressed(b));
        assert_eq!(w.poll(Nanos(41_000), &mut fired, 8), 1);
        assert_eq!(fired[0].id, a);
        Ok(())
    }

    #[test]
    fn a_deadline_many_revolutions_out_fires_once() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let revolution = 1024 * 8;
        let deadline = 1_000 * revolution + 2_048;
        w.schedule(Nanos(deadline), 1)?;
        for step in 1..20 {
            assert_eq!(w.poll(Nanos(step * revolution), &mut fired, 8), 0);
        }
        assert_eq!(w.poll(Nanos(deadline - 1), &mut fired, 8), 0);
        assert_eq!(w.poll(Nanos(deadline), &mut fired, 8), 1);
        assert_eq!(w.poll(Nanos(deadline + revolution), &mut fired, 8), 0);
        Ok(())
    }

    #[test]
    fn a_jump_visits_occupied_spokes_once() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        w.schedule(Nanos(1 << 10), 1)?;
        w.schedule(Nanos(3 << 10), 2)?;
        let now = 1_000_000_000_i64 << 10;
        // Past `now`, on its own spoke, so the jump must count it without firing it.
        w.schedule(Nanos(now + (5 << 10)), 3)?;
        assert_eq!(w.poll(Nanos(now), &mut fired, 8), 2);
        assert_eq!(w.spokes_visited(), 3);
        assert!(w.spokes_visited() <= 8);
        Ok(())
    }

    #[test]
    fn next_deadline_matches_a_scan_of_the_slots() -> Result<(), TimerError> {
        let mut w = tiny()?;
        w.schedule(Nanos(90_000), 1)?;
        let early = w.schedule(Nanos(40_000), 2)?;
        w.schedule(Nanos(70_000), 3)?;
        assert_eq!(w.next_deadline(), Some(Nanos(40_000)));
        assert!(w.cancel(early));
        assert_eq!(w.next_deadline(), Some(Nanos(70_000)));
        assert_eq!(w.next_deadline(), fired_min_remaining(&w));
        Ok(())
    }

    fn fired_min_remaining(w: &TimerWheel) -> Option<Nanos> {
        let mut min = None;
        for spoke in 0..w.spokes {
            let base = spoke * w.stride;
            for slot in 0..w.stride {
                let deadline = w.slab.deadline[base + slot];
                if deadline != NIL {
                    min = Some(min.map_or(deadline, |m: i64| m.min(deadline)));
                }
            }
        }
        min.map(Nanos)
    }

    #[test]
    fn limit_leaves_the_rest_for_the_next_poll() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        for token in 0_u32..5 {
            w.schedule(Nanos(10_000 + i64::from(token)), u64::from(token))?;
        }
        assert_eq!(w.poll(Nanos(20_000), &mut fired, 2), 2);
        assert_eq!(fired[0].token, 0);
        assert_eq!(fired[1].token, 1);
        assert_eq!(w.poll(Nanos(20_000), &mut fired, 8), 3);
        assert_eq!(
            fired.iter().map(|item| item.token).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
        Ok(())
    }

    #[test]
    fn an_idle_poll_does_not_visit_spokes() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        w.schedule(Nanos(1_000_000), 1)?;
        assert_eq!(w.poll(Nanos(10), &mut fired, 8), 0);
        assert_eq!(w.spokes_visited(), 0);
        Ok(())
    }

    #[test]
    fn overflow_saturates_and_does_not_free_the_slot() -> Result<(), TimerError> {
        let mut w = tiny()?;
        let mut fired = Vec::new();
        let deadline = i64::MAX - 10;
        let id = w.schedule_repeating(Nanos(deadline), 100, 1)?;
        assert_eq!(w.poll(Nanos(deadline), &mut fired, 4), 1);
        assert_eq!(fired[0].missed, 0);
        assert!(w.cancel(id));
        Ok(())
    }

    #[test]
    fn a_full_spoke_grows_and_keeps_every_timer() -> Result<(), TimerError> {
        let mut w = wheel(1024, 8, 1)?;
        let mut fired = Vec::new();
        let mut ids = Vec::new();
        for token in 0..3 {
            ids.push(w.schedule(Nanos(2_048), token)?);
        }
        assert_eq!(w.poll(Nanos(2_048), &mut fired, 8), 3);
        assert_eq!(fired.len(), ids.len());
        Ok(())
    }

    #[test]
    fn filling_every_record_grows_and_keeps_every_timer() -> Result<(), TimerError> {
        // 8 records. 32 timers double the slab twice and leave none free.
        // Each deadline is shared by two timers, and every fourth repeats.
        let mut w = wheel(1024, 8, 1)?;
        let mut fired = Vec::new();
        let mut want = Vec::new();
        for token in 0..32_u64 {
            let deadline = 2_048 + 500 * i64::try_from(token * 7 % 16).unwrap_or(0);
            let id = if token % 4 == 0 {
                w.schedule_repeating(Nanos(deadline), 100_000, token)?
            } else {
                w.schedule(Nanos(deadline), token)?
            };
            want.push((deadline, id.seq(), id));
        }
        assert_eq!(
            (w.stride, w.free.len()),
            (4, 0),
            "two doublings, every record in use"
        );
        want.sort_unstable();
        assert_eq!(w.poll(Nanos(20_000), &mut fired, 64), 32);
        let order: Vec<(i64, u32, TimerId)> = fired
            .iter()
            .map(|f| (f.deadline.0, f.id.seq(), f.id))
            .collect();
        assert_eq!(order, want, "(deadline, seq) order across the growth");
        // Each re-arm relinked its own record: no growth, none dropped.
        assert_eq!((w.stride, w.len()), (4, 8));
        assert_eq!(w.poll(Nanos(120_000), &mut fired, 64), 8);
        Ok(())
    }

    #[test]
    fn a_poll_at_the_end_of_time_returns_for_every_tick_size() -> Result<(), TimerError> {
        let mut fired = Vec::new();
        for shift in 0..63 {
            let mut w = wheel(1 << shift, 8, 4)?;
            // An empty wheel's cursor is `i64::MAX`, so this poll is not idle.
            assert_eq!(w.poll(Nanos(i64::MAX), &mut fired, 8), 0, "tick 2^{shift}");
            let last = w.schedule(Nanos(i64::MAX), 1)?;
            let repeating = w.schedule_repeating(Nanos(i64::MAX - 1), 1, 2)?;
            assert_eq!(w.poll(Nanos(i64::MAX), &mut fired, 8), 2, "tick 2^{shift}");
            let order: Vec<(TimerId, i64)> = fired.iter().map(|f| (f.id, f.deadline.0)).collect();
            assert_eq!(order, [(repeating, i64::MAX - 1), (last, i64::MAX)]);
            // The re-arm saturates at the end of time and stays live.
            assert_eq!(w.next_deadline(), Some(Nanos(i64::MAX)), "tick 2^{shift}");
            assert_eq!(w.poll(Nanos(i64::MAX), &mut fired, 8), 1, "tick 2^{shift}");
            assert!(w.cancel(repeating));
            assert!(w.is_empty());
        }
        Ok(())
    }

    #[test]
    fn sim_events_precede_timers_at_the_same_timestamp() {
        let earlier_timer = SimKey::timer(4, 1);
        let later_message = SimKey::event(5, 0, 1, 0);
        assert!(earlier_timer < later_message);
        let message = SimKey::event(5, 1, 9, 3);
        let timer = SimKey::timer(5, 1);
        assert!(message < timer);
        let far_feed = SimKey::event(5, 2, 0, 0);
        let near_feed = SimKey::event(5, 0, 100, 100);
        assert!(near_feed < far_feed);
    }

    #[test]
    fn nil_and_a_non_positive_period_are_rejected() -> Result<(), TimerError> {
        let mut w = tiny()?;
        assert_eq!(w.schedule(Nanos(NIL), 1), Err(TimerError::NilDeadline));
        assert_eq!(
            w.schedule_repeating(Nanos(10), 0, 1),
            Err(TimerError::Period)
        );
        assert_eq!(aligned_deadline(10, 0), Err(TimerError::Period));
        assert!(
            TimerWheel::new(Settings {
                tick_ns: 1000,
                ticks_per_wheel: 8,
                timers_per_spoke: 1,
            })
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn random_ops_match_a_sorted_model() -> Result<(), TimerError> {
        run_script(0x1234_5678_9abc, 2_000)
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, .. ProptestConfig::default() })]
        #[test]
        fn property_matches_a_sorted_model(seed in 1u64..u64::MAX) {
            let result = run_script(seed, 400);
            prop_assert!(result.is_ok(), "{result:?}");
        }
    }

    fn run_script(seed: u64, ops: usize) -> Result<(), TimerError> {
        let mut w = wheel(1024, 32, 4)?;
        let mut fired = Vec::new();
        let mut model: BTreeMap<(i64, u32), Model> = BTreeMap::new();
        let mut by_id: BTreeMap<TimerId, (i64, u32)> = BTreeMap::new();
        let mut rng = seed | 1;
        let base = 1_000_000_000_000_i64;
        for _ in 0..ops {
            rng = xorshift(rng);
            let roll = rng % 100;
            if roll < 45 {
                rng = xorshift(rng);
                let repeating = rng.is_multiple_of(2);
                rng = xorshift(rng);
                let deadline = base + i64::try_from(rng % 50_000).unwrap_or(0);
                rng = xorshift(rng);
                let token = rng;
                rng = xorshift(rng);
                let period = 500 + i64::try_from(rng % 4_000).unwrap_or(0);
                let id = if repeating {
                    w.schedule_repeating(Nanos(deadline), period, token)
                } else {
                    w.schedule(Nanos(deadline), token)
                };
                let Ok(id) = id else {
                    return Err(TimerError::Capacity);
                };
                let period = if repeating { period } else { 0 };
                model.insert((deadline, id.seq()), Model { id, token, period });
                by_id.insert(id, (deadline, id.seq()));
            } else if roll < 70 {
                if let Some(id) = by_id.keys().next().copied() {
                    let lived = by_id.remove(&id);
                    let cancelled = w.cancel(id);
                    assert_eq!(cancelled, lived.is_some());
                    if let Some(key) = lived {
                        model.remove(&key);
                    }
                }
            } else {
                rng = xorshift(rng);
                let jump: i64 = if rng.is_multiple_of(5) { 80_000 } else { 1_000 };
                rng = xorshift(rng);
                let now = base + i64::try_from(rng % 50_000).unwrap_or(0) + jump;
                rng = xorshift(rng);
                let limit = 1 + usize::try_from(rng % 4).unwrap_or(0);
                let n = w.poll(Nanos(now), &mut fired, limit);
                let mut expected = Vec::new();
                let due: Vec<(i64, u32)> = model
                    .range(..=(now, u32::MAX))
                    .map(|(key, _)| *key)
                    .take(limit)
                    .collect();
                for key in due {
                    let Some(item) = model.remove(&key) else {
                        return Err(TimerError::Capacity);
                    };
                    by_id.remove(&item.id);
                    let (missed, next) = if item.period == 0 {
                        (0, key.0)
                    } else {
                        reference_grid(key.0, item.period, now)
                    };
                    expected.push((item.id, item.token, key.0, missed, item.period));
                    if item.period > 0 {
                        let id = item.id;
                        let seq = id.seq();
                        model.insert((next, seq), item);
                        by_id.insert(id, (next, seq));
                    }
                }
                assert_eq!(n, expected.len(), "seed {seed}");
                for (got, exp) in fired.iter().zip(expected.iter()) {
                    assert_eq!(
                        (got.id, got.token, got.deadline.0, got.missed, got.period),
                        *exp,
                        "seed {seed}"
                    );
                }
            }
            let want = model.keys().next().map(|(deadline, _)| *deadline);
            let got = w.next_deadline().map(|t| t.0);
            assert_eq!(got, want, "seed {seed}");
        }
        Ok(())
    }

    struct Model {
        id: TimerId,
        token: u64,
        period: i64,
    }

    fn reference_grid(deadline: i64, period: i64, now: i64) -> (u64, i64) {
        let missed = if now > deadline {
            let late = now.saturating_sub(deadline).cast_unsigned();
            late.checked_div(period.cast_unsigned()).unwrap_or(0)
        } else {
            0
        };
        let steps = missed.saturating_add(1);
        let next = i64::try_from(steps)
            .ok()
            .and_then(|steps| steps.checked_mul(period))
            .and_then(|delta| deadline.checked_add(delta))
            .filter(|sum| *sum != i64::MIN)
            .unwrap_or(i64::MAX);
        (missed, next)
    }

    fn xorshift(mut x: u64) -> u64 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    }
}

/// What the public behaviour does not show: the spoke lists, the free list,
/// the idle cursor and the columns' spare room.
#[cfg(test)]
mod invariants {
    use super::*;

    fn check(w: &TimerWheel) {
        let recs = &w.slab.deadline.0;
        let slots = recs.len();
        assert_eq!(slots, w.spokes * w.stride);
        assert_eq!((w.head.len(), w.periods.len()), (w.spokes, slots));
        let mut linked = vec![false; slots];
        let mut min = i64::MAX;
        for (spoke, &first) in w.head.iter().enumerate() {
            let (mut prev, mut h) = (END, first);
            let mut earliest = i64::MAX;
            while h != END {
                let rec = recs[ix(h)];
                assert!(!linked[ix(h)], "{h} is linked twice");
                linked[ix(h)] = true;
                assert_eq!(rec.prev, prev, "{h}'s back link");
                assert_ne!(rec.seq, 0, "{h} is linked but free");
                assert_eq!(w.spoke_of(rec.deadline), spoke, "{h} is on another spoke");
                assert!(
                    rec.repeating == 0 || (rec.repeating == 1 && w.periods[ix(h)] > 0),
                    "{h} repeats without a period"
                );
                min = min.min(rec.deadline);
                earliest = earliest.min(rec.deadline);
                (prev, h) = (h, rec.next);
            }
            if first != END && recs[ix(first)].stale == 0 {
                assert_eq!(
                    recs[ix(first)].deadline,
                    earliest,
                    "spoke {spoke}'s head is not stale and not its earliest"
                );
            }
        }
        let mut free = vec![false; slots];
        for &h in &w.free {
            assert!(!free[ix(h)], "{h} is free twice");
            free[ix(h)] = true;
        }
        for (h, rec) in recs.iter().enumerate() {
            assert_ne!(linked[h], free[h], "{h} must be either linked or free");
            if free[h] {
                assert_eq!((rec.deadline, rec.seq), (NIL, 0), "{h} is free but set");
            }
        }
        assert_eq!(w.len(), linked.iter().filter(|&&l| l).count());
        if w.is_empty() {
            assert_eq!((w.next_tick_start, w.floor_tick), (i64::MAX, i64::MAX));
        } else {
            assert!(
                w.next_tick_start == SCAN || w.next_tick_start == min,
                "the cursor is neither the earliest deadline nor SCAN"
            );
            assert!(
                w.floor_tick <= min >> w.tick_shift,
                "the floor is past the earliest tick"
            );
        }
        // A release, or a batch of every timer, never allocates.
        let room = [
            w.free.capacity(),
            w.due.capacity(),
            w.inflight.capacity(),
            w.dead.capacity(),
        ];
        assert!(room.iter().all(|&cap| cap >= slots), "{room:?} < {slots}");
    }

    #[test]
    fn the_sequence_skips_zero_when_it_wraps() -> Result<(), TimerError> {
        let mut w = TimerWheel::new(Settings::default())?;
        w.next_seq = u32::MAX;
        let last = w.schedule(Nanos(1_000), 1)?;
        let wrapped = w.schedule(Nanos(2_000), 2)?;
        assert_eq!((last.seq(), wrapped.seq()), (u32::MAX, 1));
        check(&w);
        Ok(())
    }

    #[test]
    fn a_search_moves_a_stale_spokes_earliest_to_its_head() -> Result<(), TimerError> {
        let mut w = TimerWheel::new(Settings {
            tick_ns: 1024,
            ticks_per_wheel: 8,
            timers_per_spoke: 4,
        })?;
        let first = w.schedule(Nanos(1_000), 1)?;
        for deadline in [1_003, 1_001, 1_002] {
            w.schedule(Nanos(deadline), 2)?;
        }
        assert!(w.cancel(first));
        assert_eq!(w.next_deadline(), Some(Nanos(1_001)));
        let head = w.slab.deadline.0[ix(w.head[w.spoke_of(1_001)])];
        assert_eq!((head.deadline, head.stale), (1_001, 0));
        check(&w);
        Ok(())
    }

    #[test]
    fn lists_free_list_and_cursor_hold_through_growth() -> Result<(), TimerError> {
        let tick = 1024;
        let mut w = TimerWheel::new(Settings {
            tick_ns: tick,
            ticks_per_wheel: 8,
            timers_per_spoke: 1,
        })?;
        let mut fired = Vec::new();
        let mut live: Vec<TimerId> = Vec::new();
        let mut now = 1_000_000 * tick;
        let mut rng = 0x2545_f491_4f6c_dd1d_u64;
        let mut most = 0;
        for step in 0..20_000_u32 {
            rng = xorshift(rng);
            // Up to four revolutions out, so a spoke mixes revolutions.
            let reach = i64::try_from(rng >> 40).unwrap_or(0) % (32 * tick);
            let pick = usize::try_from(rng >> 32).unwrap_or(0) % live.len().max(1);
            match rng % 16 {
                0..=6 => live.push(if rng.is_multiple_of(3) {
                    w.schedule_repeating(Nanos(now + reach), 1 + reach, rng)?
                } else {
                    w.schedule(Nanos(now + reach), rng)?
                }),
                7..=9 if !live.is_empty() => {
                    assert!(w.cancel(live.swap_remove(pick)));
                }
                10 if !live.is_empty() => {
                    // A journalled firing, as the simulation replays one.
                    let rec = w.slab.deadline.0[ix(live[pick].handle())];
                    let one = w.journal_fire(rec.token, Nanos(rec.deadline), Nanos(now), 0);
                    let one = one.ok_or(TimerError::Capacity)?;
                    if one.period == 0 {
                        live.swap_remove(pick);
                    }
                }
                _ => {
                    now += if rng.is_multiple_of(5) {
                        3 * reach
                    } else {
                        reach / 16
                    };
                    let limit = 1 + usize::try_from(rng >> 59).unwrap_or(0);
                    if w.poll(Nanos(now), &mut fired, limit) > 0 {
                        live.retain(|id| !fired.iter().any(|f| f.id == *id && f.period == 0));
                    }
                }
            }
            check(&w);
            assert_eq!(w.len(), live.len());
            if step % 3 == 0 {
                let next = w.next_deadline();
                check(&w);
                assert_eq!(next.is_none(), live.is_empty());
            }
            most = most.max(w.len());
        }
        assert!(
            w.stride >= 8,
            "{most} live timers doubled the slab only to {}",
            w.stride
        );
        Ok(())
    }

    const fn xorshift(mut x: u64) -> u64 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    }
}
