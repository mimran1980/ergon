//! A single-level deadline wheel on UNIX-epoch nanoseconds.
//!
//! Live and sim poll the same [`TimerWheel`]. The clock that supplies `now`
//! is the only difference. Schedule and cancel are O(1) in the spoke size
//! fixed at construction. An idle [`TimerWheel::poll`] is one compare.
//! After [`TimerWheel::new`] sizes the slabs, a steady-state poll does not
//! allocate. A full spoke doubles every spoke once, on that cold path.
//!
//! [`TimerId`] is `(seq << 32) | handle`. The spoke lives beside the handle,
//! so a repeating re-arm that changes spoke keeps the same id.

use crate::clock::Nanos;

/// Free-slot sentinel. A deadline of this value is rejected.
const NIL: i64 = i64::MIN;
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
    /// Slots reserved in each spoke before the slab doubles.
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

/// Per-slot columns. A spoke scan reads only the contiguous `deadline` run.
struct Slab {
    deadline: Box<[i64]>,
    handle: Box<[u32]>,
}

/// One timer, indexed by handle. `seq == 0` marks a free handle. A
/// repeating re-arm moves the slot, never the handle, so the id survives.
/// One record keeps a schedule or cancel to a single cache line.
#[derive(Clone, Copy, Default)]
struct Rec {
    token: u64,
    period: i64,
    seq: u32,
    slot: u32,
}

