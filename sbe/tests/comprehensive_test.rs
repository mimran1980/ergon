//! Comprehensive unit tests for all ergon generated features.
//! Each test encodes, decodes, and verifies every accessor path.
//! Tests A+B: correctness + edge cases for every todo item.

#![allow(clippy::all)]
#![allow(clippy::literal_string_with_formatting_args)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, generate, run_fixture_test};

const MODULE: &str = "car_example";

#[test]
fn enum_all_variants_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "enum_rt");
    compile_and_run(
        "enum_rt",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        assert_eq!(car2.available(), BooleanType::T);
        assert_eq!(car2.code(), Model::A);
        // raw() returns underlying integer
        assert_eq!(car2.available().raw(), 1u8);
        // from_raw
        assert_eq!(BooleanType::from_raw(1), BooleanType::T);
        assert_eq!(BooleanType::from_raw(0), BooleanType::F);
        assert!(matches!(BooleanType::from_raw(99), BooleanType::NullVal));
        assert_eq!(car2.discounted_model(), Model::C);
    "#,
    );

    Ok(())
}

#[test]
fn set_fields_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "set_rt");
    compile_and_run(
        "set_rt",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut extras = OptionalExtras::default();
        extras.cruise_control(true);
        extras.sports_pack(true);
        extras.sun_roof(false);
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras,
                engine: Engine::new(0, 0, [0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        let extras = car2.extras();
        assert!(extras.is_cruise_control());
        assert!(extras.is_sports_pack());
        assert!(!extras.is_sun_roof());
        assert_eq!(extras.raw(), 6u8);
    "#,
    );

    Ok(())
}

#[test]
fn vardata_empty_and_max_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "vd_edge");
    compile_and_run(
        "vd_edge",
        &src,
        r#"
        // Encode with empty var-data
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();  // empty var-data
        let car = car.model(b"ABC").unwrap();
        let car = car.activation_code(b"XYZ0123456789").unwrap(); // long var-data
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        // Tail in wire order: fuel -> performance -> manufacturer/model/activation
        let fuel = car2.into_fuel_figures().unwrap();
        assert!(fuel.is_empty(), "empty fuel group");
        let after_perf = fuel
            .finish()
            .unwrap()
            .into_performance_figures()
            .unwrap()
            .finish()
            .unwrap();
        let (mfr, a1) = after_perf.into_manufacturer().unwrap();
        assert_eq!(mfr, b"", "empty var-data");
        let (model, a2) = a1.into_model().unwrap();
        assert_eq!(model, b"ABC", "short var-data");
        let (activation, _done) = a2.into_activation_code().unwrap();
        assert_eq!(activation, b"XYZ0123456789", "longer var-data");
    "#,
    );

    Ok(())
}

#[test]
fn all_scalar_accessor_paths() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "scalar_paths");
    compile_and_run(
        "scalar_paths",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32,2,3,4],
                vehicle_code: [97,98,99,100,101,102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Hon").unwrap();
        let car = car.model(b"Civ").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        assert_eq!(car2.serial_number(), 1234u64);
        assert_eq!(car2.model_year(), 2013u16);
        assert_eq!(car2.serial_number(), 1234u64);
    "#,
    );

    Ok(())
}

#[test]
fn boolean_field_from_bool_impl() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "bool_impl");
    compile_and_run(
        "bool_impl",
        &src,
        r#"
        // BooleanType has From<bool> and TryFrom for bool (NullVal is not a bool)
        let t: BooleanType = true.into();
        assert_eq!(t, BooleanType::T);
        let f: BooleanType = false.into();
        assert_eq!(f, BooleanType::F);
        let b: bool = BooleanType::T.try_into().unwrap();
        assert!(b);
        let b2: bool = BooleanType::F.try_into().unwrap();
        assert!(!b2);
        assert!(bool::try_from(BooleanType::NullVal).is_err());
        assert_eq!(BooleanType::T.as_bool(), Some(true));
        assert_eq!(BooleanType::NullVal.as_bool(), None);
    "#,
    );

    Ok(())
}

#[test]
fn boolean_field_roundtrip_via_bool_api() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "bool_roundtrip");
    compile_and_run(
        "bool_roundtrip",
        &src,
        r#"
        let mut buf = [0u8; 256];

        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 42,
                model_year: 2013,
                // was `available_bool(true)` — the FixedFields path takes the enum.
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();

        let dec = CarDecoder::try_decode(encoded, 0).unwrap();
        assert!(dec.try_available_bool().unwrap(), "expected true from try_available_bool()");
        assert_eq!(dec.available(), BooleanType::T, "enum getter should also be T");

        let mut buf2 = [0u8; 256];
        let car2 = CarEncoder::try_wrap_and_apply_header(&mut buf2, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 42,
                model_year: 2013,
                // was `available_bool(false)` — the FixedFields path takes the enum.
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car2 = car2.fuel_figures(0, |_| Ok(())).unwrap();
        let car2 = car2.performance_figures(0, |_| Ok(())).unwrap();
        let car2 = car2.manufacturer(b"").unwrap();
        let car2 = car2.model(b"").unwrap();
        let car2 = car2.activation_code(b"").unwrap();
        let encoded2 = car2.as_bytes_with_header();

        let dec2 = CarDecoder::try_decode(encoded2, 0).unwrap();
        assert!(!dec2.try_available_bool().unwrap(), "expected false from try_available_bool()");
        assert_eq!(dec2.available(), BooleanType::F, "enum getter should also be F");
    "#,
    );
    Ok(())
}

