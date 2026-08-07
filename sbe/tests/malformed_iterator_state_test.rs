//! Malformed iteration must be terminal and truthful.
//!
//! Both decoders here walk a stream by deriving each position from the length
//! of the thing before it. Once one of those lengths fails to decode, the
//! boundary is gone: every later offset would be a guess. So neither decoder is
//! allowed to imply progress after its framing becomes untrusted.
//!
//! - A dynamic group stores the first entry error, yields it once, then reports
//!   itself finished. `finish` / `skip_remaining` return the stored error rather
//!   than building a later message stage at a meaningless offset. Only an
//!   explicit `rewind` — back to the start proven at wrap time — clears it.
//! - `FrameCursor` fuses on any prefix, bounds, or frame-decode error: exactly
//!   one `Err`, then permanent `None`.
//!
//! It also pins the size contract: a dynamic group is not `ExactSizeIterator`,
//! because a size-based allocation must not trust a count the wire has not
//! justified.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]

mod common;
use common::{Paths, compile_and_run, compile_fails_with_diagnostics, generate};

// ─── Size contract ─────────────────────────────────────────────────────────

#[test]
fn dynamic_groups_are_not_exact_size_iterators() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "mi_exact_dyn");
    compile_fails_with_diagnostics(
        "mi_exact_dyn",
        &src,
        r#"
        fn needs_exact<I: ExactSizeIterator>(_: I) {}

        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&6u16.to_le_bytes());
        buf[2..4].copy_from_slice(&0u16.to_le_bytes());
        let group = FuelFiguresDecoder::wrap(&buf, 0, 0).unwrap();
        // ILLEGAL: fuelFigures entries carry var-data, so the count is a claim
        // the wire has not yet justified.
        needs_exact(group);
        "#,
        &["ExactSizeIterator"],
    );
    Ok(())
}

#[test]
fn fixed_stride_groups_keep_exact_size() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "mi_exact_fixed");
    compile_and_run(
        "mi_exact_fixed",
        &src,
        r#"
        fn exact_len<I: ExactSizeIterator>(iter: I) -> usize { iter.len() }

        // `acceleration` is flat: the whole region was proven at wrap, so the
        // count is trustworthy and the exact size is kept.
        let mut buf = [0u8; 4 + 12];
        buf[0..2].copy_from_slice(&6u16.to_le_bytes());
        buf[2..4].copy_from_slice(&2u16.to_le_bytes());
        let group = PerformanceFiguresAccelerationDecoder::wrap(&buf, 0, 0)?;
        assert_eq!(exact_len(group), 2);
        "#,
    );
    Ok(())
}

// ─── Poisoned group progression ────────────────────────────────────────────

#[test]
fn malformed_entry_yields_one_error_then_reports_finished() -> Result<(), Box<dyn std::error::Error>>
{
    let (_, src) = generate(&Paths::example_schema(), "mi_poison");
    compile_and_run(
        "mi_poison",
        &src,
        r#"
        // Two declared fuelFigures entries. The first entry's var-data length
        // prefix claims far more bytes than the buffer holds, so its extent
        // cannot be decoded — and with it, the start of the second entry.
        let mut buf = [0u8; 4 + 6 + 4 + 2];
        buf[0..2].copy_from_slice(&6u16.to_le_bytes());  // entry blockLength
        buf[2..4].copy_from_slice(&2u16.to_le_bytes());  // count = 2
        // entry 0 fixed block (speed + mpg) is fine…
        buf[4..6].copy_from_slice(&30u16.to_le_bytes());
        buf[6..10].copy_from_slice(&35.9f32.to_le_bytes());
        // …but its var-data length prefix is a lie.
        buf[10..14].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());

        let mut group = FuelFiguresDecoder::wrap(&buf, 0, 0)?;
        assert_eq!(group.remaining(), 2);

        // Exactly one error…
        let first = group.next();
        assert!(matches!(first, Some(Err(_))), "first poll must surface the error");

        // …then finished. Not the same error forever, and not a second entry
        // decoded from an offset the failed entry never established.
        assert!(group.next().is_none(), "a poisoned group must not yield again");
        assert!(group.next().is_none(), "and must stay finished");

        // size_hint is conservative and collapses once poisoned.
        assert_eq!(group.size_hint(), (0, Some(0)));

        // Completion returns the stored error instead of a later stage.
        let mut poisoned = FuelFiguresDecoder::wrap(&buf, 0, 0)?;
        let _ = poisoned.next();
        // (finish is only reachable on an attached group — see below.)
        "#,
    );
    Ok(())
}

