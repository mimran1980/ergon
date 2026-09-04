//! Mutable ordered decoder (`Decoder::ordered()`) plus three-lane parity.
//!
//! Lanes:
//! 1. Random-access flyweight getters (any order, rescans preceding tails)
//! 2. Compile-time staged `into_*` / `visit_entries`
//! 3. Mutable ordered cursor with runtime `OutOfOrder` checks
//!
//! The standard group `Iterator` is compatibility/partial traversal, not a
//! fourth message-decoding lane.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, compile_and_run, compile_fails_with_diagnostics, generate, generate_domain_with,
};
use ergo_sbe::{ConversionSelector, GenerationConfig, GenerationProfile, Generator, Schema, parse};

fn assert_out_of_order_src(src: &str) {
    assert!(
        src.contains("CarOrderedDecoder"),
        "must emit mutable ordered decoder"
    );
    assert!(
        src.contains("fn ordered("),
        "must emit ordered() conversion"
    );
    assert!(
        src.contains("DecodeError::OutOfOrder"),
        "must emit OutOfOrder checks"
    );
    assert!(
        src.contains("core::str::from_utf8"),
        "ordered text helpers must stay no_std (core::str)"
    );
}

/// A truncated fixed-stride group must be rejected before any entry reaches
/// the visitor callback.
///
/// The dynamic-entry path checks `min_entry_extent` per entry, but a
/// fixed-stride group has no per-entry extent to compute, so `visit_entries`
/// hands each entry straight to the callback. Without an up-front
/// `count * blockLength` check, a large `numInGroup` on a short buffer walks
/// past the end. The random-access lane has always validated this region; the
/// ordered lane did not, and this pins the parity.
#[test]
fn ordered_visit_entries_rejects_truncated_fixed_stride_group()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(
        &Paths::sbe_tool_test_resource("basic-group-schema.xml"),
        "mo_fixed_truncated",
    );
    compile_and_run(
        "mo_fixed_truncated",
        &src,
        r#"
        let row = EntriesEntry {
            tag_group1: {
                let mut symbol = [0u8; 20];
                symbol[..3].copy_from_slice(b"ABC");
                symbol
            },
            tag_group2: 101,
        };
        // Fixed-stride group, no var-data: the direct const helper is the
        // exact-size API for this shape (no staged builder is generated).
        const LEN: usize = TestMessage1Encoder::compute_length_with_header(1);
        let mut buf = [0u8; LEN];
        let len = LEN;
        let actual = TestMessage1Encoder::try_wrap_and_apply_header(&mut buf, 0)?
            .fixed(&TestMessage1FixedFields { tag1: 0 })
            .entries(1, |group| group.bulk_add(core::slice::from_ref(&row)))?
            .encoded_length_with_header();
        assert_eq!(len, actual, "EncodedLength must match the encoder");

        // One byte short of the declared entries region. The region is proven
        // in-bounds when the group is entered, so this is rejected before a
        // visitor could ever be handed an entry.
        let truncated = &buf[..actual - 1];
        let mut ordered = TestMessage1Decoder::try_decode(truncated, 0)?.ordered();
        // The guard type is not Debug, so map to the error before asserting.
        let err = ordered.entries().err();
        assert!(
            matches!(err, Some(sbe_rt::DecodeError::BufferTooShort { .. })),
            "truncated fixed-stride entries must be rejected; got {err:?}",
        );

        // Nothing may reach the callback either.
        let mut ordered = TestMessage1Decoder::try_decode(truncated, 0)?.ordered();
        let mut visited = 0usize;
        if let Ok(group) = ordered.entries() {
            let _ = group.visit_entries(|_e| -> Result<(), sbe_rt::DecodeError> {
                visited += 1;
                Ok(())
            });
        }
        assert_eq!(visited, 0, "no entry may be visited from a truncated group");

        // The untruncated buffer still visits every entry.
        let mut ordered = TestMessage1Decoder::try_decode(&buf[..actual], 0)?.ordered();
        let mut seen = 0usize;
        ordered
            .entries()?
            .visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
                assert_eq!(entry.tag_group2(), 101);
                seen += 1;
                Ok(())
            })?;
        assert_eq!(seen, 1);
        "#,
    );
    Ok(())
}

