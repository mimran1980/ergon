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

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, assert_source_ok, compile_and_run, compile_and_run_two_modules,
    compile_and_run_with_feature, compile_fails, generate, run_fixture_test,
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
        src.contains("#[allow(non_camel_case_types)]"),
        "generated code must suppress non_camel_case_types"
    );
    assert!(
        src.contains("#[allow(non_snake_case)]"),
        "generated code must suppress non_snake_case"
    );
    assert!(
        src.contains("#[allow(clippy::identity_op)]"),
        "generated code must suppress clippy::identity_op"
    );
    assert!(
        src.contains("#[allow(clippy::eq_op)]"),
        "generated code must suppress clippy::eq_op"
    );
    assert!(
        src.contains("#[allow(clippy::needless_borrow)]"),
        "generated code must suppress clippy::needless_borrow"
    );
    assert!(
        src.contains("#[allow(clippy::manual_range_contains)]"),
        "generated code must suppress clippy::manual_range_contains"
    );
    assert!(
        src.contains("#[allow(unused_imports)]"),
        "generated code must suppress unused_imports"
    );
    assert!(
        src.contains("#[allow(unused_variables)]"),
        "generated code must suppress unused_variables"
    );
    assert!(
        src.contains("#[allow(unused_mut)]"),
        "generated code must suppress unused_mut"
    );
    assert!(
        src.contains("#[allow(dead_code)]"),
        "generated code must suppress dead_code"
    );
    // ponytail: #[allow(unused_unsafe)] removed along with scalar raw_* methods —
    // enum/set/composite raw_* return the underlying repr directly without wrapping unsafe
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

#[test]
fn generate_composite_with_enum_set_and_nested_composite() {
    // composite-elements-schema.xml has a composite ("outer") containing an
    // enum, a set, and a nested composite ("inner"); the rc4 variant adds
    // explicit offsets. Generating these exercises the composite field-type
    // codegen branches (enum/set/nested-composite-in-composite + offsets).
    let path = Paths::sbe_tool_test_resource("composite-elements-schema.xml");
    let (_s, src) = generate(&path, "comp_elems");
    // Generating exercises the enum/set/nested-composite-in-composite branches;
    // assert the result is valid Rust and contains the outer composite.
    assert_source_ok(&src, &["Outer"]);

    let path_rc4 = Paths::sbe_tool_test_resource("composite-elements-schema-rc4.xml");
    let (_s2, src2) = generate(&path_rc4, "comp_rc4");
    assert!(
        src2.contains("OuterWithOffsets"),
        "rc4 composite must generate"
    );
}

#[test]
fn generate_composite_with_named_type_member_refs() {
    // composite-field-refs.xml defines a composite "Widget" whose members
    // reference a named enum (Colour), set (Flags), and composite (Inner).
    // Generating it exercises the composite field-type codegen arms that handle
    // enum/set/nested-composite *member references* (a shape no other fixture has).
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/composite-field-refs.xml");
    let (_s, src) = generate(&path, "comp_field_refs");
    assert_source_ok(&src, &["Widget", "Colour", "Flags", "Inner"]);
}

#[test]
fn generate_versioned_set_enum_composite_fields() {
    // extension-schema.xml has message fields with sinceVersion > 0 of set
    // (ASet), enum (AEnum), and composite (AComposite) types. Generating it
    // exercises the versioned-field codegen branches (Option<T> accessor shape).
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/extension-schema.xml");
    let (_s, src) = generate(&path, "ext_versioned");
    syn::parse_file(&src).expect("extension-schema generates valid Rust");
}

#[test]
fn generate_enums_over_every_integer_encoding_type() {
    // enum-encoding-types.xml has one enum per int/uint encoding type. Generating
    // it exercises max_encoding_value for every integer primitive (the enum NULL
    // const = encodingType.maxValue()) and the enum codegen for each width.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/enum-encoding-types.xml");
    let (_s, src) = generate(&path, "enum_enc");
    syn::parse_file(&src).expect("enum-encoding-types generates valid Rust");
}

#[test]
fn generate_big_endian_and_unbounded_var_data_schemas() {
    // Big-endian schema exercises the BE byte-order codegen branches; the
    // unbounded-var-data schema (no maxLength attr) exercises the var-data
    // accessor's else branch.
    let (_s1, src1) = generate(&Paths::bigendian_schema(), "be_schema");
    syn::parse_file(&src1).expect("bigendian schema generates valid Rust");

    let (_s2, src2) = generate(&Paths::basic_variable_length_schema(), "unbounded_vd");
    syn::parse_file(&src2).expect("unbounded var-data schema generates valid Rust");
}

#[test]
fn generate_multi_message_schema() {
    // binance_spot_3_5.xml has 92 messages with shared types (enums, sets,
    // composites). Generating it exercises the multi-message codegen branches
    // (shared_types collection, header-type dispatch, multi-message paths).
    let path = Paths::sbe_tool_test_resource("binance_spot_3_5.xml");
    let (_s, src) = generate(&path, "binance_mm");
    syn::parse_file(&src).expect("binance multi-message schema generates valid Rust");
}

#[test]
fn generator_config_getter() {
    use ergosbe::{GenerationConfig, Generator};
    let generator = Generator::new(GenerationConfig::new("cfg_test"));
    // Exercises Generator::config() (the getter was previously uncovered).
    let _ = generator.config();
}

#[test]
fn generate_multi_schema_entry_point() {
    // generate_multi() generates multiple schemas into separate modules with
    // shared-type tracking. No other test exercises it.
    use ergosbe::{GenerationConfig, Generator, Schema, parse_file};
    let ir1 = parse_file(&Paths::example_schema()).unwrap();
    let s1 = Schema::from_ir(ir1);
    let ir2 = parse_file(&Paths::l3_orderbook_schema()).unwrap();
    let s2 = Schema::from_ir(ir2);
    let g = Generator::new(GenerationConfig::new("multi_test"));
    let ms = g.generate_multi(&[(&s1, "mod1"), (&s2, "mod2")]);
    let count = ms.modules().count();
    assert!(count >= 2, "expected >=2 modules, got {count}");
}

#[test]
fn generate_coverage_edges_schema() {
    // coverage-edges.xml exercises: group-name dedup, constant set field,
    // and BooleanType in a group entry (the _bool accessor).
    use std::path::PathBuf;
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas/coverage-edges.xml");
    let (_s, src) = generate(&path, "covedges");
    syn::parse_file(&src).expect("coverage-edges generates valid Rust");
}