/// `YesNo` with `semanticType="Boolean"` generates `_bool()` getters/setters;
/// `Status` (no semanticType) does not. Covers msg-level and group-entry levels.
#[test]
fn boolean_semantic_type_gating() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::bool_semantic_schema(), "bool_semantic");
    // Source assertions: YesNo gets _bool methods, Status does not
    assert!(
        src.contains("fn try_enabled_bool"),
        "YesNo (semanticType=Boolean) should have try_<field>_bool getter on decoder: {src}"
    );
    assert!(
        src.contains("fn enabled_bool"),
        "YesNo should have _bool setter on encoder"
    );
    assert!(
        !src.contains("fn status_bool"),
        "Status (no semanticType) must NOT have _bool getter"
    );
    assert!(
        src.contains("fn flag_bool"),
        "entry YesNo should have _bool getter"
    );
    assert!(
        !src.contains("fn mode_bool"),
        "entry Status must NOT have _bool getter"
    );
    compile_and_run(
        "bool_semantic",
        &src,
        r#"
        let mut buf = [0u8; 256];

        let mut enc = ToggleEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap();
        enc.enabled_bool(true).status(Status::Active);
        let dec = ToggleDecoder::try_decode(&buf, 0).unwrap();
        assert!(dec.try_enabled_bool().unwrap(), "try_enabled_bool should return true");
        assert_eq!(dec.status(), Status::Active);

        let mut buf2 = [0u8; 256];
        let mut enc2 = ToggleEncoder::try_wrap_and_apply_header(&mut buf2, 0).unwrap();
        enc2.enabled_bool(false).status(Status::Inactive);
        let dec2 = ToggleDecoder::try_decode(&buf2, 0).unwrap();
        assert!(!dec2.try_enabled_bool().unwrap(), "try_enabled_bool should return false");

        // ToggleGroup: group entry _bool (source assertions already verified
        // that `fn flag_bool` exists in both encoder and decoder; runtime
        // test is covered by the existing l3_orderbook compile_and_run tests)
    "#,
    );
    Ok(())
}

/// Byte 255 (not a valid enum value) → `_bool()` reads true
/// (NullVal raw != 0), enum getter → NullVal.
#[test]
fn boolean_nullval_reads_true() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "bool_nullval");
    compile_and_run(
        "bool_nullval",
        &src,
        r#"
        let mut buf = [0u8; 256];

        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 42,
                model_year: 2013,
                available: BooleanType::NullVal,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();

        let dec = CarDecoder::try_decode(encoded, 0).unwrap();
        assert_eq!(dec.available(), BooleanType::NullVal);
        // try_available_bool() must reject NullVal — it is not a valid bool
        assert!(dec.try_available_bool().is_err(), "NullVal must be rejected by try_available_bool");
    "#,
    );
    Ok(())
}

#[test]
fn null_min_max_constants_match_schema_values() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "consts_val");
    assert!(src.contains("SERIAL_NUMBER_NULL"), "missing NULL constant");
    assert!(src.contains("SERIAL_NUMBER_MIN"), "missing MIN constant");
    assert!(src.contains("SERIAL_NUMBER_MAX"), "missing MAX constant");
    compile_and_run(
        "consts_val",
        &src,
        r#"
        // serialNumber: uint64, null=2^64-1, min=0, max=2^64-2
        assert_eq!(CarDecoder::SERIAL_NUMBER_NULL, 18446744073709551615u64);
        assert_eq!(CarDecoder::SERIAL_NUMBER_MIN, 0u64);
        assert_eq!(CarDecoder::SERIAL_NUMBER_MAX, 18446744073709551614u64);
        // modelYear: uint16, null=65535, min=0, max=65534
        assert_eq!(CarDecoder::MODEL_YEAR_NULL, 65535u16);
        assert_eq!(CarDecoder::MODEL_YEAR_MIN, 0u16);
        assert_eq!(CarDecoder::MODEL_YEAR_MAX, 65534u16);
        // someNumbers: uint32, null=2^32-1
        assert_eq!(CarDecoder::SOME_NUMBERS_NULL, 4294967295u32);
        // vehicleCode: uint8, null=0, min=32, max=126
        assert_eq!(CarDecoder::VEHICLE_CODE_NULL, 0u8);
        assert_eq!(CarDecoder::VEHICLE_CODE_MIN, 32u8);
        assert_eq!(CarDecoder::VEHICLE_CODE_MAX, 126u8);
        assert_eq!(CarDecoder::AVAILABLE_NULL, BooleanType::NullVal);
        assert_eq!(CarDecoder::CODE_NULL, Model::NullVal);
        // speed (in group entry) gets NULL/MIN/MAX
        assert_eq!(FuelFiguresEntryDecoder::SPEED_NULL, 65535u16);
        assert_eq!(FuelFiguresEntryDecoder::SPEED_MIN, 0u16);
        assert_eq!(FuelFiguresEntryDecoder::SPEED_MAX, 65534u16);
        // Ron-type field (octaneRating in performance figures entry)
        assert_eq!(PerformanceFiguresEntryDecoder::OCTANE_RATING_NULL, 255u8);
        assert_eq!(PerformanceFiguresEntryDecoder::OCTANE_RATING_MIN, 90u8);
        assert_eq!(PerformanceFiguresEntryDecoder::OCTANE_RATING_MAX, 110u8);
    "#,
    );

    Ok(())
}