/// All three lanes decode the same Car payload to the same values.
#[test]
fn three_lanes_decode_identical_values() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "three_lanes");
    assert_out_of_order_src(&src);
    compile_and_run(
        "three_lanes",
        &src,
        r#"
        let mut storage = [0u8; 512];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: *b"abcdef",
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, *b"ABC", 1i8, BooleanType::T, Booster::new(BoostType::TURBO, 200)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban") })?;
                g.add(|mut e| { e.speed(55).mpg(49.0); e.usage_description(b"Hwy") })?;
                Ok(())
            })?
            .performance_figures(1, |g| {
                g.add(|mut e| {
                    e.octane_rating(95);
                    e.acceleration(2, |a| {
                        a.add(|row| { row.mph(30).seconds(4.0); Ok(()) })?;
                        a.add(|row| { row.mph(60).seconds(7.5); Ok(()) })?;
                        Ok(())
                    })
                })?;
                Ok(())
            })?
            .manufacturer(b"Honda")?
            .model(b"Civic")?
            .activation_code(b"abc")?
            .encoded_length_with_header();
        let encoded = &storage[..len];

        // Lane 1: random access (any order; manufacturer before groups).
        let random = CarDecoder::try_decode(encoded, 0)?;
        let r_sn = random.serial_number();
        let r_year = random.model_year();
        let r_code = random.code();
        let r_eng = random.engine().capacity();
        let r_mfr = random.manufacturer()?.to_vec();
        let mut r_fuel = Vec::new();
        for e in random.fuel_figures()? {
            let e = e?;
            r_fuel.push((e.speed(), e.usage_description()?.to_vec()));
        }
        let mut r_octane = Vec::new();
        let mut r_acc = Vec::new();
        for e in random.performance_figures()? {
            let e = e?;
            r_octane.push(e.octane_rating());
            for a in e.acceleration()? {
                r_acc.push((a.mph(), a.seconds().to_bits()));
            }
        }
        let r_model = random.model()?.to_vec();
        let r_code_vd = random.activation_code()?.to_vec();

        // Lane 2: compile-time staged visit_entries.
        let mut s_fuel = Vec::new();
        let mut s_octane = Vec::new();
        let mut s_acc = Vec::new();
        let staged = CarDecoder::try_decode(encoded, 0)?;
        let s_sn = staged.serial_number();
        let s_year = staged.model_year();
        let s_code = staged.code();
        let s_eng = staged.engine().capacity();
        let (s_mfr, staged) = staged
            .into_fuel_figures()?
            .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                let speed = entry.speed();
                let (usage, complete) = entry.into_usage_description()?;
                s_fuel.push((speed, usage.to_vec()));
                Ok(complete)
            })?
            .into_performance_figures()?
            .visit_entries(|entry| -> Result<_, sbe_rt::DecodeError> {
                s_octane.push(entry.octane_rating());
                entry.into_acceleration()?.visit_entries(|a| -> Result<(), sbe_rt::DecodeError> {
                    s_acc.push((a.mph(), a.seconds().to_bits()));
                    Ok(())
                })
            })?
            .into_manufacturer()?;
        let (s_model, staged) = staged.into_model()?;
        let (s_code_vd, _) = staged.into_activation_code()?;

        // Lane 3: mutable ordered.
        let mut o_fuel = Vec::new();
        let mut o_octane = Vec::new();
        let mut o_acc = Vec::new();
        let mut ordered = CarDecoder::try_decode(encoded, 0)?.ordered();
        let o_sn = ordered.serial_number();
        let o_year = ordered.model_year();
        let o_code = ordered.code();
        let o_eng = ordered.engine().capacity();
        ordered.fuel_figures()?.visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
            o_fuel.push((entry.speed(), entry.usage_description()?.to_vec()));
            Ok(())
        })?;
        ordered.performance_figures()?.visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
            o_octane.push(entry.octane_rating());
            entry.acceleration()?.visit_entries(|a| -> Result<(), sbe_rt::DecodeError> {
                o_acc.push((a.mph(), a.seconds().to_bits()));
                Ok(())
            })?;
            Ok(())
        })?;
        let o_mfr = ordered.manufacturer()?.to_vec();
        let o_model = ordered.model()?.to_vec();
        let o_code_vd = ordered.activation_code()?.to_vec();
        let complete = ordered.finish()?;
        assert_eq!(complete.encoded_length_with_header(), len);

        assert_eq!((r_sn, s_sn, o_sn), (1234, 1234, 1234));
        assert_eq!((r_year, s_year, o_year), (2013, 2013, 2013));
        assert_eq!(r_code, s_code);
        assert_eq!(s_code, o_code);
        assert_eq!((r_eng, s_eng, o_eng), (2000, 2000, 2000));
        assert_eq!(r_fuel, s_fuel);
        assert_eq!(s_fuel, o_fuel);
        assert_eq!(r_octane, s_octane);
        assert_eq!(s_octane, o_octane);
        assert_eq!(r_acc, s_acc);
        assert_eq!(s_acc, o_acc);
        assert_eq!(r_mfr, s_mfr);
        assert_eq!(s_mfr.as_ref(), o_mfr.as_slice());
        assert_eq!(r_model, s_model);
        assert_eq!(s_model.as_ref(), o_model.as_slice());
        assert_eq!(r_code_vd, s_code_vd);
        assert_eq!(s_code_vd.as_ref(), o_code_vd.as_slice());
    "#,
    );
    Ok(())
}

