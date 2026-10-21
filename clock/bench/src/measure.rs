//! The measurements.
//!
//! A read's cost, the distribution of back-to-back deltas, backwards steps across two threads, a clock's rate and
//! drift against a reference clock, and the checks every clock passes before it is
//! timed (a clock that is wrong fails, never reports a fast number).
// Benchmark arithmetic: counts and nanoseconds far under 2^52 move between
// u64 and f64 throughout.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::sys;
use ergon_clock::advanced::{monotonic_raw_ns, realtime_ns};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// The value at quantile `q` (0 to 1) of sorted `v`, nearest rank.
#[must_use]
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let i = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

/// One read's cost: the mean over a batch of back-to-back reads, for many
/// batches; the median, p99, max and min of the batch means.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cost {
    /// Batches timed.
    pub batches: usize,
    /// Reads a batch.
    pub reads: usize,
    /// Median of the batch means, ns.
    pub median_ns: f64,
    /// p99 of the batch means, ns.
    pub p99_ns: f64,
    /// The slowest batch's mean, ns.
    pub max_ns: f64,
    /// The fastest batch's mean, ns.
    pub min_ns: f64,
}

impl Cost {
    fn from_means(mut means: Vec<f64>, reads: usize) -> Self {
        means.sort_by(f64::total_cmp);
        Self {
            batches: means.len(),
            reads,
            median_ns: quantile(&means, 0.5),
            p99_ns: quantile(&means, 0.99),
            max_ns: means.last().copied().unwrap_or(f64::NAN),
            min_ns: means.first().copied().unwrap_or(f64::NAN),
        }
    }
}

/// Times `read`: a warm-up of 100,000 reads, then `batches` batches of
/// `reads` back-to-back reads, each result through `black_box` (as
/// Criterion's `iter` does), each batch timed by `std::time::Instant`.
pub fn cost<F: FnMut() -> u64>(mut read: F, batches: usize, reads: usize) -> Cost {
    for _ in 0..100_000 {
        black_box(read());
    }
    let mut means = Vec::with_capacity(batches);
    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..reads {
            black_box(read());
        }
        means.push(start.elapsed().as_nanos() as f64 / reads as f64);
    }
    Cost::from_means(means, reads)
}

/// Times a conversion over pre-captured raw values (cycling through
/// `inputs`, each through `black_box` so none is folded at compile time).
pub fn cost_convert<F: Fn(u64) -> u64>(
    convert: F,
    inputs: &[u64],
    batches: usize,
    reads: usize,
) -> Cost {
    // a mask, not a remainder: a division would cost more than the
    // conversion (the largest power of two of the inputs are used)
    let mask = (1_usize << inputs.len().max(1).ilog2()) - 1;
    let at = |i: usize| inputs[i & mask];
    for i in 0..100_000 {
        black_box(convert(black_box(at(i))));
    }
    let mut means = Vec::with_capacity(batches);
    for _ in 0..batches {
        let start = Instant::now();
        for i in 0..reads {
            black_box(convert(black_box(at(i))));
        }
        means.push(start.elapsed().as_nanos() as f64 / reads as f64);
    }
    Cost::from_means(means, reads)
}

/// The distribution of the gap between consecutive reads on one thread.
///
/// The smallest non-zero gap is the clock's resolution (or its cost, if
/// greater); the rest is its cost plus anything that interrupts the thread.
#[derive(Clone, Debug, PartialEq)]
pub struct Deltas {
    /// Reads.
    pub reads: usize,
    /// The smallest non-zero gap, ns.
    pub min_nonzero_ns: f64,
    /// Median gap, ns.
    pub p50_ns: f64,
    /// p99 gap, ns.
    pub p99_ns: f64,
    /// p99.99 gap, ns.
    pub p9999_ns: f64,
    /// The largest gap, ns.
    pub max_ns: f64,
    /// Reads equal to the one before.
    pub zeros: usize,
    /// Reads lower than the one before.
    pub backwards: usize,
    /// The largest step back, ns.
    pub max_backwards_ns: f64,
    /// Every gap of 10 us or more: (`CLOCK_REALTIME` at its end, ns; the gap,
    /// ns), the largest 20, for attributing them (a trace, a gap spinner).
    pub gaps: Vec<(i64, f64)>,
    /// The thread's voluntary and involuntary context switches over the
    /// pass (Linux).
    pub switches: (u64, u64),
    /// Page faults over the pass (the thread's on Linux, the process's
    /// elsewhere): none where the pass paid nothing for its own memory. Off
    /// Linux the count is the whole process's (`sys::minor_faults`), so a
    /// non-zero delta cannot be laid at this pass's door — see `resident`.
    pub faults: u64,
    /// Pages of the sample buffer resident just before the timed pass and
    /// just after it (`sys::pages_resident`). Scoped to the buffer, so
    /// nothing else in the process moves it. Every page at both ends where
    /// the pass paid nothing for its own memory. A refused `mincore`
    /// records 0, so a caller that expects every page fails closed.
    pub resident: (usize, usize),
}