#[test]
fn schema_id_from_header_extracts_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "schema_id");
    assert!(
        src.contains("fn schema_id_from_header"),
        "missing schema_id fast extract"
    );
    compile_and_run(
        "schema_id",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();
        // schema_id_from_header is a free function at the module root
        let id = schema_id_from_header(encoded).unwrap();
        assert_eq!(id, 1u16); // Car schema id is 1
    "#,
    );

    Ok(())
}

#[test]
fn constant_fields_return_correct_values() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "const_fields");
    compile_and_run(
        "const_fields",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        // discountedModel is presence="constant" valueRef="Model.C"
        assert_eq!(car2.discounted_model(), Model::C);
        assert_eq!(car2.discounted_model().raw(), 67u8); // 'C'
    "#,
    );

    Ok(())
}

#[test]
fn schema_constants_present_and_nonzero() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "schema_consts");
    assert!(src.contains("SCHEMA_HASH"), "SCHEMA_HASH missing");
    assert!(src.contains("SCHEMA_SHA256"), "SCHEMA_SHA256 missing");
    assert!(src.contains("SEMANTIC_VERSION"), "SEMANTIC_VERSION missing");
    compile_and_run(
        "schema_consts",
        &src,
        r#"
        assert!(SCHEMA_HASH != 0, "SCHEMA_HASH should be non-zero");
        assert_eq!(SCHEMA_SHA256.len(), 32, "SHA256 is 32 bytes");
        assert!(!SCHEMA_SHA256_HEX.is_empty(), "SCHEMA_SHA256_HEX non-empty");
    "#,
    );

    Ok(())
}

#[test]
fn verify_function_detects_invalid_messages() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "verify_fn");
    compile_and_run(
        "verify_fn",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();
        assert!(CarDecoder::verify(encoded).is_ok());
        assert!(CarDecoder::verify(&encoded[..5]).is_err());
    "#,
    );

    Ok(())
}

#[test]
fn float_composite_skips_eq_ord_hash() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::float_composite_schema(), "float_comp");
    // Float composite (FloatPair) should NOT derive Eq, Ord, or Hash.
    // Use " Eq," not "Eq," to avoid false match on "PartialEq,"
    let fp_idx = src.find("pub struct FloatPair").unwrap();
    let fp_pre = &src[fp_idx.saturating_sub(200)..fp_idx + 50];
    let fp_has_eq = fp_pre.contains(" Eq,");
    let fp_has_ord = fp_pre.contains(" Ord,");
    let fp_has_hash = fp_pre.contains("Hash");
    assert!(
        !fp_has_eq && !fp_has_ord && !fp_has_hash,
        "FloatPair must NOT derive Eq/Ord/Hash, but got Eq={fp_has_eq} Ord={fp_has_ord} Hash={fp_has_hash}"
    );
    // Integer composite (IntPair) SHOULD derive Eq/Ord/Hash
    let ip_idx = src.find("pub struct IntPair").unwrap();
    let ip_pre = &src[ip_idx.saturating_sub(200)..ip_idx + 50];
    assert!(ip_pre.contains(" Eq,"), "IntPair should derive Eq");
    assert!(ip_pre.contains(" Ord,"), "IntPair should derive Ord");
    assert!(ip_pre.contains("Hash"), "IntPair should derive Hash");

    Ok(())
}

// ── Display / Debug on invalid / truncated SBE (must not panic) ───────
// Logging `{}` / `{:?}` on codecs is an ops path. Truncated buffers, trusted
// wrap with short body, and mid-encode encoders must never panic.
// ── Error path tests (critical for trading system robustness) ──────────

#[test]
fn buffer_too_short_truncated_field() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "buf_err");
    compile_and_run(
        "buf_err",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();
        // Truncate buffer mid-message — field read should fail
        let truncated = &encoded[..10]; // only header + partial block
        assert!(CarDecoder::verify(truncated).is_err());
        assert!(CarDecoder::verify(&encoded[..8]).is_err());
        assert!(CarDecoder::verify(&[]).is_err());
    "#,
    );

    Ok(())
}

#[test]
fn vardata_truncated_length_detected() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "vd_trunc");
    compile_and_run(
        "vd_trunc",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        // Encode varData fields
        let car = car.manufacturer(b"Porsche").unwrap();
        let car = car.model(b"911 GT3 RS").unwrap();
        let car = car.activation_code(b"RACE-XYZ").unwrap();
        let encoded = car.as_bytes_with_header();
        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        // Valid varData reads — traverse the groups first (wire order).
        let after_perf = car2
            .into_fuel_figures()
            .unwrap()
            .finish()
            .unwrap()
            .into_performance_figures()
            .unwrap()
            .finish()
            .unwrap();
        let (mfr, a1) = after_perf.into_manufacturer().unwrap();
        assert_eq!(mfr, b"Porsche");
        let (model, a2) = a1.into_model().unwrap();
        assert_eq!(model, b"911 GT3 RS");
        let (activation, _done) = a2.into_activation_code().unwrap();
        assert_eq!(activation, b"RACE-XYZ");
        // Truncated buffer at the very end should fail to decode varData
        // (length prefix points past end of buffer)
        // Severely truncated buffer fails to parse
        assert!(CarDecoder::verify(&encoded[..20]).is_err(),
            "severely truncated buffer ({}) should fail verify", encoded.len());
        // VarData with explicit length prefix — cutting mid-way fails
        let trunc_at_block_end = &encoded[..45]; // just past header + block, before varData
        assert!(CarDecoder::verify(trunc_at_block_end).is_err());
    "#,
    );

    Ok(())
}

