//! Port of upstream `simple-binary-encoding/rust/tests/baseline_test.rs`.
//!
//! Decodes the Java-generated binary fixture `car_example_baseline_data.sbe`
//! using ErgoSBE-generated code, then encodes from scratch and verifies
//! round-trip decode produces the same logical values.
//!
//! # Known codegen gaps (not tested here)
//!
//! - `Engine::manufacturer_code()` returns `u8` instead of `[u8; 3]`
//! - `Engine::fuel()` returns `u8` instead of `&[u8; 6]` / `&str`
//! - `Engine` missing `efficiency`, `booster_enabled`, `booster` fields
//! - `Booster` missing `BoostType` enum
//!
//! These gaps mean byte-exact wire match against the Java-generated fixture
//! is impossible today.  We test what does work and document the gaps.

mod common;
use common::{
    Paths, assert_source_ok, compile_and_run, compile_and_run_with_feature, generate,
    run_fixture_test,
};

const MODULE: &str = "car_example";

// ── Structural verification ──────────────────────────────────────────

#[test]
fn generated_code_has_lint_suppressions() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // Item-level allow attributes — NOT expect, because the exact set of
    // lints that fire depends on the schema.  Using #[expect] would produce
    // false-positive stale-suppression warnings when a schema doesn'''t trigger
    // the suppressed lint, breaking CI for end users.
    assert!(
        src.contains("#![allow(non_camel_case_types)]"),
        "generated code must suppress non_camel_case_types"
    );
    assert!(
        src.contains("#![allow(non_snake_case)]"),
        "generated code must suppress non_snake_case"
    );
    assert!(
        src.contains("#![allow(clippy::identity_op)]"),
        "generated code must suppress clippy::identity_op"
    );
    assert!(
        src.contains("#![allow(clippy::eq_op)]"),
        "generated code must suppress clippy::eq_op"
    );
    assert!(
        src.contains("#![allow(clippy::needless_borrow)]"),
        "generated code must suppress clippy::needless_borrow"
    );
    assert!(
        src.contains("#![allow(clippy::manual_range_contains)]"),
        "generated code must suppress clippy::manual_range_contains"
    );
    assert!(
        src.contains("#![allow(unused_imports)]"),
        "generated code must suppress unused_imports"
    );
    assert!(
        src.contains("#![allow(unused_variables)]"),
        "generated code must suppress unused_variables"
    );
    assert!(
        src.contains("#![allow(unused_mut)]"),
        "generated code must suppress unused_mut"
    );
    assert!(
        src.contains("#![allow(dead_code)]"),
        "generated code must suppress dead_code"
    );
    // Item-level suppressions: raw_* accessors wrap unsafe with #[allow(unused_unsafe)]
    assert!(
        src.contains("#[allow(unused_unsafe)]"),
        "raw_* accessors must suppress unused_unsafe"
    );
}

#[test]
fn generated_code_contains_expected_types() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert_source_ok(
        &src,
        &[
            "CarDecoder",
            "CarEncoder",
            "MessageHeader",
            "Engine",
            "Booster",
            "OptionalExtras",
            "Model",
            "BooleanType",
            "FuelFiguresDecoder",
            "FuelFiguresEntryDecoder",
            "PerformanceFiguresDecoder",
            "PerformanceFiguresEntryDecoder",
            "AccelerationDecoder",
            "AccelerationEntryDecoder",
            "FuelFiguresEncoder",
            "PerformanceFiguresEncoder",
            "AccelerationEntryEncoder",
            "GroupSizeEncoding",
            "VarStringEncoding",
            "VarAsciiEncoding",
            "VarDataEncoding",
        ],
    );
}

// ── Wire decode (binary fixture) ─────────────────────────────────────

