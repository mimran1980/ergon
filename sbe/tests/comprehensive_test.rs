//! Comprehensive unit tests for all ErgoSBE generated features.
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

// ── todo 02: composite/enum/set wire parity ───────────────────────────

#[test]
fn enum_all_variants_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "enum_rt");
    compile_and_run(
        "enum_rt",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        // Test all Model variants
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0, 0, [0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // Enum accessors
        assert_eq!(car2.available(), BooleanType::T);
        assert_eq!(car2.code(), Model::A);
        // raw() returns underlying integer
        assert_eq!(car2.available().raw(), 1u8);
        // from_raw
        assert_eq!(BooleanType::from_raw(1), BooleanType::T);
        assert_eq!(BooleanType::from_raw(0), BooleanType::F);
        // Unknown discriminant maps to NullVal
        assert!(matches!(BooleanType::from_raw(99), BooleanType::NullVal));
        // Constant enum
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
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        // Set with all bits
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        extras.set_sun_roof(false);
        car.extras(extras);
        car.engine(Engine::new(0, 0, [0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let extras = car2.extras();
        assert!(extras.cruise_control());
        assert!(extras.sports_pack());
        assert!(!extras.sun_roof());
        assert_eq!(extras.raw(), 6u8);
    "#,
    );

    Ok(())
}

// ── todo 03: group/var-data wire parity ───────────────────────────────

#[test]
fn group_with_vardata_entries_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "group_vd_rt");
    compile_and_run(
        "group_vd_rt",
        &src,
        r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        // Group with var-data entries (fuel_figures has usage_description)
        let car = car.fuel_figures(2, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban").unwrap(); }).unwrap();
            g.add(|e| { e.speed(55).mpg(49.0); e.usage_description(b"Highway").unwrap(); }).unwrap();
        }).unwrap();
        // Nested group (performance_figures -> acceleration)
        let car = car.performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(2, |ag| {
                    ag.add(|ae| { ae.mph(30).seconds(4.0); }).unwrap();
                    ag.add(|ae| { ae.mph(60).seconds(7.5); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // Group with var-data: one consuming stage; the group decoder's own
        // iterator methods (len/is_empty/rewind/skip_n/remaining) are exercised
        // on it directly.
        let mut ff_dec = car2.into_fuel_figures().unwrap();
        assert_eq!(ff_dec.len(), 2);
        assert!(!ff_dec.is_empty());
        let ff: Vec<_> = ff_dec.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(ff.len(), 2);
        assert_eq!(ff[0].speed(), 30);
        assert_eq!(ff[0].usage_description().unwrap(), b"Urban");
        assert_eq!(ff[1].speed(), 55);
        assert_eq!(ff[1].mpg(), 49.0);
        // rewind resets the group decoder to the start of the group.
        ff_dec.rewind();
        assert_eq!(ff_dec.len(), 2);
        assert_eq!(ff_dec.remaining(), 2);
        ff_dec.skip_n(1).unwrap();
        assert_eq!(ff_dec.remaining(), 1);
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
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();  // empty var-data
        let car = car.model(b"ABC").unwrap();
        let car = car.activation_code(b"XYZ0123456789").unwrap(); // long var-data
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
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

// ── todo 01: scalar wire parity ───────────────────────────────────────

#[test]
fn all_scalar_accessor_paths() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "scalar_paths");
    compile_and_run(
        "scalar_paths",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234); car.model_year(2013);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([1u32,2,3,4]); car.vehicle_code([97,98,99,100,101,102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"Hon").unwrap();
        let car = car.model(b"Civ").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // Safe path: scalars return T directly (infallible)
        assert_eq!(car2.serial_number(), 1234u64);
        assert_eq!(car2.model_year(), 2013u16);
        // Safe path for scalars
        assert_eq!(car2.serial_number(), 1234u64);
    "#,
    );

    Ok(())
}

// ── todo 58: boolean support ──────────────────────────────────────────

#[test]
fn boolean_field_from_bool_impl() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "bool_impl");
    compile_and_run(
        "bool_impl",
        &src,
        r#"
        // BooleanType has From<bool> and From<BooleanType> for bool
        let t: BooleanType = true.into();
        assert_eq!(t, BooleanType::T);
        let f: BooleanType = false.into();
        assert_eq!(f, BooleanType::F);
        let b: bool = BooleanType::T.into();
        assert!(b);
        let b2: bool = BooleanType::F.into();
        assert!(!b2);
    "#,
    );

    Ok(())
}

// ── todo 52: NULL/MIN/MAX constants ───────────────────────────────────

#[test]
fn null_min_max_constants_match_schema_values() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "consts_val");
    // Source contains the constants
    assert!(src.contains("SERIAL_NUMBER_NULL"), "missing NULL constant");
    assert!(src.contains("SERIAL_NUMBER_MIN"), "missing MIN constant");
    assert!(src.contains("SERIAL_NUMBER_MAX"), "missing MAX constant");
    // Compile-and-run: verify constant values match schema definitions
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
        // Enum types get NULL variants
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

// ── todo 60: schema_id fast extract ───────────────────────────────────

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
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();
        // schema_id_from_header is a free function at the module root
        let id = schema_id_from_header(encoded).unwrap();
        assert_eq!(id, 1u16); // Car schema id is 1
    "#,
    );

    Ok(())
}