#[test]
fn generate_custom_header_type_schema() {
    // custom-header-type.xml uses headerType="foo" (not "messageHeader"). This
    // exercises the MessageHeader type-alias codegen branch (line 217).
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/custom-header-type.xml");
    let (_s, src) = generate(&path, "custom_hdr");
    assert!(
        src.contains("pub type MessageHeader = Foo"),
        "custom header type alias"
    );
}

#[test]
fn generate_constant_value_schema() {
    // constant-value-types.xml has message fields of presence="constant" with
    // float, double, and int64 types. Generating it covers the constant_value_expr
    // formatting branches (f32/f64/i64 format strings, 537/541/544).
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/constant-value-types.xml");
    let (_s, src) = generate(&path, "const_vals");
    syn::parse_file(&src).expect("constant-value-types generates valid Rust");
}

#[test]
fn generate_constant_set_field() {
    // constant-set-field.xml has a SET field with presence="constant".
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/constant-set-field.xml");
    let (_s, src) = generate(&path, "const_set");
    syn::parse_file(&src).expect("constant-set-field generates valid Rust");
}

#[test]
fn generate_vardata_without_max_length() {
    // vardata-no-maxlength.xml has a custom var-data encoding whose length
    // field (uint8, no maxValue) gives max_length=None, triggering the
    // else branch of the var-data accessor generation.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/vardata-no-maxlength.xml");
    let (_s, src) = generate(&path, "vd_nomax");
    syn::parse_file(&src).expect("vardata-no-maxlength generates valid Rust");
}

#[test]
fn generate_schema_with_include_file() {
    // Exercises the include processing code path in read_include_file +
    // parse_schema (lines 540-564 in xml.rs). types-include.xml defines
    // types referenced by schema-with-include.xml.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/schema-with-include.xml");
    let (_s, src) = generate(&path, "schema_with_inc");
    syn::parse_file(&src).expect("schema-with-include generates valid Rust");
}

#[test]
fn generate_group_entry_with_composite_enum_set_fields() {
    // group-entry-field-types.xml has a group whose entry has fields of type
    // Composite (Inner), Enum (Colour), and Set (Flags). Covers the
    // FieldType::Composite/Enum/Set size arms in group encoder/decoder.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/group-entry-field-types.xml");
    let (_s, src) = generate(&path, "gentry");
    syn::parse_file(&src).expect("group-entry-field-types generates valid Rust");
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

        assert_eq!([1u32, 2, 3, 4], car.some_numbers(), "someNumbers");
        assert_eq!([97, 98, 99, 100, 101, 102], car.vehicle_code(), "vehicleCode");

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

        // Group: fuelFigures (3 entries) — consuming stages, wire order.
        let mut fuel = car.into_fuel_figures().unwrap();
        let fuel_figures: Vec<_> = fuel.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
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
        let mut perf_iter = fuel.finish().unwrap().into_performance_figures().unwrap();
        let perf: Vec<_> = perf_iter.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
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

        // Var-data fields — continue the consuming chain in wire order.
        let after_perf = perf_iter.finish().unwrap();
        let (manufacturer, c1) = after_perf.into_manufacturer().unwrap();
        assert_eq!(b"Honda", manufacturer, "manufacturer");
        let (model, c2) = c1.into_model().unwrap();
        assert_eq!(b"Civic VTi", model, "model");
        let (activation_code, _done) = c2.into_activation_code().unwrap();
        assert_eq!(b"abcdef", activation_code, "activationCode");
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
        assert_eq!([1u32, 2, 3, 4], car2.some_numbers(), "rt.someNumbers");
        assert_eq!([97, 98, 99, 100, 101, 102], car2.vehicle_code(), "rt.vehicleCode");

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

        let mut fuel = car2.into_fuel_figures().unwrap();
        let ff2: Vec<_> = fuel.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(3, ff2.len());
        assert_eq!(30, ff2[0].speed());  assert!((ff2[0].mpg() - 35.9).abs() < 0.01);
        assert_eq!(b"Urban Cycle", ff2[0].usage_description().unwrap());
        assert_eq!(55, ff2[1].speed());  assert!((ff2[1].mpg() - 49.0).abs() < 0.01);
        assert_eq!(b"Combined Cycle", ff2[1].usage_description().unwrap());
        assert_eq!(75, ff2[2].speed());  assert!((ff2[2].mpg() - 40.0).abs() < 0.01);
        assert_eq!(b"Highway Cycle", ff2[2].usage_description().unwrap());

        let mut perf_iter = fuel.finish().unwrap().into_performance_figures().unwrap();
        let pf2: Vec<_> = perf_iter.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(2, pf2.len());
        assert_eq!(95, pf2[0].octane_rating());
        let a0: Vec<_> = pf2[0].acceleration().unwrap().collect::<Vec<_>>();
        assert_eq!(3, a0.len());
        assert_eq!(30, a0[0].mph());  assert!((a0[0].seconds() - 4.0).abs() < 0.01);

        let after_perf = perf_iter.finish().unwrap();
        let (mfr, c1) = after_perf.into_manufacturer().unwrap();
        assert_eq!(b"Honda", mfr, "rt.manufacturer");
        let (model, c2) = c1.into_model().unwrap();
        assert_eq!(b"Civic VTi", model, "rt.model");
        let (activation_code, _done) = c2.into_activation_code().unwrap();
        assert_eq!(b"abcdef", activation_code, "rt.activationCode");
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

        // Write empty tails to reach the complete stage (as_bytes is
        // completion-only per DECISIONS.md §2).
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
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
        assert!(car2.into_fuel_figures().unwrap().is_empty(), "0 fuel figures → is_empty == true");

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
        assert!(!car2.into_fuel_figures().unwrap().is_empty(), "3 fuel figures → is_empty == false");
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

// ── compute_encoded_length (todo 116) ────────────────────────────────

#[test]
fn compute_encoded_length_matches_actual() {
    let (_schema, src) = generate(&Paths::example_schema(), "pre_encode_len");
    compile_and_run(
        "pre_encode_len",
        &src,
        r#"
        // Baseline: zero groups / zero var-data
        // Groups and var-data are always present (dim headers + length prefixes even at 0)
        let empty = <CarEncoder>::compute_encoded_length(0, 0, 0, 0, 0);
        assert_eq!(empty, 61); // 41 (block) + 2×4 (group dims) + 3×4 (vardata prefixes)
        let empty_full = <CarEncoder>::compute_encoded_length_with_message_header(0, 0, 0, 0, 0);
        assert_eq!(empty_full, 69); // 61 + 8-byte header

        // DECISIONS.md §2: header-inclusive length must use the dedicated helper.
        let body = <CarEncoder>::compute_encoded_length(1, 0, 5, 4, 6);
        let full = <CarEncoder>::compute_encoded_length_with_message_header(1, 0, 5, 4, 6);
        assert!(full > body, "full length must exceed body length");

        // Computed length must be ≤ MAX_ENCODED_LENGTH (worst-case bound)
        let computed = <CarEncoder>::compute_encoded_length(3, 2, 100, 100, 100);
        assert!(computed <= <CarEncoder>::MAX_ENCODED_LENGTH,
            "computed {computed} exceeds MAX_ENCODED_LENGTH {}",
            <CarEncoder>::MAX_ENCODED_LENGTH);

        // Encode a simple message (no nested groups, no entry var-data)
        // and verify the pre-computed length matches actual encoded length
        let body_len = <CarEncoder>::compute_encoded_length(0, 0, 5, 4, 6);
        let full_len = body_len + 8;
        let mut buf = vec![0u8; full_len];
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
        let car = car.model(b"Civc").unwrap();
        let car = car.activation_code(b"abc123").unwrap();
        assert_eq!(body_len, car.encoded_length(), "body_len mismatch");
        assert_eq!(full_len, car.encoded_length_with_header(), "full_len mismatch");
    "#,
    );
}

// ── entries() iterator for fixed-entry groups (todo 114) ─────────────

#[test]
fn fixed_entry_group_entries_iterator() {
    let (_schema, src) = generate(&Paths::example_schema(), "entries_iter");
    compile_and_run(
        "entries_iter",
        &src,
        r#"
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
        let car = car.performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(3, |ag| {
                    ag.add(|ae| { ae.mph(30).seconds(4.0); }).unwrap();
                    ag.add(|ae| { ae.mph(60).seconds(7.5); }).unwrap();
                    ag.add(|ae| { ae.mph(100).seconds(12.2); }).unwrap();
                }).unwrap();
            }).unwrap();
        }).unwrap();
        let car = car.manufacturer(b"Hon").unwrap();
        let car = car.model(b"Civ").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let perf: Vec<_> = car2
            .into_fuel_figures()
            .unwrap()
            .finish()
            .unwrap()
            .into_performance_figures()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut accel = perf[0].acceleration().unwrap();
        assert_eq!(accel.len(), 3);
        let a0 = accel.next().unwrap();
        assert_eq!(a0.mph(), 30);
        assert!((a0.seconds() - 4.0).abs() < 0.01);
        let a1 = accel.next().unwrap();
        assert_eq!(a1.mph(), 60);
        let a2 = accel.next().unwrap();
        assert_eq!(a2.mph(), 100);
        assert!(accel.next().is_none());
    "#,
    );
}

