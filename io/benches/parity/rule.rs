//! The parity gate's decision rule: percentiles, the paired difference of
//! one A/B pair, and the verdict over the pairs. Pure functions, tested on
//! every platform.
//!
//! A pair is `BLOCKS` short blocks, the two arms alternating A B B A ...
//! Each adjacent (A, B) blocks form a sub-pair. The pair's difference is the
//! median over its sub-pairs of (mio block p50 - `epoll-spin` block p50): a
//! change in the host (a VM's CPU moving between core types or clock
//! speeds) lasting longer than a block moves both blocks of a sub-pair, so
//! it cancels in the difference. A pooled p50 over a pair does not: when the
//! host spends near half the pair in each of two speeds, the pooled median
//! jumps between them.

/// A/B pairs per socket count. Seven is the smallest count for which the sign
/// test below can reach a one-sided p below 0.07 (8 / 128 = 0.0625).
pub const PAIRS: usize = 7;

/// Pairs one side must win, beyond the tolerance, for the gate to fail.
/// Under "no difference" the chance of 6 or more of 7 going one way is
/// 8 / 128 = 0.0625 per side; the tolerance makes it far rarer still, since
/// a pair only counts when it clears `tolerance_ns`.
pub const SIGN_MIN: usize = 6;

/// Absolute floor of the tolerance, ns.
///
/// Set from the spread of the paired difference on the Docker Linux lane
/// (`io/benches/results/mio-parity-docker-20261024T145651Z/summary.md`:
/// arm64 and amd64 under Docker Desktop on Apple silicon, 10 runs, 140
/// pairs at 5.5-14.4 µs p50). Its median was +0.19 to +0.27 µs (mio
/// slower: its read to `WouldBlock` is in the timed path) and its standard
/// deviation 0.51-0.85 µs. At 0.5 µs up to 10 of 35 pairs lay beyond it on
/// one side: 6 of 7 by chance about once in 350 tests, too often for CI.
/// At 1.0 µs up to 4 of 35: about once in 70,000.
pub const TOL_ABS_NS: u64 = 1_000;

/// Relative tolerance, percent of the pair's mean p50: the spread grew with
/// the round trip (standard deviation 4-8 % of p50 in the runs above),
/// so a slower host (a CI runner's loopback) gets a proportionally wider
/// band. At the Docker lane's 10 µs it equals the floor.
pub const TOL_PCT: u64 = 10;

/// Percentiles of one round's round trips, ns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pct {
    /// Median.
    pub p50: u64,
    /// 99th percentile.
    pub p99: u64,
    /// Largest sample.
    pub max: u64,
}

/// p50 / p99 / max of `samples` (sorted in place). `None` when empty.
#[must_use]
pub fn percentiles(samples: &mut [u64]) -> Option<Pct> {
    samples.sort_unstable();
    let n = samples.len();
    Some(Pct {
        p50: *samples.get(n / 2)?,
        p99: *samples.get(n * 99 / 100)?,
        max: *samples.last()?,
    })
}

/// The tolerance for one pair: the larger of `TOL_ABS_NS` and `TOL_PCT` of
/// the two arms' mean p50.
#[must_use]
pub const fn tolerance_ns(a: u64, b: u64) -> u64 {
    let pct = (a / 2 + b / 2) * TOL_PCT / 100;
    if pct > TOL_ABS_NS { pct } else { TOL_ABS_NS }
}

/// The median over sub-pairs of `mio[j] - epoll[j]` (block p50s, ns): what
/// the verdict reads as "mio's p50 minus `epoll-spin`'s" for one pair.
/// `None` when empty or the lengths differ.
#[must_use]
pub fn paired_diff_ns(epoll: &[u64], mio: &[u64]) -> Option<i64> {
    if epoll.len() != mio.len() {
        return None;
    }
    let mut d: Vec<i64> = epoll
        .iter()
        .zip(mio)
        .map(|(&e, &m)| i64::try_from(m).unwrap_or(i64::MAX) - i64::try_from(e).unwrap_or(i64::MAX))
        .collect();
    d.sort_unstable();
    let n = d.len();
    if n == 0 {
        None
    } else if n % 2 == 1 {
        Some(d[n / 2])
    } else {
        Some(i64::midpoint(d[n / 2 - 1], d[n / 2]))
    }
}

/// What the pairs show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Neither side won `SIGN_MIN` pairs beyond the tolerance.
    Level {
        /// Pairs `epoll-spin` won beyond the tolerance.
        epoll_ahead: usize,
        /// Pairs mio won beyond the tolerance.
        mio_ahead: usize,
    },
    /// `epoll-spin` won this many pairs beyond the tolerance: a suspected
    /// defect (the mio loop is doing more work than it should, or less is
    /// being timed on the `Driver` side).
    EpollAhead(usize),
    /// mio won this many pairs beyond the tolerance: the `Driver` has drifted
    /// from mio's mechanism, or added per-poll cost.
    MioAhead(usize),
}