#[test]
fn decode_baseline_fixture() {
    run_fixture_test(
        "baseline_decode",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let car = CarDecoder::wrap_and_apply_header(FIXTE, 0).unwrap();

        // Scalar fields
        assert_eq!(1234, car.serial_number(), "serial_number");
        assert_eq!(2013, car.model_year(), "model_year");
        assert_eq!(BooleanType::T, car.available(), "available");
        assert_eq!(Model::A, car.code(), "code");

        assert_eq!([1u32, 2, 3, 4], car.some_numbers().unwrap(), "someNumbers");
        assert_eq!([97, 98, 99, 100, 101, 102], car.vehicle_code().unwrap(), "vehicleCode");

        let extras = car.extras();
        assert_eq!(6, extras.raw(), "extras raw");
        assert!(extras.cruise_control(), "cruiseControl");
        assert!(extras.sports_pack(), "sportsPack");
        assert!(!extras.sun_roof(), "sunRoof");

        // discountedModel is presence="constant" valueRef="Model.C"
        assert_eq!(Model::C, car.discounted_model(), "discountedModel");

        // Engine: capacity and numCylinders are at the correct wire offsets
        // (35 and 37 in the Car message).  The remaining engine fields differ
        // because the codegen emits a 7-byte Engine struct while the wire has
        // a 10-byte engine (maxRpm/fuel constant gap, manufacturerCode char[3]
        // vs u8, missing efficiency/booster/boosterEnabled).  Those gaps make
        // byte-exact decode from the fixture impossible for those fields.
        // The round-trip test (below) verifies self-consistent encode/decode
        // for all engine fields.
        let engine = car.engine();
        assert_eq!(2000, engine.capacity(), "engine.capacity");
        assert_eq!(4, engine.num_cylinders(), "engine.numCylinders");

        // Group: fuelFigures (3 entries)
        let fuel_figures: Vec<_> = car.fuel_figures().unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(3, fuel_figures.len(), "fuelFigures count");

        assert_eq!(30, fuel_figures[0].speed(), "ff[0].speed");
        assert!((fuel_figures[0].mpg() - 35.9).abs() < 0.01, "ff[0].mpg");
        assert_eq!(b"Urban Cycle",   fuel_figures[0].usage_description().unwrap(), "ff[0].usage");

        assert_eq!(55, fuel_figures[1].speed(), "ff[1].speed");
        assert!((fuel_figures[1].mpg() - 49.0).abs() < 0.01, "ff[1].mpg");
        assert_eq!(b"Combined Cycle", fuel_figures[1].usage_description().unwrap(), "ff[1].usage");

        assert_eq!(75, fuel_figures[2].speed(), "ff[2].speed");
        assert!((fuel_figures[2].mpg() - 40.0).abs() < 0.01, "ff[2].mpg");
        assert_eq!(b"Highway Cycle",  fuel_figures[2].usage_description().unwrap(), "ff[2].usage");

        // Group: performanceFigures (2 entries), each with nested acceleration group
        let perf: Vec<_> = car.performance_figures().unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(2, perf.len(), "performanceFigures count");

        // --- 95 octane ---
        assert_eq!(95, perf[0].octane_rating(), "pf[0].octaneRating");
        let accel0: Vec<_> = perf[0].acceleration().unwrap().collect::<Vec<_>>();
        assert_eq!(3, accel0.len(), "pf[0].acceleration count");
        assert_eq!(30,  accel0[0].mph(), "pf[0].acc[0].mph");
        assert!((accel0[0].seconds() - 4.0).abs() < 0.01, "pf[0].acc[0].seconds");
        assert_eq!(60,  accel0[1].mph(), "pf[0].acc[1].mph");
        assert!((accel0[1].seconds() - 7.5).abs() < 0.01, "pf[0].acc[1].seconds");
        assert_eq!(100, accel0[2].mph(), "pf[0].acc[2].mph");
        assert!((accel0[2].seconds() - 12.2).abs() < 0.01, "pf[0].acc[2].seconds");

        // --- 99 octane ---
        assert_eq!(99, perf[1].octane_rating(), "pf[1].octaneRating");
        let accel1: Vec<_> = perf[1].acceleration().unwrap().collect::<Vec<_>>();
        assert_eq!(3, accel1.len(), "pf[1].acceleration count");
        assert_eq!(30,  accel1[0].mph(), "pf[1].acc[0].mph");
        assert!((accel1[0].seconds() - 3.8).abs() < 0.01, "pf[1].acc[0].seconds");
        assert_eq!(60,  accel1[1].mph(), "pf[1].acc[1].mph");
        assert!((accel1[1].seconds() - 7.1).abs() < 0.01, "pf[1].acc[1].seconds");
        assert_eq!(100, accel1[2].mph(), "pf[1].acc[2].mph");
        assert!((accel1[2].seconds() - 11.8).abs() < 0.01, "pf[1].acc[2].seconds");

        // Var-data fields
        assert_eq!(b"Honda",     car.manufacturer().unwrap(), "manufacturer");
        assert_eq!(b"Civic VTi", car.model().unwrap(), "model");
        assert_eq!(b"abcdef",    car.activation_code().unwrap(), "activationCode");
        "#,
    );
}

// ── Display output verification ───────────────────────────────────────