// ── array accessor fast path (todo 108) ─────────────────────────────

#[test]
fn array_accessor_all_paths_return_same_values() {
    let (_schema, src) = generate(&Paths::example_schema(), "array_paths");
    compile_and_run(
        "array_paths",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
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
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();

        // Safe path — some_numbers returns Result
        let safe: [u32; 4] = car2.some_numbers();
        assert_eq!(safe, [1u32, 2, 3, 4]);

        // vehicle_code (byte array)
        let vs: [u8; 6] = car2.vehicle_code();
        assert_eq!(vs, [97, 98, 99, 100, 101, 102]);
    "#,
    );
}

// ── Display group entries (todo 113) ──────────────────────────────────

#[test]
fn display_shows_group_entry_fields_not_just_count() {
    let (_schema, src) = generate(&Paths::example_schema(), "display_entries");
    compile_and_run(
        "display_entries",
        &src,
        r#"
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
        let car = car.fuel_figures(2, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban").unwrap(); }).unwrap();
            g.add(|e| { e.speed(55).mpg(49.0); e.usage_description(b"Comb").unwrap(); }).unwrap();
        }).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let display = format!("{}", car2);

        // Display must include entry field values, not just "N entries"
        assert!(display.contains("speed"), "Display missing 'speed': {display}");
        assert!(display.contains("mpg"), "Display missing 'mpg': {display}");
        assert!(display.contains("30"), "Display missing speed value 30: {display}");
        assert!(display.contains("55"), "Display missing speed value 55: {display}");
        // Must NOT show stale "N entries" count-only format
        assert!(!display.contains("2 entries"), "Display should not show raw count: {display}");
        // Also shows message-level scalars
        assert!(display.contains("serial_number"), "Display missing serial_number: {display}");
        assert!(display.contains("1234"), "Display missing serial_number value: {display}");
    "#,
    );
}

// ── composite flyweight default (todo 112) ───────────────────────────

#[test]
fn composite_default_is_flyweight_as_struct_is_eager_copy() {
    let (_schema, src) = generate(&Paths::example_schema(), "composite_api");
    compile_and_run(
        "composite_api",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
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
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();

        // Default: flyweight (zero-copy from buffer)
        let fly: EngineDecoder = car2.engine();
        assert_eq!(fly.capacity(), 2000);
        assert_eq!(fly.num_cylinders(), 4);

        // Eager copy: value struct
        let eager: Engine = car2.engine_as_struct();
        assert_eq!(eager.capacity(), 2000);
        assert_eq!(eager.num_cylinders(), 4);

        // Both paths produce identical values
        assert_eq!(fly.capacity(), eager.capacity());
        assert_eq!(fly.num_cylinders(), eager.num_cylinders());

        // Deprecated alias still works
        #[allow(deprecated)]
        {
            let old: EngineDecoder = car2.engine();
            assert_eq!(old.capacity(), 2000);
        }
    "#,
    );
}

// ── bound-check-disabled gates (todo 115) ────────────────────────────