/// Empty groups invoke the callback zero times and still advance.
#[test]
fn empty_groups_zero_callbacks() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_empty");
    compile_and_run(
        "mo_empty",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"")?
            .model(b"")?
            .activation_code(b"")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut car = CarDecoder::try_decode(encoded, 0)?.ordered();
        let fuel = car.fuel_figures()?;
        assert!(fuel.is_empty());
        assert_eq!(fuel.remaining_entries(), 0);
        let mut calls = 0usize;
        fuel.visit_entries(|_| -> Result<(), sbe_rt::DecodeError> {
            calls += 1;
            Ok(())
        })?;
        assert_eq!(calls, 0);
        let perf = car.performance_figures()?;
        assert!(perf.is_empty());
        perf.visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?;
        assert!(car.manufacturer()?.is_empty());
        assert!(car.model()?.is_empty());
        assert!(car.activation_code()?.is_empty());
        let done = car.finish()?;
        assert_eq!(done.encoded_length_with_header(), len);
    "#,
    );
    Ok(())
}

/// Wrong order, repeat, and access after completion leave the cursor unchanged.
#[test]
fn out_of_order_repeat_and_after_complete() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_ooo");
    compile_and_run(
        "mo_ooo",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut car = CarDecoder::try_decode(encoded, 0)?.ordered();
        match car.manufacturer() {
            Err(sbe_rt::DecodeError::OutOfOrder { owner, expected, requested }) => {
                assert_eq!(owner, "Car");
                assert_eq!(expected, "fuelFigures");
                assert_eq!(requested, "manufacturer");
            }
            Err(e) => panic!("expected OutOfOrder, got {e:?}"),
            Ok(_) => panic!("expected OutOfOrder, got Ok"),
        }
        // Cursor unchanged: the correct method still works.
        car.fuel_figures()?.skip_remaining()?;
        match car.fuel_figures() {
            Err(sbe_rt::DecodeError::OutOfOrder { expected, requested, .. }) => {
                assert_eq!(expected, "performanceFigures");
                assert_eq!(requested, "fuelFigures");
            }
            Err(e) => panic!("expected repeat OutOfOrder, got {e:?}"),
            Ok(_) => panic!("expected repeat OutOfOrder, got Ok"),
        }
        car.performance_figures()?.skip_remaining()?;
        assert_eq!(car.manufacturer()?, b"M");
        assert_eq!(car.model()?, b"N");
        assert_eq!(car.activation_code()?, b"P");
        match car.manufacturer() {
            Err(sbe_rt::DecodeError::OutOfOrder { expected, requested, .. }) => {
                assert_eq!(expected, "<complete>");
                assert_eq!(requested, "manufacturer");
            }
            Err(e) => panic!("expected complete OutOfOrder, got {e:?}"),
            Ok(_) => panic!("expected complete OutOfOrder, got Ok"),
        }
        let done = car.finish()?;
        assert_eq!(done.encoded_length_with_header(), len);
        assert_eq!(
            format!("{}", sbe_rt::DecodeError::OutOfOrder {
                owner: "Car",
                expected: "fuelFigures",
                requested: "manufacturer",
            }),
            "Car: expected 'fuelFigures', requested 'manufacturer'"
        );
    "#,
    );
    Ok(())
}