#[test]
fn poisoned_group_completion_returns_the_stored_error() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "mi_finish");
    compile_and_run(
        "mi_finish",
        &src,
        r#"
        // A full Car frame, then the first fuelFigures var-data prefix is
        // corrupted so the group cannot establish its own extent.
        let mut buf = [0u8; 512];
        let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F,
                                    Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"city").unwrap(); Ok(()) })?;
                g.add(|mut e| { e.speed(60).mpg(28.1); e.usage_description(b"road").unwrap(); Ok(()) })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"H")?
            .model(b"C")?
            .activation_code(b"A")?
            .encoded_length_with_header();

        // Sanity: the untouched frame walks cleanly to the next stage.
        let clean = CarDecoder::try_from(&buf[..len])?.into_fuel_figures()?;
        assert!(clean.finish().is_ok());

        // Corrupt the first entry's var-data length prefix.
        let group_start = 8 + 45;
        let prefix = group_start + 4 + 6; // dimension + first fixed block
        buf[prefix..prefix + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());

        let mut group = CarDecoder::try_from(&buf[..len])?.into_fuel_figures()?;
        let first = group.next();
        assert!(matches!(first, Some(Err(_))), "the corrupt entry must error");
        assert!(group.next().is_none(), "and must not yield again");

        // finish() must not fabricate the next stage at the failed position.
        let Err(_) = group.finish() else {
            panic!("a poisoned group must not complete into a message stage");
        };

        // skip_remaining() reports the same failure.
        let mut group = CarDecoder::try_from(&buf[..len])?.into_fuel_figures()?;
        let _ = group.next();
        let Err(_) = group.skip_remaining() else {
            panic!("skip_remaining must not complete a poisoned group either");
        };
        "#,
    );
    Ok(())
}

#[test]
fn rewind_clears_poison_and_retries_from_the_proven_start() -> Result<(), Box<dyn std::error::Error>>
{
    let (_, src) = generate(&Paths::example_schema(), "mi_rewind");
    compile_and_run(
        "mi_rewind",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F,
                                    Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"city").unwrap(); Ok(()) })?;
                g.add(|mut e| { e.speed(60).mpg(28.1); e.usage_description(b"road").unwrap(); Ok(()) })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"H")?
            .model(b"C")?
            .activation_code(b"A")?
            .encoded_length_with_header();

        let mut saved = [0u8; 512];
        saved[..len].copy_from_slice(&buf[..len]);

        let group_start = 8 + 45;
        let prefix = group_start + 4 + 6;
        buf[prefix..prefix + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());

        let mut group = CarDecoder::try_from(&buf[..len])?.into_fuel_figures()?;
        assert!(matches!(group.next(), Some(Err(_))));
        assert!(group.next().is_none());

        // Rewinding a still-broken buffer re-runs from the proven start and
        // hits the same wall — it clears poison, it does not repair the wire.
        group.rewind();
        assert_eq!(group.remaining(), 2, "rewind restores the declared count");
        assert!(matches!(group.next(), Some(Err(_))), "the wire is still broken");

        // The same group over an intact buffer iterates normally after rewind.
        let mut good = CarDecoder::try_from(&saved[..len])?.into_fuel_figures()?;
        let Some(Ok(first)) = good.next() else { panic!("first entry") };
        assert_eq!(first.speed(), 30);
        good.rewind();
        assert_eq!(good.remaining(), 2);
        let Some(Ok(again)) = good.next() else { panic!("first entry after rewind") };
        assert_eq!(again.speed(), 30);
        assert!(good.finish().is_ok(), "an unpoisoned group still completes");
        "#,
    );
    Ok(())
}

// ─── FrameCursor fuse ──────────────────────────────────────────────────────