#[test]
fn bounds_checks_active_by_default_nth_always_checked() {
    let (_schema, src) = generate(&Paths::example_schema(), "bounds_default");
    compile_and_run(
        "bounds_default",
        &src,
        r#"
        // Encode a message with 0 fuel_figures so the decoder is valid
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(1);
        car.model_year(2000);
        car.available(BooleanType::F);
        car.code(Model::A);
        car.some_numbers([0u32; 4]);
        car.vehicle_code([0u8; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(0, 0, [0, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let mut ff = car2.into_fuel_figures().unwrap();
        // nth() bounds check is ALWAYS present (trust boundary — external idx input)
        let result = ff.nth(999);
        assert!(result.is_err(), "nth(999) on 0-entry group must return Err");
    "#,
    );
}

#[test]
fn bounds_checks_disabled_with_feature_flag() {
    let (_schema, src) = generate(&Paths::example_schema(), "bounds_disabled");
    compile_and_run_with_feature(
        "bounds_disabled",
        &src,
        r#"
        // With bound-check-disabled, wrap_and_apply_header uses unsafe fast path
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
        let car = car.fuel_figures(1, |g| {
            g.add(|e| { e.speed(30).mpg(35.9); e.usage_description(b"Urb").unwrap(); }).unwrap();
        }).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Hon").unwrap();
        let car = car.model(b"Civ").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes();

        let car2 = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        // Field accessors work (without bounds checks in fast path)
        assert_eq!(car2.serial_number(), 1234);
        assert_eq!(car2.model_year(), 2013);
        // Group iteration works
        let ff: Vec<_> = car2.into_fuel_figures().unwrap()
            .collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(ff.len(), 1);
        assert_eq!(ff[0].speed(), 30);
    "#,
        "bound-check-disabled",
    );
}

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

        // All var-data fields encode successfully via the checked path
        // (unchecked paths removed — checked path is canonical)
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
        assert_eq!([1u32, 2, 3, 4], car2.some_numbers());
        assert_eq!([97, 98, 99, 100, 101, 102], car2.vehicle_code());
        let mut fuel = car2.into_fuel_figures().unwrap();
        let ff: Vec<_> = fuel.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(2, ff.len());
        let after_perf = fuel
            .finish()
            .unwrap()
            .into_performance_figures()
            .unwrap()
            .finish()
            .unwrap();
        let (mfr, c1) = after_perf.into_manufacturer().unwrap();
        assert_eq!(b"Honda", mfr);
        let (model, c2) = c1.into_model().unwrap();
        assert_eq!(b"Civic", model);
        let (activation_code, _done) = c2.into_activation_code().unwrap();
        assert_eq!(b"12345", activation_code);
    "#;

    compile_and_run("bndchk_off", &src, test_body);
    compile_and_run_with_feature("bndchk_on", &src, test_body, "bound-check-disabled");
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
    // Group decoder wrap — #[inline] precedes `pub fn wrap(`, but acting_version
    // is on a subsequent line when prettyplease breaks the signature. Check
    // that within 4 lines after #[inline], one line has `fn wrap(` and another
    // (or the same) has `acting_version`.
    let inline_wrap_ok = lines
        .windows(5)
        .filter(|w| w[0].trim() == "#[inline]")
        .any(|w| {
            let window = &w[1..]; // 4 lines after #[inline]
            let has_wrap = window.iter().any(|line| {
                let t = line.trim();
                !t.starts_with("pub fn encoded_length") && t.contains("fn wrap(")
            });
            let has_acting = window
                .iter()
                .any(|line| line.trim().contains("acting_version"));
            has_wrap && has_acting
        });
    assert!(inline_wrap_ok, "group decoder `wrap` missing #[inline]");

    // Encoder entry-point methods
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.contains("fn wrap_and_apply_header(")),
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

    // #[must_use] on encoder setters returning &mut Self. Annotation may be
    // stacked with #[inline], so check within a 3-line window after #[must_use].
    let must_use_ok = |fn_prefix: &str| -> bool {
        lines
            .windows(4)
            .filter(|w| w[0].trim().starts_with("#[must_use"))
            .any(|w| {
                w[1..].iter().any(|line| {
                    let t = line.trim();
                    t.starts_with(fn_prefix) && t.contains("&mut Self")
                })
            })
    };
    assert!(
        must_use_ok("pub fn serial_number("),
        "encoder serial_number setter missing #[must_use]"
    );
    assert!(
        must_use_ok("pub fn model_year("),
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
        let _ = CarEncoder::<'_>::wrap_and_apply_header(&mut buf, 0);
        assert_eq!(
            &buf[0..8],
            &CarEncoder::<'_>::HEADER_TEMPLATE,
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

#[test]
fn encoder_wrap_short_buffer_returns_error() {
    let (_schema, src) = generate(&Paths::example_schema(), "short_buf");
    compile_and_run(
        "short_buf",
        &src,
        r#"
        let total_needed = 8 + CarEncoder::BLOCK_LENGTH;

        // Buffer too short: wrap_and_apply_header checks header + BLOCK_LENGTH
        let mut short_header = [0u8; 7];
        assert!(matches!(
            CarEncoder::wrap_and_apply_header(&mut short_header, 0),
            Err(sbe_rt::EncodeError::BufferTooShort { needed, available: 7 })
            if needed == total_needed
        ));

        // Body too short: wrap checks header + BLOCK_LENGTH
        let mut short_body = vec![0u8; total_needed - 1];
        assert!(matches!(
            CarEncoder::wrap(&mut short_body, 0),
            Err(sbe_rt::EncodeError::BufferTooShort { .. })
        ));

        // Exactly right size works
        let mut exact = vec![0u8; total_needed];
        let _encoder = CarEncoder::wrap_and_apply_header(&mut exact, 0).unwrap();
    "#,
    );
}

#[test]
fn incomplete_encoder_has_no_complete_bytes() {
    let (_schema, src) = generate(&Paths::example_schema(), "incomplete_bytes");
    compile_fails(
        "incomplete_bytes",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let mut encoder = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        encoder.serial_number(1);
        let _ = encoder.as_bytes();
    "#,
    );
}

#[test]
fn u8_dimension_type_generates_correctly() {
    let schema_path = Paths::sbe_tool_test_resource("u8-dimension-schema.xml");
    let (_schema, src) = generate(&schema_path, "u8dim");

    // u8 group dimension template = 2 bytes (blockLength + numInGroup, both uint8)
    assert!(
        src.contains("pub const GROUP_DIM_TEMPLATE: [u8; 2] ="),
        "u8 dimension type must produce 2-byte GROUP_DIM_TEMPLATE, got:\n{src}"
    );

    // Verify the generated code compiles
    syn::parse_file(&src).expect("generated code for u8 schema is not valid Rust");
}

#[test]
fn constant_field_in_message_header_does_not_affect_offsets() {
    let schema_path = Paths::sbe_tool_test_resource("constant-header-field.xml");
    let (_schema, src) = generate(&schema_path, "consthdr");

    // The messageHeader with a constant field should still be 8 bytes
    // (4 × uint16 required fields). The constant field occupies no wire space.
    assert!(
        src.contains("pub const HEADER_TEMPLATE: [u8; 8] ="),
        "HEADER_TEMPLATE must be 8 bytes even with constant header field, got:\n...{}",
        &src[src.find("HEADER_TEMPLATE").unwrap_or(0)..]
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Verify the generated code is valid Rust
    syn::parse_file(&src).expect("generated code is not valid Rust");
}

// ── Versioning forward/backward compatibility (todo 04) ────────────────

#[test]
fn forward_compat_v2_decoder_reads_v1_bytes() {
    let v1_path = Paths::sbe_tool_test_resource("versioned-message-v1.xml");
    let v2_path = Paths::sbe_tool_test_resource("versioned-message-v2.xml");
    let (_s1, v1_src) = generate(&v1_path, "versmsg_v1");
    let (_s2, v2_src) = generate(&v2_path, "versmsg_v2");

    compile_and_run_two_modules(
        "fwd_compat",
        "versmsg_v1",
        &v1_src,
        "versmsg_v2",
        &v2_src,
        r#"
        // ── Encode a V1 message ──
        let mut buf = vec![0u8; 256];
        let mut e = versmsg_v1::VersionedMessageV1Encoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        e.field_a1(100);
        e.field_b1(200);
        let e = e.string1(b"v1data").unwrap();
        let encoded = e.as_bytes();

        // ── Decode with V2 decoder (forward compat) ──
        let d = versmsg_v2::VersionedMessageV2Decoder::wrap_and_apply_header(encoded, 0).unwrap();

        // Common fields (sinceVersion=0) — must decode correctly
        assert_eq!(d.field_a1(), 100, "FieldA1 should survive forward compat");
        assert_eq!(d.field_b1(), 200, "FieldB1 should survive forward compat");

        // V2-only fields (sinceVersion=2, acting_version=1) — return None
        assert_eq!(d.field_c2(), None, "FieldC2 should be None (sinceVersion > actingVersion)");
        assert_eq!(d.field_d2(), None, "FieldD2 should be None (sinceVersion > actingVersion)");
        assert_eq!(d.field_e2(), None, "FieldE2 should be None (sinceVersion > actingVersion)");

        // Var-data — must be readable at correct tail offset
        let (s1, _done) = d.into_string1().unwrap();
        assert_eq!(s1, b"v1data", "String1 should survive forward compat");
    "#,
    );
}

#[test]
fn backward_compat_v1_decoder_reads_v2_bytes() {
    let v1_path = Paths::sbe_tool_test_resource("versioned-message-v1.xml");
    let v2_path = Paths::sbe_tool_test_resource("versioned-message-v2.xml");
    let (_s1, v1_src) = generate(&v1_path, "versmsg_v1");
    let (_s2, v2_src) = generate(&v2_path, "versmsg_v2");

    compile_and_run_two_modules(
        "bwd_compat",
        "versmsg_v1",
        &v1_src,
        "versmsg_v2",
        &v2_src,
        r#"
        // ── Encode a V2 message with all fields ──
        let mut buf = vec![0u8; 256];
        let mut e = versmsg_v2::VersionedMessageV2Encoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        e.field_a1(42);
        e.field_b1(99);
        e.field_c2(111);
        e.field_d2(222);
        e.field_e2(333);
        let e = e.string1(b"v2extra").unwrap();
        let encoded = e.as_bytes();

        // ── Decode with V1 decoder (backward compat) ──
        let d = versmsg_v1::VersionedMessageV1Decoder::wrap_and_apply_header(encoded, 0).unwrap();

        // Known fields must be correct
        assert_eq!(d.field_a1(), 42, "FieldA1 should survive backward compat");
        assert_eq!(d.field_b1(), 99, "FieldB1 should survive backward compat");

        // Var-data: tail offset must skip the extra 12 bytes of V2 fixed fields
        // (V2 blockLength=20, V1 compiled BLOCK_LENGTH=8, acting_block_length=20)
        let (s1, _done) = d.into_string1().unwrap();
        assert_eq!(s1, b"v2extra", "String1 should be at correct tail offset after V2 fixed fields");
    "#,
    );
}

// ── AnyMessage dispatch + FrameCursor (todo 05) ──────────────────────

#[test]
fn anymessage_decode_dispatches_by_template_id() {
    let (_schema, src) = generate(&Paths::example_schema(), "am_decode");
    compile_and_run(
        "am_decode",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(42);
        car.model_year(2020);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(2000, 4, [49, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes();

        // decode dispatches on templateId
        let msg = AnyMessage::decode(encoded, 0).unwrap();
        match msg {
            AnyMessage::Car(d) => {
                assert_eq!(d.serial_number(), 42);
                assert_eq!(d.model_year(), 2020);
            }
            _ => panic!("expected Car, got Unknown"),
        }
    "#,
    );
}

#[test]
fn anymessage_decode_frame_validates_length() {
    let (_schema, src) = generate(&Paths::example_schema(), "am_frame");
    compile_and_run(
        "am_frame",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car.serial_number(99);
        car.model_year(2021);
        car.available(BooleanType::F);
        car.code(Model::B);
        car.some_numbers([9, 8, 7, 6]);
        car.vehicle_code([49, 50, 51, 52, 53, 54]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(1500, 6, [50, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"Toyo").unwrap();
        let car = car.model(b"Corolla").unwrap();
        let car = car.activation_code(b"xyz").unwrap();
        let encoded = car.as_bytes();
        let total_len = encoded.len();

        // decode_frame with correct frame length
        let frame = AnyMessage::decode_frame(encoded, 0, total_len).unwrap();
        assert_eq!(frame.len, total_len);
        assert_eq!(frame.range.start, 0);
        assert_eq!(frame.range.end, total_len);
        match frame.message {
            AnyMessage::Car(d) => assert_eq!(d.serial_number(), 99),
            _ => panic!("expected Car"),
        }

        // decode_frame with too-short frame length → error
        let result = AnyMessage::decode_frame(encoded, 0, 10);
        assert!(result.is_err(), "decode_frame with insufficient frame_len must error");
    "#,
    );
}

#[test]
fn anymessage_unknown_template_forwards_payload() {
    let (_schema, src) = generate(&Paths::example_schema(), "am_unknown");
    compile_and_run(
        "am_unknown",
        &src,
        r#"
        // Construct a message with a non-existent templateId (99)
        // Header: blockLength(2) templateId(2) schemaId(2) version(2) = 8 bytes LE
        let mut buf = vec![0u8; 64];
        buf[0..2].copy_from_slice(&16u16.to_le_bytes());   // blockLength
        buf[2..4].copy_from_slice(&99u16.to_le_bytes());   // templateId = unknown
        buf[4..6].copy_from_slice(&1u16.to_le_bytes());    // schemaId = correct
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());    // version
        buf[8..24].fill(0xAB);                             // 16 bytes of payload

        // decode without frame → error for unknown template
        let result = AnyMessage::decode(&buf, 0);
        assert!(result.is_err(), "bare decode of unknown template must error");

        // decode_frame with frame_len → Unknown variant
        let frame = AnyMessage::decode_frame(&buf, 0, 24).unwrap();
        match frame.message {
            AnyMessage::Unknown { header, payload } => {
                assert_eq!(header.template_id(), 99);
                // payload = &buf[pos..pos+frame_len] = &buf[0..24] = 24 bytes incl header
                assert_eq!(payload.len(), 24);
                assert_eq!(payload[8], 0xAB); // body starts at offset 8
            }
            _ => panic!("expected Unknown"),
        }
    "#,
    );
}

#[test]
fn framecursor_iterates_length_prefixed_frames() {
    let (_schema, src) = generate(&Paths::example_schema(), "fc_iter");
    compile_and_run(
        "fc_iter",
        &src,
        r#"
        // Build two messages
        let mut buf = vec![0u8; 512];
        let mut car1 = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        car1.serial_number(10);
        car1.model_year(2022);
        car1.available(BooleanType::T);
        car1.code(Model::C);
        car1.some_numbers([0, 0, 0, 0]);
        car1.vehicle_code([0; 6]);
        car1.extras(OptionalExtras::default());
        car1.engine(Engine::new(1000, 3, [51, 0, 0]));
        let car1 = car1.fuel_figures(0, |_| {}).unwrap();
        let car1 = car1.performance_figures(0, |_| {}).unwrap();
        let car1 = car1.manufacturer(b"").unwrap();
        let car1 = car1.model(b"").unwrap();
        let car1 = car1.activation_code(b"").unwrap();
        let e1 = car1.as_bytes().to_vec();

        let mut car2 = CarEncoder::wrap_and_apply_header(&mut buf[e1.len()..], 0).unwrap();
        car2.serial_number(20);
        car2.model_year(2023);
        car2.available(BooleanType::F);
        car2.code(Model::A);
        car2.some_numbers([5, 6, 7, 8]);
        car2.vehicle_code([97; 6]);
        car2.extras(OptionalExtras::default());
        car2.engine(Engine::new(2000, 4, [52, 0, 0]));
        let car2 = car2.fuel_figures(0, |_| {}).unwrap();
        let car2 = car2.performance_figures(0, |_| {}).unwrap();
        let car2 = car2.manufacturer(b"BMW").unwrap();
        let car2 = car2.model(b"X5").unwrap();
        let car2 = car2.activation_code(b"").unwrap();
        let e2 = car2.as_bytes().to_vec();

        // Build length-prefixed frame buffer (u32 LE length prefix)
        let mut framed = Vec::new();
        framed.extend_from_slice(&(e1.len() as u32).to_le_bytes());
        framed.extend_from_slice(&e1);
        framed.extend_from_slice(&(e2.len() as u32).to_le_bytes());
        framed.extend_from_slice(&e2);

        // Iterate with FrameCursor
        let cursor = FrameCursor::new(&framed, FramingPolicy::LengthPrefixU32);
        let frames: Vec<_> = cursor.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(frames.len(), 2, "FrameCursor should yield 2 frames");
        match frames[0].message {
            AnyMessage::Car(d) => assert_eq!(d.serial_number(), 10),
            _ => panic!("frame 0 should be Car"),
        }
        match frames[1].message {
            AnyMessage::Car(d) => assert_eq!(d.serial_number(), 20),
            _ => panic!("frame 1 should be Car"),
        }
    "#,
    );
}

#[test]
fn sbemessage_trait_provides_constants() {
    let (_schema, src) = generate(&Paths::example_schema(), "sbe_trait");
    compile_and_run(
        "sbe_trait",
        &src,
        r#"
        // Associated constants on decoder
        assert_eq!(CarDecoder::SCHEMA_ID, 1);
        assert_eq!(CarDecoder::TEMPLATE_ID, 1);
        assert_eq!(CarDecoder::BLOCK_LENGTH, 41);
    "#,
    );
}

// ── Real-world schema compilation (todo 19) ──────────────────────────

#[test]
fn binance_spot_schema_compiles() {
    let schema_path = Paths::sbe_tool_test_resource("binance_spot_3_5.xml");
    let (_schema, src) = generate(&schema_path, "binance_spot");
    syn::parse_file(&src).expect("Binance spot schema must generate valid Rust");
    assert!(src.contains("pub mod prelude"));
}

#[test]
fn cme_fix_binary_schema_compiles() {
    let schema_path = Paths::sbe_tool_test_resource("cme_templates_FixBinary.xml");
    let (_schema, src) = generate(&schema_path, "cme_fix");
    syn::parse_file(&src).expect("CME FIX Binary schema must generate valid Rust");
    assert!(src.contains("pub mod prelude"));
}

#[test]
fn fix_message_samples_schema_compiles() {
    let schema_path = Paths::sbe_tool_test_resource("fix-message-samples.xml");
    let (_schema, src) = generate(&schema_path, "fix_samples");
    syn::parse_file(&src).expect("FIX message samples schema must generate valid Rust");
}

#[test]
fn ilink_binary_schema_compiles() {
    let schema_path = Paths::sbe_tool_test_resource("ilinkbinary.xml");
    let (_schema, src) = generate(&schema_path, "ilink");
    syn::parse_file(&src).expect("iLink Binary schema must generate valid Rust");
    assert!(src.contains("pub mod prelude"));
}

// ── Group entry wire blockLength versioning (todo 145) ────────────────

#[test]
fn v2_decoder_reads_v1_group_entries_using_wire_blocklength() {
    let v1_path = Paths::sbe_tool_test_resource("group-versioning-v1.xml");
    let v2_path = Paths::sbe_tool_test_resource("group-versioning-v2.xml");
    let (_s1, v1_src) = generate(&v1_path, "grpvers_v1");
    let (_s2, v2_src) = generate(&v2_path, "grpvers_v2");

    compile_and_run_two_modules(
        "grp_wire_bl",
        "grpvers_v1",
        &v1_src,
        "grpvers_v2",
        &v2_src,
        r#"
        // ── Encode a V1 message with 2 group entries and trailer ──
        let mut buf = vec![0u8; 256];
        let mut e = grpvers_v1::GroupMsgEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let after_entries = e.entries(2, |g| {
            g.add(|entry| { entry.price(100).qty(10); }).unwrap();
            g.add(|entry| { entry.price(200).qty(20); }).unwrap();
        }).unwrap();
        let complete = after_entries.trailer(b"v1_trailer").unwrap();
        let encoded = complete.as_bytes();

        // ── Decode with V2 decoder (forward compat) ──
        let d = grpvers_v2::GroupMsgDecoder::try_from(encoded).unwrap();

        // V2 decoder sees V1 entries (blockLength=12 on wire, not compiled 16)
        let mut entries_iter = d.into_entries().unwrap();
        let entries: Vec<_> = entries_iter.by_ref().collect::<Vec<_>>();
        assert_eq!(entries.len(), 2, "should find 2 entries");

        // Common fields (sinceVersion=0) — must decode
        assert_eq!(entries[0].price(), 100);
        assert_eq!(entries[0].qty(), 10);
        assert_eq!(entries[1].price(), 200);
        assert_eq!(entries[1].qty(), 20);

        // Trailer var-data must be at correct offset after group entries.
        // This proves the iterator advances by wire blockLength, not compiled.
        let (trailer, _done) = entries_iter.finish().unwrap().into_trailer().unwrap();
        assert_eq!(trailer, b"v1_trailer",
            "trailer must be at correct offset after V1-size group entries");
    "#,
    );
}

#[test]
fn var_data_after_version_mismatched_group_at_correct_offset() {
    let v2_path = Paths::sbe_tool_test_resource("group-versioning-v2.xml");
    let v1_path = Paths::sbe_tool_test_resource("group-versioning-v1.xml");
    let (_s1, v2_src) = generate(&v2_path, "grpvers_v2b");
    let (_s2, v1_src) = generate(&v1_path, "grpvers_v1b");

    compile_and_run_two_modules(
        "grp_var_offset",
        "grpvers_v2b",
        &v2_src,
        "grpvers_v1b",
        &v1_src,
        r#"
        // ── Encode a V2 message with group entries + trailer ──
        let mut buf = vec![0u8; 256];
        let mut e = grpvers_v2b::GroupMsgEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let after_entries = e.entries(2, |g| {
            g.add(|entry| { entry.price(111).qty(22).flags(0xABCD); }).unwrap();
            g.add(|entry| { entry.price(333).qty(44).flags(0xEF01); }).unwrap();
        }).unwrap();
        let complete = after_entries.trailer(b"v2_trailer_data").unwrap();
        let encoded = complete.as_bytes();

        // ── Decode with V1 decoder (backward compat) ──
        let d = grpvers_v1b::GroupMsgDecoder::try_from(encoded).unwrap();

        // V1 decoder sees V2 entries (wire blockLength=16, compiled blockLength=12)
        let mut entries_iter = d.into_entries().unwrap();
        let entries: Vec<_> = entries_iter.by_ref().collect::<Vec<_>>();
        assert_eq!(entries.len(), 2, "should find 2 entries");

        // Known fields correct — V1 decoder reads V2 entries using wire blockLength=16
        assert_eq!(entries[0].price(), 111);
        assert_eq!(entries[0].qty(), 22);
        assert_eq!(entries[1].price(), 333);
        assert_eq!(entries[1].qty(), 44);

        // Trailer must skip the extra 4 bytes per entry (flags field)
        // that V1 doesn't know about but the wire blockLength accounts for
        let (trailer, _done) = entries_iter.finish().unwrap().into_trailer().unwrap();
        assert_eq!(trailer, b"v2_trailer_data",
            "trailer must be at correct offset after V2-size group entries");
    "#,
    );
}

// ── Regression: upstream issue schemas (todo 21) ─────────────────────────

/// Every upstream issue-*.xml schema must either parse cleanly or produce
/// a structured error (never panic). Phase 2 regression gate.
#[test]
fn upstream_issue_schemas_parse_or_error_gracefully() {
    let schemas: &[(&str, bool)] = &[
        ("issue435.xml", true),
        ("issue472.xml", true),
        ("issue483.xml", true),
        ("issue488.xml", true),
        ("issue496.xml", true),
        ("issue505.xml", true),
        ("issue560.xml", true),
        ("issue567-valid.xml", true),
        ("issue567-invalid.xml", true), // ErgoSBE parser handles this; "invalid" refers to upstream tool behaviour
        ("issue661.xml", true),
        ("issue827.xml", true),
        ("issue835.xml", true),
        ("issue847.xml", true),
        ("issue848.xml", true),
        ("issue849.xml", true),
        ("issue889.xml", true),
        ("issue895.xml", true),
        ("issue910.xml", true),
        ("issue967.xml", true),
        ("issue972.xml", true),
        ("issue984.xml", true),
        ("issue987.xml", true),
        ("issue1007.xml", true),
        ("issue1028.xml", true),
        ("issue1057.xml", true),
        ("issue1066.xml", true),
    ];

    let mut parsed = 0usize;
    let mut errored = 0usize;

    for (name, expect_valid) in schemas {
        let path = Paths::sbe_tool_test_resource(name);
        match ergosbe::parse_file(&path) {
            Ok(_ir) => {
                parsed += 1;
                if !expect_valid {
                    eprintln!("UNEXPECTED PASS: {name} (expected parse error)");
                }
            }
            Err(e) => {
                errored += 1;
                let msg = format!("{e}");
                assert!(!msg.is_empty(), "error for {name} must have a message");
                if *expect_valid {
                    eprintln!("PARSE FAIL: {name}: {msg}");
                }
            }
        }
    }

    assert!(parsed + errored > 0, "no schemas processed");
    println!(
        "issue schemas: {parsed} parsed, {errored} errored ({} total)",
        parsed + errored
    );
}

// ── Performance regression locks (prevent reintroduction of slow shapes) ──

#[test]
fn generated_encoder_has_no_phantomdata_or_state_generic() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        !src.contains("core::marker::PhantomData"),
        "encoder must not use PhantomData (SROA barrier)"
    );
    assert!(
        !src.contains("car_encoder_state"),
        "encoder must not use car_encoder_state module (no generic state)"
    );
    assert!(
        !src.contains("State ="),
        "encoder struct must not have a State generic parameter"
    );
}

#[test]
fn generated_encoder_has_concrete_stage_structs() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        src.contains("pub struct CarAfterFuelFigures"),
        "encoder must generate concrete CarAfterFuelFigures stage struct"
    );
    assert!(
        src.contains("pub struct CarComplete"),
        "encoder must generate CarComplete terminal struct"
    );
}

#[test]
fn generated_code_uses_one_slice_indexing() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        !src.contains("[offset..][.."),
        "generated code must use one-slice indexing [offset..offset+N], not [offset..][..N]"
    );
}

#[test]
fn generated_decoder_has_consuming_stages_and_rewind() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // DECISIONS.md §10: the out-of-order skip_to_<later>() surface is removed.
    assert!(
        !src.contains("skip_to_fuel_figures"),
        "decoder must NOT emit the removed skip_to_<later>() out-of-order surface"
    );
    // The concrete consuming tail stages are the public tail-traversal contract.
    assert!(
        src.contains("pub fn into_fuel_figures"),
        "decoder must have the consuming into_fuel_figures() stage"
    );
    assert!(
        src.contains("pub fn rewind(&self) -> Self"),
        "decoder must have rewind() returning Self"
    );
}

#[test]
fn generated_decoder_validates_template_and_schema_id() {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        src.contains("TEMPLATE_ID") && src.contains("SCHEMA_ID"),
        "decoder wrap_and_apply_header must check both template_id and schema_id"
    );
}

// ── Task 4: nested-message decode via var-data ──────────────────────────

#[test]
fn nested_message_decode_via_vardata() {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "nested_msg");
    compile_and_run(
        "nested_msg",
        &src,
        r#"
        // Encode inner message in a separate buffer first
        let inner_len = InnerEncoder::compute_encoded_length_with_message_header(
            b"nested".len(),
        );
        let mut inner_buf = vec![0u8; inner_len];
        let mut inner = InnerEncoder::wrap_and_apply_header(&mut inner_buf, 0).unwrap();
        inner.value(42);
        let inner_complete = inner.label(b"nested").unwrap();
        let inner_bytes = inner_complete.as_bytes().to_vec();

        // Encode outer message, appending inner bytes as payload
        let outer_len = OuterEncoder::compute_encoded_length_with_message_header(
            b"test-app".len(),
            inner_bytes.len(),
        );
        let mut buf = vec![0u8; outer_len];
        let mut outer = OuterEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        outer.trace_id(7);
        let after_name = outer.app_name(b"test-app").unwrap();
        let complete = after_name.payload(&inner_bytes).unwrap();
        assert_eq!(complete.as_bytes().len(), outer_len);

        // Decode: into_app_name -> into_payload_as_message
        let outer_decoder = OuterDecoder::wrap_and_apply_header(&buf, 0).unwrap();
        let (app_name, after_name) = outer_decoder.into_app_name().unwrap();
        assert_eq!(app_name, b"test-app");
        let (frame, complete) = after_name.into_payload_as_message().unwrap();
        match frame.message {
            AnyMessage::Inner(inner) => {
                assert_eq!(inner.value(), 42);
                assert_eq!(inner.into_label().unwrap().0, b"nested");
            }
            _ => panic!("expected Inner"),
        }
        assert_eq!(complete.encoded_length_with_header(), outer_len);
    "#,
    );
}