#[test]
fn decoder_display() {
    run_fixture_test(
        "display_test",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let car = CarDecoder::wrap_and_apply_header(FIXTE, 0).unwrap();
        let s = format!("{}", car);
        assert!(s.contains("serial_number: 1234"), "display serial_number");
        assert!(s.contains("model_year: 2013"), "display model_year");
        assert!(s.contains("available: BooleanType::T"), "display available");
        assert!(s.contains("code: Model::A"), "display code");
        assert!(s.contains("fuel_figures: ["), "display fuel_figures entries");
        assert!(s.contains("performance_figures: ["), "display performance_figures entries");
        assert!(s.contains("manufacturer: 5 bytes"), "display manufacturer bytes");
        assert!(s.contains("model: 9 bytes"), "display model bytes");
        assert!(s.contains("activation_code: 6 bytes"), "display activation_code bytes");
        assert!(s.starts_with("Car {"), "display starts with Car {{");
        assert!(s.ends_with(" }"), "display ends with }}");
        "#,
    );
}

// ── Encode from scratch and verify round-trip decode ─────────────────

#[test]
fn encode_baseline_roundtrip() {
    run_fixture_test(
        "baseline_encode",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        // ── Encode from scratch ──
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();

        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);

        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);

        car.engine(Engine::new(2000, 4, [49, 0, 0]));

        let car = car.fuel_figures(3, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle").unwrap(); }).unwrap();
            g.add(|e| { e.speed(55).mpg(49.0); e.usage_description(b"Combined Cycle").unwrap(); }).unwrap();
            g.add(|e| { e.speed(75).mpg(40.0); e.usage_description(b"Highway Cycle").unwrap(); }).unwrap();
        }).unwrap();

        let car = car.performance_figures(2, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(3, |a| {
                    a.add(|x| { x.mph(30).seconds(4.0); }).unwrap();
                    a.add(|x| { x.mph(60).seconds(7.5); }).unwrap();
                    a.add(|x| { x.mph(100).seconds(12.2); }).unwrap();
                }).unwrap();
            }).unwrap();
            g.add(|e| {
                e.octane_rating(99);
                e.acceleration(3, |a| {
                    a.add(|x| { x.mph(30).seconds(3.8); }).unwrap();
                    a.add(|x| { x.mph(60).seconds(7.1); }).unwrap();
                    a.add(|x| { x.mph(100).seconds(11.8); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();

        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic VTi").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();

        // ── Decode the just-encoded bytes ──
        let encoded = car.as_bytes();
        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();

        assert_eq!(1234, car2.serial_number(), "rt.serial_number");
        assert_eq!(2013, car2.model_year(), "rt.model_year");
        assert_eq!(BooleanType::T, car2.available(), "rt.available");
        assert_eq!(Model::A, car2.code(), "rt.code");
        assert_eq!([1u32, 2, 3, 4], car2.some_numbers().unwrap(), "rt.someNumbers");
        assert_eq!([97, 98, 99, 100, 101, 102], car2.vehicle_code().unwrap(), "rt.vehicleCode");

        let extras2 = car2.extras();
        assert!(extras2.cruise_control(), "rt.cruiseControl");
        assert!(extras2.sports_pack(), "rt.sportsPack");
        assert!(!extras2.sun_roof(), "rt.sunRoof");

        let e2 = car2.engine();
        assert_eq!(2000, e2.capacity(), "rt.engine.capacity");
        assert_eq!(4, e2.num_cylinders(), "rt.engine.numCylinders");
        assert_eq!(9000, e2.max_rpm(), "rt.engine.maxRpm");
        assert_eq!([49, 0, 0], e2.manufacturer_code(), "rt.engine.manufacturerCode");
        assert_eq!("Petrol", e2.fuel(), "rt.engine.fuel");

        let ff2: Vec<_> = car2.fuel_figures().unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(3, ff2.len());
        assert_eq!(30, ff2[0].speed());  assert!((ff2[0].mpg() - 35.9).abs() < 0.01);
        assert_eq!(b"Urban Cycle", ff2[0].usage_description().unwrap());
        assert_eq!(55, ff2[1].speed());  assert!((ff2[1].mpg() - 49.0).abs() < 0.01);
        assert_eq!(b"Combined Cycle", ff2[1].usage_description().unwrap());
        assert_eq!(75, ff2[2].speed());  assert!((ff2[2].mpg() - 40.0).abs() < 0.01);
        assert_eq!(b"Highway Cycle", ff2[2].usage_description().unwrap());

        let pf2: Vec<_> = car2.performance_figures().unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(2, pf2.len());
        assert_eq!(95, pf2[0].octane_rating());
        let a0: Vec<_> = pf2[0].acceleration().unwrap().collect::<Vec<_>>();
        assert_eq!(3, a0.len());
        assert_eq!(30, a0[0].mph());  assert!((a0[0].seconds() - 4.0).abs() < 0.01);

        assert_eq!(b"Honda",     car2.manufacturer().unwrap(), "rt.manufacturer");
        assert_eq!(b"Civic VTi", car2.model().unwrap(), "rt.model");
        assert_eq!(b"abcdef",    car2.activation_code().unwrap(), "rt.activationCode");
        "#,
    );
}

// ── Byte-exact encode (scalar header fields, full message) ───────────

#[test]
fn encode_byte_exact_scalar() {
    run_fixture_test(
        "scalar_byte_exact",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();

        // Set same scalar values as the fixture
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);

        // Engine (composite) — same values as fixture
        car.engine(Engine::new(2000, 4, [49, 0, 0]));

        let encoded = car.as_bytes();

        // Compare non-blockLength header bytes: templateId, schemaId, version
        assert_eq!(&FIXTE[2..8], &encoded[2..8], "header metadata mismatch");

        // Compare scalar body: serialNumber through extras (body offsets 0..35)
        // BlockLength changed from 45 to 41 (constants no longer occupy wire space),
        // so header[0..2] differs from fixture. Scalar body at offsets 0..34 is identical.
        let header_size = 8usize;
        assert_eq!(
            &FIXTE[header_size .. header_size + 35],
            &encoded[header_size .. header_size + 35],
            "scalar body mismatch"
        );
        "#,
    );
}

// ── Composite byte-exact encode (Engine) ──────────────────────────────

#[test]
fn composite_byte_exact_engine() {
    run_fixture_test(
        "engine_byte_exact",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        // Encode Engine with known values matching the fixture
        let engine = Engine::new(2000, 4, [49, 50, 51]);

        // Verify Engine wire bytes match fixture at body_offset 35
        // Fixture engine starts at body_offset 35 (file position 43).
        // Our Engine is 6 bytes (capacity + numCylinders + manufacturerCode),
        // which matches the first 6 bytes of the fixture's 10-byte engine block.
        // The remaining 4 bytes (maxRpm constant, efficiency, boosterEnabled, booster)
        // are either constant or reference fields not yet generated.
        let header_size = 8usize;
        let engine_offset = 35usize;
        let engine_size = 6usize;
        assert_eq!(
            &FIXTE[header_size + engine_offset .. header_size + engine_offset + engine_size],
            &engine.0[..],
            "engine wire bytes mismatch"
        );
        "#,
    );
}

// ── Zero-parse schemaId extraction ───────────────────────────────────

#[test]
fn schema_id_from_header_car_example() {
    run_fixture_test(
        "schema_id_from_header",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let schema_id = schema_id_from_header(FIXTE);
        assert_eq!(Some(1), schema_id, "schema_id from header");

        assert_eq!(None, schema_id_from_header(&[]), "empty buffer");
        assert_eq!(None, schema_id_from_header(&[0u8; 1]), "too short buffer");
        "#,
    );
}

// ── Constants verification ───────────────────────────────────────────

#[test]
fn constants_match_upstream() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    assert!(src.contains("pub const SCHEMA_ID: u16 = 1;"));
    assert!(src.contains("pub const SCHEMA_VERSION: u16 = 0;"));
    assert!(src.contains("pub const TEMPLATE_ID: u16 = 1;"));
    assert!(src.contains("pub const BLOCK_LENGTH: usize = 41;"));
}

// ── Group decoder is_empty() inherent method ──────────────────────

#[test]
fn group_decoder_is_empty() {
    let (_schema, src) = generate(&Paths::example_schema(), "is_empty_group");
    compile_and_run(
        "is_empty_group",
        &src,
        r#"
        // ── 0 fuel figures → is_empty() == true ──
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic VTi").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes();
        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        assert!(car2.fuel_figures().unwrap().is_empty(), "0 fuel figures → is_empty == true");

        // ── 3 fuel figures → is_empty() == false ──
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(3, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle").unwrap(); }).unwrap();
            g.add(|e| { e.speed(55).mpg(49.0); e.usage_description(b"Combined Cycle").unwrap(); }).unwrap();
            g.add(|e| { e.speed(75).mpg(40.0); e.usage_description(b"Highway Cycle").unwrap(); }).unwrap();
        }).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic VTi").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes();
        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        assert!(!car2.fuel_figures().unwrap().is_empty(), "3 fuel figures → is_empty == false");
    "#,
    );
}

// ── iter_fast (todo 109) — DELETED ─────────────────────────────────
// iter_fast was removed. For groups with var-data tails (total_tail > 0),
// advancing by ENTRY_BLOCK_LENGTH produces wrong positions because
// entries are not contiguous in the buffer — var-data of previous entries
// pushes later entries forward. For total_tail == 0, the standard Iterator
// already uses ENTRY_BLOCK_LENGTH. iter_fast was redundant.
//
// Test coverage: the standard Iterator's ENTRY_BLOCK_LENGTH fast path
// is verified by decode_baseline_fixture (fuel_figures[0].speed == 30 etc.)
// and group_decoder_is_empty.

// ── #[cold] on error Display impls (todo 54) ──────────────────────────

#[test]
fn generated_code_has_cold_annotations() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // sbe_rt emits #[cold] on all three error Display impls
    let cold_count = src.matches("#[cold]").count();
    assert!(
        cold_count >= 3,
        "expected >=3 #[cold] annotations (DecodeError, EncodeError, VerifyError Display impls), found {cold_count}"
    );
}

