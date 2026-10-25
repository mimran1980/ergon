//! `limit::Budget` on a fake clock: window safety checked by brute force
//! (every span, and server windows of any alignment with send jitter), why
//! the ring has extra slots and why window limits do not use GCRA, all-or-none
//! charges, the reserve, refunds, server usage, throttle holds and backoff,
//! the halt after a ban, and the re-send-within-deadline helper.
#![allow(missing_docs)]

use std::net::IpAddr;
use std::num::NonZeroU64;

use ergon_io::Error;
use ergon_io::limit::{
    BucketId, BucketSpec, Budget, Charge, Class, Limited, MAX_CHARGES, Model, Rate, Resend,
    SourceIp, SourceIps, Ticket, Why,
};

type R<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const MS: i64 = 1_000_000;
const S: i64 = 1_000 * MS;
/// An arbitrary clock origin (a multiple of every sub-window used here).
const T0: i64 = 1_760_000_000 * S;

/// splitmix64: a fixed seed gives the same run every time.
struct Rng(u64);

impl Rng {
    const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n` > 0).
    const fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn below_i64(&mut self, n: i64) -> R<i64> {
        Ok(i64::try_from(self.below(u64::try_from(n)?))?)
    }
}

/// A budget with one bucket, and that bucket's id.
fn one(spec: BucketSpec) -> R<(Budget, BucketId)> {
    let name = spec.name.clone();
    let b = Budget::new(vec![spec])?;
    let id = b.id(&name).ok_or("bucket not found by name")?;
    Ok((b, id))
}

/// 10 per 1000 ms in 10 sub-windows; guard = one sub-window (100 ms).
fn w10() -> BucketSpec {
    BucketSpec::rolling("w", 10, 1_000, 10)
}

const fn c(bucket: BucketId, cost: u32) -> Charge {
    Charge::new(bucket, cost)
}

fn ok(r: Result<Ticket, Limited>) -> R<Ticket> {
    r.map_err(|l| format!("refused: {l:?}").into())
}

fn refused(r: Result<Ticket, Limited>) -> R<Limited> {
    match r {
        Ok(t) => Err(format!("expected a refusal, got {t:?}").into()),
        Err(l) => Ok(l),
    }
}

const fn is_config(e: &Error) -> bool {
    matches!(e, Error::Config(_))
}

/// Tries one charge of 1..=`max_cost` units at random times (gaps 1..=`gap`
/// ns) from `start` for `dur`; returns (send time, units) of each that passed.
fn greedy(
    b: &mut Budget,
    id: BucketId,
    start: i64,
    dur: i64,
    gap: i64,
    max_cost: u32,
    rng: &mut Rng,
) -> R<Vec<(i64, u64)>> {
    let mut sent = Vec::new();
    let mut t = start;
    while t < start + dur {
        let cost = u32::try_from(1 + rng.below(u64::from(max_cost)))?;
        if b.try_charge(&[c(id, cost)], Class::Normal, t).is_ok() {
            sent.push((t, u64::from(cost)));
        }
        t += 1 + rng.below_i64(gap)?;
    }
    Ok(sent)
}

/// The most units in any half-open span `[t, t + span)` (`sends` sorted by time).
fn max_in_span(sends: &[(i64, u64)], span: i64) -> u64 {
    let (mut j, mut sum, mut best) = (0, 0, 0);
    for i in 0..sends.len() {
        while j < sends.len() && sends[j].0 < sends[i].0 + span {
            sum += sends[j].1;
            j += 1;
        }
        best = best.max(sum);
        sum -= sends[i].1;
    }
    best
}

/// The most units in one server window `[phase + k·w, phase + (k+1)·w)`.
fn max_in_fixed_windows(arrivals: &[(i64, u64)], w: i64, phase: i64) -> u64 {
    let mut best = 0;
    let mut cur = (i64::MIN, 0);
    for &(t, u) in arrivals {
        let k = (t - phase).div_euclid(w);
        if k == cur.0 {
            cur.1 += u;
        } else {
            cur = (k, u);
        }
        best = best.max(cur.1);
    }
    best
}

// --- window safety ---------------------------------------------------------

#[test]
fn rolling_never_exceeds_the_limit_in_any_span_of_window_plus_guard() -> R {
    let mut rng = Rng(7);
    // (limit, window ms, slots, guard ms, max cost)
    let cases = [
        (10, 1_000, 10, None, 1),
        (10, 1_000, 10, Some(0), 3),
        (10, 1_000, 10, Some(50), 2),
        (10, 1_000, 10, Some(250), 1),
        (7, 2_000, 4, None, 2),
        (100, 10_000, 100, None, 9),
        (3, 60_000, 60, Some(1_500), 1),
    ];
    for (limit, window_ms, slots, guard, max_cost) in cases {
        let mut spec = BucketSpec::rolling("w", limit, window_ms, slots);
        if let Some(g) = guard {
            spec = spec.with_guard_ms(g);
        }
        let guard_ns = i64::from(spec.guard_ms) * MS;
        let (window, slot) = (
            i64::from(window_ms) * MS,
            i64::from(window_ms / u32::from(slots)) * MS,
        );
        let (mut b, id) = one(spec)?;
        let start = T0 + rng.below_i64(window)?;
        let gap = (window / 40).max(1);
        let sends = greedy(&mut b, id, start, 20 * window, gap, max_cost, &mut rng)?;
        let span_max = max_in_span(&sends, window + guard_ns);
        assert!(
            span_max <= u64::from(limit),
            "{limit}/{window_ms} ms/{slots}, guard {guard_ns} ns: {span_max} units in one span"
        );
        // Not over-cautious: a span longer by two sub-windows reaches the limit
        // (within one charge of it).
        let near = max_in_span(&sends, window + guard_ns + 2 * slot);
        assert!(
            near + u64::from(max_cost) > u64::from(limit),
            "{limit}/{window_ms} ms: only {near} units even in a longer span"
        );
    }
    Ok(())
}

#[test]
fn rolling_is_safe_at_the_server_for_any_window_alignment_with_jitter_up_to_the_guard() -> R {
    let mut rng = Rng(11);
    let (limit, window_ms, slots) = (10, 1_000, 10);
    for guard_ms in [100, 40, 230] {
        let spec = BucketSpec::rolling("w", limit, window_ms, slots).with_guard_ms(guard_ms);
        let (window, jitter) = (i64::from(window_ms) * MS, i64::from(guard_ms) * MS);
        let (mut b, id) = one(spec)?;
        let sends = greedy(&mut b, id, T0 + 3 * MS, 30 * window, MS, 1, &mut rng)?;
        // Worst case first: early sends delayed by the whole jitter, later
        // ones not. The server then sees every send-time span of window + jitter.
        assert!(max_in_span(&sends, window + jitter) <= u64::from(limit));
        // Random delays, then server windows fixed at random phases, and rolling.
        for _ in 0..20 {
            let mut arrivals: Vec<(i64, u64)> = sends
                .iter()
                .map(|&(t, u)| Ok((t + 7 * MS + rng.below_i64(jitter + 1)?, u)))
                .collect::<R<_>>()?;
            arrivals.sort_unstable();
            let phase = rng.below_i64(window)?;
            assert!(max_in_fixed_windows(&arrivals, window, phase) <= u64::from(limit));
            assert!(max_in_span(&arrivals, window) <= u64::from(limit));
        }
    }
    Ok(())
}

#[test]
fn jitter_at_window_edges_breaks_a_window_with_no_guard() -> R {
    // Guard 0 and a send-to-server spread of 50 ms: a burst at the end of a
    // sub-window, then one as soon as its slot expires, land in one server
    // window. The default guard (one sub-window) prevents it.
    let (limit, window, jitter) = (10, 1_000 * MS, 50 * MS);
    let mut broke = false;
    for phase_ms in (0..100).step_by(5) {
        let start = T0 + phase_ms * MS;
        let mut rng = Rng(3);
        let (mut bare, id) = one(w10().with_guard_ms(0))?;
        let sends = greedy(&mut bare, id, start, 10 * window, MS, 1, &mut rng)?;
        broke |= max_in_span(&sends, window + jitter) > limit;

        let mut rng = Rng(3);
        let (mut guarded, id) = one(w10())?;
        let sends = greedy(&mut guarded, id, start, 10 * window, MS, 1, &mut rng)?;
        assert!(max_in_span(&sends, window + jitter) <= limit);
    }
    assert!(broke, "no phase showed the edge failure without a guard");
    Ok(())
}

/// The tempting design this crate does not use: exactly `slots` sub-windows.
struct ExactSlots {
    counts: Vec<u64>,
    slot_ns: i64,
    head: i64,
    limit: u64,
}

impl ExactSlots {
    fn try_charge(&mut self, cost: u64, now: i64) -> R<bool> {
        let abs = now.div_euclid(self.slot_ns);
        let n = i64::try_from(self.counts.len())?;
        if abs - self.head >= n {
            self.counts.fill(0);
        } else {
            for k in self.head + 1..=abs {
                self.counts[usize::try_from(k.rem_euclid(n))?] = 0;
            }
        }
        self.head = abs;
        if self.counts.iter().sum::<u64>() + cost > self.limit {
            return Ok(false);
        }
        self.counts[usize::try_from(abs.rem_euclid(n))?] += cost;
        Ok(true)
    }
}

#[test]
fn a_ring_of_exactly_slots_sub_windows_lets_more_than_the_limit_through() -> R {
    let (limit, window, slot) = (10_u64, 1_000 * MS, 100 * MS);
    let mut exact = ExactSlots {
        counts: vec![0; 10],
        slot_ns: slot,
        head: i64::MIN / 2,
        limit,
    };
    let (mut budget, id) = one(w10().with_guard_ms(0))?;
    // A burst late in a sub-window, then one attempt every millisecond.
    let (mut by_exact, mut by_budget) = (Vec::new(), Vec::new());
    let mut t = T0 + 95 * MS;
    while t < T0 + 5 * window {
        if exact.try_charge(1, t)? {
            by_exact.push((t, 1));
        }
        if budget.try_charge(&[c(id, 1)], Class::Normal, t).is_ok() {
            by_budget.push((t, 1));
        }
        t += MS;
    }
    let over = max_in_span(&by_exact, window);
    assert!(over > limit, "exact slots stayed within the limit: {over}");
    assert!(max_in_span(&by_budget, window) <= limit);
    Ok(())
}

#[test]
fn gcra_with_burst_l_and_rate_l_per_w_lets_2l_through_in_one_window() -> R {
    // Why window limits use Rolling, not Decay.
    let drain = Rate {
        units: 10,
        per_ms: 1_000,
    };
    let (mut b, id) = one(BucketSpec::decay("g", 10, drain))?;
    let mut sends = Vec::new();
    let mut t = T0;
    while t < T0 + 3 * S {
        if b.try_charge(&[c(id, 1)], Class::Normal, t).is_ok() {
            sends.push((t, 1));
        }
        t += MS;
    }
    assert!(max_in_span(&sends, S) >= 19);
    Ok(())
}

// --- charging ---------------------------------------------------------------

#[test]
fn one_full_bucket_means_no_bucket_is_charged() -> R {
    let mut b = Budget::new(vec![
        BucketSpec::rolling("a", 2, 1_000, 10),
        BucketSpec::rolling("b", 1, 1_000, 10),
    ])?;
    let (a, bb) = (b.id("a").ok_or("a")?, b.id("b").ok_or("b")?);
    ok(b.try_charge(&[c(a, 1), c(bb, 1)], Class::Normal, T0))?;
    let l = refused(b.try_charge(&[c(a, 1), c(bb, 1)], Class::Normal, T0))?;
    assert_eq!((l.bucket, l.why), (bb, Why::Full));
    assert_eq!(b.stats(a, T0)?.used, 1, "a must not keep a partial charge");
    // The first part refused: the later one is not charged either.
    let l = refused(b.try_charge(&[c(bb, 1), c(a, 1)], Class::Normal, T0))?;
    assert_eq!(l.bucket, bb);
    assert_eq!(b.stats(a, T0)?.used, 1);
    // The same bucket twice counts twice.
    let (mut one_b, x) = one(BucketSpec::rolling("x", 1, 1_000, 10))?;
    refused(one_b.try_charge(&[c(x, 1), c(x, 1)], Class::Normal, T0))?;
    assert_eq!(one_b.stats(x, T0)?.used, 0);
    ok(one_b.try_charge(&[c(x, 1)], Class::Normal, T0))?;
    Ok(())
}

// A request is charged whole or not at all, so its readiness is the whole
// request's: two parts on one bucket are their sum, not each part judged
// alone. Costs [6, 6] against a capacity of 10 can never fit, whatever the
// clock says.
#[test]
fn parts_on_one_bucket_are_judged_together() -> R {
    let (mut b, x) = one(BucketSpec::rolling("x", 10, 1_000, 10))?;
    let l = refused(b.try_charge(&[c(x, 6), c(x, 6)], Class::Normal, T0))?;
    assert_eq!(l.why, Why::Never, "{l:?}");
    assert_eq!(
        b.stats(x, T0)?.used,
        0,
        "nothing of a refused request is charged"
    );

    // With room for the sum it passes, and the sum is what is charged.
    ok(b.try_charge(&[c(x, 6), c(x, 4)], Class::Normal, T0))?;
    assert_eq!(b.stats(x, T0)?.used, 10);

    // A sum that fits only after a wait says when, from the empty state the
    // caller retries from: 8 + 8 against 10 with 6 already charged frees
    // 4 units at T0 + 1.2 s.
    let (mut b, x) = one(w10())?;
    ok(b.try_charge(&[c(x, 6)], Class::Normal, T0))?;
    let l = refused(b.try_charge(&[c(x, 4), c(x, 4)], Class::Normal, T0))?;
    assert_eq!(
        (l.why, l.ready_at_ns),
        (Why::Full, T0 + 1_200 * MS),
        "{l:?}"
    );
    assert_eq!(
        b.stats(x, T0)?.used,
        6,
        "the refused request left the 6 in place"
    );

    // The later of two buckets decides, and a whole-request Never outranks a
    // wait on another bucket.
    let mut b = Budget::new(vec![
        BucketSpec::rolling("x", 10, 1_000, 10),
        BucketSpec::rolling("y", 5, 2_000, 10),
    ])?;
    let (x, y) = (b.id("x").ok_or("x")?, b.id("y").ok_or("y")?);
    let l = refused(b.try_charge(&[c(x, 6), c(x, 6), c(y, 1)], Class::Normal, T0))?;
    assert_eq!((l.bucket, l.why), (x, Why::Never), "{l:?}");
    let l = refused(b.try_charge(&[c(x, 4), c(y, 4), c(y, 4)], Class::Normal, T0))?;
    assert_eq!((l.bucket, l.why), (y, Why::Never), "{l:?}");
    Ok(())
}

#[test]
fn a_refusal_names_the_part_that_frees_up_last() -> R {
    let mut b = Budget::new(vec![
        BucketSpec::rolling("a", 1, 1_000, 10),
        BucketSpec::rolling("b", 1, 10_000, 10),
    ])?;
    let (a, bb) = (b.id("a").ok_or("a")?, b.id("b").ok_or("b")?);
    ok(b.try_charge(&[c(a, 1), c(bb, 1)], Class::Normal, T0))?;
    let l = refused(b.try_charge(&[c(a, 1), c(bb, 1)], Class::Normal, T0))?;
    // a frees after 1.2 s, b after 12 s: b decides when to ask again.
    assert_eq!((l.bucket, l.ready_at_ns), (bb, T0 + 12 * S));
    assert_eq!(l.wait_ns(T0 + S), u64::try_from(11 * S)?);
    assert_eq!(l.wait_ns(T0 + 20 * S), 0);
    assert_eq!(b.stats(a, T0)?.limited, 1);
    assert_eq!(b.stats(bb, T0)?.limited, 1);
    Ok(())
}

#[test]
fn ready_at_is_exact_for_rolling_and_decay() -> R {
    // Rolling: 10 units at T0 + 50 ms sit in the sub-window [T0, T0 + 100 ms),
    // which leaves the 12-slot ring when T0 + 1200 ms begins.
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0 + 50 * MS))?;
    let l = refused(b.try_charge(&[c(id, 1)], Class::Normal, T0 + 60 * MS))?;
    assert_eq!((l.why, l.ready_at_ns), (Why::Full, T0 + 1_200 * MS));
    refused(b.try_charge(&[c(id, 1)], Class::Normal, T0 + 1_200 * MS - 1))?;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0 + 1_200 * MS))?;

    // Decay: 10 units, draining 10 per second: one more fits 100 ms later.
    let drain = Rate {
        units: 10,
        per_ms: 1_000,
    };
    let (mut d, id) = one(BucketSpec::decay("d", 10, drain))?;
    ok(d.try_charge(&[c(id, 10)], Class::Normal, T0))?;
    let l = refused(d.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    assert_eq!((l.why, l.ready_at_ns), (Why::Full, T0 + 100 * MS));
    refused(d.try_charge(&[c(id, 1)], Class::Normal, T0 + 100 * MS - 1))?;
    ok(d.try_charge(&[c(id, 1)], Class::Normal, T0 + 100 * MS))?;
    assert_eq!(d.stats(id, T0 + 100 * MS)?.used, 10);
    Ok(())
}

#[test]
fn decay_guard_lowers_the_usable_level_by_drain_times_guard() -> R {
    let drain = Rate {
        units: 10,
        per_ms: 1_000,
    };
    let (mut d, id) = one(BucketSpec::decay("d", 10, drain).with_guard_ms(100))?;
    ok(d.try_charge(&[c(id, 9)], Class::Normal, T0))?;
    refused(d.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    let (mut fresh, id) = one(BucketSpec::decay("d", 10, drain).with_guard_ms(100))?;
    let l = refused(fresh.try_charge(&[c(id, 10)], Class::Reduce, T0))?;
    assert_eq!(l.why, Why::Never);
    Ok(())
}

#[test]
fn reserve_is_for_reduce_only_and_a_hold_blocks_every_class() -> R {
    let (mut b, id) = one(w10().with_reserve(2))?;
    for _ in 0..8 {
        ok(b.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    }
    let l = refused(b.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    assert_eq!(l.why, Why::Full);
    ok(b.try_charge(&[c(id, 1)], Class::Reduce, T0))?;
    ok(b.try_charge(&[c(id, 1)], Class::Reduce, T0))?;
    refused(b.try_charge(&[c(id, 1)], Class::Reduce, T0))?;

    let (mut fresh, id) = one(w10().with_reserve(2))?;
    let l = refused(fresh.try_charge(&[c(id, 9)], Class::Normal, T0))?;
    assert_eq!(
        l.why,
        Why::Never,
        "9 can never fit under limit - reserve = 8"
    );
    let l = refused(fresh.try_charge(&[c(id, 11)], Class::Reduce, T0))?;
    assert_eq!(l.why, Why::Never);
    fresh.throttled(id, Some(1_000), T0)?;
    let l = refused(fresh.try_charge(&[c(id, 1)], Class::Reduce, T0 + S))?;
    assert_eq!(l.why, Why::Held, "a cancel must not go into a hold");
    Ok(())
}

#[test]
fn never_for_parts_that_cannot_fit() -> R {
    let (mut b, id) = one(w10())?;
    let l = refused(b.try_charge(&[c(id, 11)], Class::Reduce, T0))?;
    assert_eq!((l.why, l.ready_at_ns), (Why::Never, i64::MAX));
    let mut many = Budget::new(
        (0..=MAX_CHARGES)
            .map(|i| BucketSpec::rolling(format!("b{i}"), 100, 1_000, 10))
            .collect(),
    )?;
    let parts: Vec<Charge> = (0..=MAX_CHARGES)
        .map(|i| Ok(c(many.id(&format!("b{i}")).ok_or("id")?, 1)))
        .collect::<R<_>>()?;
    let l = refused(many.try_charge(&parts, Class::Normal, T0))?;
    assert_eq!((l.why, l.bucket), (Why::Never, parts[MAX_CHARGES].bucket));
    let t = ok(many.try_charge(&parts[..MAX_CHARGES], Class::Normal, T0))?;
    assert_eq!(t.parts().len(), MAX_CHARGES);
    // An id from another, larger budget.
    let stranger = parts[MAX_CHARGES].bucket;
    let l = refused(b.try_charge(&[c(stranger, 1)], Class::Normal, T0))?;
    assert_eq!((l.why, l.bucket), (Why::Never, stranger));
    assert!(b.stats(stranger, T0).is_err());
    assert!(b.throttled(stranger, None, T0).is_err());
    assert!(b.observe_used(stranger, 1, T0).is_err());
    assert!(b.force(c(stranger, 1), T0).is_err());
    Ok(())
}

#[test]
fn an_empty_charge_passes_and_takes_nothing() -> R {
    let (mut b, id) = one(w10())?;
    let t = ok(b.try_charge(&[], Class::Normal, T0))?;
    assert_eq!(t.parts(), []);
    assert_eq!(b.stats(id, T0)?.charged, 0);
    Ok(())
}

#[test]
fn a_long_idle_frees_the_whole_window() -> R {
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0))?;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0 + 100 * S))?;
    refused(b.try_charge(&[c(id, 1)], Class::Normal, T0 + 100 * S))?;
    Ok(())
}

#[test]
fn time_before_zero_works() -> R {
    let (mut b, id) = one(w10())?;
    let t = -5 * S + 50 * MS;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, t))?;
    let l = refused(b.try_charge(&[c(id, 1)], Class::Normal, t))?;
    assert_eq!(l.ready_at_ns, -5 * S + 1_200 * MS);
    ok(b.try_charge(&[c(id, 10)], Class::Normal, l.ready_at_ns))?;
    Ok(())
}

#[test]
fn a_clock_that_goes_back_is_clamped() -> R {
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0 + S))?;
    let ahead = refused(b.try_charge(&[c(id, 1)], Class::Normal, T0 + S))?;
    let back = refused(b.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    assert_eq!(back.ready_at_ns, ahead.ready_at_ns);
    assert_eq!(b.stats(id, T0)?.used, 10);
    ok(b.try_charge(&[c(id, 1)], Class::Normal, ahead.ready_at_ns))?;
    Ok(())
}

#[test]
fn force_charges_past_the_limit() -> R {
    let (mut b, id) = one(w10())?;
    b.force(c(id, 15), T0)?;
    let l = refused(b.try_charge(&[c(id, 1)], Class::Reduce, T0))?;
    assert_eq!(l.why, Why::Full);
    let s = b.stats(id, T0)?;
    assert_eq!((s.used, s.peak, s.forced, s.charged), (15, 15, 15, 0));
    Ok(())
}

// --- refunds and server usage ----------------------------------------------

#[test]
fn refund_gives_back_a_charge_in_the_window_and_is_a_no_op_after_it() -> R {
    let (mut b, id) = one(w10())?;
    let t = ok(b.try_charge(&[c(id, 3)], Class::Normal, T0 + 10 * MS))?;
    assert_eq!(t.at_ns(), T0 + 10 * MS);
    b.refund(&t, T0 + 20 * MS)?;
    let s = b.stats(id, T0 + 20 * MS)?;
    assert_eq!((s.used, s.refunded), (0, 3));

    // Charged in [T0 + 100 ms, T0 + 200 ms): in the ring until T0 + 1300 ms.
    let late = ok(b.try_charge(&[c(id, 3)], Class::Normal, T0 + 150 * MS))?;
    let kept = ok(b.try_charge(&[c(id, 2)], Class::Normal, T0 + 1_299 * MS))?;
    b.refund(&late, T0 + 1_299 * MS)?;
    assert_eq!(b.stats(id, T0 + 1_299 * MS)?.used, 2);
    b.refund(&kept, T0 + 1_300 * MS)?;
    assert_eq!(b.stats(id, T0 + 1_300 * MS)?.used, 0);
    ok(b.try_charge(&[c(id, 4)], Class::Normal, T0 + 1_300 * MS))?;
    b.refund(&late, T0 + 1_300 * MS)?; // expired: must not touch newer charges
    assert_eq!(b.stats(id, T0 + 1_300 * MS)?.used, 4);
    assert_eq!(b.stats(id, T0 + 1_300 * MS)?.refunded, 3 + 3 + 2);
    Ok(())
}

#[test]
fn decay_refund_returns_only_what_has_not_drained() -> R {
    let drain = Rate {
        units: 10,
        per_ms: 1_000,
    };
    let (mut d, id) = one(BucketSpec::decay("d", 10, drain))?;
    let t = ok(d.try_charge(&[c(id, 5)], Class::Normal, T0))?;
    ok(d.try_charge(&[c(id, 5)], Class::Normal, T0))?;
    // After 300 ms, 3 units drained; the refund gives back at most 5 - 3 = 2.
    d.refund(&t, T0 + 300 * MS)?;
    assert_eq!(d.stats(id, T0 + 300 * MS)?.used, 5);
    Ok(())
}

#[test]
fn observe_used_raises_the_count_and_never_lowers_it() -> R {
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 2)], Class::Normal, T0))?;
    b.observe_used(id, 5, T0)?;
    let s = b.stats(id, T0)?;
    assert_eq!((s.used, s.foreign, s.server_used), (5, 3, Some(5)));
    b.observe_used(id, 4, T0)?;
    b.observe_used(id, 0, T0)?;
    let s = b.stats(id, T0)?;
    assert_eq!((s.used, s.foreign, s.server_used), (5, 3, Some(0)));
    assert_eq!(s.peak, 5);
    // Server units are new: they leave with the newest sub-window.
    ok(b.try_charge(&[c(id, 5)], Class::Normal, T0))?;
    refused(b.try_charge(&[c(id, 1)], Class::Normal, T0 + 1_199 * MS))?;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0 + 1_200 * MS))?;
    Ok(())
}

#[test]
fn server_used_is_none_before_any_report() -> R {
    let (b, id) = one(w10())?;
    assert_eq!(b.stats(id, T0)?.server_used, None);
    Ok(())
}

#[test]
fn set_limit_follows_the_server_and_rejects_bad_values() -> R {
    let (mut b, id) = one(w10().with_reserve(2))?;
    ok(b.try_charge(&[c(id, 5)], Class::Normal, T0))?;
    b.set_limit(id, 6)?;
    assert_eq!(b.stats(id, T0)?.limit, 6);
    refused(b.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    ok(b.try_charge(&[c(id, 1)], Class::Reduce, T0))?;
    b.set_limit(id, 20)?;
    ok(b.try_charge(&[c(id, 12)], Class::Normal, T0))?;
    assert!(b.set_limit(id, 0).is_err_and(|e| is_config(&e)));
    assert!(b.set_limit(id, 2).is_err_and(|e| is_config(&e)));
    let (mut g, gid) = one(BucketSpec::gate("ip"))?;
    assert!(g.set_limit(gid, 5).is_err_and(|e| is_config(&e)));
    Ok(())
}

// --- holds ------------------------------------------------------------------

#[test]
fn throttled_holds_for_the_server_wait_plus_guard_and_leaves_the_count_alone() -> R {
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 3)], Class::Normal, T0 + 50 * MS))?;
    let end = b.throttled(id, Some(500), T0 + 60 * MS)?;
    assert_eq!(end, T0 + 660 * MS, "500 ms server wait + 100 ms guard");
    assert_eq!(b.held_until(id, T0 + 60 * MS)?, Some(end));
    let l = refused(b.try_charge(&[c(id, 1)], Class::Normal, T0 + 100 * MS))?;
    assert_eq!((l.why, l.ready_at_ns), (Why::Held, end));
    // The server named the wait, so the hold is the whole cost: the count is
    // still the 3 units charged before it, and a day bucket would not stay
    // empty for a day.
    assert_eq!(b.held_until(id, end)?, None);
    ok(b.try_charge(&[c(id, 7)], Class::Normal, end))?;
    let s = b.stats(id, end)?;
    assert_eq!(s.throttles, 1);
    assert_eq!(s.held_ns, u64::try_from(600 * MS)?);
    assert_eq!(s.foreign, 0, "marking full is not server usage");
    Ok(())
}

#[test]
fn throttled_without_a_wait_marks_the_bucket_full_for_a_window() -> R {
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 3)], Class::Normal, T0 + 50 * MS))?;
    let end = b.throttled(id, None, T0 + 60 * MS)?;
    assert_eq!(end, T0 + 60 * MS + S + 100 * MS, "first backoff + guard");
    assert_eq!(b.held_until(id, end)?, None);
    // No server wait: the local count is not trusted for a window.
    let l = refused(b.try_charge(&[c(id, 1)], Class::Normal, end))?;
    assert_eq!((l.why, l.ready_at_ns), (Why::Full, T0 + 1_200 * MS));
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0 + 1_200 * MS))?;
    Ok(())
}

#[test]
fn throttled_without_a_wait_backs_off_doubling_capped_and_resets_when_quiet() -> R {
    let (mut b, id) = one(BucketSpec::gate("ip"))?;
    let mut now = T0;
    for want_s in [1, 2, 4, 8, 16, 32, 64, 64] {
        let end = b.throttled(id, None, now)?;
        assert_eq!(end - now, want_s * S);
        now = end;
    }
    // A throttle that arrives during a hold is the same episode: no new strike.
    let end = b.throttled(id, None, now)?;
    let again = b.throttled(id, None, now + S)?;
    assert_eq!(again, now + S + 64 * S);
    assert!(again > end);
    // Quiet for the longest backoff after the hold: back to the first.
    let quiet = again + 64 * S;
    assert_eq!(b.throttled(id, None, quiet)? - quiet, S);
    let custom = BucketSpec::gate("x").with_backoff_ms(200, 500);
    let (mut b, id) = one(custom)?;
    let e1 = b.throttled(id, None, T0)?;
    let e2 = b.throttled(id, None, e1)?;
    let e3 = b.throttled(id, None, e2)?;
    assert_eq!((e1 - T0, e2 - e1, e3 - e2), (200 * MS, 400 * MS, 500 * MS));
    Ok(())
}

#[test]
fn hold_the_narrowest_bucket_so_cancels_still_go() -> R {
    let mut b = Budget::new(vec![
        BucketSpec::rolling("ip", 100, 1_000, 10),
        BucketSpec::rolling("orders", 10, 10_000, 10),
    ])?;
    let (ip, orders) = (b.id("ip").ok_or("ip")?, b.id("orders").ok_or("orders")?);
    b.throttled(orders, Some(1_000), T0)?;
    let l = refused(b.try_charge(&[c(ip, 1), c(orders, 1)], Class::Normal, T0 + MS))?;
    assert_eq!((l.bucket, l.why), (orders, Why::Held));
    ok(b.try_charge(&[c(ip, 1)], Class::Reduce, T0 + MS))?;
    assert_eq!(
        b.stats(ip, T0 + MS)?.used,
        1,
        "the refused order took no ip weight"
    );
    Ok(())
}

#[test]
fn a_ban_halts_until_cleared_even_after_the_server_wait() -> R {
    let (mut b, id) = one(w10())?;
    let ten_min = NonZeroU64::new(600_000).ok_or("zero")?;
    let end = b.banned(id, ten_min, T0)?;
    assert_eq!(end, T0 + 600 * S + 100 * MS);
    assert_eq!(b.held_until(id, T0)?, Some(i64::MAX));
    let l = refused(b.try_charge(&[c(id, 1)], Class::Reduce, end + 3_600 * S))?;
    assert_eq!((l.why, l.ready_at_ns), (Why::Banned, i64::MAX));
    let s = b.stats(id, end + 3_600 * S)?;
    assert!(s.halted);
    assert_eq!((s.bans, s.throttles), (1, 0));
    assert_eq!(b.halted().collect::<Vec<_>>(), vec![id]);
    assert!(b.clear_halt(id)?);
    assert!(!b.clear_halt(id)?);
    assert_eq!(b.halted().count(), 0);
    assert!(!b.stats(id, end + 3_600 * S)?.halted);
    ok(b.try_charge(&[c(id, 1)], Class::Normal, end + 3_600 * S))?;

    // Cleared early: the server's wait still holds.
    let t1 = end + 4_000 * S;
    let end = b.banned(id, ten_min, t1)?;
    assert!(b.clear_halt(id)?);
    assert_eq!(b.held_until(id, t1 + S)?, Some(end));
    let l = refused(b.try_charge(&[c(id, 1)], Class::Normal, t1 + S))?;
    assert_eq!(l.why, Why::Held);
    ok(b.try_charge(&[c(id, 1)], Class::Normal, end))?;
    assert_eq!(b.stats(id, end)?.bans, 2);
    Ok(())
}

#[test]
fn reset_clears_the_count_but_keeps_holds_and_halts() -> R {
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 10)], Class::Normal, T0))?;
    let end = b.throttled(id, Some(200), T0)?;
    b.reset(id, T0 + MS)?;
    assert_eq!(b.stats(id, T0 + MS)?.used, 0);
    assert_eq!(
        refused(b.try_charge(&[c(id, 1)], Class::Normal, T0 + MS))?.why,
        Why::Held
    );
    ok(b.try_charge(&[c(id, 10)], Class::Normal, end))?;
    b.banned(id, NonZeroU64::MIN, end)?;
    b.reset(id, end + S)?;
    assert_eq!(
        refused(b.try_charge(&[c(id, 1)], Class::Normal, end + S))?.why,
        Why::Banned
    );
    Ok(())
}

#[test]
fn a_gate_has_only_holds() -> R {
    let (mut g, id) = one(BucketSpec::gate("ip"))?;
    ok(g.try_charge(&[c(id, 1_000_000)], Class::Normal, T0))?;
    let s = g.stats(id, T0)?;
    assert_eq!((s.used, s.limit, s.charged), (0, 0, 1_000_000));
    g.throttled(id, Some(1_000), T0)?;
    assert_eq!(
        refused(g.try_charge(&[c(id, 1)], Class::Normal, T0))?.why,
        Why::Held
    );
    ok(g.try_charge(&[c(id, 1)], Class::Normal, T0 + S))?;
    g.banned(id, NonZeroU64::MIN, T0 + S)?;
    assert_eq!(
        refused(g.try_charge(&[c(id, 1)], Class::Normal, T0 + 2 * S))?.why,
        Why::Banned
    );
    Ok(())
}

// --- re-send within a deadline ---------------------------------------------

#[test]
fn resend_only_inside_the_orders_own_deadline() -> R {
    let (mut b, id) = one(BucketSpec::rolling("orders", 2, 1_000, 10))?;
    let parts = [c(id, 1)];
    // Past its deadline: dropped, nothing charged.
    assert!(matches!(
        b.try_resend(&parts, Class::Normal, T0 - 1, T0),
        Resend::Stale(None)
    ));
    assert_eq!(b.stats(id, T0)?.used, 0);
    // Inside it and the budget allows: send.
    let Resend::Send(t) = b.try_resend(&parts, Class::Normal, T0 + 5 * MS, T0) else {
        return Err("expected Send".into());
    };
    assert_eq!(t.parts(), &parts);
    // Throttled: the hold (and the full mark) outlast a 5 ms deadline.
    b.throttled(id, Some(1), T0)?;
    match b.try_resend(&parts, Class::Normal, T0 + 5 * MS, T0 + MS) {
        Resend::Stale(Some(l)) => assert_eq!(l.why, Why::Held),
        other => return Err(format!("expected Stale(Held), got {other:?}").into()),
    }

    // Refused for now, but free before the deadline: wait, then send.
    let drain = Rate {
        units: 1,
        per_ms: 10,
    };
    let (mut d, id) = one(BucketSpec::decay("d", 1, drain))?;
    ok(d.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    let Resend::Wait(l) = d.try_resend(&[c(id, 1)], Class::Normal, T0 + 20 * MS, T0) else {
        return Err("expected Wait".into());
    };
    assert_eq!(l.ready_at_ns, T0 + 10 * MS);
    assert!(matches!(
        d.try_resend(&[c(id, 1)], Class::Normal, T0 + 20 * MS, l.ready_at_ns),
        Resend::Send(_)
    ));
    // Free exactly at the deadline still counts as inside it.
    let Resend::Wait(l) = d.try_resend(&[c(id, 1)], Class::Normal, T0 + 20 * MS, T0 + 10 * MS)
    else {
        return Err("expected Wait at the deadline".into());
    };
    assert_eq!(l.ready_at_ns, T0 + 20 * MS);
    assert!(matches!(
        d.try_resend(&[c(id, 1)], Class::Normal, T0 + 19 * MS, T0 + 10 * MS),
        Resend::Stale(Some(_))
    ));

    // A halt never ends by itself: stale.
    d.banned(id, NonZeroU64::MIN, T0 + 30 * MS)?;
    match d.try_resend(&[c(id, 1)], Class::Normal, i64::MAX - 1, T0 + 40 * MS) {
        Resend::Stale(Some(l)) => assert_eq!(l.why, Why::Banned),
        other => return Err(format!("expected Stale(Banned), got {other:?}").into()),
    }
    Ok(())
}

// --- configuration ----------------------------------------------------------

#[test]
fn defaults_follow_the_design() {
    let s = BucketSpec::rolling("w", 6_000, 60_000, 60);
    assert_eq!(
        (s.guard_ms, s.reserve, s.backoff_ms),
        (1_000, 0, [1_000, 64_000])
    );
    assert_eq!(
        s.model,
        Model::Rolling {
            limit: 6_000,
            window_ms: 60_000,
            slots: 60
        }
    );
    let d = BucketSpec::decay(
        "d",
        15,
        Rate {
            units: 1,
            per_ms: 3_000,
        },
    );
    assert_eq!(d.guard_ms, 0);
    assert_eq!(BucketSpec::gate("g").model, Model::Gate);
}

#[test]
fn bad_specs_are_config_errors() -> R {
    let rate = Rate {
        units: 10,
        per_ms: 1_000,
    };
    let bad = [
        BucketSpec::rolling("w", 0, 1_000, 10),
        BucketSpec::rolling("w", 10, 0, 10),
        BucketSpec::rolling("w", 10, 1_000, 0),
        BucketSpec::rolling("w", 10, 1_000, 3),
        w10().with_reserve(10),
        w10().with_guard_ms(u32::MAX),
        BucketSpec::decay("d", 0, rate),
        BucketSpec::decay(
            "d",
            10,
            Rate {
                units: 0,
                per_ms: 1,
            },
        ),
        BucketSpec::decay(
            "d",
            10,
            Rate {
                units: 1,
                per_ms: 0,
            },
        ),
        BucketSpec::decay("d", 10, rate).with_reserve(10),
        BucketSpec::decay("d", 10, rate).with_guard_ms(1_000),
        BucketSpec::gate("g").with_reserve(1),
        w10().with_backoff_ms(0, 10),
        w10().with_backoff_ms(10, 5),
    ];
    for spec in bad {
        let shown = format!("{spec:?}");
        match Budget::new(vec![spec]) {
            Err(e) => assert!(is_config(&e), "{shown}: {e}"),
            Ok(_) => return Err(format!("accepted {shown}").into()),
        }
    }
    assert!(Budget::new(Vec::new()).is_err_and(|e| is_config(&e)));
    assert!(Budget::new(vec![w10(), w10()]).is_err_and(|e| is_config(&e)));
    Ok(())
}

#[test]
fn add_a_bucket_after_start() -> R {
    let (mut b, w) = one(w10())?;
    let inst = b.add(BucketSpec::rolling("BTC-USDT", 60, 2_000, 20))?;
    assert_eq!(b.id("BTC-USDT"), Some(inst));
    assert_eq!(b.name(inst), Some("BTC-USDT"));
    ok(b.try_charge(&[c(w, 1), c(inst, 60)], Class::Normal, T0))?;
    refused(b.try_charge(&[c(inst, 1)], Class::Normal, T0))?;
    assert!(
        b.add(BucketSpec::rolling("BTC-USDT", 1, 1_000, 10))
            .is_err_and(|e| is_config(&e))
    );
    assert!(
        b.add(BucketSpec::rolling("bad", 0, 1_000, 10))
            .is_err_and(|e| is_config(&e))
    );
    assert_eq!(b.id("bad"), None);
    Ok(())
}

#[test]
fn stats_count_charges_refusals_and_peaks() -> R {
    let (mut b, id) = one(w10())?;
    ok(b.try_charge(&[c(id, 4)], Class::Normal, T0))?;
    ok(b.try_charge(&[c(id, 6)], Class::Normal, T0))?;
    refused(b.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    refused(b.try_charge(&[c(id, 1)], Class::Normal, T0))?;
    let s = b.stats(id, T0)?;
    assert_eq!(
        (s.charged, s.limited, s.used, s.peak, s.limit),
        (10, 2, 10, 10, 10)
    );
    let later = b.stats(id, T0 + 5 * S)?;
    assert_eq!((later.used, later.peak), (0, 10));
    assert_eq!(later.held_until_ns, None);
    Ok(())
}

#[test]
fn limited_is_an_error_with_a_message() -> R {
    let (_, id) = one(w10())?;
    let l = Limited {
        bucket: id,
        ready_at_ns: 5,
        why: Why::Full,
    };
    let e: Box<dyn std::error::Error> = Box::new(l);
    assert!(e.to_string().contains("full"), "{e}");
    Ok(())
}

// ---- SourceIps: several source IPs, each with its own budget --------------

/// Three source IPs, each with its own connect bucket (2 a second) and its
/// own request bucket, in one budget with the account's bucket.
fn three_ips() -> R<(Budget, SourceIps, [IpAddr; 3])> {
    let ips: [IpAddr; 3] = [
        "10.0.0.1".parse()?,
        "10.0.0.2".parse()?,
        "10.0.0.3".parse()?,
    ];
    let mut specs = vec![BucketSpec::rolling("account-orders", 100, 10_000, 10)];
    for ip in ips {
        specs.push(BucketSpec::rolling(format!("connects {ip}"), 2, 1_000, 10));
        specs.push(BucketSpec::rolling(
            format!("weight {ip}"),
            6_000,
            60_000,
            60,
        ));
    }
    let budget = Budget::new(specs)?;
    let mut sources = Vec::new();
    for ip in ips {
        let id = |name: String| budget.id(&name).ok_or("no bucket");
        sources.push(SourceIp {
            ip,
            connects: id(format!("connects {ip}"))?,
            sends: Some(id(format!("weight {ip}"))?),
        });
    }
    Ok((budget, SourceIps::new(sources)?, ips))
}

/// Connects go round the IPs, each charged to its own bucket: six pass at
/// once (two per IP), the seventh is refused until the soonest IP frees,
/// and then that IP passes.
#[test]
fn source_ips_spread_connects_each_within_its_own_budget() -> R {
    let (mut budget, mut pool, ips) = three_ips()?;
    let mut picked = Vec::new();
    for _ in 0..6 {
        let p = pool
            .try_connect(&mut budget, T0)
            .map_err(|l| format!("refused: {l}"))?;
        assert_eq!(p.ticket.parts().len(), 1);
        assert_eq!(
            Some(p.ticket.parts()[0].bucket),
            pool.get(p.ip).map(|s| s.connects)
        );
        picked.push(p.ip);
    }
    assert_eq!(picked, [ips[0], ips[1], ips[2], ips[0], ips[1], ips[2]]);
    for ip in ips {
        let s = pool.get(ip).ok_or("no ip")?;
        assert_eq!(budget.stats(s.connects, T0)?.used, 2, "{ip}");
    }
    let Err(l) = pool.try_connect(&mut budget, T0) else {
        return Err("a seventh connect passed".into());
    };
    assert_eq!(l.why, Why::Full);
    assert!(l.ready_at_ns > T0, "{l}");
    // Each IP was asked once and refused; nothing was charged.
    for ip in ips {
        let s = pool.get(ip).ok_or("no ip")?;
        assert_eq!(budget.stats(s.connects, T0)?.used, 2, "{ip}");
    }
    let p = pool
        .try_connect(&mut budget, l.ready_at_ns)
        .map_err(|e| format!("still refused at ready_at_ns: {e}"))?;
    assert!(ips.contains(&p.ip));
    Ok(())
}

/// A throttled IP is passed over; the others keep connecting. A connect
/// that never went out is refunded with its ticket.
#[test]
fn a_held_source_ip_is_passed_over_and_a_refund_gives_back_its_connect() -> R {
    let (mut budget, mut pool, ips) = three_ips()?;
    let held = pool.get(ips[1]).ok_or("no ip")?.connects;
    budget.throttled(held, Some(5_000), T0)?;
    let mut picked = Vec::new();
    for _ in 0..4 {
        let p = pool
            .try_connect(&mut budget, T0)
            .map_err(|l| l.to_string())?;
        picked.push(p.ip);
    }
    assert_eq!(picked, [ips[0], ips[2], ips[0], ips[2]]);
    let Err(l) = pool.try_connect(&mut budget, T0) else {
        return Err("a fifth connect passed".into());
    };
    // The soonest is a full IP freeing in about a second, not the held one.
    assert_eq!(l.why, Why::Full);
    assert!(l.ready_at_ns < T0 + 5_000 * MS, "{l}");

    let (mut budget, mut pool, _) = three_ips()?;
    let p = pool
        .try_connect(&mut budget, T0)
        .map_err(|l| l.to_string())?;
    let connects = pool.get(p.ip).ok_or("no ip")?.connects;
    assert_eq!(budget.stats(connects, T0)?.used, 1);
    budget.refund(&p.ticket, T0)?;
    assert_eq!(budget.stats(connects, T0)?.used, 0);
    Ok(())
}

/// The IP a connection is bound to names its request bucket: requests are
/// charged to that IP's weight and the account's count together.
#[test]
fn a_request_charges_its_connections_source_ip() -> R {
    let (mut budget, mut pool, ips) = three_ips()?;
    let orders = budget.id("account-orders").ok_or("no bucket")?;
    let p = pool
        .try_connect(&mut budget, T0)
        .map_err(|l| l.to_string())?;
    let weight = pool
        .get(p.ip)
        .and_then(|s| s.sends)
        .ok_or("no sends bucket")?;
    budget
        .try_charge(&[c(weight, 5), c(orders, 1)], Class::Normal, T0)
        .map_err(|l| l.to_string())?;
    assert_eq!(budget.stats(weight, T0)?.used, 5);
    for other in &ips[1..] {
        let w = pool.get(*other).and_then(|s| s.sends).ok_or("no sends")?;
        assert_eq!(budget.stats(w, T0)?.used, 0, "{other}");
    }
    assert_eq!(pool.ips().len(), 3);
    Ok(())
}

#[test]
fn source_ips_refuse_a_bad_list() -> R {
    let (budget, _, _) = three_ips()?;
    let a = budget.id("connects 10.0.0.1").ok_or("no bucket")?;
    let b = budget.id("connects 10.0.0.2").ok_or("no bucket")?;
    let ip1: IpAddr = "10.0.0.1".parse()?;
    let ip2: IpAddr = "10.0.0.2".parse()?;
    let src = |ip, connects| SourceIp {
        ip,
        connects,
        sends: None,
    };
    for (list, why) in [
        (vec![], "no source IPs"),
        (vec![src(ip1, a), src(ip1, b)], "listed twice"),
        (vec![src(ip1, a), src(ip2, a)], "share a connects bucket"),
        (vec![src("0.0.0.0".parse()?, a)], "unspecified"),
    ] {
        match SourceIps::new(list) {
            Err(Error::Config(m)) => assert!(m.contains(why), "{why}: {m}"),
            other => return Err(format!("{why}: {:?}", other.err()).into()),
        }
    }
    Ok(())
}
