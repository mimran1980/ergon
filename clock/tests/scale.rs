//! The conversion: ticks to nanoseconds by one 32.32 multiply, exactly the
//! formula the DPDK transport's C clock uses, so the two can share one
//! calibration.
#![allow(missing_docs)]

use ergon_clock::advanced::Scale;

const NS: u64 = 1_000_000_000;

/// A 1 GHz counter (Graviton's `CNTFRQ_EL0`) is one nanosecond a tick:
/// its factor is exactly 2^32, so the conversion is exact.
#[test]
fn a_1ghz_counter_is_one_nanosecond_a_tick() -> Result<(), Box<dyn std::error::Error>> {
    let s = Scale::from_hz(NS, 1_000, 5_000).ok_or("1 GHz refused")?;
    assert_eq!(s.mult(), 1 << 32);
    assert_eq!(s.hz(), NS);
    assert_eq!(s.to_ns(1_000), 5_000);
    assert_eq!(s.to_ns(1_000 + NS), 5_000 + NS);
    assert_eq!(s.ticks_to_ns(123_456_789), 123_456_789);
    assert_eq!(s.ns_to_ticks(123_456_789), 123_456_789);
    Ok(())
}

/// A second of ticks is a second within a nanosecond at the rates the lab
/// sees: Apple silicon's 24 MHz, a 2.4 GHz TSC (c7i), an odd 2.899 GHz one.
#[test]
fn a_second_of_ticks_is_a_second() -> Result<(), Box<dyn std::error::Error>> {
    for hz in [
        24_000_000,
        2_400_000_000,
        2_899_999_000,
        1_000_000_000,
        3_000_000_000,
    ] {
        let s = Scale::from_hz(hz, 0, 0).ok_or("refused")?;
        let ns = s.ticks_to_ns(hz);
        assert!(ns.abs_diff(NS) <= 1, "{hz} Hz: {hz} ticks are {ns} ns");
        // and 100 s of them within a microsecond (the factor's rounding)
        let ns = s.ticks_to_ns(hz * 100);
        assert!(
            ns.abs_diff(100 * NS) <= 1_000,
            "{hz} Hz: 100 s of ticks are {ns} ns"
        );
        assert_eq!(s.hz(), hz, "the rate read back from the factor");
    }
    Ok(())
}

/// The formula the C transport uses (`ns_base + ((t - base) * mult) >> 32`,
/// a u128 product), checked against an independent u128 computation over
/// values that exercise the product's high bits.
#[test]
fn the_conversion_is_the_transport_formula() -> Result<(), Box<dyn std::error::Error>> {
    let mut x = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = || {
        // xorshift64*: deterministic, no dependency
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };
    for hz in [24_000_000_u64, 1_000_000_000, 2_400_000_000, 3_700_000_000] {
        let base_ticks = next() >> 8;
        let base_ns = next() >> 4;
        let s = Scale::from_hz(hz, base_ticks, base_ns).ok_or("refused")?;
        for _ in 0..10_000 {
            // up to ~2^50 ticks ahead: days at GHz rates
            let t = base_ticks + (next() >> 14);
            let want = base_ns.wrapping_add(u64::try_from(
                (u128::from(t - base_ticks) * u128::from(s.mult())) >> 32,
            )?);
            assert_eq!(s.to_ns(t), want, "{hz} Hz, {t} ticks");
        }
    }
    Ok(())
}

/// An unordered read on another core can come back a little behind the
/// calibration's base: it converts to a little before the base time (the
/// signed product's shift floors, so within a nanosecond of exact), never to
/// a wrapped, enormous value.
#[test]
fn a_tick_behind_the_base_is_a_little_earlier_not_a_wrap() -> Result<(), Box<dyn std::error::Error>>
{
    let s = Scale::from_hz(NS, 1_000_000, 5_000_000).ok_or("refused")?;
    assert_eq!(s.to_ns(999_990), 4_999_990);
    let s = Scale::from_hz(2_400_000_000, 1_000_000, 5_000_000).ok_or("refused")?;
    let ns = s.to_ns(1_000_000 - 24);
    assert!(
        (4_999_989..=4_999_990).contains(&ns),
        "24 ticks at 2.4 GHz before the base: {ns}"
    );
    Ok(())
}