// ── Const assertions in generated code (todo 56) ──────────────────────

#[test]
fn generated_code_has_const_assertions() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        src.contains(
            "const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);"
        ) || src.contains(
            "const _ENCODED_LEN: () = assert!(Self::ENCODED_LENGTH >= Self::BLOCK_LENGTH);"
        ),
        "generated code must have a compile-time assertion for ENCODED_LENGTH >= BLOCK_LENGTH"
    );
    assert!(
        src.contains("const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == "),
        "generated code must have a compile-time assertion for HEADER_TEMPLATE length"
    );
    assert!(
        src.contains(
            "const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == "
        ),
        "generated code must have a compile-time assertion for GROUP_DIM_TEMPLATE length"
    );
    assert!(
        src.contains("const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == "),
        "generated code must have a compile-time assertion for BLOCK_LENGTH == N"
    );
}

// ── BooleanType support (todo 58) ─────────────────────────────────────

#[test]
fn generated_code_has_boolean_from_impls() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    // BooleanType implements From<u8> (the underlying encoding type).
    assert!(
        src.contains("impl From<u8> for BooleanType"),
        "BooleanType must implement From<u8>"
    );
    assert!(
        src.contains("impl From<BooleanType> for u8"),
        "BooleanType must implement From<BooleanType> for u8"
    );

    // From<bool> conversion (todo 58)
    assert!(
        src.contains("impl From<bool> for BooleanType"),
        "BooleanType must implement From<bool>"
    );
    assert!(
        src.contains("impl From<BooleanType> for bool"),
        "BooleanType must implement From<BooleanType> for bool"
    );

    // From<bool> maps true → Self::T and false → Self::F
    assert!(
        src.contains("if val { Self::T } else { Self::F }"),
        "BooleanType From<bool> must map true/false to T/F variants"
    );

    // Encoder bool setter (todo 58)
    assert!(
        src.contains("available_bool"),
        "Car encoder must have available_bool"
    );

    // Decoder bool getter (todo 58)
    assert!(
        src.contains("available_bool"),
        "Car decoder must have available_bool"
    );
}

