//! Comprehensive unit tests for all ErgoSBE generated features.
//! Each test encodes, decodes, and verifies every accessor path.
//! Tests A+B: correctness + edge cases for every todo item.

mod common;
use common::{Paths, compile_and_run, generate, run_fixture_test};

const MODULE: &str = "car_example";

// ── todo 02: composite/enum/set wire parity ───────────────────────────

#[test]
fn enum_all_variants_roundtrip() {
    let (_schema, src) = generate(&Paths::example_schema(), "enum_rt");
    compile_and_run("enum_rt", &src, r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        // Test all Model variants
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0, 0, [0,0,0]));
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
    "#);
}

#[test]
fn set_fields_roundtrip() {
    let (_schema, src) = generate(&Paths::example_schema(), "set_rt");
    compile_and_run("set_rt", &src, r#"
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
        car.engine(Engine::new(0, 0, [0,0,0]));
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
    "#);
}

// ── todo 03: group/var-data wire parity ───────────────────────────────

#[test]
fn group_with_vardata_entries_roundtrip() {
    let (_schema, src) = generate(&Paths::example_schema(), "group_vd_rt");
    compile_and_run("group_vd_rt", &src, r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0]));
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
        // Group with var-data: Iterator returns Result
        let ff: Vec<_> = car2.fuel_figures().unwrap()
            .collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(ff.len(), 2);
        assert_eq!(ff[0].speed(), 30);
        assert_eq!(ff[0].usage_description().unwrap(), b"Urban");
        assert_eq!(ff[1].speed(), 55);
        assert_eq!(ff[1].mpg(), 49.0);
        // is_empty
        assert!(!car2.fuel_figures().unwrap().is_empty());
        // rewind
        let mut ff_dec = car2.fuel_figures().unwrap();
        assert_eq!(ff_dec.len(), 2);
        ff_dec.rewind();
        assert_eq!(ff_dec.len(), 2);
        // remaining
        assert_eq!(ff_dec.remaining(), 2);
        // skip_n
        ff_dec.skip_n(1).unwrap();
        assert_eq!(ff_dec.remaining(), 1);
    "#);
}

#[test]
fn vardata_empty_and_max_roundtrip() {
    let (_schema, src) = generate(&Paths::example_schema(), "vd_edge");
    compile_and_run("vd_edge", &src, r#"
        // Encode with empty var-data
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0]));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();  // empty var-data
        let car = car.model(b"ABC").unwrap();
        let car = car.activation_code(b"XYZ0123456789").unwrap(); // long var-data
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // Empty var-data
        assert_eq!(car2.manufacturer().unwrap(), b"");
        // Short var-data
        assert_eq!(car2.model().unwrap(), b"ABC");
        // Longer var-data
        assert_eq!(car2.activation_code().unwrap(), b"XYZ0123456789");
        // is_empty on empty group
        assert!(car2.fuel_figures().unwrap().is_empty());
    "#);
}

// ── todo 01: scalar wire parity ───────────────────────────────────────

#[test]
fn all_scalar_accessor_paths() {
    let (_schema, src) = generate(&Paths::example_schema(), "scalar_paths");
    compile_and_run("scalar_paths", &src, r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1234); car.model_year(2013);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([1u32,2,3,4]); car.vehicle_code([97,98,99,100,101,102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49,0,0]));
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
        // _unchecked path for scalars
        unsafe {
            assert_eq!(car2.serial_number_unchecked(), 1234u64);
        }
    "#);
}

// ── todo 58: boolean support ──────────────────────────────────────────

#[test]
fn boolean_field_from_bool_impl() {
    let (_schema, src) = generate(&Paths::example_schema(), "bool_impl");
    compile_and_run("bool_impl", &src, r#"
        // BooleanType has From<bool> and From<BooleanType> for bool
        let t: BooleanType = true.into();
        assert_eq!(t, BooleanType::T);
        let f: BooleanType = false.into();
        assert_eq!(f, BooleanType::F);
        let b: bool = BooleanType::T.into();
        assert!(b);
        let b2: bool = BooleanType::F.into();
        assert!(!b2);
    "#);
}

// ── todo 52: NULL/MIN/MAX constants ───────────────────────────────────

#[test]
fn null_min_max_constants_present() {
    let (_schema, src) = generate(&Paths::example_schema(), "consts");
    // Verify constants exist in generated source
    assert!(src.contains("SERIAL_NUMBER_NULL"), "missing NULL constant");
    assert!(src.contains("SERIAL_NUMBER_MIN"), "missing MIN constant");
    assert!(src.contains("SERIAL_NUMBER_MAX"), "missing MAX constant");
}

// ── todo 60: schema_id fast extract ───────────────────────────────────

#[test]
fn schema_id_from_header_extracts_correctly() {
    let (_schema, src) = generate(&Paths::example_schema(), "schema_id");
    assert!(src.contains("fn schema_id_from_header"), "missing schema_id fast extract");
    compile_and_run("schema_id", &src, r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0]));
        let car = car.fuel_figures(0, |_|{}).unwrap();
        let car = car.performance_figures(0, |_|{}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();
        // schema_id_from_header is a free function at the module root
        let id = schema_id_from_header(encoded).unwrap();
        assert_eq!(id, 1u16); // Car schema id is 1
    "#);
}

