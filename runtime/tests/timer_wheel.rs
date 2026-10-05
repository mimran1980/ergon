//! The wheel's contract through its public API, beyond the unit tests: ids
//! and a repeating id across slab doublings, a jump over crowded spokes, a
//! tick-aligned model, a wide wheel against a sorted model, and a prefaulted
//! wheel.

use std::collections::BTreeMap;

use ergon_runtime::clock::Nanos;
use ergon_runtime::timer::{Fired, Settings, TimerError, TimerId, TimerWheel};
use proptest::prelude::*;

const TICK: i64 = 1024;

fn wheel(spokes: u32, per: u32) -> Result<TimerWheel, TimerError> {
    TimerWheel::new(Settings {
        tick_ns: TICK,
        ticks_per_wheel: spokes,
        timers_per_spoke: per,
    })
}

#[test]
fn ids_issued_before_growth_cancel_and_fire_after_it() -> Result<(), TimerError> {
    // 8 records: the ninth live timer doubles the slab, the seventeenth
    // doubles it again.
    let mut w = wheel(8, 1)?;
    let mut fired = Vec::new();
    let repeating = w.schedule_repeating(Nanos(10_000), 5_000, 99)?;
    let mut before = Vec::new();
    for i in 0..7_i64 {
        before.push(w.schedule(Nanos(20_000 + 1_000 * i), i.cast_unsigned())?);
    }
    for i in 0..10_i64 {
        w.schedule(Nanos(40_000 + 1_000 * i), 100 + i.cast_unsigned())?;
    }
    assert_eq!(w.len(), 18, "more than the 16 records of one doubling");

    assert!(w.cancel(before[0]));
    assert!(!w.cancel(before[0]), "a cancelled id stays dead");

    assert_eq!(w.poll(Nanos(10_000), &mut fired, 64), 1);
    assert_eq!(fired[0].id, repeating);
    assert_eq!(w.poll(Nanos(15_000), &mut fired, 64), 1);
    assert_eq!(fired[0].id, repeating, "a re-arm keeps the pre-growth id");
    assert!(w.cancel(repeating));

    assert_eq!(w.next_deadline(), Some(Nanos(21_000)));
    let n = w.poll(Nanos(1_000_000), &mut fired, 64);
    let tokens: Vec<u64> = fired.iter().map(|f| f.token).collect();
    let want: Vec<u64> = (1..7).chain(100..110).collect();
    assert_eq!(n, want.len());
    assert_eq!(tokens, want, "(deadline, seq) order across the growth");
    assert!(
        fired
            .iter()
            .take(6)
            .zip(&before[1..])
            .all(|(f, id)| f.id == *id)
    );
    assert!(w.is_empty());
    assert_eq!(w.next_deadline(), None);
    Ok(())
}

#[test]
fn ids_from_before_a_doubling_still_cancel() -> Result<(), TimerError> {
    let mut w = wheel(8, 1)?;
    // 200 timers on one spoke: the slab doubles from 8 records to 256.
    let ids = (0..200)
        .map(|token| w.schedule(Nanos(5 * TICK), token))
        .collect::<Result<Vec<_>, _>>()?;
    for id in ids.iter().step_by(2) {
        assert!(w.cancel(*id), "a pre-growth id must still name its timer");
        assert!(!w.cancel(*id));
    }
    let mut fired = Vec::new();
    assert_eq!(w.poll(Nanos(5 * TICK), &mut fired, 256), 100);
    let tokens: Vec<u64> = fired.iter().map(|f| f.token).collect();
    let want: Vec<u64> = (1..200).step_by(2).collect();
    assert_eq!(tokens, want, "survivors fire in schedule order");
    assert!(w.is_empty());
    Ok(())
}

#[test]
fn a_repeating_id_survives_growth_and_moves() -> Result<(), TimerError> {
    let mut w = wheel(8, 1)?;
    let rep = w.schedule_repeating(Nanos(TICK), TICK, 7)?;
    let mut fired = Vec::new();
    for step in 1..40 {
        // One timer far out per step keeps the slab growing while the
        // repeating timer moves to the next spoke on every firing.
        w.schedule(Nanos(1_000_000 * TICK + step), 200)?;
        w.schedule(Nanos((step + 1) * TICK), 100)?;
        // The previous step's one-shot is due beside the repeating timer.
        let due = if step == 1 { 1 } else { 2 };
        assert_eq!(w.poll(Nanos(step * TICK), &mut fired, 64), due);
        let mine: Vec<&Fired> = fired.iter().filter(|f| f.token == 7).collect();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].id, rep);
        assert_eq!(mine[0].deadline, Nanos(step * TICK));
    }
    assert!(w.len() > 32, "the slab doubled from 8 records to 64");
    assert!(w.cancel(rep));
    assert!(!w.cancel(rep));
    Ok(())
}

