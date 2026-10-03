//! Wheel against a pre-reserved lazy-delete binary heap.
//!
//! The heap is the structure this wheel has to beat, held to the wheel's
//! contract so both arms do the same logical work: a token and period per
//! timer, ids that stay safe when a handle is reused, a repeating timer
//! re-pushed under its id, and the same fired record per expiry. Cancel only
//! clears the record; the tombstone is popped when it reaches the head, on
//! the clock. Nothing on a timed path allocates.
//!
//! Each scenario prints `PERCENTILES` and a `GATE` line, and the process exits
//! non-zero when the wheel's p50 is slower in any of them. Criterion then
//! writes `estimates.json`. `--gate-only` skips Criterion.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use criterion::{Criterion, Throughput};
use ergon_runtime::clock::Nanos;
use ergon_runtime::timer::{Fired, Settings, TimerId, TimerWheel};

const N: usize = 1_000;
const N_I64: i64 = 1_000;
const SAMPLES: usize = 1_024;
const WARMUP: usize = 32;
const IDLE_REPS: usize = 20_000;
const FIRE_REPS: usize = 1_000;
const BASE: i64 = 1_700_000_000_000_000_000;
const TICK: i64 = 1024;
/// Far enough out that the idle and fire scenarios' population never fires.
const FAR: i64 = 60_000_000_000;
/// Past every deadline the cancel scenario schedules.
const PAST_ALL: i64 = BASE + 2 * TICK * N_I64;
/// A churn timer is due `2N` ticks after it is scheduled and cancelled after
/// `N`, as an order timeout cancelled by its fill.
const HORIZON: i64 = 2 * TICK * N_I64;

#[derive(Clone, Copy, Default)]
struct HeapRec {
    token: u64,
    period: i64,
    /// Zero when the handle is free.
    seq: u32,
}

/// What the heap reports per expiry: the same fields as the wheel's [`Fired`].
#[derive(Clone, Copy)]
struct HeapFired {
    id: u64,
    token: u64,
    deadline: i64,
    missed: u64,
    period: i64,
}

struct LazyHeap {
    heap: BinaryHeap<Reverse<(i64, u32, u32)>>,
    recs: Vec<HeapRec>,
    free: Vec<u32>,
    next_seq: u32,
}

impl LazyHeap {
    fn new(timers: usize) -> Self {
        let top = u32::try_from(timers).unwrap_or(u32::MAX);
        Self {
            // Room for a tombstone per live timer.
            heap: BinaryHeap::with_capacity(timers * 2),
            recs: vec![HeapRec::default(); timers],
            free: (0..top).rev().collect(),
            next_seq: 1,
        }
    }

    #[inline]
    fn schedule(&mut self, deadline: i64, token: u64, period: i64) -> u64 {
        let Some(handle) = self.free.pop() else {
            eprintln!("lazy heap out of handles");
            std::process::exit(2);
        };
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1).max(1);
        self.recs[handle as usize] = HeapRec { token, period, seq };
        self.heap.push(Reverse((deadline, seq, handle)));
        (u64::from(seq) << 32) | u64::from(handle)
    }

    #[inline]
    fn cancel(&mut self, id: u64) -> bool {
        let handle = u32::try_from(id & 0xffff_ffff).unwrap_or(u32::MAX);
        let seq = u32::try_from(id >> 32).unwrap_or(0);
        match self.recs.get_mut(handle as usize) {
            Some(rec) if seq != 0 && rec.seq == seq => {
                rec.seq = 0;
                self.free.push(handle);
                true
            }
            _ => false,
        }
    }

    /// Pop what is due at `now` in `(deadline, seq)` order, tombstones
    /// included, and report up to `limit` live expiries.
    #[inline]
    fn poll(&mut self, now: i64, fired: &mut Vec<HeapFired>, limit: usize) -> usize {
        match self.heap.peek() {
            Some(Reverse((deadline, _, _))) if *deadline <= now => {}
            _ => return 0,
        }
        fired.clear();
        while fired.len() < limit {
            let Some(&Reverse((deadline, seq, handle))) = self.heap.peek() else {
                break;
            };
            if deadline > now {
                break;
            }
            self.heap.pop();
            let rec = self.recs[handle as usize];
            if rec.seq != seq {
                continue;
            }
            let missed = if rec.period == 0 {
                self.recs[handle as usize].seq = 0;
                self.free.push(handle);
                0
            } else {
                let late = u64::try_from(now - deadline).unwrap_or(0);
                let missed = late / u64::try_from(rec.period).unwrap_or(1);
                let steps = i64::try_from(missed + 1).unwrap_or(i64::MAX);
                self.heap
                    .push(Reverse((deadline + steps * rec.period, seq, handle)));
                missed
            };
            fired.push(HeapFired {
                id: (u64::from(seq) << 32) | u64::from(handle),
                token: rec.token,
                deadline,
                missed,
                period: rec.period,
            });
        }
        fired.len()
    }

    fn reset(&mut self) {
        let top = u32::try_from(self.recs.len()).unwrap_or(u32::MAX);
        self.heap.clear();
        self.recs.fill(HeapRec::default());
        self.free.clear();
        self.free.extend((0..top).rev());
    }
}