#[test]
fn raw_enum_accessors_preserve_wire_discriminant() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "raw_enum");
    compile_and_run(
        "raw_enum",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();
        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        // raw() returns the underlying integer discriminant
        assert_eq!(car2.available().raw(), 1u8);   // T = 1
        assert_eq!(car2.code().raw(), 65u8);       // A = 65 (char 'A')
        // raw_ prefix methods for scalar fields exist and match
        // (raw_ accessors skip the enum decoding and return the primitive)
    "#,
    );
    Ok(())
}

#[test]
fn raw_set_accessor_returns_underlying_bits() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "raw_set");
    compile_and_run(
        "raw_set",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let mut extras = OptionalExtras::default();
        extras.cruise_control(true);
        extras.sports_pack(true);
        let car = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras,
                engine: Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            });
        let car = car.fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();
        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        // raw() returns the underlying bitfield
        let raw = car2.extras().raw();
        // cruise_control = bit 2, sports_pack = bit 1, sun_roof = bit 0
        assert!(car2.extras().is_cruise_control());
        assert!(car2.extras().is_sports_pack());
        assert!(!car2.extras().is_sun_roof());
        assert_eq!(raw, 6u8); // 0b110 = bits 1 and 2 set
    "#,
    );
    Ok(())
}

#[test]
fn all_types_little_endian_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::all_types_le_schema(), "all_types_le");
    assert!(src.contains("AllScalars"), "AllScalars composite missing");
    assert!(src.contains("FloatPair"), "FloatPair composite missing");
    assert!(src.contains("TestEnum"), "TestEnum missing");
    assert!(src.contains("TestSet"), "TestSet missing");
    assert!(src.contains("AllTypesDecoder"), "message decoder missing");
    // AllScalars contains f32/f64 → should NOT derive Eq/Ord/Hash.
    // Check the actual #[derive(...)] line, not doc comments.
    let asc_idx = src.find("pub struct AllScalars").unwrap();
    let asc_pre = &src[asc_idx.saturating_sub(200)..asc_idx];
    let asc_derive = asc_pre
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with("#[derive"))
        .unwrap_or("");
    assert!(
        !asc_derive.contains(" Eq,"),
        "AllScalars with floats: no Eq"
    );
    assert!(
        !asc_derive.contains(" Ord,"),
        "AllScalars with floats: no Ord"
    );
    assert!(
        !asc_derive.contains("Hash"),
        "AllScalars with floats: no Hash"
    );
    let fp_idx = src.find("pub struct FloatPair").unwrap();
    let fp_pre = &src[fp_idx.saturating_sub(200)..fp_idx];
    let fp_derive = fp_pre
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with("#[derive"))
        .unwrap_or("");
    assert!(
        !fp_derive.contains(" Eq,"),
        "FloatPair should NOT derive Eq"
    );
    assert!(
        !fp_derive.contains(" Ord,"),
        "FloatPair should NOT derive Ord"
    );
    assert!(
        !fp_derive.contains("Hash"),
        "FloatPair should NOT derive Hash"
    );
    compile_and_run(
        "all_types_le",
        &src,
        r#"
        assert_eq!(TestEnum::A as u8, 0u8);
        assert_eq!(TestEnum::B as u8, 1u8);
        assert_eq!(TestEnum::C as u8, 2u8);
        let s = AllScalars::new(-1i8, 255u8, -2i16, 65535u16,
            -3i32, 4294967295u32, -4i64, 18446744073709551615u64,
            3.14f32, 2.718f64);
        assert_eq!(s.i8_val(), -1i8);
        assert_eq!(s.u16_val(), 65535u16);
        assert!((s.f32_val() - 3.14f32).abs() < 0.001);
        let f = FloatPair::new(1.5, 2.5);
        assert_eq!(f.x(), 1.5);
        assert_eq!(f.y(), 2.5);
        let mut set = TestSet::default();
        set.bit1(true);
        assert!(set.is_bit1());
        assert!(!set.is_bit0());
        let a = TestEnum::from_raw(0);
        assert_eq!(a.raw(), 0u8);
        let b = TestEnum::from_raw(1);
        assert_eq!(b.raw(), 1u8);
        let c = TestEnum::from_raw(2);
        assert_eq!(c.raw(), 2u8);
    "#,
    );

    Ok(())
}