/// Dropping a group guard leaves the parent at the group start; retry works.
#[test]
fn dropped_guard_retries_from_group_start() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_drop");
    compile_and_run(
        "mo_drop",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(1, |g| {
                g.add(|mut e| { e.speed(10).mpg(1.0); e.usage_description(b"aa") })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut car = CarDecoder::try_decode(encoded, 0)?.ordered();
        {
            let fuel = car.fuel_figures()?;
            assert_eq!(fuel.remaining_entries(), 1);
        }
        let mut speeds = Vec::new();
        car.fuel_figures()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            speeds.push(e.speed());
            Ok(())
        })?;
        assert_eq!(speeds, vec![10]);
        car.performance_figures()?.skip_remaining()?;
        assert_eq!(car.manufacturer()?, b"M");
        let _ = car.finish()?;
    "#,
    );
    Ok(())
}

/// Callback error does not commit; retry walks from the group start.
#[test]
fn callback_error_does_not_commit() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_cb_err");
    compile_and_run(
        "mo_cb_err",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(10).mpg(1.0); e.usage_description(b"aa") })?;
                g.add(|mut e| { e.speed(20).mpg(2.0); e.usage_description(b"bb") })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut car = CarDecoder::try_decode(encoded, 0)?.ordered();
        let err = car.fuel_figures()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            if e.speed() == 20 {
                return Err(sbe_rt::DecodeError::InvalidAscii { field: "cb" });
            }
            Ok(())
        });
        match err {
            Err(sbe_rt::DecodeError::InvalidAscii { field }) => assert_eq!(field, "cb"),
            other => panic!("expected callback error, got {other:?}"),
        }
        let mut speeds = Vec::new();
        car.fuel_figures()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            speeds.push(e.speed());
            Ok(())
        })?;
        assert_eq!(speeds, vec![10, 20]);
        let _ = car.finish()?;
    "#,
    );
    Ok(())
}

/// Unread entry suffix is skipped once so the group can advance.
#[test]
fn unread_entry_suffix_is_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_suffix");
    compile_and_run(
        "mo_suffix",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(10).mpg(1.0); e.usage_description(b"aaa") })?;
                g.add(|mut e| { e.speed(20).mpg(2.0); e.usage_description(b"bbbb") })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut car = CarDecoder::try_decode(encoded, 0)?.ordered();
        let mut speeds = Vec::new();
        car.fuel_figures()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            speeds.push(e.speed());
            Ok(())
        })?;
        assert_eq!(speeds, vec![10, 20]);
        car.performance_figures()?.skip_remaining()?;
        assert_eq!(car.manufacturer()?, b"M");
        let _ = car.finish()?;
    "#,
    );
    Ok(())
}

/// `finish` / `skip_remaining` skip unconsumed suffix at message and group level.
#[test]
fn finish_and_skip_remaining() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_finish");
    compile_and_run(
        "mo_finish",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(1, |g| {
                g.add(|mut e| { e.speed(10).mpg(1.0); e.usage_description(b"aa") })?;
                Ok(())
            })?
            .performance_figures(1, |g| {
                g.add(|mut e| { e.octane_rating(95); e.acceleration(0, |_| Ok(())) })?;
                Ok(())
            })?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut car = CarDecoder::try_decode(encoded, 0)?.ordered();
        car.fuel_figures()?.skip_remaining()?;
        let done = car.finish()?;
        assert_eq!(done.encoded_length_with_header(), len);
        assert_eq!(done.as_bytes_with_header(), encoded);
    "#,
    );
    Ok(())
}

/// Text validation fails before commit so the raw accessor still works.
#[test]
fn text_validation_does_not_commit() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_utf8");
    compile_and_run(
        "mo_utf8",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"Ho")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        // Corrupt the manufacturer payload to invalid UTF-8.
        let honda = storage[..len].windows(2).position(|w| w == b"Ho").expect("payload");
        storage[honda] = 0xFF;
        let encoded = &storage[..len];
        let mut car = CarDecoder::try_decode(encoded, 0)?.ordered();
        car.fuel_figures()?.skip_remaining()?;
        car.performance_figures()?.skip_remaining()?;
        match car.manufacturer_as_str() {
            Err(sbe_rt::DecodeError::InvalidUtf8 { field, .. }) => assert_eq!(field, "manufacturer"),
            other => panic!("expected InvalidUtf8, got {other:?}"),
        }
        let raw = car.manufacturer()?;
        assert_eq!(raw[0], 0xFF);
        assert_eq!(car.model()?, b"N");
        let _ = car.finish()?;
    "#,
    );
    Ok(())
}