/// `into_payload_as_message` only exists after preceding fields are consumed.
#[test]
fn nested_message_as_message_requires_ordered_consumption() {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "nested_msg_cf");
    compile_fails(
        "nested_msg_cf",
        &src,
        r#"
        let mut buf = vec![0u8; 256];
        let mut outer = OuterEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        outer.trace_id(7);
        let _complete = outer.app_name(b"t").unwrap();
        let dec = OuterDecoder::wrap_and_apply_header(&buf, 0).unwrap();
        let _ = dec.into_payload_as_message();
    "#,
    );
}

#[test]
fn bounded_nested_payload_encode_via_with() {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "bounded_nested");
    compile_and_run(
        "bounded_nested",
        &src,
        r#"
        let inner_len = InnerEncoder::compute_encoded_length_with_message_header(
            b"nested".len(),
        );
        let outer_len = OuterEncoder::compute_encoded_length_with_message_header(
            b"test-app".len(),
            inner_len,
        );
        let mut buf = vec![0u8; outer_len];
        let mut outer = OuterEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        outer.trace_id(7);
        let complete = outer
            .app_name(b"test-app").unwrap()
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                assert_eq!(payload.len(), inner_len);
                let mut inner = InnerEncoder::wrap_and_apply_header(payload, 0)?;
                inner.value(42);
                let inner_complete = inner.label(b"nested")?;
                assert_eq!(inner_complete.as_bytes_with_header().len(), payload.len());
                Ok(())
            }).unwrap();
        assert_eq!(complete.as_bytes().len(), outer_len);

        // Verify decode works
        let dec = OuterDecoder::wrap_and_apply_header(&buf, 0).unwrap();
        let (_app_name, after_name) = dec.into_app_name().unwrap();
        let (frame, _complete) = after_name.into_payload_as_message().unwrap();
        if let AnyMessage::Inner(inner) = frame.message {
            assert_eq!(inner.value(), 42);
        } else {
            panic!("expected Inner");
        }
    "#,
    );
}