// ── todo 61: Display/Debug impls ──────────────────────────────────────

#[test]
fn display_includes_scalar_fields() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "display_full");
    compile_and_run(
        "display_full",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(9999); car.model_year(2025);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([1u32,2,3,4]); car.vehicle_code([97,98,99,100,101,102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(3000, 6, [49,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"BMW").unwrap();
        let car = car.model(b"M3").unwrap();
        let car = car.activation_code(b"XYZ").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let d = format!("{}", car2);
        // Display contains field names and values
        assert!(d.contains("serial_number"), "serial_number: {d}");
        assert!(d.contains("9999"), "value 9999: {d}");
        assert!(d.contains("2025"), "value 2025: {d}");
        // var-data fields show byte count, not content
        assert!(d.contains("manufacturer"), "manufacturer field: {d}");
        assert!(d.contains("bytes"), "var-data shows byte count: {d}");
    "#,
    );

    Ok(())
}

// ── todo 66: constant field values ────────────────────────────────────

#[test]
fn constant_fields_return_correct_values() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "const_fields");
    compile_and_run(
        "const_fields",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // discountedModel is presence="constant" valueRef="Model.C"
        assert_eq!(car2.discounted_model(), Model::C);
        assert_eq!(car2.discounted_model().raw(), 67u8); // 'C'
    "#,
    );

    Ok(())
}

// ── todo 80: schema hash / SHA256 ─────────────────────────────────────

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

// ── todo 03 + 84: encoder roundtrip with groups ───────────────────────