/// Records `reads` back-to-back reads (one store each), then the gaps
/// between them in ns (`ns_per_unit` converts the read's units).
///
/// The warm-up fills the whole buffer with reads first, so every page of
/// it is faulted in before the timed pass: a fresh buffer's first touches
/// (thousands of faults, about a microsecond each) would otherwise be gaps
/// in the rows measured before the allocator had memory to reuse.
pub fn deltas<F: Fn() -> u64>(read: &F, ns_per_unit: f64, reads: usize) -> Deltas {
    let mut values = vec![0_u64; reads.max(2)];
    for v in &mut values {
        *v = read();
    }
    // Residency of the buffer is read outside the timing bracket: it is the
    // uncontaminated statement of what the warm-up achieved, and of what the
    // timed pass could still fault on.
    let resident_before = sys::pages_resident(&values);
    let (v0, s0, f0) = (realtime_ns(), sys::context_switches(), sys::minor_faults());
    for v in &mut values {
        *v = read();
    }
    let (s1, f1) = (sys::context_switches(), sys::minor_faults());
    let resident_after = sys::pages_resident(&values);
    let first = values[0];
    let mut gaps = Vec::with_capacity(values.len() - 1);
    let (mut zeros, mut backwards, mut max_back) = (0, 0, 0_u64);
    let mut long = Vec::new();
    for w in values.windows(2) {
        if w[1] < w[0] {
            backwards += 1;
            max_back = max_back.max(w[0] - w[1]);
            continue;
        }
        let gap = w[1] - w[0];
        if gap == 0 {
            zeros += 1;
        }
        let ns = gap as f64 * ns_per_unit;
        if ns >= 10_000.0 {
            let at = v0 + (w[1].wrapping_sub(first) as f64 * ns_per_unit) as i64;
            long.push((at, ns));
        }
        gaps.push(ns);
    }
    gaps.sort_by(f64::total_cmp);
    long.sort_by(|a, b| b.1.total_cmp(&a.1));
    long.truncate(20);
    Deltas {
        reads,
        min_nonzero_ns: gaps.iter().copied().find(|&g| g > 0.0).unwrap_or(f64::NAN),
        p50_ns: quantile(&gaps, 0.5),
        p99_ns: quantile(&gaps, 0.99),
        p9999_ns: quantile(&gaps, 0.9999),
        max_ns: gaps.last().copied().unwrap_or(f64::NAN),
        zeros,
        backwards,
        max_backwards_ns: max_back as f64 * ns_per_unit,
        gaps: long,
        switches: (s1.0 - s0.0, s1.1 - s0.1),
        faults: f1.saturating_sub(f0),
        resident: (resident_before, resident_after),
    }
}

/// Two threads on two CPUs reading one clock in turn.
///
/// One reads and publishes its value, the other sees it and reads its own. A second read
/// lower than the first is a backwards step across cores: the counters'
/// skew, or an unordered read taken before the handoff it follows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skew {
    /// Handoffs each way.
    pub rounds: usize,
    /// The least second-minus-first, a to b, ns (one-way latency plus read
    /// cost, minus skew).
    pub min_ab_ns: f64,
    /// The same, b to a.
    pub min_ba_ns: f64,
    /// `(min_ab - min_ba) / 2`: the counters' offset, b's ahead of a's, ns.
    pub offset_ns: f64,
    /// Second reads lower than the first, a to b.
    pub inversions_ab: usize,
    /// The same, b to a.
    pub inversions_ba: usize,
    /// The largest such step back, ns.
    pub max_inversion_ns: f64,
}