/// Nested-message helper commits only after `decode_frame` succeeds.
#[test]
fn nested_message_error_does_not_commit() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "mo_nested_msg");
    compile_and_run(
        "mo_nested_msg",
        &src,
        r#"
        let outer_len = OuterEncoder::compute_encoded_length_with_message_header(
            b"app".len(),
            1,
        );
        let mut storage = [0u8; 256];
        assert!(outer_len <= storage.len());
        let len = OuterEncoder::try_wrap_and_apply_header(&mut storage[..outer_len], 0)?
            .fixed(&OuterFixedFields { trace_id: 7 })
            .app_name(b"app")?
            .payload(&[0xFF])?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut outer = OuterDecoder::try_decode(encoded, 0)?.ordered();
        assert_eq!(outer.app_name()?, b"app");
        match outer.payload_as_message() {
            Err(_) => {}
            Ok(_) => panic!("expected nested-message decode failure"),
        }
        assert_eq!(outer.payload()?, &[0xFF]);
        let _ = outer.finish()?;
    "#,
    );
    Ok(())
}

/// Truncated group header is a wire error and does not advance the ordinal.
#[test]
fn wire_error_does_not_advance_cursor() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_trunc");
    compile_and_run(
        "mo_trunc",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        // Cut inside the first group dimension header.
        let encoded = &storage[..8 + 45 + 1];
        let mut car = match CarDecoder::try_wrap(encoded, 0, 45, 0) {
            Ok(c) => c.ordered(),
            Err(_) => {
                // Header/body may fail try_wrap on short buffer; wrap_unchecked then ordered.
                let c = unsafe { CarDecoder::wrap_unchecked(encoded, 0, 45, 0) };
                c.ordered()
            }
        };
        match car.fuel_figures() {
            Err(sbe_rt::DecodeError::BufferTooShort { .. }) => {}
            Err(e) => panic!("expected BufferTooShort, got {e:?}"),
            Ok(_) => panic!("expected BufferTooShort, got Ok"),
        }
        match car.manufacturer() {
            Err(sbe_rt::DecodeError::OutOfOrder { expected, requested, .. }) => {
                assert_eq!(expected, "fuelFigures");
                assert_eq!(requested, "manufacturer");
            }
            Err(e) => panic!("expected OutOfOrder after failed group, got {e:?}"),
            Ok(_) => panic!("expected OutOfOrder after failed group, got Ok"),
        }
    "#,
    );
    Ok(())
}