fn wheel() -> TimerWheel {
    TimerWheel::new(Settings::default()).unwrap_or_else(|err| {
        eprintln!("timer wheel: {err}");
        std::process::exit(2);
    })
}

#[inline]
fn schedule(wheel: &mut TimerWheel, deadline: i64, token: u64) -> TimerId {
    wheel
        .schedule(Nanos(deadline), token)
        .unwrap_or_else(|err| {
            eprintln!("schedule: {err}");
            std::process::exit(2);
        })
}

fn at(i: usize) -> i64 {
    BASE + TICK * i64::try_from(i).unwrap_or(0)
}

/// One measured operation batch, run on both structures. `wheel` and `heap`
/// are timed; `reset_*` are not.
trait Scenario {
    const NAME: &'static str;
    /// Operations per timed call, for throughput.
    const OPS: usize;
    fn wheel(&mut self) -> usize;
    fn heap(&mut self) -> usize;
    fn reset_wheel(&mut self) {}
    fn reset_heap(&mut self) {}
}

/// Ascending deadlines are the heap's best case: each push sifts nowhere.
/// Mixed is the same deadlines in a fixed scrambled order, as live code mixes
/// an order timeout, a stale-book timer and housekeeping periods.
struct Schedule<const MIXED: bool> {
    wheel: TimerWheel,
    ids: Vec<TimerId>,
    heap: LazyHeap,
    heap_ids: Vec<u64>,
}

impl<const MIXED: bool> Schedule<MIXED> {
    fn new() -> Self {
        Self {
            wheel: wheel(),
            ids: Vec::with_capacity(N),
            heap: LazyHeap::new(N),
            heap_ids: Vec::with_capacity(N),
        }
    }

    fn deadline(i: usize) -> i64 {
        if MIXED { at((i * 7_919) % N) } else { at(i) }
    }
}

impl<const MIXED: bool> Scenario for Schedule<MIXED> {
    const NAME: &'static str = if MIXED { "schedule_mixed" } else { "schedule" };
    const OPS: usize = N;

    fn wheel(&mut self) -> usize {
        for i in 0..N {
            let id = schedule(&mut self.wheel, black_box(Self::deadline(i)), i as u64);
            self.ids.push(id);
        }
        self.ids.len()
    }

    fn heap(&mut self) -> usize {
        for i in 0..N {
            let id = self
                .heap
                .schedule(black_box(Self::deadline(i)), i as u64, 0);
            self.heap_ids.push(id);
        }
        self.heap_ids.len()
    }

    fn reset_wheel(&mut self) {
        for id in self.ids.drain(..) {
            self.wheel.cancel(id);
        }
    }

    fn reset_heap(&mut self) {
        self.heap_ids.clear();
        self.heap.reset();
    }
}

/// Cancel every timer, then poll past all of them: the heap pops its
/// tombstones, the wheel has nothing left, so both end empty.
struct Cancel {
    wheel: TimerWheel,
    ids: Vec<TimerId>,
    fired: Vec<Fired>,
    heap: LazyHeap,
    heap_ids: Vec<u64>,
    heap_fired: Vec<HeapFired>,
}