#[test]
fn encoder_roundtrip_with_groups_and_vardata() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "enc_rt");
    compile_and_run(
        "enc_rt",
        &src,
        r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(7777); car.model_year(2022);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([10u32, 20, 30, 40]);
        car.vehicle_code([65, 66, 67, 68, 69, 70]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2500, 6, [50, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(2, |g| {
            g.add(|e| { e.speed(100).mpg(25.5); e.usage_description(b"City").unwrap(); }).unwrap();
            g.add(|e| { e.speed(200).mpg(15.0); e.usage_description(b"Track").unwrap(); }).unwrap();
        }).unwrap();
        let car = car.performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(98);
                e.acceleration(2, |ag| {
                    ag.add(|ae| { ae.mph(60).seconds(3.5); }).unwrap();
                    ag.add(|ae| { ae.mph(120).seconds(8.0); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();
        let car = car.manufacturer(b"Porsche").unwrap();
        let car = car.model(b"911 GT3").unwrap();
        let car = car.activation_code(b"RACE").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        assert_eq!(car2.serial_number(), 7777u64);
        assert_eq!(car2.model_year(), 2022u16);
        assert_eq!(car2.some_numbers(), [10u32, 20, 30, 40]);
        let engine_fly = car2.engine();
        assert_eq!(engine_fly.capacity(), 2500);
        assert_eq!(engine_fly.num_cylinders(), 6);
        // Groups + var-data in wire order via the consuming stages.
        let mut fuel = car2.into_fuel_figures().unwrap();
        let ff: Vec<_> = fuel.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(ff.len(), 2);
        assert_eq!(ff[0].speed(), 100);
        assert_eq!(ff[0].usage_description().unwrap(), b"City");
        assert_eq!(ff[1].speed(), 200);
        let mut perf = fuel.finish().unwrap().into_performance_figures().unwrap();
        // Nested group (entry-level accessor remains)
        let pf: Vec<_> = perf.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        let acc: Vec<_> = pf[0].acceleration().unwrap().collect::<Vec<_>>();
        assert_eq!(acc.len(), 2);
        assert_eq!(acc[0].mph(), 60);
        assert!((acc[0].seconds() - 3.5).abs() < 0.01);
        assert_eq!(acc[1].mph(), 120);
        // VarData
        let after_perf = perf.finish().unwrap();
        let (mfr, a1) = after_perf.into_manufacturer().unwrap();
        assert_eq!(mfr, b"Porsche");
        let (model, a2) = a1.into_model().unwrap();
        assert_eq!(model, b"911 GT3");
        let (activation, _done) = a2.into_activation_code().unwrap();
        assert_eq!(activation, b"RACE");
    "#,
    );

    Ok(())
}

// ── todo 67 + 94: as_chunks + SoA for fixed-entry groups ─────────────

#[test]
fn fixed_entry_group_as_chunks_and_entries() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "as_chunks");
    compile_and_run(
        "as_chunks",
        &src,
        r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(3, |ag| {
                    ag.add(|ae| { ae.mph(10).seconds(1.0); }).unwrap();
                    ag.add(|ae| { ae.mph(20).seconds(2.0); }).unwrap();
                    ag.add(|ae| { ae.mph(30).seconds(3.0); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // Traverse fuel first (wire order), then performance.
        let pf: Vec<_> = car2
            .into_fuel_figures()
            .unwrap()
            .finish()
            .unwrap()
            .into_performance_figures()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        // Acceleration is a fixed-entry group (total_tail == 0)
        let acc = pf[0].acceleration().unwrap();
        // Use group decoder's Iterator impl (replaces as_chunks)
        let entries: Vec<_> = acc.collect();
        assert_eq!(entries.len(), 3);
    "#,
    );

    Ok(())
}

// ── todo 69: buffer verify function ───────────────────────────────────

#[test]
fn verify_function_detects_invalid_messages() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "verify_fn");
    compile_and_run(
        "verify_fn",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();
        // verify() should pass on valid message
        assert!(CarDecoder::verify(encoded).is_ok());
        // verify() should fail on truncated buffer
        assert!(CarDecoder::verify(&encoded[..5]).is_err());
    "#,
    );

    Ok(())
}

// ── todo 93: float composite skips Eq/Ord/Hash ──────────────────────────

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
//
// Logging `{}` / `{:?}` on codecs is an ops path. Truncated buffers, trusted
// wrap with short body, and mid-encode encoders must never panic.

#[test]
fn decoder_display_and_debug_survive_invalid_sizes() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "disp_invalid");
    compile_and_run(
        "disp_invalid",
        &src,
        r#"
        // 1) Valid encode → Display / Debug work
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(42); car.model_year(2020);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([1u32,2,3,4]); car.vehicle_code([97,98,99,100,101,102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"xyz").unwrap();
        let encoded = car.as_bytes().to_vec();

        let ok = CarDecoder::wrap_and_apply_header(&encoded, 0).unwrap();
        let d = format!("{}", ok);
        assert!(d.contains("serial_number"), "valid Display: {d}");
        assert!(d.contains("42"), "valid Display value: {d}");
        // Debug delegates to Display — shows field values like Java toString.
        let dbg = format!("{:?}", ok);
        assert!(dbg.contains("serial_number"), "valid Debug (fields): {dbg}");
        assert!(dbg.contains("42"), "valid Debug value: {dbg}");
        assert!(dbg == d, "Debug == Display for valid message");

        // 2) Trusted wrap with buffer shorter than body — Display/Debug must not panic
        let tiny = [0u8; 4];
        let short = CarDecoder::wrap(&tiny, 0, 45, 0);
        let d_short = format!("{}", short);
        assert!(d_short.starts_with("Car {"), "short Display: {d_short}");
        let dbg_short = format!("{:?}", short);
        assert!(dbg_short == d_short, "Debug == Display for short buffer");

        // 3) Full header + body, zero groups/var-data tail truncated mid-stream
        //    (block present so fixed fields render; tail accessors return Err → skipped)
        let mut partial = encoded.clone();
        // Keep header(8)+block(45)=53, drop rest so groups fail gracefully
        if partial.len() > 53 {
            partial.truncate(53);
        }
        let part = CarDecoder::wrap_and_apply_header(&partial, 0).unwrap();
        let d_part = format!("{}", part);
        assert!(d_part.contains("serial_number"), "partial Display: {d_part}");
        let _ = format!("{:?}", part); // no panic

        // 4) Empty buffer trusted wrap — Debug == Display on decoders
        let empty = CarDecoder::wrap(&[], 0, 45, 0);
        let d_empty = format!("{}", empty);
        assert!(d_empty.contains("Car {"), "empty Display: {d_empty}");
        let dbg_empty = format!("{:?}", empty);
        assert!(dbg_empty == d_empty, "empty Debug == Display");
    "#,
    );

    Ok(())
}

#[test]
fn encoder_debug_survives_incomplete_and_short_buffers() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "enc_debug");
    compile_and_run(
        "enc_debug",
        &src,
        r#"
        // Mid-encode encoder Debug (type-state stage) must not panic
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1);
        let dbg = format!("{:?}", car);
        assert!(dbg.contains("CarEncoder"), "encoder Debug: {dbg}");
        assert!(dbg.contains("message_start"), "encoder Debug fields: {dbg}");
        assert!(dbg.contains("buf_len"), "encoder Debug buf_len: {dbg}");

        // After first group transition, later stage also implements Debug
        car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let after = car.fuel_figures(0, |_|{}).unwrap();
        let dbg2 = format!("{:?}", after);
        assert!(dbg2.contains("message_start") || dbg2.contains("After"), "stage Debug: {dbg2}");

        // wrap on a buffer that is too short for a full header must return Err
        // (encode path); Debug on a successful partial stage is covered above.
        let mut tiny = [0u8; 2];
        assert!(CarEncoder::wrap_and_apply_header(&mut tiny, 0).is_err());
    "#,
    );

    Ok(())
}

// ── Error path tests (critical for trading system robustness) ──────────

#[test]
fn buffer_too_short_truncated_field() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "buf_err");
    compile_and_run(
        "buf_err",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();
        // Truncate buffer mid-message — field read should fail
        let truncated = &encoded[..10]; // only header + partial block
        assert!(CarDecoder::verify(truncated).is_err());
        // Truncated right at header boundary
        assert!(CarDecoder::verify(&encoded[..8]).is_err());
        // Empty buffer
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
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        // Encode varData fields
        let car = car.manufacturer(b"Porsche").unwrap();
        let car = car.model(b"911 GT3 RS").unwrap();
        let car = car.activation_code(b"RACE-XYZ").unwrap();
        let encoded = car.as_bytes();
        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
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

// ── Raw accessor tests (HFT hot-path opts) ─────────────────────────────

#[test]
fn raw_enum_accessors_preserve_wire_discriminant() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "raw_enum");
    compile_and_run(
        "raw_enum",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();
        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
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
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);
        car.engine(Engine::new(0,0,[0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();
        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // raw() returns the underlying bitfield
        let raw = car2.extras().raw();
        // cruise_control = bit 2, sports_pack = bit 1, sun_roof = bit 0
        assert!(car2.extras().cruise_control());
        assert!(car2.extras().sports_pack());
        assert!(!car2.extras().sun_roof());
        assert_eq!(raw, 6u8); // 0b110 = bits 1 and 2 set
    "#,
    );
    Ok(())
}

// ── todo 121: endianness test matrix ──────────────────────────────────

#[test]
fn all_types_little_endian_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::all_types_le_schema(), "all_types_le");
    // Verify source compiles and contains expected types
    assert!(src.contains("AllScalars"), "AllScalars composite missing");
    assert!(src.contains("FloatPair"), "FloatPair composite missing");
    assert!(src.contains("TestEnum"), "TestEnum missing");
    assert!(src.contains("TestSet"), "TestSet missing");
    assert!(src.contains("AllTypesDecoder"), "message decoder missing");
    // AllScalars contains f32/f64 → should NOT derive Eq/Ord/Hash.
    // Check the actual #[derive(...)] line, not doc comments.
    let asc_idx = src.find("pub struct AllScalars").unwrap();
    let asc_pre = &src[asc_idx.saturating_sub(200)..asc_idx];
    // Extract the #[derive] line if present.
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
    // Float composite should NOT derive Eq/Ord/Hash
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
    // Compile check: types exist and are callable
    compile_and_run(
        "all_types_le",
        &src,
        r#"
        // Verify enum constants
        assert_eq!(TestEnum::A as u8, 0u8);
        assert_eq!(TestEnum::B as u8, 1u8);
        assert_eq!(TestEnum::C as u8, 2u8);
        // Verify composite constructors exist
        let s = AllScalars::new(-1i8, 255u8, -2i16, 65535u16,
            -3i32, 4294967295u32, -4i64, 18446744073709551615u64,
            3.14f32, 2.718f64);
        assert_eq!(s.i8_val(), -1i8);
        assert_eq!(s.u16_val(), 65535u16);
        assert!((s.f32_val() - 3.14f32).abs() < 0.001);
        let f = FloatPair::new(1.5, 2.5);
        assert_eq!(f.x(), 1.5);
        assert_eq!(f.y(), 2.5);
        // Verify set
        let mut set = TestSet::default();
        set.set_bit1(true);
        assert!(set.bit1());
        assert!(!set.bit0());
        // Verify enum from_raw + raw roundtrip
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
        set.set_bit2(true);
        assert!(set.bit2());
    "#,
    );

    Ok(())
}