/// Durations to ticks, for thresholds precomputed once (rate gates, stall
/// limits): the inverse of the conversion within a tick, or within the
/// ticks of one nanosecond where a tick is shorter (whole nanoseconds lose
/// up to one).
#[test]
fn nanoseconds_to_ticks_inverts_the_conversion() -> Result<(), Box<dyn std::error::Error>> {
    for hz in [24_000_000_u64, 1_000_000_000, 2_400_000_000, 3_700_000_000] {
        let s = Scale::from_hz(hz, 0, 0).ok_or("refused")?;
        for ticks in [0_u64, 1, 7, 1_000, 123_456_789, 10_u64.pow(15)] {
            let back = s.ns_to_ticks(s.ticks_to_ns(ticks));
            assert!(
                back.abs_diff(ticks) <= hz.div_ceil(NS).max(1),
                "{hz} Hz: {ticks} ticks -> {back}"
            );
        }
        assert_eq!(s.ns_to_ticks(NS).abs_diff(hz), 0, "{hz} Hz: a second");
    }
    Ok(())
}

/// No rate below 1 kHz or above 100 GHz (a register firmware never set
/// reads 0), no factor of 0 or of 2^63 and over (the conversion's product
/// is signed).
#[test]
fn implausible_rates_and_factors_are_refused() {
    for hz in [0, 1, 999, 100_000_000_001, u64::MAX] {
        assert!(Scale::from_hz(hz, 0, 0).is_none(), "{hz} Hz accepted");
    }
    assert!(Scale::from_hz(1_000, 0, 0).is_some());
    assert!(Scale::from_hz(100_000_000_000, 0, 0).is_some());
    for mult in [0, 1 << 63, u64::MAX] {
        assert!(
            Scale::from_mult(mult, 0, 0).is_none(),
            "factor {mult} accepted"
        );
    }
    assert!(Scale::from_mult(1 << 32, 0, 0).is_some());
}

/// P3: a re-anchor reads at its anchor what the old segment reads there,
/// plus the forward step, then runs at the new factor; the base is the old
/// segment's reading, not the old base. A factor out of range, or a time
/// that would overflow, is refused.
#[test]
fn a_reanchor_continues_from_where_the_old_segment_reads() -> Result<(), Box<dyn std::error::Error>>
{
    let mut x = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };
    for _ in 0..100_000 {
        let hz = 24_000_000 + next() % 3_000_000_000;
        let base_ticks = next() >> 4;
        let base_ns = (next() >> 4) + NS;
        let s = Scale::from_hz(hz, base_ticks, base_ns).ok_or("refused")?;
        // an anchor up to a day either side of the base
        let at = base_ticks
            .wrapping_add(next() % (hz * 86_400))
            .wrapping_sub(hz * 43_200);
        let mult = s.mult() - s.mult() / 2_000 + next() % (s.mult() / 1_000 + 1);
        let forward = if next() % 2 == 0 {
            0
        } else {
            next() % (10 * NS)
        };
        let r = s.retuned(at, mult, forward).ok_or("retune refused")?;
        assert_eq!(
            r.to_ns(at),
            s.to_ns(at) + forward,
            "continuity at the anchor"
        );
        assert_eq!(r.base_ticks(), at);
        assert_eq!(r.mult(), mult);
        let later = at + next() % (hz * 10);
        assert_eq!(
            r.to_ns(later) - r.to_ns(at),
            r.ticks_to_ns(later - at),
            "the new factor from the anchor"
        );
    }
    let s = Scale::from_hz(NS, 1_000, 5_000).ok_or("refused")?;
    for mult in [
        0,
        1 << 63,
        u64::MAX,
        Scale::from_hz(999, 0, 0).map_or(u64::MAX, |s| s.mult()),
    ] {
        assert!(
            s.retuned(2_000, mult, 0).is_none(),
            "factor {mult} accepted"
        );
    }
    assert!(
        s.retuned(2_000, 1 << 32, u64::MAX).is_none(),
        "overflow accepted"
    );
    assert_eq!(
        s.retuned(1_000, 1 << 32, 0),
        Some(s),
        "a re-anchor at the base, same factor: the same"
    );
    Ok(())
}