// ── VarData maxLength enforcement (todo 30) ───────────────────────────

#[test]
fn generated_code_has_vardata_maxlength() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // EncodeError must have VarDataTooLong variant
    assert!(
        src.contains("VarDataTooLong { field: &'static str, max_length: usize, actual: usize }"),
        "EncodeError must have VarDataTooLong variant"
    );
    // Encoder methods must emit a max_length check that returns VarDataTooLong
    assert!(
        src.contains("return Err(sbe_rt::EncodeError::VarDataTooLong {"),
        "encoder var-data methods must return VarDataTooLong on overflow"
    );
    // Display impl must describe the VarDataTooLong error
    assert!(
        src.contains("var data too long for field"),
        "EncodeError Display must describe VarDataTooLong"
    );
}

// ── Codegen gap documentation: `<ref>` inside composites ───────────────

#[test]
fn composite_ref_gaps_documented() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    // Engine struct is currently [u8; 6] (capacity + numCylinders +
    // manufacturerCode).  The schema also defines three <ref> members
    // (efficiency, boosterEnabled, booster) that would add 4 more bytes
    // for a correct total of 10 bytes, but the XML parser's parse_composite
    // only handles <type> children — <ref> elements are silently skipped.
    assert!(
        src.contains("pub struct Engine(pub [u8; 6]);"),
        "Engine should be [u8; 6] (3 <type> children, 3 <ref> children \
         skipped — fix parse_composite to handle <ref> for a correct 10-byte struct)"
    );
    assert!(
        !src.contains("pub fn efficiency("),
        "Engine::efficiency() should be generated from <ref name=\"efficiency\" type=\"Percentage\"/>, \
         but <ref> is not yet handled inside composites"
    );
    assert!(
        !src.contains("pub fn booster_enabled("),
        "Engine::booster_enabled() should be generated from <ref name=\"boosterEnabled\" type=\"BooleanType\"/>, \
         but <ref> is not yet handled inside composites"
    );
    assert!(
        !src.contains("pub fn booster("),
        "Engine::booster() should be generated from <ref name=\"booster\" type=\"Booster\"/>, \
         but <ref> is not yet handled inside composites"
    );

    // Booster struct is [u8; 1] (just horsePower).  Schema also has an
    // inline <enum name="BoostType"> that is skipped by parse_composite.
    assert!(
        src.contains("pub struct Booster(pub [u8; 1]);"),
        "Booster should be [u8; 1] (inline BoostType enum skipped)"
    );

    // Verify the referenced types do exist in the generated output
    // (they are top-level types, just not inlined into Engine/Booster).
    // Percentage is a <type primitiveType="int8"/> that gets inlined
    // as i8 at the use site -- no standalone type is generated for it.
    assert!(
        src.contains("pub enum BooleanType"),
        "BooleanType should exist as a top-level enum"
    );
    assert!(
        src.contains("pub struct Booster"),
        "Booster composite should exist as a top-level type"
    );

    // BoostType inline enum is defined inside the Booster <composite>
    // but parse_composite only handles <type> children, so it is also skipped.
    assert!(
        !src.contains("BoostType"),
        "BoostType inline enum inside Booster <composite> is also skipped by parse_composite"
    );
}