// ── Task 6: SbeDecimal converter seam ──────────────────────────────────

#[test]
fn decimal_converter_enable_config() {
    let config = ergosbe::GenerationConfig::new("decimal_test")
        .enable_decimal_converters("Decimal");
    assert_eq!(config.decimal_composites, vec!["Decimal"]);
    assert!(ergosbe::GenerationConfig::default().decimal_composites.is_empty());
}

#[test]
fn decimal_converter_emits_sbe_decimal_trait() {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let ir = ergosbe::parse_file(&path).unwrap();
    let schema = ergosbe::Schema::from_ir(ir);
    let config = ergosbe::GenerationConfig::new("decimal_test")
        .enable_decimal_converters("Decimal");
    let g = ergosbe::Generator::new(config);
    // try_generate validates the composite
    let modules = g.try_generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;

    // SbeDecimal trait emitted
    assert!(src.contains("pub trait SbeDecimal"), "SbeDecimal trait missing");
    assert!(src.contains("fn try_from_sbe"), "try_from_sbe missing");
    assert!(src.contains("fn try_into_sbe"), "try_into_sbe missing");
}

#[test]
fn decimal_converter_rejects_invalid_composite() {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="bad" id="99" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="BadDecimal"><type name="mantissa" primitiveType="int32"/><type name="exponent" primitiveType="int8"/></composite>
</types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let ir = ergosbe::parse(xml).unwrap();
    let schema = ergosbe::Schema::from_ir(ir);
    let config = ergosbe::GenerationConfig::new("bad_decimal")
        .enable_decimal_converters("BadDecimal");
    let g = ergosbe::Generator::new(config);
    let err = g.try_generate(&schema).unwrap_err();
    assert!(matches!(err, ergosbe::GenerateError::InvalidDecimalComposite { .. }));
}

#[test]
fn decimal_converter_composite_roundtrip() {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let (_schema, src) = generate(&path, "decimal_rt");
    compile_and_run(
        "decimal_rt",
        &src,
        r#"
        // Round-trip raw Decimal composite values
        let mut buf = vec![0u8; 256];
        let mut enc = OrderEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        enc.price(Decimal::new(12345, -2));  // 123.45
        enc.size(Decimal::new(100, 0));       // 100
        let encoded = enc.as_ref().to_vec();

        let dec = OrderDecoder::wrap_and_apply_header(&encoded, 0).unwrap();
        let price = dec.price();
        assert_eq!(price.mantissa(), 12345);
        assert_eq!(price.exponent(), -2);
        let size = dec.size();
        assert_eq!(size.mantissa(), 100);
        assert_eq!(size.exponent(), 0);
    "#,
    );
}