#[test]
fn frame_cursor_emits_one_error_then_permanent_none() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "mi_frames");
    compile_and_run(
        "mi_frames",
        &src,
        r#"
        fn assert_fused(label: &str, buf: &[u8], policy: FramingPolicy) {
            let mut cursor = FrameCursor::new(buf, policy);
            let first = cursor.next();
            assert!(
                matches!(first, Some(Err(_))),
                "{label}: the first poll must surface the error"
            );
            assert!(cursor.next().is_none(), "{label}: must not repeat the error");
            assert!(cursor.next().is_none(), "{label}: must stay fused");
        }

        // 1. Truncated length prefix (u32 policy, two bytes available).
        assert_fused("short u32 prefix", &[1, 0], FramingPolicy::LengthPrefixU32Le);
        // 2. Truncated length prefix (u16 policy, one byte available).
        assert_fused("short u16 prefix", &[1], FramingPolicy::LengthPrefixU16Le);
        // 3. Prefix declares more than the buffer holds.
        let mut overrun = Vec::new();
        overrun.extend_from_slice(&64u16.to_le_bytes());
        overrun.extend_from_slice(&[0u8; 8]);
        assert_fused("frame overruns buffer", &overrun, FramingPolicy::LengthPrefixU16Le);
        // 4. Frame length is present but the body is not a decodable message.
        let mut bad_body = Vec::new();
        bad_body.extend_from_slice(&8u16.to_le_bytes());
        bad_body.extend_from_slice(&[0xFF; 8]);
        assert_fused("undecodable frame", &bad_body, FramingPolicy::LengthPrefixU16Le);
        // 5. Fixed framing whose declared length runs past the buffer.
        assert_fused("fixed overrun", &[0u8; 4], FramingPolicy::Fixed(64));

        // Empty input yields nothing at all — not an error.
        let mut empty = FrameCursor::new(&[], FramingPolicy::LengthPrefixU16Le);
        assert!(empty.next().is_none());
        "#,
    );
    Ok(())
}

#[test]
fn successful_multi_frame_iteration_is_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "mi_frames_ok");
    compile_and_run(
        "mi_frames_ok",
        &src,
        r#"
        let mut body = [0u8; 256];
        let frame_len = CarEncoder::wrap_and_apply_header(&mut body, 0)
            .fixed(&CarFixedFields {
                serial_number: 7,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0; 4],
                vehicle_code: *b"ABCDEF",
                extras: OptionalExtras::default(),
                engine: Engine::new(1, 1, [0; 3], 0i8, BooleanType::F,
                                    Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"H")?
            .model(b"C")?
            .activation_code(b"A")?
            .encoded_length_with_header();

        // Three well-formed frames behind u16 prefixes.
        let mut stream = Vec::new();
        for _ in 0..3 {
            stream.extend_from_slice(&(frame_len as u16).to_le_bytes());
            stream.extend_from_slice(&body[..frame_len]);
        }

        let frames: Vec<_> = FrameCursor::new(&stream, FramingPolicy::LengthPrefixU16Le)
            .collect::<Result<Vec<_>, _>>()
            .expect("a well-formed stream must iterate cleanly");
        assert_eq!(frames.len(), 3);
        for frame in &frames {
            assert_eq!(frame.len, frame_len);
            match &frame.message {
                AnyMessage::Car(car) => assert_eq!(car.serial_number(), 7),
                _ => panic!("expected a Car"),
            }
        }

        // A trailing truncated frame still fuses after the three good ones.
        stream.extend_from_slice(&(frame_len as u16).to_le_bytes());
        stream.extend_from_slice(&body[..4]);
        let mut cursor = FrameCursor::new(&stream, FramingPolicy::LengthPrefixU16Le);
        assert!(matches!(cursor.next(), Some(Ok(_))));
        assert!(matches!(cursor.next(), Some(Ok(_))));
        assert!(matches!(cursor.next(), Some(Ok(_))));
        assert!(matches!(cursor.next(), Some(Err(_))));
        assert!(cursor.next().is_none(), "cursor must fuse after the bad tail");
        "#,
    );
    Ok(())
}

#[test]
fn frame_cursor_is_documented_as_fused() -> Result<(), Box<dyn std::error::Error>> {
    let (_, src) = generate(&Paths::example_schema(), "mi_fused_doc");
    assert!(
        src.contains("impl<'a> core::iter::FusedIterator for FrameCursor<'a> {}"),
        "FrameCursor must implement FusedIterator"
    );
    assert!(
        src.contains("Fused after the first error."),
        "the fuse must be documented where users read it"
    );
    Ok(())
}