// ── VarData maxLength runtime check (todo 30) ───────────────────────────

#[test]
fn vardata_maxlength_runtime() {
    let (_schema, src) = generate(&Paths::example_schema(), "vardata_max_len");
    compile_and_run(
        "vardata_max_len",
        &src,
        r#"
        // Encode activationCode within maxLength (1073741824) via checked → OK
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        assert!(car.activation_code(b"12345").is_ok(), "activationCode within maxLength via checked");

        // Encode same data via _unchecked → OK
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer_unchecked(b"Honda").unwrap();
        let car = car.model_unchecked(b"Civic").unwrap();
        assert!(car.activation_code_unchecked(b"12345").is_ok(), "activationCode within maxLength via unchecked");

        // Both paths produce identical encoded output
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"12345").unwrap();
        let checked_bytes = car.as_bytes().to_vec();

        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer_unchecked(b"Honda").unwrap();
        let car = car.model_unchecked(b"Civic").unwrap();
        let car = car.activation_code_unchecked(b"12345").unwrap();
        let unchecked_bytes = car.as_bytes().to_vec();

        assert_eq!(checked_bytes, unchecked_bytes, "checked and unchecked encodings must match");
        "#,
    );
}

// ── Boolean round-trip via bool setter/getter (todo 58) ──────────────────

#[test]
fn boolean_roundtrip_runtime() {
    let (_schema, src) = generate(&Paths::example_schema(), "bool_rt");
    compile_and_run(
        "bool_rt",
        &src,
        r#"
        // Encode with available_bool(true), decode, verify
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available_bool(true);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"12345").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let available = car2.available();
        assert_eq!(available, BooleanType::T, "round-trip available via available_bool(true)");
        assert_ne!(available.raw(), 0, "BooleanType::T raw != 0");

        // Encode with available_bool(false), decode, verify
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::F);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"12345").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let available = car2.available();
        assert_eq!(available, BooleanType::F, "round-trip available via BooleanType::F");
        assert_eq!(available.raw(), 0, "BooleanType::F raw == 0");
        "#,
    );

    // Also verify From<bool> conversion compiles and works
    assert!(src.contains("impl From<bool> for BooleanType"));
    assert!(src.contains("impl From<BooleanType> for bool"));
}

// ── Bound-check-disabled feature toggle (todo 07) ───────────────────────