/// Version-absent tails consume zero bytes; present v1 tails still walk.
#[test]
fn version_absent_and_present_tails() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::ordered_decoder_version_tails_schema(), "mo_version");
    compile_and_run(
        "mo_version",
        &src,
        r#"
        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&4u16.to_le_bytes());
        buf[2..4].copy_from_slice(&1u16.to_le_bytes());
        buf[4..6].copy_from_slice(&77u16.to_le_bytes());
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());
        buf[8..12].copy_from_slice(&7u32.to_le_bytes());
        buf[12..14].copy_from_slice(&2u16.to_le_bytes());
        buf[14..16].copy_from_slice(&1u16.to_le_bytes());
        buf[16..18].copy_from_slice(&30u16.to_le_bytes());
        buf[18] = 3;
        buf[19..22].copy_from_slice(b"urb");
        buf[22] = 2;
        buf[23..25].copy_from_slice(b"hi");
        let encoded = &buf[..25];

        let mut dec = VersionedTailsDecoder::wrap(encoded, 0, 4, 0).ordered();
        assert_eq!(dec.seq(), 7);
        match dec.extra_figures() {
            Err(sbe_rt::DecodeError::OutOfOrder { expected, requested, .. }) => {
                assert_eq!(expected, "figures");
                assert_eq!(requested, "extraFigures");
            }
            Err(e) => panic!("expected OutOfOrder, got {e:?}"),
            Ok(_) => panic!("expected OutOfOrder, got Ok"),
        }
        let mut speeds = Vec::new();
        dec.figures()?.visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
            speeds.push(entry.speed());
            let extras = entry.extras()?;
            assert!(extras.is_empty());
            extras.visit_entries(|_| -> Result<(), sbe_rt::DecodeError> { Ok(()) })?;
            assert_eq!(entry.label()?, b"urb");
            Ok(())
        })?;
        assert_eq!(speeds, vec![30]);
        let extra = dec.extra_figures()?;
        assert!(extra.is_empty());
        extra.skip_remaining()?;
        assert_eq!(dec.note()?, b"hi");
        assert!(dec.extra_note()?.is_empty());
        let done = dec.finish()?;
        assert_eq!(done.encoded_length_with_header(), encoded.len());

        let mut storage = [0u8; 128];
        let len = VersionedTailsEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&VersionedTailsFixedFields { seq: 9 })
            .figures(1, |g| {
                g.add(|mut e| {
                    e.speed(40);
                    e.extras(1, |x| {
                        x.add(|mut row| { row.flag(7); Ok(()) })?;
                        Ok(())
                    })?
                    .label(b"v1")
                })?;
                Ok(())
            })?
            .extra_figures(1, |g| {
                g.add(|mut e| { e.amp(11); Ok(()) })?;
                Ok(())
            })?
            .note(b"n")?
            .extra_note(b"x")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut dec = VersionedTailsDecoder::try_decode(encoded, 0)?.ordered();
        let mut flags = Vec::new();
        dec.figures()?.visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
            assert_eq!(entry.speed(), 40);
            entry.extras()?.visit_entries(|row| -> Result<(), sbe_rt::DecodeError> {
                flags.push(row.flag());
                Ok(())
            })?;
            assert_eq!(entry.label()?, b"v1");
            Ok(())
        })?;
        assert_eq!(flags, vec![7]);
        let mut amps = Vec::new();
        dec.extra_figures()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            amps.push(e.amp());
            Ok(())
        })?;
        assert_eq!(amps, vec![11]);
        assert_eq!(dec.note()?, b"n");
        assert_eq!(dec.extra_note()?, b"x");
        let _ = dec.finish()?;
    "#,
    );
    Ok(())
}

/// Schema field named `ordered` is renamed; the `ordered()` lane method wins
/// the name. The fixture carries a group on purpose — a fixed-block message
/// generates no `ordered()` at all, so the clash this guards can only happen
/// on a message with tails.
#[test]
fn schema_field_named_ordered_is_renamed() -> Result<(), Box<dyn std::error::Error>> {
    const XML: &str = r#"<messageSchema package="ordclash" id="1" version="0" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
  </types>
  <message name="Msg" id="1" blockLength="4">
    <field name="ordered" id="1" type="uint32" offset="0"/>
    <group name="legs" id="2" dimensionType="groupSizeEncoding" blockLength="4">
      <field name="qty" id="3" type="uint32" offset="0"/>
    </group>
  </message>
</messageSchema>"#;
    let schema = Schema::from_ir(parse(XML)?);
    let src = Generator::new(GenerationConfig::new("ordclash"))
        .generate(&schema)?
        .modules()
        .next()
        .expect("one module")
        .source
        .clone();
    assert!(
        src.contains("fn ordered_field("),
        "field named ordered must be renamed"
    );
    assert!(
        src.contains("fn ordered("),
        "ordered() conversion must remain"
    );
    compile_and_run(
        "ordclash",
        &src,
        r#"
        let mut storage = [0u8; MsgEncoder::compute_length_with_header(1)];
        let len = MsgEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&MsgFixedFields { ordered: 7 })
            .legs(1, |legs| { legs.add(|l| { l.qty(3u32); Ok(()) })?; Ok(()) })?
            .encoded_length_with_header();
        assert_eq!(storage.len(), len);
        let dec = MsgDecoder::try_decode(&storage[..len], 0)?;
        assert_eq!(dec.ordered_field(), 7);
        let mut ordered = dec.ordered();
        assert_eq!(ordered.ordered_field(), 7);
        assert_eq!(ordered.acting_version(), 0);
        ordered.legs()?.visit_entries(|e| -> Result<(), sbe_rt::DecodeError> {
            assert_eq!(e.qty(), 3);
            Ok(())
        })?;
    "#,
    );
    Ok(())
}