/// One direction of [`skew`]: (least difference, inversions, largest
/// inversion), in the read's units, signed.
fn one_way<F: Fn() -> u64 + Sync>(
    read: &F,
    rounds: usize,
    first: Option<usize>,
    second: Option<usize>,
) -> (i64, usize, u64) {
    let seq = AtomicU64::new(0);
    let value = AtomicU64::new(0);
    std::thread::scope(|s| {
        let reader = s.spawn(|| {
            if let Some(cpu) = second {
                let _ = sys::pin(cpu);
            }
            let (mut least, mut inversions, mut worst) = (i64::MAX, 0, 0_u64);
            for round in 0..rounds as u64 {
                while seq.load(Ordering::Acquire) != 2 * round + 1 {
                    std::hint::spin_loop();
                }
                let mine = read();
                let theirs = value.load(Ordering::Relaxed);
                let d = mine.wrapping_sub(theirs).cast_signed();
                least = least.min(d);
                if d < 0 {
                    inversions += 1;
                    worst = worst.max(d.unsigned_abs());
                }
                seq.store(2 * round + 2, Ordering::Release);
            }
            (least, inversions, worst)
        });
        let writer = s.spawn(|| {
            if let Some(cpu) = first {
                let _ = sys::pin(cpu);
            }
            for round in 0..rounds as u64 {
                value.store(read(), Ordering::Relaxed);
                seq.store(2 * round + 1, Ordering::Release);
                while seq.load(Ordering::Acquire) != 2 * round + 2 {
                    std::hint::spin_loop();
                }
            }
        });
        let _ = writer.join();
        reader.join().unwrap_or((i64::MAX, 0, 0))
    })
}

/// [`Skew`] between `cpu_a` and `cpu_b` (unpinned where `None`, or where
/// the system cannot pin), `rounds` handoffs each way.
pub fn skew<F: Fn() -> u64 + Sync>(
    read: &F,
    ns_per_unit: f64,
    rounds: usize,
    cpu_a: Option<usize>,
    cpu_b: Option<usize>,
) -> Skew {
    let (ab, inv_ab, worst_ab) = one_way(read, rounds, cpu_a, cpu_b);
    let (ba, inv_ba, worst_ba) = one_way(read, rounds, cpu_b, cpu_a);
    let (ab, ba) = (ab as f64 * ns_per_unit, ba as f64 * ns_per_unit);
    Skew {
        rounds,
        min_ab_ns: ab,
        min_ba_ns: ba,
        offset_ns: (ab - ba) / 2.0,
        inversions_ab: inv_ab,
        inversions_ba: inv_ba,
        max_inversion_ns: worst_ab.max(worst_ba) as f64 * ns_per_unit,
    }
}

/// (`reference` at the middle of the tightest of 8 brackets, the read): a
/// clock sampled against a reference clock (ns).
pub fn against<R: Fn() -> u64 + ?Sized, F: Fn() -> u64 + ?Sized>(
    reference: &R,
    read: &F,
) -> (u64, u64) {
    let mut best = (u64::MAX, 0, 0);
    for _ in 0..8 {
        let a = reference();
        let v = read();
        let b = reference();
        let width = b.wrapping_sub(a);
        if width < best.0 {
            best = (width, a + width / 2, v);
        }
    }
    (best.1, best.2)
}

/// [`against`] `CLOCK_MONOTONIC_RAW`.
pub fn against_raw<F: Fn() -> u64 + ?Sized>(read: &F) -> (u64, u64) {
    against(&monotonic_raw_ns, read)
}

/// A clock's drift from its reference between two samples ([`against`]):
/// (what it counted, ns, minus what the reference counted; that as ppm of
/// the reference's).
#[must_use]
pub fn drift(start: (u64, u64), end: (u64, u64), ns_per_unit: f64) -> (f64, f64) {
    let reference = end.0.wrapping_sub(start.0).cast_signed() as f64;
    let ours = end.1.wrapping_sub(start.1).cast_signed() as f64 * ns_per_unit;
    let error = ours - reference;
    (error, error / reference * 1e6)
}