impl Cancel {
    fn new() -> Self {
        let mut s = Self {
            wheel: wheel(),
            ids: Vec::with_capacity(N),
            fired: Vec::with_capacity(N),
            heap: LazyHeap::new(N),
            heap_ids: Vec::with_capacity(N),
            heap_fired: Vec::with_capacity(N),
        };
        s.reset_wheel();
        s.reset_heap();
        s
    }
}

impl Scenario for Cancel {
    const NAME: &'static str = "cancel";
    const OPS: usize = N;

    fn wheel(&mut self) -> usize {
        for id in &self.ids {
            black_box(self.wheel.cancel(black_box(*id)));
        }
        self.wheel
            .poll(black_box(Nanos(PAST_ALL)), &mut self.fired, N)
    }

    fn heap(&mut self) -> usize {
        for id in &self.heap_ids {
            black_box(self.heap.cancel(black_box(*id)));
        }
        self.heap.poll(black_box(PAST_ALL), &mut self.heap_fired, N)
    }

    fn reset_wheel(&mut self) {
        self.ids.clear();
        for i in 0..N {
            self.ids.push(schedule(&mut self.wheel, at(i), i as u64));
        }
    }

    fn reset_heap(&mut self) {
        self.heap.reset();
        self.heap_ids.clear();
        for i in 0..N {
            self.heap_ids.push(self.heap.schedule(at(i), i as u64, 0));
        }
    }
}

/// `N` timers wait a minute out; every poll finds nothing due.
struct Idle {
    wheel: TimerWheel,
    fired: Vec<Fired>,
    heap: LazyHeap,
    heap_fired: Vec<HeapFired>,
}

impl Idle {
    fn new() -> Self {
        let mut s = Self {
            wheel: wheel(),
            fired: Vec::with_capacity(8),
            heap: LazyHeap::new(N),
            heap_fired: Vec::with_capacity(8),
        };
        for i in 0..N {
            schedule(&mut s.wheel, at(i) + FAR, i as u64);
            s.heap.schedule(at(i) + FAR, i as u64, 0);
        }
        s
    }
}

impl Scenario for Idle {
    const NAME: &'static str = "idle_poll";
    const OPS: usize = IDLE_REPS;

    fn wheel(&mut self) -> usize {
        let mut n = 0;
        for _ in 0..IDLE_REPS {
            n += self.wheel.poll(black_box(Nanos(BASE)), &mut self.fired, 8);
        }
        n
    }

    fn heap(&mut self) -> usize {
        let mut n = 0;
        for _ in 0..IDLE_REPS {
            n += self.heap.poll(black_box(BASE), &mut self.heap_fired, 8);
        }
        n
    }
}

/// One repeating timer due per poll while `N` far timers wait: the fire path,
/// including finding the next deadline, under a realistic population.
struct Fire {
    wheel: TimerWheel,
    fired: Vec<Fired>,
    now_wheel: i64,
    heap: LazyHeap,
    heap_fired: Vec<HeapFired>,
    now_heap: i64,
}

impl Fire {
    fn new() -> Self {
        let mut s = Self {
            wheel: wheel(),
            fired: Vec::with_capacity(8),
            now_wheel: BASE,
            heap: LazyHeap::new(N + 1),
            heap_fired: Vec::with_capacity(8),
            now_heap: BASE,
        };
        for i in 0..N {
            schedule(&mut s.wheel, at(i) + FAR, i as u64);
            s.heap.schedule(at(i) + FAR, i as u64, 0);
        }
        if s.wheel
            .schedule_repeating(Nanos(BASE), TICK, u64::MAX)
            .is_err()
        {
            eprintln!("schedule_repeating failed");
            std::process::exit(2);
        }
        s.heap.schedule(BASE, u64::MAX, TICK);
        s
    }
}

impl Scenario for Fire {
    const NAME: &'static str = "fire";
    const OPS: usize = FIRE_REPS;