/// Lean and Full both emit and run the mutable ordered lane.
#[test]
fn lean_and_full_profiles() -> Result<(), Box<dyn std::error::Error>> {
    for (module, profile) in [
        ("mo_lean", GenerationProfile::Lean),
        ("mo_full", GenerationProfile::Full),
    ] {
        let (_schema, src) =
            generate_domain_with(&Paths::example_schema(), module, |c| c.profile(profile));
        assert!(src.contains("fn ordered("), "{module} must emit ordered()");
        compile_and_run(
            module,
            &src,
            r#"
            let mut storage = [0u8; 256];
            let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
                .fixed(&CarFixedFields {
                    serial_number: 1,
                    model_year: 0,
                    available: BooleanType::F,
                    code: Model::NullVal,
                    some_numbers: [0u32; 4],
                    vehicle_code: [0u8; 6],
                    extras: OptionalExtras::default(),
                    engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
                })
                .fuel_figures(0, |_| Ok(()))?
                .performance_figures(0, |_| Ok(()))?
                .manufacturer(b"M")?
                .model(b"N")?
                .activation_code(b"P")?
                .encoded_length_with_header();
            let mut car = CarDecoder::try_decode(&storage[..len], 0)?.ordered();
            car.fuel_figures()?.skip_remaining()?;
            car.performance_figures()?.skip_remaining()?;
            assert_eq!(car.manufacturer()?, b"M");
            let _ = car.finish()?;
        "#,
        );
    }
    Ok(())
}

/// Configured conversion getters are forwarded onto the ordered decoder.
#[test]
fn domain_conversion_forwards() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate_domain_with(&Paths::example_schema(), "mo_conv", |c| {
        c.with_conversion(ConversionSelector::named_type("BooleanType"))
            .with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
    });
    compile_and_run(
        "mo_conv",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 0,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(0, |_| Ok(()))?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let mut car = CarDecoder::try_decode(&storage[..len], 0)?.ordered();
        let available: bool = car.try_available()?;
        assert!(available);
        let _ = car.finish()?;
    "#,
    );
    Ok(())
}

/// L3 bids then asks, including nested orders, through the mutable ordered lane.
#[test]
fn l3_mutable_ordered() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::l3_orderbook_schema(), "mo_l3");
    compile_and_run(
        "mo_l3",
        &src,
        r#"
        let mut storage = [0u8; 512];
        let len = L3BookEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&L3BookFixedFields { timestamp: 99, sequence: 7 })
            .bids(2, |g| {
                g.add(|mut lvl| {
                    lvl.price(100).qty(10);
                    lvl.orders(2, |o| {
                        o.add(|mut ord| { ord.order_qty(4); ord.order_id(b"ord-1") })?;
                        o.add(|mut ord| { ord.order_qty(6); ord.order_id(b"ord-2") })?;
                        Ok(())
                    })
                })?;
                g.add(|mut lvl| {
                    lvl.price(101).qty(5);
                    lvl.orders(0, |_| Ok(()))
                })?;
                Ok(())
            })?
            .asks(1, |g| {
                g.add(|mut lvl| {
                    lvl.price(200).qty(20);
                    lvl.orders(1, |o| {
                        o.add(|mut ord| { ord.order_qty(8); ord.order_id(b"ask-1") })
                    })
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let mut book = L3BookDecoder::try_decode(encoded, 0)?.ordered();
        assert_eq!(book.timestamp(), 99);
        let mut prices = Vec::new();
        let mut ids: Vec<Vec<u8>> = Vec::new();
        book.bids()?.visit_entries(|lvl| -> Result<(), sbe_rt::DecodeError> {
            prices.push(lvl.price());
            lvl.orders()?.visit_entries(|ord| -> Result<(), sbe_rt::DecodeError> {
                ids.push(ord.order_id()?.to_vec());
                Ok(())
            })?;
            Ok(())
        })?;
        assert_eq!(prices, vec![100, 101]);
        assert_eq!(ids, vec![b"ord-1".to_vec(), b"ord-2".to_vec()]);
        let mut ask_prices = Vec::new();
        book.asks()?.visit_entries(|lvl| -> Result<(), sbe_rt::DecodeError> {
            ask_prices.push(lvl.price());
            lvl.orders()?.skip_remaining()?;
            Ok(())
        })?;
        assert_eq!(ask_prices, vec![200]);
        let done = book.finish()?;
        assert_eq!(done.encoded_length_with_header(), len);
    "#,
    );
    Ok(())
}

/// Parent cursor cannot be used while a group guard borrows it.
#[test]
fn cf_parent_borrowed_by_group_guard() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_cf_parent");
    compile_fails_with_diagnostics(
        "mo_cf_parent",
        &src,
        r#"
        let buf = [0u8; 16];
        let mut car = unsafe { CarDecoder::wrap_unchecked(&buf, 0, 45, 0) }.ordered();
        let figures = car.fuel_figures().unwrap();
        let _ = car.serial_number();
        let _ = figures;
    "#,
        &["cannot borrow"],
    );
    Ok(())
}

/// Nested group guard borrows the entry; the parent entry is unusable until it ends.
#[test]
fn cf_entry_borrowed_by_nested_guard() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_cf_nested");
    compile_fails_with_diagnostics(
        "mo_cf_nested",
        &src,
        r#"
        let buf = [0u8; 16];
        let mut car = unsafe { CarDecoder::wrap_unchecked(&buf, 0, 45, 0) }.ordered();
        car.fuel_figures().unwrap().skip_remaining().unwrap();
        car.performance_figures().unwrap().visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
            let acc = entry.acceleration()?;
            let _ = entry.octane_rating();
            acc.skip_remaining()?;
            Ok(())
        }).unwrap();
    "#,
        &["cannot borrow"],
    );
    Ok(())
}