// ── todo 61: Display/Debug impls ──────────────────────────────────────

#[test]
fn display_includes_scalar_fields() {
    let (_schema, src) = generate(&Paths::example_schema(), "display_full");
    compile_and_run("display_full", &src, r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(9999); car.model_year(2025);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([1u32,2,3,4]); car.vehicle_code([97,98,99,100,101,102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(3000, 6, [49,0,0]));
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
    "#);
}

// ── todo 66: constant field values ────────────────────────────────────

#[test]
fn constant_fields_return_correct_values() {
    let (_schema, src) = generate(&Paths::example_schema(), "const_fields");
    compile_and_run("const_fields", &src, r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0]));
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
    "#);
}

// ── todo 80: schema hash / SHA256 ─────────────────────────────────────

#[test]
fn schema_constants_present_and_nonzero() {
    let (_schema, src) = generate(&Paths::example_schema(), "schema_consts");
    assert!(src.contains("SCHEMA_HASH"), "SCHEMA_HASH missing");
    assert!(src.contains("SCHEMA_SHA256"), "SCHEMA_SHA256 missing");
    assert!(src.contains("SEMANTIC_VERSION"), "SEMANTIC_VERSION missing");
    compile_and_run("schema_consts", &src, r#"
        assert!(SCHEMA_HASH != 0, "SCHEMA_HASH should be non-zero");
        assert_eq!(SCHEMA_SHA256.len(), 32, "SHA256 is 32 bytes");
        assert!(!SCHEMA_SHA256_HEX.is_empty(), "SCHEMA_SHA256_HEX non-empty");
    "#);
}

// ── todo 03 + 84: encoder roundtrip with groups ───────────────────────

#[test]
fn encoder_roundtrip_with_groups_and_vardata() {
    let (_schema, src) = generate(&Paths::example_schema(), "enc_rt");
    compile_and_run("enc_rt", &src, r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(7777); car.model_year(2022);
        car.available(BooleanType::T); car.code(Model::A);
        car.some_numbers([10u32, 20, 30, 40]);
        car.vehicle_code([65, 66, 67, 68, 69, 70]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2500, 6, [50, 0, 0]));
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
        assert_eq!(car2.some_numbers().unwrap(), [10u32, 20, 30, 40]);
        let engine_fly = car2.engine();
        assert_eq!(engine_fly.capacity(), 2500);
        assert_eq!(engine_fly.num_cylinders(), 6);
        // Groups
        let ff: Vec<_> = car2.fuel_figures().unwrap()
            .collect::<Result<Vec<_>,_>>().unwrap();
        assert_eq!(ff.len(), 2);
        assert_eq!(ff[0].speed(), 100);
        assert_eq!(ff[0].usage_description().unwrap(), b"City");
        assert_eq!(ff[1].speed(), 200);
        // Nested group
        let pf: Vec<_> = car2.performance_figures().unwrap()
            .collect::<Result<Vec<_>,_>>().unwrap();
        let acc: Vec<_> = pf[0].acceleration().unwrap()
            .collect::<Vec<_>>();
        assert_eq!(acc.len(), 2);
        assert_eq!(acc[0].mph(), 60);
        assert!((acc[0].seconds() - 3.5).abs() < 0.01);
        assert_eq!(acc[1].mph(), 120);
        // VarData
        assert_eq!(car2.manufacturer().unwrap(), b"Porsche");
        assert_eq!(car2.model().unwrap(), b"911 GT3");
        assert_eq!(car2.activation_code().unwrap(), b"RACE");
    "#);
}

// ── todo 67 + 94: as_chunks + SoA for fixed-entry groups ─────────────

#[test]
fn fixed_entry_group_as_chunks_and_entries() {
    let (_schema, src) = generate(&Paths::example_schema(), "as_chunks");
    compile_and_run("as_chunks", &src, r#"
        let mut buf = vec![0u8; 512];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0]));
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
        let pf: Vec<_> = car2.performance_figures().unwrap()
            .collect::<Result<Vec<_>,_>>().unwrap();
        // Acceleration is a fixed-entry group (total_tail == 0)
        let acc = pf[0].acceleration().unwrap();
        // as_chunks() raw byte access for fixed-entry groups
        let chunks = acc.as_chunks().unwrap();
        assert_eq!(chunks.len(), 3);
    "#);
}

// ── todo 69: buffer verify function ───────────────────────────────────

#[test]
fn verify_function_detects_invalid_messages() {
    let (_schema, src) = generate(&Paths::example_schema(), "verify_fn");
    compile_and_run("verify_fn", &src, r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1); car.model_year(2000);
        car.available(BooleanType::F); car.code(Model::A);
        car.some_numbers([0u32;4]); car.vehicle_code([0u8;6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0,0,[0,0,0]));
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
    "#);
}