/// Agrona-style deadline wheel.
///
/// The agent is not called from [`TimerWheel::poll`]. The runtime walks
/// [`Fired`] afterwards, so a callback can schedule and cancel freely.
pub struct TimerWheel {
    tick_shift: u32,
    mask: u64,
    spokes: usize,
    stride: usize,
    count: usize,
    next_seq: u32,
    next_tick_start: i64,
    /// No live deadline has a tick below this, so the next-deadline search
    /// walks forward from here instead of scanning the slab.
    floor_tick: i64,
    last_visits: u64,
    slab: Slab,
    timers: Box<[Rec]>,
    /// Free handles, lowest on top, so reuse is deterministic.
    free: Vec<u32>,
    occupied: Box<[u64]>,
    spoke_len: Box<[u32]>,
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
    /// steady-state path until a spoke doubles.
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
            tick_shift: settings.tick_ns.trailing_zeros(),
            mask: u64::from(settings.ticks_per_wheel - 1),
            spokes,
            stride,
            count: 0,
            next_seq: 1,
            next_tick_start: i64::MAX,
            floor_tick: i64::MAX,
            last_visits: 0,
            slab: Slab {
                deadline: vec![NIL; slots].into_boxed_slice(),
                handle: vec![0; slots].into_boxed_slice(),
            },
            timers: vec![Rec::default(); slots].into_boxed_slice(),
            free: (0..top).rev().collect(),
            occupied: vec![0; spokes.div_ceil(64)].into_boxed_slice(),
            spoke_len: vec![0; spokes].into_boxed_slice(),
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
    #[inline]
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
        if seq == 0 || self.timers.get(h).map(|r| r.seq) != Some(seq) {
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

    /// Earliest live deadline. `None` when the wheel is empty. Cached until
    /// a schedule, cancel or fire changes it. The sim driver polls at this
    /// time, so a timer fires at its deadline rather than up to a tick late.
    #[must_use]
    pub fn next_deadline(&mut self) -> Option<Nanos> {
        if self.count == 0 {
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
        self.count
    }

    /// Write every column once, so zero-initialised pages are faulted in
    /// now rather than by the first live timer.
    pub fn prefault(&mut self) {
        let free = self.slab.deadline.len();
        self.slab.handle.fill(0);
        self.timers.fill(Rec::default());
        self.spoke_len.fill(0);
        self.occupied.fill(0);
        self.slab.deadline.fill(NIL);
        debug_assert_eq!(self.count, 0, "prefault on a wheel with timers");
        debug_assert_eq!(free, self.free.len(), "every handle is free");
    }

    /// `true` when no timer is scheduled.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    fn insert(&mut self, deadline: i64, period: i64, token: u64) -> Result<TimerId, TimerError> {
        if deadline == NIL {
            return Err(TimerError::NilDeadline);
        }
        let spoke = self.spoke_of(deadline);
        let Some(handle) = self.free.pop() else {
            return self.insert_grown(deadline, period, token);
        };
        if ix(self.spoke_len[spoke]) == self.stride {
            self.free.push(handle);
            return self.insert_grown(deadline, period, token);
        }
        Ok(self.place(spoke, handle, deadline, period, token))
    }

    /// The slab is full for this deadline: double it, then place. Out of line
    /// so the hot insert stays a leaf the caller can inline.
    #[cold]
    #[inline(never)]
    fn insert_grown(
        &mut self,
        deadline: i64,
        period: i64,
        token: u64,
    ) -> Result<TimerId, TimerError> {
        self.grow()?;
        let handle = self.free.pop().ok_or(TimerError::Capacity)?;
        let spoke = self.spoke_of(deadline);
        Ok(self.place(spoke, handle, deadline, period, token))
    }

    #[inline]
    fn place(
        &mut self,
        spoke: usize,
        handle: u32,
        deadline: i64,
        period: i64,
        token: u64,
    ) -> TimerId {
        let seq = self.take_seq();
        let flat = self.push_slot(spoke, deadline, handle);
        self.timers[ix(handle)] = Rec {
            token,
            period,
            seq,
            slot: flat,
        };
        self.count += 1;
        self.note_scheduled(deadline);
        TimerId::pack(seq, handle)
    }

    /// Free a live handle and its slot. Returns the deadline it held.
    #[inline]
    fn release(&mut self, h: usize) -> i64 {
        let deadline = self.remove_slot(h);
        self.timers[h].seq = 0;
        // `h` came from a `u32` handle.
        self.free.push(u32::try_from(h).unwrap_or(u32::MAX));
        self.count -= 1;
        deadline
    }

    /// Append to `spoke`'s packed run. The caller has made room. Returns the
    /// flat slot.
    #[inline]
    fn push_slot(&mut self, spoke: usize, deadline: i64, handle: u32) -> u32 {
        let len = self.spoke_len[spoke];
        let flat = spoke * self.stride + ix(len);
        self.slab.deadline[flat] = deadline;
        self.slab.handle[flat] = handle;
        if len == 0 {
            self.occupied[spoke / 64] |= 1u64 << (spoke % 64);
        }
        self.spoke_len[spoke] = len + 1;
        // The slab was sized to fit `u32` indices.
        u32::try_from(flat).unwrap_or(u32::MAX)
    }

    /// Take `h` out of its spoke, moving the spoke's last timer into the hole
    /// so every spoke stays a packed run: insert is an append and a scan stops
    /// at the run's end. Returns the deadline it held.
    #[inline]
    fn remove_slot(&mut self, h: usize) -> i64 {
        let flat = ix(self.timers[h].slot);
        let deadline = self.slab.deadline[flat];
        let spoke = self.spoke_of(deadline);
        let len = self.spoke_len[spoke] - 1;
        let last = spoke * self.stride + ix(len);
        if flat != last {
            let moved = self.slab.handle[last];
            self.slab.deadline[flat] = self.slab.deadline[last];
            self.slab.handle[flat] = moved;
            self.timers[ix(moved)].slot = self.timers[h].slot;
        }
        self.slab.deadline[last] = NIL;
        self.spoke_len[spoke] = len;
        if len == 0 {
            self.occupied[spoke / 64] &= !(1u64 << (spoke % 64));
        }
        deadline
    }

    #[inline(never)]
    #[cold]
    fn poll_cold(&mut self, now: i64, fired: &mut Vec<Fired>, limit: usize) -> usize {
        self.last_visits = 0;
        if self.next_tick_start == SCAN {
            self.recompute_next();
            if self.count == 0 || now < self.next_tick_start {
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
        let rec = self.timers[h];
        let deadline = self.slab.deadline[ix(rec.slot)];
        let (token, period) = (rec.token, rec.period);
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
            token,
            deadline: Nanos(deadline),
            missed,
            period,
        });
        self.inflight.push(id);
        self.dead.push(0);
    }

    fn rearm(&mut self, h: usize, old_deadline: i64, next: i64) {
        let old_spoke = self.spoke_of(old_deadline);
        let new_spoke = self.spoke_of(next);
        if new_spoke == old_spoke {
            self.slab.deadline[ix(self.timers[h].slot)] = next;
            return;
        }
        if ix(self.spoke_len[new_spoke]) == self.stride && self.grow().is_err() {
            log::error!("timer wheel full: repeating timer {h} dropped");
            self.release(h);
            return;
        }
        let handle = self.slab.handle[ix(self.timers[h].slot)];
        self.remove_slot(h);
        self.timers[h].slot = self.push_slot(new_spoke, next, handle);
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
        while tick <= end_tick {
            let spoke = usize::try_from(tick.cast_unsigned() & self.mask).unwrap_or(0);
            if self.spoke_len[spoke] > 0 {
                self.visit_spoke(spoke, now);
            }
            tick += 1;
        }
    }

    fn visit_occupied(&mut self, now: i64) {
        for word in 0..self.occupied.len() {
            let mut bits = self.occupied[word];
            while bits != 0 {
                let spoke = word * 64 + ix(bits.trailing_zeros());
                bits &= bits - 1;
                self.visit_spoke(spoke, now);
            }
        }
    }

    fn visit_spoke(&mut self, spoke: usize, now: i64) {
        self.last_visits += 1;
        let base = spoke * self.stride;
        for flat in base..base + ix(self.spoke_len[spoke]) {
            let deadline = self.slab.deadline[flat];
            if deadline <= now {
                let handle = self.slab.handle[flat];
                self.due.push(Due {
                    handle,
                    deadline,
                    seq: self.timers[ix(handle)].seq,
                });
            }
        }
    }

    fn recompute_next(&mut self) {
        if self.count == 0 {
            self.next_tick_start = i64::MAX;
            self.floor_tick = i64::MAX;
            return;
        }
        if let Some(min) = self.first_within_revolution() {
            // The minimum is a lower bound too; without this a floor that only
            // full polls advance drifts a revolution behind and every
            // recompute falls through to the full scan.
            self.next_tick_start = min;
            self.floor_tick = min >> self.tick_shift;
            return;
        }
        // Every timer is a revolution or more past the floor: rare, cold.
        let mut min = i64::MAX;
        for word in 0..self.occupied.len() {
            let mut bits = self.occupied[word];
            while bits != 0 {
                let spoke = word * 64 + ix(bits.trailing_zeros());
                bits &= bits - 1;
                let base = spoke * self.stride;
                for &deadline in &self.slab.deadline[base..base + ix(self.spoke_len[spoke])] {
                    min = min.min(deadline);
                }
            }
        }
        self.next_tick_start = min;
        self.floor_tick = min >> self.tick_shift;
    }

    /// Walk occupied spokes forward from `floor_tick` for one revolution. The
    /// first spoke holding a deadline on its tick in this revolution holds the
    /// minimum: later spokes are later ticks, and anything off-revolution is
    /// at least a revolution later.
    fn first_within_revolution(&self) -> Option<i64> {
        let floor = self.floor_tick;
        let start = usize::try_from(floor.cast_unsigned() & self.mask).unwrap_or(0);
        let mut off = 0;
        while off < self.spokes {
            let spoke = (start + off) & (self.spokes - 1);
            let bit = spoke % 64;
            let bits = self.occupied[spoke / 64] >> bit;
            if bits == 0 {
                // Stop at the wrap so spokes before `start` are not skipped.
                off += (64 - bit).min(self.spokes - spoke);
                continue;
            }
            let skip = ix(bits.trailing_zeros());
            off += skip;
            let spoke = spoke + skip;
            let tick = floor.saturating_add(i64::try_from(off).unwrap_or(i64::MAX));
            let base = spoke * self.stride;
            let min = self.slab.deadline[base..base + ix(self.spoke_len[spoke])]
                .iter()
                .copied()
                .filter(|&d| d >> self.tick_shift == tick)
                .min();
            if min.is_some() {
                return min;
            }
            off += 1;
        }
        None
    }

    /// An empty wheel caches `i64::MAX`, so a first timer always lowers it.
    /// The floor only needs lowering when this deadline is the new minimum, or
    /// the minimum is unknown: otherwise its tick is already at or above it.
    #[inline]
    fn note_scheduled(&mut self, deadline: i64) {
        if deadline < self.next_tick_start {
            self.next_tick_start = deadline;
        } else if self.next_tick_start != SCAN {
            return;
        }
        self.floor_tick = self.floor_tick.min(deadline >> self.tick_shift);
    }

    const fn note_cancelled(&mut self, deadline: i64) {
        if self.count == 0 {
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

    #[cold]
    fn grow(&mut self) -> Result<(), TimerError> {
        let old_stride = self.stride;
        let new_stride = old_stride.checked_mul(2).ok_or(TimerError::Capacity)?;
        let slots = self
            .spokes
            .checked_mul(new_stride)
            .ok_or(TimerError::Capacity)?;
        let top = u32::try_from(slots).map_err(|_| TimerError::Capacity)?;
        let spokes = self.spokes;
        self.slab.deadline = grow_column(&self.slab.deadline, spokes, old_stride, new_stride, NIL);
        self.slab.handle = grow_column(&self.slab.handle, spokes, old_stride, new_stride, 0);
        let old = self.timers.len();
        for rec in &mut self.timers {
            if rec.seq != 0 {
                let flat = ix(rec.slot);
                let moved = (flat / old_stride) * new_stride + flat % old_stride;
                rec.slot = u32::try_from(moved).map_err(|_| TimerError::Capacity)?;
            }
        }
        self.timers = extend(&self.timers, slots);
        let first_new = u32::try_from(old).map_err(|_| TimerError::Capacity)?;
        self.free.extend((first_new..top).rev());
        self.stride = new_stride;
        self.due.reserve(slots.saturating_sub(self.due.capacity()));
        self.inflight
            .reserve(slots.saturating_sub(self.inflight.capacity()));
        self.dead
            .reserve(slots.saturating_sub(self.dead.capacity()));
        log::warn!("timer wheel doubled spoke stride from {old_stride} to {new_stride}");
        Ok(())
    }

    const fn take_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        if self.next_seq == 0 {
            self.next_seq = 1;
        }
        seq
    }

    #[inline]
    fn spoke_of(&self, deadline: i64) -> usize {
        let tick = deadline >> self.tick_shift;
        usize::try_from(tick.cast_unsigned() & self.mask).unwrap_or(0)
    }
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

fn extend<T: Copy + Default>(old: &[T], len: usize) -> Box<[T]> {
    let mut next = vec![T::default(); len];
    next[..old.len()].copy_from_slice(old);
    next.into_boxed_slice()
}

fn grow_column<T: Copy>(
    old: &[T],
    spokes: usize,
    old_stride: usize,
    new_stride: usize,
    fill: T,
) -> Box<[T]> {
    let mut next = vec![fill; spokes * new_stride];
    for spoke in 0..spokes {
        let from = spoke * old_stride;
        let to = spoke * new_stride;
        next[to..to + old_stride].copy_from_slice(&old[from..from + old_stride]);
    }
    next.into_boxed_slice()
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