#[test]
fn a_jump_over_crowded_spokes_fires_everything_once() -> Result<(), TimerError> {
    let mut w = wheel(8, 2)?;
    for token in 0..150 {
        w.schedule(Nanos(3 * TICK + token), token.cast_unsigned())?;
    }
    for token in 0..70 {
        w.schedule(Nanos(6 * TICK + token), 1000 + token.cast_unsigned())?;
    }
    let mut fired = Vec::new();
    let now = 1_000 * 8 * TICK;
    assert_eq!(w.poll(Nanos(now), &mut fired, 1_000), 220);
    assert_eq!(w.spokes_visited(), 2);
    let deadlines: Vec<i64> = fired.iter().map(|f| f.deadline.0).collect();
    let mut sorted = deadlines.clone();
    sorted.sort_unstable();
    assert_eq!(deadlines, sorted);
    assert_eq!(w.next_deadline(), None);
    Ok(())
}

struct Model {
    id: TimerId,
    token: u64,
    period: i64,
}

/// The unit tests' sorted model with every deadline and period a multiple of
/// the tick, so each re-arm lands on the start of a later spoke, and one
/// record per spoke, so the slab doubles.
fn aligned_script(seed: u64, ops: usize) -> Result<(), TimerError> {
    let mut w = wheel(16, 1)?;
    let mut fired = Vec::new();
    let mut model: BTreeMap<(i64, u32), Model> = BTreeMap::new();
    let mut by_id: BTreeMap<TimerId, (i64, u32)> = BTreeMap::new();
    let mut rng = seed | 1;
    let mut floor = 1_000_000 * TICK;
    for _ in 0..ops {
        rng = xorshift(rng);
        let roll = rng % 100;
        rng = xorshift(rng);
        if roll < 50 {
            let repeating = rng.is_multiple_of(3);
            rng = xorshift(rng);
            let deadline = floor + TICK * i64::try_from(rng % 60).unwrap_or(0);
            rng = xorshift(rng);
            let token = rng;
            rng = xorshift(rng);
            let period = TICK * (1 + i64::try_from(rng % 20).unwrap_or(0));
            let id = if repeating {
                w.schedule_repeating(Nanos(deadline), period, token)?
            } else {
                w.schedule(Nanos(deadline), token)?
            };
            let period = if repeating { period } else { 0 };
            model.insert((deadline, id.seq()), Model { id, token, period });
            by_id.insert(id, (deadline, id.seq()));
        } else if roll < 65 {
            let pick = usize::try_from(rng).unwrap_or(0) % by_id.len().max(1);
            if let Some(id) = by_id.keys().nth(pick).copied() {
                let key = by_id.remove(&id);
                assert!(w.cancel(id), "seed {seed}");
                if let Some(key) = key {
                    model.remove(&key);
                }
            }
        } else {
            let jump = if rng.is_multiple_of(7) {
                40 * TICK
            } else {
                TICK
            };
            rng = xorshift(rng);
            let now = floor + jump + i64::try_from(rng % 3).unwrap_or(0) * TICK;
            floor = now;
            let n = w.poll(Nanos(now), &mut fired, usize::MAX);
            let due: Vec<(i64, u32)> = model.range(..=(now, u32::MAX)).map(|(k, _)| *k).collect();
            assert_eq!(n, due.len(), "seed {seed}");
            for (key, got) in due.into_iter().zip(fired.iter()) {
                let item = model.remove(&key).ok_or(TimerError::Capacity)?;
                by_id.remove(&item.id);
                let (missed, next) = if item.period == 0 {
                    (0, key.0)
                } else {
                    let missed = (now - key.0) / item.period;
                    (missed.cast_unsigned(), key.0 + (missed + 1) * item.period)
                };
                assert_eq!(
                    (got.id, got.token, got.deadline.0, got.missed, got.period),
                    (item.id, item.token, key.0, missed, item.period),
                    "seed {seed}"
                );
                if item.period > 0 {
                    let id = item.id;
                    model.insert((next, key.1), item);
                    by_id.insert(id, (next, key.1));
                }
            }
        }
        let want = model.keys().next().map(|(deadline, _)| Nanos(*deadline));
        assert_eq!(w.next_deadline(), want, "seed {seed}");
        assert_eq!(w.len(), model.len(), "seed {seed}");
    }
    Ok(())
}