/// The clock's rate against `CLOCK_MONOTONIC_RAW` over `window_ns` (spun):
/// its error, ppm. The check every timed clock passes first.
pub fn rate_ppm<F: Fn() -> u64>(read: &F, ns_per_unit: f64, window_ns: u64) -> f64 {
    let start = against_raw(read);
    while monotonic_raw_ns() - start.0 < window_ns {
        std::hint::spin_loop();
    }
    drift(start, against_raw(read), ns_per_unit).1
}

/// Reads lower than the one before over `reads` back-to-back reads.
pub fn backwards<F: Fn() -> u64>(read: &F, reads: usize) -> usize {
    let mut last = read();
    let mut n = 0;
    for _ in 0..reads {
        let now = read();
        if now < last {
            n += 1;
        }
        last = now;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_are_nearest_rank() {
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert!((quantile(&v, 0.5) - 51.0).abs() < 1e-9 || (quantile(&v, 0.5) - 50.0).abs() < 1e-9);
        assert!((quantile(&v, 0.0) - 1.0).abs() < 1e-9);
        assert!((quantile(&v, 1.0) - 100.0).abs() < 1e-9);
        assert!((quantile(&v, 0.99) - 99.0).abs() < 1e-9);
        assert!(quantile(&[], 0.5).is_nan());
    }

    #[test]
    fn cost_orders_its_statistics() {
        let c = Cost::from_means(vec![3.0, 1.0, 2.0, 10.0], 7);
        assert_eq!((c.batches, c.reads), (4, 7));
        assert!((c.min_ns - 1.0).abs() < 1e-9 && (c.max_ns - 10.0).abs() < 1e-9);
        assert!(c.min_ns <= c.median_ns && c.median_ns <= c.p99_ns && c.p99_ns <= c.max_ns);
    }

    /// A stepped source: every gap is the step, nothing goes backwards;
    /// one that steps back once is counted, its step measured.
    #[test]
    fn deltas_count_gaps_and_steps_back() {
        let next = AtomicU64::new(1_000);
        let d = deltas(&|| next.fetch_add(5, Ordering::Relaxed), 2.0, 1_000);
        assert_eq!((d.backwards, d.zeros), (0, 0));
        assert!(
            (d.min_nonzero_ns - 10.0).abs() < 1e-9 && (d.max_ns - 10.0).abs() < 1e-9,
            "{d:?}"
        );
        let n = AtomicU64::new(0);
        let back = || {
            let i = n.fetch_add(1, Ordering::Relaxed);
            if i == 20_500 { 0 } else { 1_000_000 + i }
        };
        // a warm-up of 20,000 reads (the whole buffer), then the timed
        // pass's 501st read steps back
        let d = deltas(&back, 1.0, 20_000);
        assert_eq!(d.backwards, 1, "{d:?}");
        assert!(d.max_backwards_ns > 1_000_000.0 - 1.0);
    }

    /// The gap pass pays nothing for its own memory while it times: a
    /// page's first touch is a fault, about a microsecond, and in the timed
    /// pass it would be a gap no clock caused. (A run's first two rows,
    /// whose buffers the allocator had not handed out before, showed
    /// p99.99s near a microsecond where the same instructions measured later
    /// showed 12 to 156 ns.) 64 MB, above any allocator's reuse threshold
    /// (glibc's rises to 32 MB at most), so its pages are fresh.
    ///
    /// The evidence is the buffer's own residency, read with `mincore`:
    /// scoped to the buffer, so nothing else in the process can move it, and
    /// unlike a gap it cannot be mistaken for an interrupt (a clean pass of
    /// a real clock here already gaps 30 to 47 us, with no faults at all).
    #[test]
    fn the_gap_pass_takes_no_page_faults_while_it_times() {
        let _alone = crate::faults_alone();
        let next = AtomicU64::new(0);
        let read = || next.fetch_add(1, Ordering::Relaxed);
        let page_u64s = sys::page_size() / std::mem::size_of::<u64>();
        assert!(page_u64s > 0, "no page size");
        let mut values = vec![0_u64; 8 << 20];
        for v in &mut values {
            *v = read();
        }
        // Pages wholly inside the buffer: a `Vec` is only element-aligned,
        // so this is `length / page_size` or one less where the allocation
        // straddles a page boundary. Not the naive length count.
        let whole = sys::pages_wholly_inside(&values);
        assert_eq!(
            sys::pages_resident(&values),
            whole,
            "the warm-up left a page of the sample buffer cold"
        );
        // Walk it again, and every 256 pages check that the pages still
        // ahead of the cursor are resident: a reclaimed page there is one
        // the pass is about to fault in.
        let mut cold = 0;
        let mut i = 0;
        while i < values.len() {
            values[i] = read();
            let tail = &values[i + 1..];
            let want = sys::pages_wholly_inside(tail);
            if want > 0 && sys::pages_resident(tail) < want {
                cold += 1;
            }
            i += page_u64s * 256;
        }
        assert_eq!(cold, 0, "{cold} stretches of the pass ran over a cold page");
        // And what deltas itself records around its timed pass. Its buffer
        // is the same length and fully touched by its own warm-up, so both
        // ends show every page it can see: `whole`, or one fewer where its
        // allocation is not page-aligned.
        let d = deltas(&read, 1.0, 8 << 20);
        assert_eq!(d.resident.0, d.resident.1, "{d:?}");
        assert!(
            d.resident.0 == whole || d.resident.0 + 1 == whole,
            "{d:?} vs wholly-inside {whole}"
        );
        // A cross-check where the counter is the calling thread's. Darwin
        // has no RUSAGE_THREAD, so there the delta is the whole process's
        // and asserting it would be asserting something else entirely.
        if sys::minor_faults_are_thread_local() {
            assert_eq!(d.faults, 0, "{} page faults in the timed pass", d.faults);
        }
    }

    /// A clock that counts nothing is 100% slow; one that counts twice as
    /// fast is 100% fast: the rate check sees both.
    #[test]
    fn the_rate_check_sees_a_wrong_clock() {
        let ppm = rate_ppm(&monotonic_raw_ns, 2.0, 2_000_000);
        assert!((ppm - 1e6).abs() < 1e5, "{ppm}");
        let ppm = rate_ppm(&|| 7, 1.0, 2_000_000);
        assert!((ppm + 1e6).abs() < 1.0, "{ppm}");
        let ppm = rate_ppm(&monotonic_raw_ns, 1.0, 2_000_000);
        assert!(ppm.abs() < 1e4, "{ppm}");
    }

    /// Drift is measured against the reference given, not always against
    /// `CLOCK_MONOTONIC_RAW` (which, where the kernel keeps time with the
    /// counter, is the counter itself): a clock is sampled at the middle of
    /// the reference's bracket, and one counting twice the reference's
    /// rate is 100% ahead of it, one counting the same is not.
    #[test]
    fn drift_is_against_the_reference_given() {
        let t = AtomicU64::new(0);
        let reference = || t.fetch_add(10, Ordering::Relaxed) + 10;
        let same = || t.load(Ordering::Relaxed);
        let doubled = || 2 * t.load(Ordering::Relaxed);
        // each bracket is 10 wide: its middle is 5 after the first read
        assert_eq!(against(&reference, &same), (15, 10));
        let s0 = (against(&reference, &same), against(&reference, &doubled));
        for _ in 0..10_000 {
            reference();
        }
        let s1 = (against(&reference, &same), against(&reference, &doubled));
        let (error, ppm) = drift(s0.0, s1.0, 1.0);
        assert!(error.abs() <= 30.0 && ppm.abs() < 500.0, "{error} {ppm}");
        let (error, ppm) = drift(s0.1, s1.1, 1.0);
        assert!(error > 99_000.0 && (ppm - 1e6).abs() < 1e4, "{error} {ppm}");
        let (_, ppm) = drift(s0.1, s1.1, 0.5);
        assert!(ppm.abs() < 1e4, "doubled ticks at half a ns each: {ppm}");
    }

    /// The handoff orders the reads: a monotonic clock never steps back
    /// across it.
    #[test]
    fn a_handoff_orders_a_monotonic_clock() {
        let s = skew(&monotonic_raw_ns, 1.0, 2_000, None, None);
        assert_eq!((s.inversions_ab, s.inversions_ba), (0, 0), "{s:?}");
        assert!(s.min_ab_ns >= 0.0 && s.min_ba_ns >= 0.0, "{s:?}");
    }
}