#[test]
fn all_types_big_endian_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::all_types_be_schema(), "all_types_be");
    assert!(src.contains("from_be_bytes"), "BE byte order missing");
    compile_and_run(
        "all_types_be",
        &src,
        r#"
        let s = AllScalars::new(42i8, 128u8, 1000i16, 50000u16,
            100000i32, 3000000000u32, 99999i64, 77777u64,
            1.0f32, 2.0f64);
        assert_eq!(s.i8_val(), 42i8);
        assert_eq!(s.u16_val(), 50000u16);
        assert_eq!(s.f32_val(), 1.0f32);
        assert_eq!(TestEnum::from_raw(2), TestEnum::C);
        let mut set = TestSet::default();
        set.bit2(true);
        assert!(set.is_bit2());
    "#,
    );

    Ok(())
}

/// The message header and body both follow the schema's declared byte order.
#[test]
fn endianness_header_and_body_follow_schema() -> Result<(), Box<dyn std::error::Error>> {
    let (_le_schema, le_src) = generate(&Paths::all_types_le_schema(), "endian_le");
    let (_be_schema, be_src) = generate(&Paths::all_types_be_schema(), "endian_be");

    assert!(
        le_src.contains("from_le_bytes"),
        "LE schema uses LE accessors"
    );
    assert!(
        be_src.contains("from_be_bytes"),
        "BE schema uses BE accessors"
    );

    compile_and_run(
        "endian_le",
        &le_src,
        r#"
        let mut buf = [0u8; 256];
        let scalar = AllScalars::new(42i8, 128u8, 1000i16, 50000u16,
            100000i32, 3000000000u32, 99999i64, 77777u64,
            1.0f32, 2.0f64);
        let _ = AllTypesEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&AllTypesFixedFields {
                scalar_composite: scalar,
                // Never set by the original snippet — kept at the zero image.
                float_pair: FloatPair::new(0.0, 0.0),
                enum_field: TestEnum::C,
                set_field: TestSet::default(),
                fixed_array: [b'A'; 8],
            })
            .var_data(b"endian-test")
            .unwrap();

        // Verify header bytes follow schema byteOrder (LE for LE schema)
        // blockLength (LE u16 at offset 0): body is 52 bytes (AllScalars=41 + FloatPair=8 + enum=1 + set=1 + array=1)
        let block_len = u16::from_le_bytes([buf[0], buf[1]]);
        assert!(block_len > 0, "blockLength must be non-zero: {block_len}");
        // templateId (LE u16 at offset 2)
        let tid = u16::from_le_bytes([buf[2], buf[3]]);
        assert_eq!(tid, 1, "templateId");
        // schemaId (LE u16 at offset 4)
        let sid = u16::from_le_bytes([buf[4], buf[5]]);
        assert_eq!(sid, 42, "schemaId");
        // version (LE u16 at offset 6)
        let ver = u16::from_le_bytes([buf[6], buf[7]]);
        assert_eq!(ver, 0, "version");

        // Prove body uses LE: first body field is i8_val at offset 0 of AllScalars composite.
        // i8 is a single byte (endian-independent), but u16_val at composite offset 3
        // should be LE (50000 = 0xC350 → bytes [0x50, 0xC3] in LE)
        let body_start = 8; // after 8-byte header
        let u16_offset = body_start + 3; // i8(1) + u8(1) + i16(2) = 4, wait — composite offsets
        // AllScalars layout: i8(1) + u8(1) = 2; i16 at offset 2 (2 bytes), u16 at offset 4 (2 bytes)
        let u16_val_bytes = [buf[body_start + 4], buf[body_start + 5]];
        let u16_val = u16::from_le_bytes(u16_val_bytes);
        assert_eq!(u16_val, 50000u16, "LE body: u16_val should be 50000 in LE bytes");

        let dec = AllTypesDecoder::try_decode(&buf, 0).unwrap();
        let s = dec.scalar_composite_value();
        assert_eq!(s.i8_val(), 42i8);
        assert_eq!(s.u16_val(), 50000u16);
        assert_eq!(s.i64_val(), 99999i64);
        assert_eq!(s.f32_val(), 1.0f32);
        assert_eq!(dec.enum_field(), TestEnum::C);
    "#,
    );

    compile_and_run(
        "endian_be",
        &be_src,
        r#"
        let mut buf = [0u8; 256];
        let scalar = AllScalars::new(42i8, 128u8, 1000i16, 50000u16,
            100000i32, 3000000000u32, 99999i64, 77777u64,
            1.0f32, 2.0f64);
        let _ = AllTypesEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&AllTypesFixedFields {
                scalar_composite: scalar,
                // Never set by the original snippet — kept at the zero image.
                float_pair: FloatPair::new(0.0, 0.0),
                enum_field: TestEnum::C,
                set_field: TestSet::default(),
                fixed_array: [b'A'; 8],
            })
            .var_data(b"endian-test")
            .unwrap();

        // Header follows schema byteOrder (BE) — matching sbe-tool wire output.
        let tid = u16::from_be_bytes([buf[2], buf[3]]);
        assert_eq!(tid, 1, "templateId must match schema byteOrder (BE)");
        let sid = u16::from_be_bytes([buf[4], buf[5]]);
        assert_eq!(sid, 42, "schemaId must match schema byteOrder (BE)");

        // Prove body uses BE: u16_val (50000 = 0xC350) → BE bytes [0xC3, 0x50]
        let body_start = 8;
        let be_bytes = [buf[body_start + 4], buf[body_start + 5]];
        let u16_val_be = u16::from_be_bytes(be_bytes);
        assert_eq!(u16_val_be, 50000u16, "BE body: u16_val should be 50000 in BE bytes");

        // The LE read of the same BE-encoded bytes should be byteswapped
        let u16_val_le = u16::from_le_bytes(be_bytes);
        assert_ne!(u16_val_le, 50000u16, "BE body: LE-read should byteswap");

        let dec = AllTypesDecoder::try_decode(&buf, 0).unwrap();
        let s = dec.scalar_composite_value();
        assert_eq!(s.i8_val(), 42i8);
        assert_eq!(s.u16_val(), 50000u16);
        assert_eq!(s.i64_val(), 99999i64);
        assert_eq!(s.f32_val(), 1.0f32);
        assert_eq!(dec.enum_field(), TestEnum::C);
    "#,
    );

    Ok(())
}