    fn wheel(&mut self) -> usize {
        let mut n = 0;
        for _ in 0..FIRE_REPS {
            n += self
                .wheel
                .poll(black_box(Nanos(self.now_wheel)), &mut self.fired, 8);
            self.now_wheel += TICK;
        }
        n
    }

    fn heap(&mut self) -> usize {
        let mut n = 0;
        for _ in 0..FIRE_REPS {
            n += self
                .heap
                .poll(black_box(self.now_heap), &mut self.heap_fired, 8);
            self.now_heap += TICK;
        }
        n
    }
}

/// Time advances a tick per step; each step cancels the oldest timer,
/// schedules one `HORIZON` out and polls. Nothing fires, and the heap's
/// tombstones reach its head and are popped on the clock.
struct Churn {
    wheel: TimerWheel,
    ids: Vec<TimerId>,
    fired: Vec<Fired>,
    now_wheel: i64,
    next_wheel: usize,
    heap: LazyHeap,
    heap_ids: Vec<u64>,
    heap_fired: Vec<HeapFired>,
    now_heap: i64,
    next_heap: usize,
}

impl Churn {
    fn new() -> Self {
        let mut s = Self {
            wheel: wheel(),
            ids: Vec::with_capacity(N),
            fired: Vec::with_capacity(8),
            now_wheel: at(N),
            next_wheel: 0,
            // Live timers plus the tombstones not yet surfaced.
            heap: LazyHeap::new(2 * N),
            heap_ids: Vec::with_capacity(N),
            heap_fired: Vec::with_capacity(8),
            now_heap: at(N),
            next_heap: 0,
        };
        for i in 0..N {
            s.ids.push(schedule(&mut s.wheel, at(i) + HORIZON, 0));
            s.heap_ids.push(s.heap.schedule(at(i) + HORIZON, 0, 0));
        }
        s
    }
}

impl Scenario for Churn {
    const NAME: &'static str = "churn";
    const OPS: usize = N;

    fn wheel(&mut self) -> usize {
        let mut n = 0;
        for _ in 0..N {
            self.now_wheel += TICK;
            let k = self.next_wheel;
            self.wheel.cancel(self.ids[k]);
            self.ids[k] = schedule(&mut self.wheel, black_box(self.now_wheel + HORIZON), 0);
            n += self
                .wheel
                .poll(black_box(Nanos(self.now_wheel)), &mut self.fired, 8);
            self.next_wheel = (k + 1) % N;
        }
        n
    }

    fn heap(&mut self) -> usize {
        let mut n = 0;
        for _ in 0..N {
            self.now_heap += TICK;
            let k = self.next_heap;
            self.heap.cancel(self.heap_ids[k]);
            self.heap_ids[k] = self.heap.schedule(black_box(self.now_heap + HORIZON), 0, 0);
            n += self
                .heap
                .poll(black_box(self.now_heap), &mut self.heap_fired, 8);
            self.next_heap = (k + 1) % N;
        }
        n
    }
}

#[inline(never)]
fn time_ns(body: impl FnOnce() -> usize) -> u128 {
    let start = Instant::now();
    black_box(body());
    start.elapsed().as_nanos()
}

fn percentile(sorted: &[u128], numer: usize, denom: usize) -> u128 {
    let last = sorted.len() - 1;
    sorted[(last * numer).div_ceil(denom).min(last)]
}

fn report(op: &str, arm: &str, samples: &mut [u128]) -> u128 {
    samples.sort_unstable();
    let p50 = percentile(samples, 1, 2);
    println!(
        "PERCENTILES {op} {arm} n={} min={} p50={p50} p99={} p99.99={} max={}",
        samples.len(),
        samples[0],
        percentile(samples, 99, 100),
        percentile(samples, 9_999, 10_000),
        samples[samples.len() - 1],
    );
    p50
}