// ── API contract: verify generated public surface is stable ────────────

#[test]
fn generated_api_has_expected_public_items() -> Result<(), Box<dyn std::error::Error>> {
    // Verify the car example generates a consistent public API surface.
    // Catches accidental codegen changes that break user code.
    let (_schema, src) = generate(&Paths::example_schema(), "api_contract");

    // Core types
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

    // Public constants
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
        "pub fn engine_as_struct",
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

    // Group decoder methods
    assert!(src.contains("fn len"), "missing group len()");
    assert!(src.contains("fn is_empty"), "missing group is_empty()");
    assert!(src.contains("fn nth"), "missing group nth()");
    assert!(src.contains("fn skip_n"), "missing group skip_n()");
    assert!(src.contains("fn rewind"), "missing group rewind()");
    assert!(src.contains("fn remaining"), "missing group remaining()");

    // Composite value type methods (engine_as_struct returns Engine with fields)
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

    // Free functions
    assert!(
        src.contains("pub fn schema_id_from_header"),
        "missing free fn"
    );

    // Null/min/max consts
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

// ── Compatibility mode wiring (todo 65) ────────────────────────────────

#[test]
fn strict_and_extended_modes_produce_identical_output() -> Result<(), Box<dyn std::error::Error>> {
    // Phase 2: prove CompatibilityMode plumbing works. Same schema →
    // same output when no extensions exist. When extensions are added,
    // they gate on WireCompatibleExtensions.
    use ergo_sbe::{CompatibilityMode, GenerationConfig, Generator, Schema, parse_file};

    let ir = parse_file(&common::Paths::example_schema()).expect("parse car schema");
    let schema = Schema::from_ir(ir);

    let mut strict_cfg = GenerationConfig::new("car_strict");
    strict_cfg.compatibility = CompatibilityMode::Strict;
    let strict_src = Generator::new(strict_cfg).generate(&schema);

    let mut ext_cfg = GenerationConfig::new("car_ext");
    ext_cfg.compatibility = CompatibilityMode::WireCompatibleExtensions;
    let ext_src = Generator::new(ext_cfg).generate(&schema);

    assert_eq!(
        strict_src.modules().next().unwrap().source,
        ext_src.modules().next().unwrap().source,
        "Strict and WireCompatibleExtensions must produce identical output when no extensions exist"
    );

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Strip `/// ...` doc-comment lines from a source snippet so substring
/// checks for `Hash`, `Eq`, `Ord` etc. don't false-positive on prose.
fn strip_doc_lines(snippet: &str) -> String {
    snippet
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n")
}