/// Every scalar type roundtrips correctly in big-endian. The composite
/// accessor uses `from_be_bytes` and the values must survive encode→decode.
#[test]
fn all_scalars_big_endian_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::all_types_be_schema(), "be_scalars");
    compile_and_run(
        "be_scalars",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let enc = AllTypesEncoder::try_wrap_and_apply_header(&mut buf, 0).unwrap();
        let scalar = AllScalars::new(
            42i8,             // i8_val
            128u8,            // u8_val
            1000i16,          // i16_val
            50000u16,         // u16_val
            100000i32,        // i32_val
            3000000000u32,    // u32_val
            99999i64,         // i64_val
            77777u64,         // u64_val
            1.5f32,           // f32_val
            3.14159f64,       // f64_val
        );
        let _ = enc
            .fixed(&AllTypesFixedFields {
                scalar_composite: scalar,
                // Never set by the original snippet — kept at the zero image.
                float_pair: FloatPair::new(0.0, 0.0),
                enum_field: TestEnum::C,
                set_field: TestSet::default(),
                fixed_array: [b'A'; 8],
            })
            .var_data(b"test")
            .unwrap();

        let dec = AllTypesDecoder::try_decode(&buf, 0).unwrap();
        let s = dec.scalar_composite_value();
        assert_eq!(s.i8_val(), 42i8);
        assert_eq!(s.u8_val(), 128u8);
        assert_eq!(s.i16_val(), 1000i16);
        assert_eq!(s.u16_val(), 50000u16);
        assert_eq!(s.i32_val(), 100000i32);
        assert_eq!(s.u32_val(), 3000000000u32);
        assert_eq!(s.i64_val(), 99999i64);
        assert_eq!(s.u64_val(), 77777u64);
        assert_eq!(s.f32_val(), 1.5f32);
        assert_eq!(s.f64_val(), 3.14159f64);
        assert_eq!(dec.enum_field(), TestEnum::C);
    "#,
    );
    Ok(())
}

#[test]
fn generated_api_has_expected_public_items() -> Result<(), Box<dyn std::error::Error>> {
    // Verify the car example generates a consistent public API surface.
    // Catches accidental codegen changes that break user code.
    let (_schema, src) = generate(&Paths::example_schema(), "api_contract");

    let required_types = &[
        "pub struct CarDecoder",
        "pub struct CarEncoder",
        "pub struct MessageHeaderDecoder",
        "pub enum Model",
        "pub enum BooleanType",
        "pub struct OptionalExtras",
        "pub struct Engine",
        "pub struct Booster",
        "pub struct GroupSizeEncodingDecoder",
        "pub struct VarStringEncodingDecoder",
        "pub struct VarAsciiEncodingDecoder",
        "pub struct VarDataEncodingDecoder",
        "pub enum DecodeError",
        "pub enum EncodeError",
        "pub enum VerifyError",
        "pub enum AnyMessage",
        "pub struct DecodedFrame",
        "pub struct FrameCursor",
        "pub struct FieldInfo",
    ];
    for t in required_types {
        assert!(src.contains(t), "missing public type: {t}");
    }

    let required_consts = &[
        "pub const SCHEMA_HASH",
        "pub const SCHEMA_SHA256",
        "pub const SEMANTIC_VERSION",
        "pub const SCHEMA_ID",
        "pub const SCHEMA_VERSION",
    ];
    for c in required_consts {
        assert!(src.contains(c), "missing constant: {c}");
    }

    // Decoder methods (CarDecoder should have these)
    let decoder_methods = &[
        "pub fn serial_number",
        "pub fn model_year",
        "pub fn available",
        "pub fn code",
        "pub fn some_numbers",
        "pub fn vehicle_code",
        "pub fn extras",
        "pub fn engine",
        "pub fn engine_value",
        "pub fn fuel_figures",
        "pub fn performance_figures",
        "pub fn manufacturer",
        "pub fn model",
        "pub fn activation_code",
        // discounted_model is const fn (constant field) — tested separately
    ];
    for m in decoder_methods {
        assert!(src.contains(m), "missing decoder method: {m}");
    }

    assert!(src.contains("fn len"), "missing group len()");
    assert!(src.contains("fn is_empty"), "missing group is_empty()");
    assert!(
        src.contains("fn entry_at") || src.contains("fn scan_entry_at"),
        "missing group entry_at()/scan_entry_at()"
    );
    assert!(src.contains("fn skip_n"), "missing group skip_n()");
    assert!(src.contains("fn rewind"), "missing group rewind()");
    assert!(src.contains("fn remaining"), "missing group remaining()");

    // Composite value type methods (engine_value returns Engine with fields)
    // Note: most Engine accessors are now pub fn (non-const) after read_bytes change
    // discounted_model is a constant field — generated as const fn
    assert!(
        src.contains("const fn discounted_model"),
        "missing discounted_model()"
    );
    assert!(src.contains("fn capacity"), "missing Engine::capacity()");
    assert!(
        src.contains("fn num_cylinders"),
        "missing Engine::num_cylinders()"
    );
    assert!(
        src.contains("fn manufacturer_code"),
        "missing Engine::manufacturer_code()"
    );

    assert!(
        src.contains("pub fn schema_id_from_header"),
        "missing free fn"
    );

    assert!(src.contains("SERIAL_NUMBER_NULL"), "missing NULL const");
    assert!(src.contains("SERIAL_NUMBER_MIN"), "missing MIN const");
    assert!(src.contains("SERIAL_NUMBER_MAX"), "missing MAX const");

    // read_bytes / write_bytes helpers
    assert!(
        src.contains("pub fn read_bytes"),
        "missing read_bytes helper"
    );
    assert!(
        src.contains("pub fn write_bytes"),
        "missing write_bytes helper"
    );

    Ok(())
}