/// Mutable group guard is not an Iterator.
#[test]
fn cf_guard_is_not_iterator() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_cf_iter");
    compile_fails_with_diagnostics(
        "mo_cf_iter",
        &src,
        r#"
        let buf = [0u8; 16];
        let mut car = unsafe { CarDecoder::wrap_unchecked(&buf, 0, 45, 0) }.ordered();
        let mut figures = car.fuel_figures().unwrap();
        let _ = figures.next();
    "#,
        &["no method named `next`"],
    );
    Ok(())
}

/// Random-access and Iterator accessors remain on the original flyweight.
#[test]
fn random_access_and_iterator_still_work() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_compat");
    compile_and_run(
        "mo_compat",
        &src,
        r#"
        let mut storage = [0u8; 256];
        let len = CarEncoder::try_wrap_and_apply_header(&mut storage, 0)?
            .fixed(&CarFixedFields {
                serial_number: 9,
                model_year: 2013,
                available: BooleanType::F,
                code: Model::NullVal,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::NullVal, 0)),
            })
            .fuel_figures(1, |g| {
                g.add(|mut e| { e.speed(10).mpg(1.0); e.usage_description(b"aa") })?;
                Ok(())
            })?
            .performance_figures(0, |_| Ok(()))?
            .manufacturer(b"M")?
            .model(b"N")?
            .activation_code(b"P")?
            .encoded_length_with_header();
        let encoded = &storage[..len];
        let car = CarDecoder::try_decode(encoded, 0)?;
        assert_eq!(car.serial_number(), 9);
        assert_eq!(car.manufacturer()?, b"M");
        let mut n = 0usize;
        for e in car.fuel_figures()? {
            n += 1;
            assert_eq!(e?.speed(), 10);
        }
        assert_eq!(n, 1);
        let mut fuel = car.into_fuel_figures()?;
        assert_eq!(fuel.next().unwrap()?.speed(), 10);
        let _ = fuel.finish()?;
    "#,
    );
    Ok(())
}

/// Metadata forwards without exposing random-access dynamic tails on the cursor.
#[test]
fn metadata_forwards_without_leaking_flyweight() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "mo_meta");
    assert!(
        src.contains("fn get_metadata(&self) -> CarDecoderMetadata"),
        "ordered decoder must forward get_metadata to the metadata facet"
    );
    compile_fails_with_diagnostics(
        "mo_meta_no_deref",
        &src,
        r#"
        let buf = [0u8; 16];
        let car = unsafe { CarDecoder::wrap_unchecked(&buf, 0, 45, 0) }.ordered();
        // Random-access group getter lives on the flyweight, not the ordered cursor.
        let _ = car.into_fuel_figures();
    "#,
        &["no method named `into_fuel_figures`"],
    );
    Ok(())
}