#[test]
fn aligned_random_ops_match_a_sorted_model() -> Result<(), TimerError> {
    aligned_script(0x5eed_1234, 3_000)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, .. ProptestConfig::default() })]
    #[test]
    fn aligned_property_matches_a_sorted_model(seed in 1u64..u64::MAX) {
        let result = aligned_script(seed, 600);
        prop_assert!(result.is_ok(), "{result:?}");
    }
}

#[test]
fn a_wide_wheel_matches_a_sorted_model() -> Result<(), TimerError> {
    // 512 spokes, so a head scan crosses many chunks; one record per spoke,
    // so the slab doubles; deadlines and jumps of many revolutions.
    let mut w = wheel(512, 1)?;
    let revolution = TICK * 512;
    let mut now = 1_000_000_000_000_i64;
    let mut fired = Vec::new();
    let mut model: BTreeMap<(i64, u32), (TimerId, u64, i64)> = BTreeMap::new();
    let mut keys: BTreeMap<TimerId, (i64, u32)> = BTreeMap::new();
    let mut ids = Vec::new();
    let mut rng = 0x9e37_79b9_7f4a_7c15_u64;
    for _ in 0..20_000 {
        rng = xorshift(rng);
        let r = i64::try_from(rng >> 16).unwrap_or(0);
        match rng % 10 {
            0..=4 => {
                let reach = if rng.is_multiple_of(7) {
                    20 * revolution
                } else {
                    revolution
                };
                let deadline = now + r % reach;
                let period = if rng.is_multiple_of(3) {
                    1 + r % (2 * revolution)
                } else {
                    0
                };
                let id = if period == 0 {
                    w.schedule(Nanos(deadline), rng)?
                } else {
                    w.schedule_repeating(Nanos(deadline), period, rng)?
                };
                model.insert((deadline, id.seq()), (id, rng, period));
                keys.insert(id, (deadline, id.seq()));
                ids.push(id);
            }
            5 | 6 if !ids.is_empty() => {
                let pick = usize::try_from(rng % 1_000).unwrap_or(0) % ids.len();
                let id = ids.swap_remove(pick);
                let key = keys.remove(&id);
                assert_eq!(w.cancel(id), key.is_some());
                if let Some(key) = key {
                    model.remove(&key);
                }
            }
            _ => {
                now += if rng.is_multiple_of(5) {
                    r % (30 * revolution)
                } else {
                    r % 4096
                };
                let n = w.poll(Nanos(now), &mut fired, usize::MAX);
                let due: Vec<(i64, u32)> = model
                    .range(..=(now, u32::MAX))
                    .map(|(key, _)| *key)
                    .collect();
                assert_eq!(n, due.len());
                for (got, key) in fired.iter().zip(due) {
                    let (id, token, period) = model.remove(&key).ok_or(TimerError::Capacity)?;
                    keys.remove(&id);
                    let missed = if period == 0 {
                        0
                    } else {
                        (now - key.0) / period
                    };
                    assert_eq!(
                        (got.id, got.token, got.deadline.0, got.missed, got.period),
                        (id, token, key.0, missed.cast_unsigned(), period)
                    );
                    if period > 0 {
                        let next = key.0 + (missed + 1) * period;
                        model.insert((next, key.1), (id, token, period));
                        keys.insert(id, (next, key.1));
                    }
                }
            }
        }
        assert_eq!(w.next_deadline(), model.keys().next().map(|k| Nanos(k.0)));
        assert_eq!(w.len(), model.len());
    }
    Ok(())
}

#[test]
fn prefault_leaves_a_working_wheel() -> Result<(), TimerError> {
    let mut w = TimerWheel::new(Settings::default())?;
    let mut fired = Vec::new();
    let id = w.schedule(Nanos(5_000), 1)?;
    assert!(w.cancel(id));
    w.prefault();
    let again = w.schedule(Nanos(6_000), 2)?;
    assert_eq!(w.poll(Nanos(6_000), &mut fired, 8), 1);
    assert_eq!(fired[0].id, again);
    assert!(!w.cancel(id));
    Ok(())
}

const fn xorshift(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}