#[test]
fn bounds_checking_switch() {
    let (_schema, src) = generate(&Paths::example_schema(), "bndchk");

    // Verify cfg gates exist in generated source
    assert!(
        src.contains(r#"#[cfg(feature = "bound-check-disabled")]"#),
        "generated code must have cfg(feature = bound-check-disabled)"
    );
    assert!(
        src.contains(r#"#[cfg(not(feature = "bound-check-disabled"))]"#),
        "generated code must have cfg(not(feature = bound-check-disabled))"
    );

    // Run the same test code both with and without the feature → field values match
    let test_body = r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(42);
        car.model_year(2000);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(2, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"U").unwrap(); }).unwrap();
            g.add(|e| { e.speed(55).mpg(49.0); e.usage_description(b"C").unwrap(); }).unwrap();
        }).unwrap();
        let car = car.performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(1, |a| {
                    a.add(|x| { x.mph(30).seconds(4.0); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"12345").unwrap();
        let encoded = car.as_bytes().to_vec();

        let car2 = CarDecoder::wrap_and_apply_header(&encoded, 0).unwrap();
        assert_eq!(42, car2.serial_number());
        assert_eq!(2000, car2.model_year());
        assert_eq!(BooleanType::T, car2.available());
        assert_eq!(Model::A, car2.code());
        assert_eq!([1u32, 2, 3, 4], car2.some_numbers().unwrap());
        assert_eq!([97, 98, 99, 100, 101, 102], car2.vehicle_code().unwrap());
        let ff: Vec<_> = car2.fuel_figures().unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(2, ff.len());
        assert_eq!(b"Honda", car2.manufacturer().unwrap());
        assert_eq!(b"Civic", car2.model().unwrap());
        assert_eq!(b"12345", car2.activation_code().unwrap());
    "#;

    compile_and_run("bndchk_off", &src, test_body);
    compile_and_run_with_feature("bndchk_on", &src, test_body, "bound-check-disabled");
}

// ── BufferTooShort needed: field size, not absolute position (todo 27) ──

#[test]
fn buffer_too_short_needed_delta() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    // Source-level: ALL BufferTooShort error constructions must use a delta
    // (field size, block length, etc.) for `needed`, NOT an absolute buffer
    // position.  Reject patterns like `needed: self.pos + ...` or
    // `needed: offset + FIELD_SIZE` that would depend on position.
    for line in src.lines() {
        if line.contains("BufferTooShort") && line.contains("needed:") {
            assert!(
                !line.contains("needed: self.pos") && !line.contains("needed: offset +"),
                "BufferTooShort `needed` must be field size (delta), \
                 not an absolute position:\n  {line}"
            );
        }
    }

    // Runtime: verify needed/available at the call site using to_string()
    // on the error type (Display shows the values).
    compile_and_run(
        "bts_delta",
        &src,
        r#"
        // 1. Decoder: header buffer too short (buf has 3 bytes, header needs 8)
        let buf = vec![0u8; 3];
        let Err(err) = CarDecoder::wrap_and_apply_header(&buf, 0) else { panic!("expected Err") };
        let msg = err.to_string();
        assert!(msg.contains("needed 8"), "header decoder: expected needed 8, got: {msg}");
        assert!(msg.contains("3 available"), "header decoder: expected 3 available, got: {msg}");

        // 3. Encoder: header buffer too short.  needed = header + blockLength
        //    = 8 + 41 = 49, NOT the absolute position.
        let mut buf = vec![0u8; 3];
        let Err(err) = CarEncoder::<'_, car_encoder_state::NeedsFuelFigures>::wrap_and_apply_header(&mut buf, 0) else { panic!("expected Err") };
        let msg = err.to_string();
        assert!(msg.contains("needed 49"), "encode: expected needed 49 (header 8 + blockLength 41), got: {msg}");
        assert!(msg.contains("available 3"), "encode: expected available 3, got: {msg}");
    "#,
    );
}

// ── #[inline] on generated methods (todo 28) ────────────────────────────

#[test]
fn generated_code_has_inline_annotations() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    // Count total #[inline] annotations across all generated methods.
    let count = src.matches("#[inline]").count();
    assert!(
        count >= 50,
        "expected >=50 #[inline] annotations across decoder/encoder/group \
         methods in the car example, found {count}"
    );

    // Spot-check that #[inline] precedes key method signatures.
    // Use line windows to handle varying indentation in the generated source.
    let lines: Vec<&str> = src.lines().collect();
    let inline_followed_by: Vec<&str> = lines
        .windows(2)
        .filter(|w| w[0].trim() == "#[inline]")
        .map(|w| w[1].trim())
        .collect();

    // Decoder checked accessor
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.starts_with("pub fn serial_number(")),
        "decoder checked accessor `serial_number` missing #[inline]"
    );
    // Decoder unchecked accessor
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.starts_with("pub const unsafe fn serial_number_unchecked(")),
        "decoder unchecked accessor `serial_number_unchecked` missing #[inline]"
    );

    // Group decoder methods
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.contains("fn fuel_figures(")),
        "group decoder accessor `fuel_figures` missing #[inline]"
    );
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.contains("fn is_empty(")),
        "group decoder `is_empty` missing #[inline]"
    );
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.contains("fn as_chunks(")),
        "group decoder `as_chunks` missing #[inline]"
    );
    // Group decoder wrap (function signature is `pub fn wrap(buf: ...)` inside
    // `impl<...> FuelFiguresDecoder<...>` -- no "Decoder" in the fn line itself)
    assert!(
        inline_followed_by
            .iter()
            .any(|s| !s.starts_with("pub fn encoded_length")
                && s.contains("fn wrap(")
                && s.contains("acting_version")),
        "group decoder `wrap` missing #[inline]"
    );

    // Encoder entry-point methods
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.ends_with("fn wrap_and_apply_header(")),
        "encoder `wrap_and_apply_header` missing #[inline]"
    );
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.starts_with("pub fn encoded_length(")
                || s.starts_with("pub fn encoded_length_with_header(")),
        "encoder `encoded_length` missing #[inline]"
    );
}

// ── #[must_use] on encoder types and methods (todo 28) ──────────────────