fn gate<S: Scenario>(mut s: S) -> bool {
    let mut wheel = vec![0_u128; SAMPLES];
    let mut heap = vec![0_u128; SAMPLES];
    for _ in 0..WARMUP {
        black_box(s.wheel());
        s.reset_wheel();
        black_box(s.heap());
        s.reset_heap();
    }
    for sample in 0..SAMPLES {
        wheel[sample] = time_ns(|| s.wheel());
        s.reset_wheel();
        heap[sample] = time_ns(|| s.heap());
        s.reset_heap();
    }
    let wheel_p50 = report(S::NAME, "wheel", &mut wheel);
    let heap_p50 = report(S::NAME, "heap", &mut heap);
    let pass = wheel_p50 <= heap_p50;
    let status = if pass { "pass" } else { "fail" };
    println!(
        "GATE {} wheel_p50={wheel_p50} heap_p50={heap_p50} {status}",
        S::NAME
    );
    pass
}

fn measure<S: Scenario>(c: &mut Criterion, mut s: S) {
    let mut group = c.benchmark_group(S::NAME);
    group.throughput(Throughput::Elements(S::OPS as u64));
    group.bench_function("wheel", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                black_box(s.wheel());
                total += start.elapsed();
                s.reset_wheel();
            }
            total
        });
    });
    group.bench_function("heap", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                black_box(s.heap());
                total += start.elapsed();
                s.reset_heap();
            }
            total
        });
    });
    group.finish();
}

/// Correctness parity before timing: one script of schedules, repeating
/// timers, cancels and polls must fire the same expiries, in the same order,
/// on both structures.
fn parity() -> bool {
    let mut wheel = wheel();
    let mut heap = LazyHeap::new(2 * N);
    let (mut fired, mut heap_fired) = (Vec::with_capacity(N), Vec::with_capacity(N));
    let (mut ids, mut heap_ids) = (Vec::with_capacity(N), Vec::with_capacity(N));
    for i in 0..N {
        let deadline = at((i * 7_919) % N);
        let period = if i % 3 == 0 { TICK * 7 } else { 0 };
        let token = i as u64;
        let id = if period == 0 {
            wheel.schedule(Nanos(deadline), token)
        } else {
            wheel.schedule_repeating(Nanos(deadline), period, token)
        };
        let Ok(id) = id else { return false };
        ids.push(id);
        heap_ids.push(heap.schedule(deadline, token, period));
    }
    for (id, heap_id) in ids.iter().zip(&heap_ids).step_by(5) {
        if wheel.cancel(*id) != heap.cancel(*heap_id) {
            return false;
        }
    }
    let mut now = BASE;
    for _ in 0..3 * N {
        now += TICK / 2 + TICK / 3;
        let n = wheel.poll(Nanos(now), &mut fired, 4);
        if n != heap.poll(now, &mut heap_fired, 4) {
            return false;
        }
        let same = fired.iter().zip(&heap_fired).take(n).all(|(w, h)| {
            (w.token, w.deadline.0, w.missed, w.period) == (h.token, h.deadline, h.missed, h.period)
                && u64::from(w.id.seq()) == h.id >> 32
        });
        if !same {
            return false;
        }
    }
    println!("PARITY wheel and heap fired the same expiries");
    true
}

fn main() -> ExitCode {
    if !parity() {
        eprintln!("PARITY wheel and heap disagree; timings would compare different work");
        return ExitCode::FAILURE;
    }
    // An array, not `&&`: every scenario reports even after one fails.
    let pass = [
        gate(Schedule::<false>::new()),
        gate(Schedule::<true>::new()),
        gate(Cancel::new()),
        gate(Idle::new()),
        gate(Fire::new()),
        gate(Churn::new()),
    ]
    .iter()
    .all(|pass| *pass);
    if !std::env::args().any(|arg| arg == "--gate-only") {
        let mut c = Criterion::default()
            .sample_size(20)
            .warm_up_time(Duration::from_millis(250))
            .measurement_time(Duration::from_secs(1))
            .configure_from_args();
        measure(&mut c, Schedule::<false>::new());
        measure(&mut c, Schedule::<true>::new());
        measure(&mut c, Cancel::new());
        measure(&mut c, Idle::new());
        measure(&mut c, Fire::new());
        measure(&mut c, Churn::new());
        c.final_summary();
    }
    if pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
