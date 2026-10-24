//! The parity gate's decision rule: percentiles, the paired difference of
//! one A/B pair, and the verdict over the pairs. Pure functions, tested on
//! every platform.
//!
//! Within a pair the two loops take turns round trip by round trip,
//! A B B A ..., so both see the same host state: a VM's CPU moving between
//! core types or clock speeds moves both. The round trips, in the order they
//! ran, are cut into blocks of `BLOCK_SLOTS`; the pair's difference is the
//! median over its blocks of (mio block p50 - `epoll-spin` block p50).

/// A/B pairs per socket count. Seven is the smallest count for which the sign
/// test below can reach a one-sided p below 0.07 (8 / 128 = 0.0625).
pub const PAIRS: usize = 7;

/// Pairs one side must win, beyond the tolerance, for the gate to fail.
/// Under "no difference" the chance of 6 or more of 7 going one way is
/// 8 / 128 = 0.0625 per side; the tolerance makes it far rarer still, since
/// a pair only counts when it clears `tolerance_ns`.
pub const SIGN_MIN: usize = 6;

/// Round trips per block, both loops together (100 each, 20 ms).
pub const BLOCK_SLOTS: usize = 200;

/// Tolerance, percent of the pair's mean p50. A 10 % slowdown of either loop
/// must fail, and the measured level must pass.
///
/// Set from `io/benches/results/mio-parity-docker-20261024T154626Z/summary.md`
/// (Docker on Apple silicon, arm64 native and amd64 under Rosetta). Nothing
/// injected, 140 pairs: −0.5 % to +6.2 % (median +2.1 to +3.1 %, mio slower
/// by its timed read to `WouldBlock`), 3 beyond 5 %, never more than 1 of 7
/// in a test. 10 % added to one loop's round trips, 56 pairs: every one
/// beyond 5 % on that side (closest −5.4 %), every test failed 7 of 7.
/// Relative, with no absolute floor: the earlier floor of 1 µs was 10 % of
/// a 10 µs round trip, so a 10 % slowdown sat on the tolerance.
pub const TOL_PCT: u64 = 5;

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

/// The tolerance for one pair: `TOL_PCT` of the two sides' mean p50.
#[must_use]
pub const fn tolerance_ns(a: u64, b: u64) -> u64 {
    (a / 2 + b / 2) * TOL_PCT / 100
}

/// The median over blocks of `mio[j] - epoll[j]` (block p50s, ns): what
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

/// Which loop ran a round trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The `Driver` with `Backend::EpollSpin`.
    Epoll,
    /// The minimal mio loop.
    Mio,
}

/// Each side's p50 per block of `BLOCK_SLOTS` round trips, from the round
/// trips in the order they ran. `None` when a block lacks either side.
#[must_use]
pub fn block_p50s(slots: &[(Side, u64)]) -> Option<(Vec<u64>, Vec<u64>)> {
    if slots.is_empty() {
        return None;
    }
    let (mut eb, mut mb) = (Vec::new(), Vec::new());
    for block in slots.chunks(BLOCK_SLOTS) {
        let side =
            |s: Side| -> Vec<u64> { block.iter().filter(|x| x.0 == s).map(|x| x.1).collect() };
        eb.push(percentiles(&mut side(Side::Epoll))?.p50);
        mb.push(percentiles(&mut side(Side::Mio))?.p50);
    }
    Some((eb, mb))
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
    fn tolerance_is_a_percentage_of_the_mean_p50() {
        assert_eq!(tolerance_ns(10_000, 10_000), 10_000 * TOL_PCT / 100);
        assert_eq!(tolerance_ns(9_000, 11_000), 10_000 * TOL_PCT / 100);
        const { assert!(TOL_PCT > 0 && TOL_PCT < 10) };
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
    fn the_pair_difference_is_the_median_of_its_blocks() {
        // Blocks in two host speeds (6 and 10 µs), mio 200 ns slower in
        // every one: the pooled p50s would differ by 4 µs either way.
        let e = [6_000, 6_000, 10_000, 10_000, 10_000];
        let m = [6_200, 6_200, 10_200, 10_200, 10_200];
        assert_eq!(paired_diff_ns(&e, &m), Some(200));
        // One outlying block does not move it.
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
        assert_eq!(verdict(&pairs(&[t; PAIRS])), Verdict::EpollAhead(7));
    }

    fn at_10us(diffs_ns: &[i64]) -> Vec<Pair> {
        diffs_ns
            .iter()
            .map(|&d| Pair {
                mio_minus_epoll: d,
                epoll_p50: 10_000,
                mio_p50: 10_000,
            })
            .collect()
    }

    #[test]
    fn a_ten_percent_slowdown_of_either_side_fails() {
        // The recorded bias: mio about 2 % slower (its read to WouldBlock).
        // The `Driver` 10 % slower on top of it: mio - epoll = 0.2 - 1.0 µs.
        assert_eq!(verdict(&at_10us(&[-800; PAIRS])), Verdict::MioAhead(7));
        // mio 10 % slower on top of it.
        assert_eq!(verdict(&at_10us(&[1_200; PAIRS])), Verdict::EpollAhead(7));
    }

    #[test]
    fn the_recorded_spread_passes() {
        // The worst recorded test: one pair at +6.2 %, the rest at the
        // median +3.1 %.
        let mut d = vec![310; PAIRS];
        d[0] = 620;
        assert_eq!(
            verdict(&at_10us(&d)),
            Verdict::Level {
                epoll_ahead: 1,
                mio_ahead: 0
            }
        );
    }

    #[test]
    fn blocks_take_each_sides_p50_in_run_order() {
        use Side::{Epoll, Mio};
        // Two blocks; in each the sides alternate, as the gate runs them.
        let mut slots = Vec::new();
        for b in 0..2u64 {
            for i in 0..(BLOCK_SLOTS as u64) {
                let side = if i % 2 == 0 { Epoll } else { Mio };
                let base = if side == Epoll { 1_000 } else { 1_100 };
                slots.push((side, base * (b + 1) + i % 3));
            }
        }
        let (e, m) = block_p50s(&slots).unwrap_or_default();
        assert_eq!(e.len(), 2);
        assert_eq!(m.len(), 2);
        assert!(
            e[0] >= 1_000 && e[0] <= 1_002 && e[1] >= 2_000 && e[1] <= 2_002,
            "{e:?}"
        );
        assert!(
            m[0] >= 1_100 && m[0] <= 1_102 && m[1] >= 2_200 && m[1] <= 2_202,
            "{m:?}"
        );
        // A block with one side missing is refused, not read as level.
        assert_eq!(block_p50s(&[(Epoll, 1); BLOCK_SLOTS]), None);
        assert_eq!(block_p50s(&[]), None);
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
            verdict(&pairs(&[t; PAIRS])),
            Verdict::Level {
                epoll_ahead: 0,
                mio_ahead: 0
            }
        );
    }
}