#[test]
fn generated_code_has_must_use_annotations() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    let count_plain = src.matches("#[must_use]").count();
    let count_msg = src.matches("#[must_use = \"").count();
    let count = count_plain + count_msg;
    assert!(
        count >= 20,
        "expected >=20 #[must_use] annotations on encoder types/methods \
         in the car example, found {count}"
    );

    let lines: Vec<&str> = src.lines().collect();
    let must_use_followed_by: Vec<&str> = lines
        .windows(2)
        .filter(|w| w[0].trim().starts_with("#[must_use"))
        .map(|w| w[1].trim())
        .collect();

    // #[must_use] on encoder struct type
    assert!(
        must_use_followed_by
            .iter()
            .any(|s| s.starts_with("pub struct CarEncoder<")),
        "CarEncoder struct missing #[must_use]"
    );
    assert!(
        must_use_followed_by
            .iter()
            .any(|s| s.starts_with("pub struct FuelFiguresEncoder<")),
        "FuelFiguresEncoder struct missing #[must_use]"
    );

    // #[must_use] on encoder setters returning &mut Self
    assert!(
        must_use_followed_by
            .iter()
            .any(|s| s.starts_with("pub fn serial_number(") && s.contains("&mut Self")),
        "encoder serial_number setter missing #[must_use]"
    );
    assert!(
        must_use_followed_by
            .iter()
            .any(|s| s.starts_with("pub fn model_year(") && s.contains("&mut Self")),
        "encoder model_year setter missing #[must_use]"
    );

    // #[must_use] on Result-returning encoder methods
    // After prettyplease formatting, `Result` is on the next line,
    // so only check for the function name.
    assert!(
        must_use_followed_by
            .iter()
            .any(|s| s.contains("fn fuel_figures<")),
        "encoder group method `fuel_figures` missing #[must_use]"
    );
    assert!(
        must_use_followed_by
            .iter()
            .any(|s| s.starts_with("pub fn add<") && s.contains("Result")),
        "group encoder `add()` missing #[must_use]"
    );
}

// ── Static HEADER_TEMPLATE and GROUP_DIM_TEMPLATE (todo 39) ──────────

#[test]
fn static_header_templates_exist() {
    let (_schema, src) = generate(&Paths::example_schema(), "static_tpl");

    // Source: verify const declarations
    assert!(
        src.contains("pub const HEADER_TEMPLATE: [u8; 8] = [41, 0, 1, 0, 1, 0, 0, 0];"),
        "HEADER_TEMPLATE must contain correct pre-computed header bytes \
         (blockLength=41, templateId=1, schemaId=1, version=0, little-endian)"
    );
    assert!(
        src.contains("pub const GROUP_DIM_TEMPLATE: [u8; 4] ="),
        "GROUP_DIM_TEMPLATE must exist as a [u8; 4] constant"
    );

    // wrap_and_apply_header must use copy_from_slice from HEADER_TEMPLATE
    assert!(
        src.contains("buf[pos..pos + 8].copy_from_slice(&Self::HEADER_TEMPLATE)"),
        "wrap_and_apply_header must use copy_from_slice from HEADER_TEMPLATE"
    );

    // Group encoder must use copy_from_slice from its GROUP_DIM_TEMPLATE
    assert!(
        src.contains(".copy_from_slice(&FuelFiguresEncoder::GROUP_DIM_TEMPLATE)"),
        "group encoder must use copy_from_slice from its GROUP_DIM_TEMPLATE"
    );

    // Runtime: wrap_and_apply_header writes HEADER_TEMPLATE bytes correctly
    compile_and_run(
        "static_tpl",
        &src,
        r#"
        let mut buf = vec![0u8; 512];
        // Drop the encoder immediately to release the mutable borrow,
        // then verify header bytes were written correctly.
        let _ = CarEncoder::<'_, car_encoder_state::NeedsFuelFigures>::wrap_and_apply_header(&mut buf, 0).unwrap();
        assert_eq!(
            &buf[0..8],
            &CarEncoder::<'_, car_encoder_state::NeedsFuelFigures>::HEADER_TEMPLATE,
            "wrap_and_apply_header must write HEADER_TEMPLATE bytes"
        );
        // Verify the template bytes are semantically correct
        let block_len = u16::from_le_bytes([buf[0], buf[1]]);
        let template_id = u16::from_le_bytes([buf[2], buf[3]]);
        let schema_id = u16::from_le_bytes([buf[4], buf[5]]);
        let version = u16::from_le_bytes([buf[6], buf[7]]);
        assert_eq!(block_len, 41, "header blockLength must be 41");
        assert_eq!(template_id, 1, "header templateId must be 1");
        assert_eq!(schema_id, 1, "header schemaId must be 1");
        assert_eq!(version, 0, "header version must be 0");
    "#,
    );
}