/// One pair as the verdict reads it, ns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    /// `paired_diff_ns`: positive when mio is slower.
    pub mio_minus_epoll: i64,
    /// `epoll-spin`'s pooled p50 (scales the tolerance).
    pub epoll_p50: u64,
    /// mio's pooled p50 (scales the tolerance).
    pub mio_p50: u64,
}

/// The verdict over `pairs`.
#[must_use]
pub fn verdict(pairs: &[Pair]) -> Verdict {
    let (mut epoll_ahead, mut mio_ahead) = (0, 0);
    for p in pairs {
        let tol = i64::try_from(tolerance_ns(p.epoll_p50, p.mio_p50)).unwrap_or(i64::MAX);
        if p.mio_minus_epoll > tol {
            epoll_ahead += 1;
        } else if -p.mio_minus_epoll > tol {
            mio_ahead += 1;
        }
    }
    if epoll_ahead >= SIGN_MIN {
        Verdict::EpollAhead(epoll_ahead)
    } else if mio_ahead >= SIGN_MIN {
        Verdict::MioAhead(mio_ahead)
    } else {
        Verdict::Level {
            epoll_ahead,
            mio_ahead,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_are_nearest_rank_over_the_sorted_round() {
        let mut s: Vec<u64> = (1..=1_000).rev().collect();
        assert_eq!(
            percentiles(&mut s),
            Some(Pct {
                p50: 501,
                p99: 991,
                max: 1_000
            })
        );
        assert_eq!(percentiles(&mut []), None);
    }

    #[test]
    fn tolerance_is_the_floor_or_the_percentage_whichever_is_larger() {
        assert_eq!(tolerance_ns(1_000, 1_000), TOL_ABS_NS);
        let big = 100 * TOL_ABS_NS; // far above the floor
        assert_eq!(tolerance_ns(big, big), big * TOL_PCT / 100);
        const { assert!(TOL_ABS_NS > 0 && TOL_PCT > 0) };
    }

    fn pairs(diffs: &[i64]) -> Vec<Pair> {
        // Both pooled p50s at 30 µs (they only scale the tolerance); a
        // positive paired diff: mio slower.
        diffs
            .iter()
            .map(|&d| Pair {
                mio_minus_epoll: d,
                epoll_p50: 30_000,
                mio_p50: 30_000,
            })
            .collect()
    }

    #[test]
    fn the_pair_difference_is_the_median_of_its_sub_pairs() {
        // Sub-pairs in two host speeds (6 and 10 µs), mio 200 ns slower in
        // every one: the pooled p50s would differ by 4 µs either way.
        let e = [6_000, 6_000, 10_000, 10_000, 10_000];
        let m = [6_200, 6_200, 10_200, 10_200, 10_200];
        assert_eq!(paired_diff_ns(&e, &m), Some(200));
        // One outlying sub-pair does not move it.
        let m2 = [6_200, 6_200, 10_200, 10_200, 2_000];
        assert_eq!(paired_diff_ns(&e, &m2), Some(200));
        // An even count: the mean of the middle two.
        assert_eq!(
            paired_diff_ns(&[0, 0, 0, 0], &[100, 300, 500, 700]),
            Some(400)
        );
        assert_eq!(paired_diff_ns(&[], &[]), None);
        assert_eq!(paired_diff_ns(&[1], &[1, 2]), None);
    }

    #[test]
    fn six_of_seven_beyond_the_tolerance_fails_either_way() {
        let t = i64::try_from(tolerance_ns(30_000, 30_000)).unwrap_or(i64::MAX) + 50;
        let six = [t, t, t, t, t, t, -t];
        assert_eq!(verdict(&pairs(&six)), Verdict::EpollAhead(6));
        let six_neg: Vec<i64> = six.iter().map(|d| -d).collect();
        assert_eq!(verdict(&pairs(&six_neg)), Verdict::MioAhead(6));
        assert_eq!(verdict(&pairs(&[t; 7])), Verdict::EpollAhead(7));
    }

    #[test]
    fn five_of_seven_or_wins_inside_the_tolerance_pass() {
        let t = i64::try_from(tolerance_ns(30_000, 30_000)).unwrap_or(i64::MAX);
        let five = [t + 50, t + 50, t + 50, t + 50, t + 50, 0, 0];
        assert_eq!(
            verdict(&pairs(&five)),
            Verdict::Level {
                epoll_ahead: 5,
                mio_ahead: 0
            }
        );
        // One side ahead in every pair, but never by more than the tolerance.
        assert_eq!(
            verdict(&pairs(&[t; 7])),
            Verdict::Level {
                epoll_ahead: 0,
                mio_ahead: 0
            }
        );
    }
}