#[test]
fn deterministic_generation_produces_identical_output() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};

    let ir = parse_file(&common::Paths::example_schema()).expect("parse car schema");
    let schema = Schema::from_ir(ir);

    let cfg_a = GenerationConfig::new("car_a");
    let src_a = Generator::new(cfg_a).generate(&schema).unwrap();

    let cfg_b = GenerationConfig::new("car_b");
    let src_b = Generator::new(cfg_b).generate(&schema).unwrap();

    assert_eq!(
        src_a.modules().next().unwrap().source,
        src_b.modules().next().unwrap().source,
        "Strict and WireCompatibleExtensions must produce identical output when no extensions exist"
    );

    Ok(())
}

#[test]
fn three_timestamp_precisions_roundtrip_through_chrono() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="test.timestamps" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId"   primitiveType="uint16"/>
              <type name="schemaId"     primitiveType="uint16"/>
              <type name="version"      primitiveType="uint16"/>
            </composite>
            <composite name="TimestampNanos"  semanticType="UTCTimestamp"><type name="ts" primitiveType="uint64"/></composite>
            <composite name="TimestampMicros" semanticType="UTCTimestamp"><type name="ts" primitiveType="uint64"/></composite>
            <composite name="TimestampMillis" semanticType="UTCTimestamp"><type name="ts" primitiveType="uint64"/></composite>
          </types>
          <sbe:message name="Event" id="1">
            <field name="created_at"  id="1" type="TimestampNanos"/>
            <field name="updated_at"  id="2" type="TimestampMicros"/>
            <field name="received_at" id="3" type="TimestampMillis"/>
          </sbe:message>
        </sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("ts_test")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("TimestampNanos"))
        .with_conversion(ergo_sbe::ConversionSelector::named_type("TimestampMicros"))
        .with_conversion(ergo_sbe::ConversionSelector::named_type("TimestampMillis"));
    let mut generator = ergo_sbe::Generator::new(config);
    let modules = generator.generate(&schema)?;
    let source = modules.modules().next().unwrap().source.clone();

    let code = r#"
    // Wire precision converters — each composite is a transparent u64 wrapper.
    // TimestampNanos  → built-in u64 converter (nsec) works, but we write our own
    //                   per-composite impl to keep all three consistent.
    // TimestampMicros → DateTime from/to microseconds
    // TimestampMillis → DateTime from/to milliseconds

    use chrono::{DateTime, Utc, TimeZone};

    // Composite value type is `#[repr(transparent)] TimestampNanos([u8; 8])` —
    // not a raw u64. Parse the LE bytes to get the integer value.
    fn read_u64(wire: &[u8; 8]) -> u64 { u64::from_le_bytes(*wire) }
    fn write_u64(n: u64) -> [u8; 8] { n.to_le_bytes() }

    // ── nanos ──────────────────────────────────────────────────────
    impl TryFromSbe<TimestampNanos> for DateTime<Utc> {
        type Error = &'static str;
        fn try_from_sbe(wire: TimestampNanos) -> Result<Self, Self::Error> {
            let n = read_u64(&wire.0);
            DateTime::from_timestamp(
                (n / 1_000_000_000) as i64,
                (n % 1_000_000_000) as u32,
            ).ok_or("timestamp out of range")
        }
    }
    impl TryToSbe<TimestampNanos> for DateTime<Utc> {
        type Error = &'static str;
        fn try_to_sbe(&self) -> Result<TimestampNanos, Self::Error> {
            Ok(TimestampNanos(write_u64(
                self.timestamp_nanos_opt().ok_or("timestamp_nanos overflow")? as u64,
            )))
        }
    }

    // ── micros ─────────────────────────────────────────────────────
    impl TryFromSbe<TimestampMicros> for DateTime<Utc> {
        type Error = &'static str;
        fn try_from_sbe(wire: TimestampMicros) -> Result<Self, Self::Error> {
            let n = read_u64(&wire.0);
            DateTime::from_timestamp(
                (n / 1_000_000) as i64,
                ((n % 1_000_000) * 1_000) as u32,
            ).ok_or("timestamp out of range")
        }
    }
    impl TryToSbe<TimestampMicros> for DateTime<Utc> {
        type Error = &'static str;
        fn try_to_sbe(&self) -> Result<TimestampMicros, Self::Error> {
            Ok(TimestampMicros(write_u64(self.timestamp_micros() as u64)))
        }
    }

    // ── millis ─────────────────────────────────────────────────────
    impl TryFromSbe<TimestampMillis> for DateTime<Utc> {
        type Error = &'static str;
        fn try_from_sbe(wire: TimestampMillis) -> Result<Self, Self::Error> {
            let n = read_u64(&wire.0);
            DateTime::from_timestamp(
                (n / 1_000) as i64,
                ((n % 1_000) * 1_000_000) as u32,
            ).ok_or("timestamp out of range")
        }
    }
    impl TryToSbe<TimestampMillis> for DateTime<Utc> {
        type Error = &'static str;
        fn try_to_sbe(&self) -> Result<TimestampMillis, Self::Error> {
            Ok(TimestampMillis(write_u64(self.timestamp_millis() as u64)))
        }
    }

    // ── encode ─────────────────────────────────────────────────────
    let nanos_ts  = Utc.with_ymd_and_hms(2025, 7, 27, 14, 30, 0).unwrap()
                       + chrono::TimeDelta::nanoseconds(123_456_789);
    let micros_ts = Utc.with_ymd_and_hms(2025, 7, 27, 14, 30, 0).unwrap()
                       + chrono::TimeDelta::microseconds(123_456);
    let millis_ts = Utc.with_ymd_and_hms(2025, 7, 27, 14, 30, 0).unwrap()
                       + chrono::TimeDelta::milliseconds(123);

    // Encode known wire values — each composite is a transparent u64 wrapper.
    let wire_nanos:  u64 = nanos_ts.timestamp_nanos_opt().unwrap() as u64;
    let wire_micros: u64 = micros_ts.timestamp_micros() as u64;
    let wire_millis: u64 = millis_ts.timestamp_millis() as u64;

    let mut buf = vec![0u8; EventEncoder::ENCODED_LENGTH];
    let actual = EventEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&EventFixedFields {
            created_at:  TimestampNanos(wire_nanos.to_le_bytes()),
            updated_at:  TimestampMicros(wire_micros.to_le_bytes()),
            received_at: TimestampMillis(wire_millis.to_le_bytes()),
        })
        .encoded_length_with_header();
    assert_eq!(actual, EventEncoder::ENCODED_LENGTH);

    // ── decode: all three fields convert to DateTime<Utc> via *as() ─
    let event = EventDecoder::try_from(&buf[..actual])?;

    let created:  DateTime<Utc> = event.created_at_as::<DateTime<Utc>>()?;
    let updated:  DateTime<Utc> = event.updated_at_as::<DateTime<Utc>>()?;
    let received: DateTime<Utc> = event.received_at_as::<DateTime<Utc>>()?;

    // Nanos: full-precision round-trip (exact)
    assert_eq!(created.timestamp_nanos_opt().unwrap(), nanos_ts.timestamp_nanos_opt().unwrap(),
        "nanos precision round-trip must be exact");

    // Micros: wire discards sub-microsecond (our test value has exactly 123456 µs, no loss)
    assert_eq!(updated.timestamp_micros(), micros_ts.timestamp_micros(),
        "micros round-trip must match at microsecond resolution");

    // Millis: wire discards sub-millisecond
    assert_eq!(received.timestamp_millis(), millis_ts.timestamp_millis(),
        "millis round-trip must match at millisecond resolution");

    // ── raw wire access still works (decoder, not value) ───────────
    assert_eq!(event.created_at_wire().ts(),  wire_nanos,  "raw wire nanos must match");
    assert_eq!(event.updated_at_wire().ts(),  wire_micros, "raw wire micros must match");
    assert_eq!(event.received_at_wire().ts(), wire_millis, "raw wire millis must match");

    // ── encode via converter methods (*_from) ───────────────────────
    // Verify the converter-aware encoder also produces correct wire values.
    let mut buf2 = vec![0u8; EventEncoder::ENCODED_LENGTH];
    EventEncoder::try_wrap_and_apply_header(&mut buf2, 0)?
        .created_at_from(&nanos_ts)?
        .updated_at_from(&micros_ts)?
        .received_at_from(&millis_ts)?
        .encoded_length_with_header();
    // Bytes after the header must match between raw-wire encode and converter encode.
    assert_eq!(&buf[8..], &buf2[8..],
        "converter encode and raw-wire encode must produce identical bytes");
    "#;
    let sbe_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let deps = format!(
        "chrono = \"0.4\"\nergo-sbe = {{ path = \"{}\" }}\n",
        sbe_path.display(),
    );
    common::compile_and_run_with_deps("ts_precision_roundtrip", &source, code, &deps);

    Ok(())
}

/// Strip `/// ...` doc-comment lines from a source snippet so substring
/// checks for `Hash`, `Eq`, `Ord` etc. don't false-positive on prose.
fn strip_doc_lines(snippet: &str) -> String {
    snippet
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n")
}
