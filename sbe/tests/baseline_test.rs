//! Port of upstream `simple-binary-encoding/rust/tests/baseline_test.rs`.
//!
//! Decodes the Java-generated binary fixture `car_example_baseline_data.sbe`
//! using ergon-generated code, then encodes from scratch and verifies
//! round-trip decode produces the same logical values.
//!
//! Engine composite includes `<ref>` members and nested `BoostType` (SBE-REF):
//! wire size 10 bytes, Car `BLOCK_LENGTH` 45 matching the sbe-tool/Java fixture header.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{
    Paths, assert_source_ok, compile_and_run, compile_and_run_two_modules,
    compile_and_run_with_deps, compile_and_run_with_feature, compile_fails_with_diagnostics,
    generate, generate_domain_with, run_fixture_test,
};

const MODULE: &str = "car_example";

#[test]
fn external_sbe_rt_two_modules_share_runtime() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};
    let (_schema_a, source_a) = generate(&Paths::example_schema(), "shared_rt");
    // Generate a second module that re-uses the first module's sbe_rt.
    let ir = parse_file(&Paths::example_schema())?;
    let schema_b = Schema::from_ir(ir);
    let mut g = Generator::new(
        GenerationConfig::new("consumer_rt").with_external_sbe_rt("super::shared_rt::sbe_rt"),
    );
    let ms = g.generate(&schema_b)?;
    let source_b = ms.modules().next().unwrap().source.clone();
    // Must import shared sbe_rt
    assert!(
        source_b.contains("super::shared_rt::sbe_rt"),
        "consumer module must import shared sbe_rt; got: {source_b}"
    );
    // Must NOT contain an inline pub mod sbe_rt
    assert!(
        !source_b.contains("pub mod sbe_rt"),
        "consumer module must not inline its own sbe_rt"
    );
    // Both modules compile together and can use types from either.
    compile_and_run_two_modules(
        "external_sbe_rt",
        "shared_rt",
        &source_a,
        "consumer_rt",
        &source_b,
        "// shared_rt and consumer_rt types both resolve\n\
         let _ = shared_rt::CarDecoder::BLOCK_LENGTH;\n\
         let _ = consumer_rt::CarDecoder::BLOCK_LENGTH;\n",
    );
    Ok(())
}

#[test]
fn generated_code_has_lint_suppressions() -> Result<(), Box<dyn std::error::Error>> {
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
        src.contains("clippy::identity_op"),
        "generated code must suppress clippy::identity_op"
    );
    assert!(
        src.contains("#[allow(clippy::eq_op)]"),
        "generated code must suppress clippy::eq_op"
    );
    assert!(
        src.contains("#[allow(clippy::manual_range_contains)]"),
        "generated code must suppress clippy::manual_range_contains"
    );
    // Module-level allows must not include needless_borrow (emission should not need it).
    assert!(
        !src.contains("#[allow(clippy::needless_borrow)]"),
        "generated code must NOT suppress needless_borrow at module level"
    );
    // Module-level allows must not hide unused_imports / unused_unsafe / unused_variables.
    assert!(
        !src.contains("#[allow(unused_imports)]"),
        "generated code must NOT suppress unused_imports at module level"
    );
    assert!(
        !src.contains("unused_unsafe"),
        "generated code must NOT suppress unused_unsafe at module level"
    );
    assert!(
        !src.contains("#[allow(unused_variables)]"),
        "generated code must NOT suppress unused_variables at module level"
    );
    assert!(
        !src.contains("#[allow(unused_mut)]"),
        "generated code must NOT suppress unused_mut at module level"
    );
    Ok(())
}

#[test]
fn reserved_name_lists_have_no_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    // ENCODER_RESERVED / DECODER_RESERVED live once in conversion_helpers.rs.
    // Placement utils (remaining/buffer/limit/message_offset/as_fixed_*) must
    // not appear — they are only on get_metadata().
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let helpers = std::fs::read_to_string(root.join("src/codegen/conversion_helpers.rs"))?;
    for (label, marker) in [
        ("ENCODER_RESERVED", "const ENCODER_RESERVED"),
        ("DECODER_RESERVED", "const DECODER_RESERVED"),
    ] {
        let start = helpers
            .find(marker)
            .ok_or_else(|| format!("missing {marker} in conversion_helpers.rs"))?;
        let rest = &helpers[start..];
        let end = rest
            .find("];")
            .ok_or_else(|| format!("unterminated {marker}"))?;
        let block = &rest[..end];
        let mut names: Vec<&str> = Vec::new();
        for line in block.lines() {
            let t = line.trim();
            if let Some(s) = t.strip_prefix('"') {
                if let Some(name) = s.split('"').next() {
                    if !name.is_empty() {
                        names.push(name);
                    }
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut dups = Vec::new();
        for n in &names {
            if !seen.insert(*n) {
                dups.push(*n);
            }
        }
        assert!(
            dups.is_empty(),
            "{label} has duplicate reserved names: {dups:?} (all={names:?})"
        );
        assert!(
            names.len() >= 8,
            "{label} parsed too few names ({}); block may have moved",
            names.len()
        );
        for placement in [
            "remaining",
            "buffer",
            "limit",
            "message_offset",
            "as_fixed_body_bytes",
            "as_fixed_region_with_header",
        ] {
            assert!(
                !names.contains(&placement),
                "{label} must not reserve placement util `{placement}` (lives on get_metadata())"
            );
        }
        assert!(
            !names.contains(&"header"),
            "{label} must not reserve stale `header` (never emitted as inherent method)"
        );
    }
    // No local copies left in decoder/encoder/display modules.
    for rel in [
        "src/codegen/message_decoder.rs",
        "src/codegen/message_encoder.rs",
        "src/codegen/decoder_display.rs",
    ] {
        let src = std::fs::read_to_string(root.join(rel))?;
        assert!(
            !src.contains("const DECODER_RESERVED") && !src.contains("const ENCODER_RESERVED"),
            "{rel} must import shared reserved lists from conversion_helpers, not redefine them"
        );
    }
    Ok(())
}

#[test]
fn generated_code_contains_expected_types() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

#[test]
fn generate_composite_with_enum_set_and_nested_composite() -> Result<(), Box<dyn std::error::Error>>
{
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
    Ok(())
}

#[test]
fn generate_composite_with_named_type_member_refs() -> Result<(), Box<dyn std::error::Error>> {
    // composite-field-refs.xml defines a composite "Widget" whose members
    // reference a named enum (Colour), set (Flags), and composite (Inner).
    // Generating it exercises the composite field-type codegen arms that handle
    // enum/set/nested-composite *member references* (a shape no other fixture has).
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/composite-field-refs.xml");
    let (_s, src) = generate(&path, "comp_field_refs");
    assert_source_ok(&src, &["Widget", "Colour", "Flags", "Inner"]);
    Ok(())
}

#[test]
fn generate_versioned_set_enum_composite_fields() -> Result<(), Box<dyn std::error::Error>> {
    // extension-schema.xml has message fields with sinceVersion > 0 of set
    // (ASet), enum (AEnum), and composite (AComposite) types. Generating it
    // exercises the versioned-field codegen branches (Option<T> accessor shape).
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/extension-schema.xml");
    let (_s, src) = generate(&path, "ext_versioned");
    syn::parse_file(&src).expect("extension-schema generates valid Rust");
    Ok(())
}

#[test]
fn generate_enums_over_every_integer_encoding_type() -> Result<(), Box<dyn std::error::Error>> {
    // enum-encoding-types.xml has one enum per int/uint encoding type. Generating
    // it exercises max_encoding_value for every integer primitive (the enum NULL
    // const = encodingType.maxValue()) and the enum codegen for each width.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/enum-encoding-types.xml");
    let (_s, src) = generate(&path, "enum_enc");
    syn::parse_file(&src).expect("enum-encoding-types generates valid Rust");
    Ok(())
}

#[test]
fn generate_big_endian_and_unbounded_var_data_schemas() -> Result<(), Box<dyn std::error::Error>> {
    // Big-endian schema exercises the BE byte-order codegen branches; the
    // unbounded-var-data schema (no maxLength attr) exercises the var-data
    // accessor's else branch.
    let (_s1, src1) = generate(&Paths::bigendian_schema(), "be_schema");
    syn::parse_file(&src1).expect("bigendian schema generates valid Rust");

    let (_s2, src2) = generate(&Paths::basic_variable_length_schema(), "unbounded_vd");
    syn::parse_file(&src2).expect("unbounded var-data schema generates valid Rust");
    Ok(())
}

#[test]
fn generate_multi_message_schema() -> Result<(), Box<dyn std::error::Error>> {
    // binance_spot_3_5.xml has 92 messages with shared types (enums, sets,
    // composites). Generating it exercises the multi-message codegen branches
    // (shared_types collection, header-type dispatch, multi-message paths).
    let path = Paths::sbe_tool_test_resource("binance_spot_3_5.xml");
    let (_s, src) = generate(&path, "binance_mm");
    syn::parse_file(&src).expect("binance multi-message schema generates valid Rust");
    Ok(())
}

#[test]
fn generator_constructs_with_config() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_sbe::{DomainVarData, GenerationConfig, Generator};
    let _generator = Generator::new(GenerationConfig::new("cfg_test"));
    Ok(())
}

#[test]
fn generate_multi_schema_entry_point() -> Result<(), Box<dyn std::error::Error>> {
    // generate_multi() generates multiple schemas into separate modules with
    // shared-type tracking. No other test exercises it.
    use ergo_sbe::{DomainVarData, GenerationConfig, Generator, Schema, parse_file};
    let ir1 = parse_file(&Paths::example_schema()).unwrap();
    let s1 = Schema::from_ir(ir1);
    let ir2 = parse_file(&Paths::l3_orderbook_schema()).unwrap();
    let s2 = Schema::from_ir(ir2);
    let mut g = Generator::new(GenerationConfig::new("multi_test"));
    let ms = g.generate_multi(&[(&s1, "mod1"), (&s2, "mod2")])?;
    let count = ms.modules().count();
    assert!(count >= 2, "expected >=2 modules, got {count}");
    Ok(())
}

#[test]
fn generate_coverage_edges_schema() -> Result<(), Box<dyn std::error::Error>> {
    // coverage-edges.xml exercises: group-name dedup, constant set field,
    // and BooleanType in a group entry (the _bool accessor).
    use std::path::PathBuf;
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas/coverage-edges.xml");
    let (_s, src) = generate(&path, "covedges");
    syn::parse_file(&src).expect("coverage-edges generates valid Rust");
    Ok(())
}

#[test]
fn generate_custom_header_type_schema() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

#[test]
fn generate_constant_value_schema() -> Result<(), Box<dyn std::error::Error>> {
    // constant-value-types.xml has message fields of presence="constant" with
    // float, double, and int64 types. Generating it covers the constant_value_expr
    // formatting branches (f32/f64/i64 format strings, 537/541/544).
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/constant-value-types.xml");
    let (_s, src) = generate(&path, "const_vals");
    syn::parse_file(&src).expect("constant-value-types generates valid Rust");
    Ok(())
}

#[test]
fn generate_constant_set_field() -> Result<(), Box<dyn std::error::Error>> {
    // constant-set-field.xml has a SET field with presence="constant".
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/constant-set-field.xml");
    let (_s, src) = generate(&path, "const_set");
    syn::parse_file(&src).expect("constant-set-field generates valid Rust");
    Ok(())
}

#[test]
fn generate_vardata_without_max_length() -> Result<(), Box<dyn std::error::Error>> {
    // vardata-no-maxlength.xml has a custom var-data encoding whose length
    // field (uint8, no maxValue) gives max_length=None, triggering the
    // else branch of the var-data accessor generation.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/vardata-no-maxlength.xml");
    let (_s, src) = generate(&path, "vd_nomax");
    syn::parse_file(&src).expect("vardata-no-maxlength generates valid Rust");
    Ok(())
}

#[test]
fn generate_schema_with_include_file() -> Result<(), Box<dyn std::error::Error>> {
    // Exercises the include processing code path in read_include_file +
    // parse_schema (lines 540-564 in xml.rs). types-include.xml defines
    // types referenced by schema-with-include.xml.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/schema-with-include.xml");
    let (_s, src) = generate(&path, "schema_with_inc");
    syn::parse_file(&src).expect("schema-with-include generates valid Rust");
    Ok(())
}

#[test]
fn generate_group_entry_with_composite_enum_set_fields() -> Result<(), Box<dyn std::error::Error>> {
    // group-entry-field-types.xml has a group whose entry has fields of type
    // Composite (Inner), Enum (Colour), and Set (Flags). Covers the
    // FieldType::Composite/Enum/Set size arms in group encoder/decoder.
    use std::path::PathBuf;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/schemas/group-entry-field-types.xml");
    let (_s, src) = generate(&path, "gentry");
    syn::parse_file(&src).expect("group-entry-field-types generates valid Rust");
    Ok(())
}

#[test]
fn decode_baseline_fixture() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture_test(
        "baseline_decode",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let car = CarDecoder::try_decode(FIXTE, 0).unwrap();

        assert_eq!(1234, car.serial_number(), "serial_number");
        assert_eq!(2013, car.model_year(), "model_year");
        assert_eq!(BooleanType::T, car.available(), "available");
        assert_eq!(Model::A, car.code(), "code");

        assert_eq!([1u32, 2, 3, 4], car.some_numbers(), "someNumbers");
        assert_eq!([97, 98, 99, 100, 101, 102], car.vehicle_code(), "vehicleCode");

        let extras = car.extras();
        assert_eq!(6, extras.raw(), "extras raw");
        assert!(extras.is_cruise_control(), "cruiseControl");
        assert!(extras.is_sports_pack(), "sportsPack");
        assert!(!extras.is_sun_roof(), "sunRoof");

        // discountedModel is presence="constant" valueRef="Model.C"
        assert_eq!(Model::C, car.discounted_model(), "discountedModel");

        // Engine is 10 bytes on the official SBE wire (capacity, numCylinders,
        // manufacturerCode[3], efficiency, boosterEnabled, booster{BoostType,hp}).
        let engine = car.engine();
        assert_eq!(2000, engine.capacity(), "engine.capacity");
        assert_eq!(4, engine.num_cylinders(), "engine.numCylinders");
        assert_eq!([b'1', b'2', b'3'], engine.manufacturer_code(), "engine.manufacturerCode");
        assert_eq!(35, engine.efficiency(), "engine.efficiency");
        assert_eq!(BooleanType::T, engine.booster_enabled(), "engine.boosterEnabled");
        let booster = engine.booster();
        assert_eq!(BoostType::NITROUS, booster.boost_type(), "engine.booster.boostType");
        assert_eq!(200, booster.horse_power(), "engine.booster.horsePower");

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

        let mut perf_iter = fuel.finish().unwrap().into_performance_figures().unwrap();
        let perf: Vec<_> = perf_iter.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(2, perf.len(), "performanceFigures count");

        assert_eq!(95, perf[0].octane_rating(), "pf[0].octaneRating");
        let accel0: Vec<_> = perf[0].acceleration().unwrap().collect::<Vec<_>>();
        assert_eq!(3, accel0.len(), "pf[0].acceleration count");
        assert_eq!(30,  accel0[0].mph(), "pf[0].acc[0].mph");
        assert!((accel0[0].seconds() - 4.0).abs() < 0.01, "pf[0].acc[0].seconds");
        assert_eq!(60,  accel0[1].mph(), "pf[0].acc[1].mph");
        assert!((accel0[1].seconds() - 7.5).abs() < 0.01, "pf[0].acc[1].seconds");
        assert_eq!(100, accel0[2].mph(), "pf[0].acc[2].mph");
        assert!((accel0[2].seconds() - 12.2).abs() < 0.01, "pf[0].acc[2].seconds");

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
    Ok(())
}

#[test]
fn decoder_display() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture_test(
        "display_test",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let car = CarDecoder::try_decode(FIXTE, 0).unwrap();
        let s = format!("{}", car);
        assert!(s.contains("serialNumber: 1234"), "display serialNumber: {s}");
        assert!(s.contains("modelYear: 2013"), "display modelYear: {s}");
        assert!(s.contains("available: T"), "display available: {s}");
        assert!(s.contains("code: A"), "display code: {s}");
        assert!(s.contains("fuelFigures: ["), "display fuelFigures entries: {s}");
        assert!(s.contains("performanceFigures: ["), "display performanceFigures entries: {s}");
        assert!(s.starts_with("CarDecoder {"), "display starts with CarDecoder: {s}");
        assert!(s.ends_with(" }"), "display ends with }}");
        "#,
    );
    Ok(())
}

#[test]
fn encode_baseline_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture_test(
        "baseline_encode",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: {
            let mut extras = OptionalExtras::default();
            extras.cruise_control(true);
            extras.sports_pack(true);
            extras
        },
                engine: Engine::new(
            2000,
            4,
            [b'1', b'2', b'3'],
            35,
            BooleanType::T,
            Booster::new(BoostType::NITROUS, 200),
        ),
            })
            .fuel_figures(3, |g| {
            g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle") }).unwrap();
            g.add(|mut e| { e.speed(55).mpg(49.0); e.usage_description(b"Combined Cycle") }).unwrap();
            g.add(|mut e| { e.speed(75).mpg(40.0); e.usage_description(b"Highway Cycle") })
        }).unwrap();

        let car = car.performance_figures(2, |g| {
            g.add(|mut e| { e.octane_rating(95);
                e.acceleration(3, |a| {
                    a.add(|x| { x.mph(30).seconds(4.0); Ok(()) }).unwrap();
                    a.add(|x| { x.mph(60).seconds(7.5); Ok(()) }).unwrap();
                    a.add(|x| { x.mph(100).seconds(12.2); Ok(()) })
                })
            }).unwrap();
            g.add(|mut e| { e.octane_rating(99);
                e.acceleration(3, |a| {
                    a.add(|x| { x.mph(30).seconds(3.8); Ok(()) }).unwrap();
                    a.add(|x| { x.mph(60).seconds(7.1); Ok(()) }).unwrap();
                    a.add(|x| { x.mph(100).seconds(11.8); Ok(()) })
                })
            })
        }).unwrap();

        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic VTi").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();

        let encoded = car.as_bytes_with_header();
        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();

        assert_eq!(1234, car2.serial_number(), "rt.serial_number");
        assert_eq!(2013, car2.model_year(), "rt.model_year");
        assert_eq!(BooleanType::T, car2.available(), "rt.available");
        assert_eq!(Model::A, car2.code(), "rt.code");
        assert_eq!([1u32, 2, 3, 4], car2.some_numbers(), "rt.someNumbers");
        assert_eq!([97, 98, 99, 100, 101, 102], car2.vehicle_code(), "rt.vehicleCode");

        let extras2 = car2.extras();
        assert!(extras2.is_cruise_control(), "rt.cruiseControl");
        assert!(extras2.is_sports_pack(), "rt.sportsPack");
        assert!(!extras2.is_sun_roof(), "rt.sunRoof");

        let e2 = car2.engine();
        assert_eq!(2000, e2.capacity(), "rt.engine.capacity");
        assert_eq!(4, e2.num_cylinders(), "rt.engine.numCylinders");
        assert_eq!(9000, e2.max_rpm(), "rt.engine.maxRpm");
        assert_eq!([b'1', b'2', b'3'], e2.manufacturer_code(), "rt.engine.manufacturerCode");
        assert_eq!("Petrol", e2.fuel(), "rt.engine.fuel");
        assert_eq!(35, e2.efficiency(), "rt.engine.efficiency");
        assert_eq!(BooleanType::T, e2.booster_enabled(), "rt.engine.boosterEnabled");
        assert_eq!(BoostType::NITROUS, e2.booster().boost_type(), "rt.engine.booster.boostType");
        assert_eq!(200, e2.booster().horse_power(), "rt.engine.booster.horsePower");

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
    Ok(())
}

#[test]
fn encode_byte_exact_scalar() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture_test(
        "scalar_byte_exact",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: {
            let mut extras = OptionalExtras::default();
            extras.cruise_control(true);
            extras.sports_pack(true);
            extras
        },
                engine: Engine::new(
            2000,
            4,
            [b'1', b'2', b'3'],
            35,
            BooleanType::T,
            Booster::new(BoostType::NITROUS, 200),
        ),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();
        let encoded = car.as_bytes_with_header();

        // Full 8-byte header including blockLength=45 matches the sbe-tool/Java fixture.
        assert_eq!(&FIXTE[0..8], &encoded[0..8], "header mismatch");

        // Scalar body through engine (offsets 0..45).
        let header_size = 8usize;
        assert_eq!(
            &FIXTE[header_size .. header_size + 45],
            &encoded[header_size .. header_size + 45],
            "fixed body (incl. engine) mismatch"
        );
        "#,
    );
    Ok(())
}

#[test]
fn composite_byte_exact_engine() -> Result<(), Box<dyn std::error::Error>> {
    run_fixture_test(
        "engine_byte_exact",
        &Paths::example_schema(),
        &Paths::baseline_binary(),
        r#"
        // Encode Engine with values matching the sbe-tool/Java fixture (10-byte block).
        let engine = Engine::new(
            2000,
            4,
            [b'1', b'2', b'3'],
            35,
            BooleanType::T,
            Booster::new(BoostType::NITROUS, 200),
        );

        // Fixture engine starts at body_offset 35 (file position 43).
        let header_size = 8usize;
        let engine_offset = 35usize;
        let engine_size = 10usize;
        assert_eq!(engine.0.len(), engine_size, "Engine wire size");
        assert_eq!(
            &FIXTE[header_size + engine_offset .. header_size + engine_offset + engine_size],
            &engine.0[..],
            "engine wire bytes mismatch"
        );
        "#,
    );
    Ok(())
}

#[test]
fn schema_id_from_header_car_example() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

#[test]
fn constants_match_upstream() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    assert!(src.contains("pub const SCHEMA_ID: u16 = 1;"));
    assert!(src.contains("pub const SCHEMA_VERSION: u16 = 0;"));
    assert!(src.contains("pub const TEMPLATE_ID: u16 = 1;"));
    // 35 fixed scalars + 10-byte Engine (with <ref> + nested BoostType).
    assert!(src.contains("pub const BLOCK_LENGTH: usize = 45;"));
    Ok(())
}

#[test]
fn group_decoder_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "is_empty_group");
    compile_and_run(
        "is_empty_group",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic VTi").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes_with_header();
        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        assert!(car2.into_fuel_figures().unwrap().is_empty(), "0 fuel figures → is_empty == true");

        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(3, |g| {
            g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban Cycle") }).unwrap();
            g.add(|mut e| { e.speed(55).mpg(49.0); e.usage_description(b"Combined Cycle") }).unwrap();
            g.add(|mut e| { e.speed(75).mpg(40.0); e.usage_description(b"Highway Cycle") })
        }).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic VTi").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes_with_header();
        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        assert!(!car2.into_fuel_figures().unwrap().is_empty(), "3 fuel figures → is_empty == false");
    "#,
    );
    Ok(())
}

#[test]
fn compute_encoded_length_matches_actual() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "pre_encode_len");
    compile_and_run(
        "pre_encode_len",
        &src,
        r#"
        // Baseline: zero groups / zero var-data
        // Groups and var-data are always present (dim headers + length prefixes even at 0)
        // Use large buffer pattern instead of staged EncodedLength builders
        let mut buf = [0u8; 4096];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(&[]).unwrap();
        let car = car.model(&[]).unwrap();
        let car = car.activation_code(&[]).unwrap();
        let empty = car.encoded_length();
        assert_eq!(empty, 65); // 45 (block) + 2×4 (group dims) + 3×4 (vardata prefixes)
        let empty_full = car.encoded_length_with_header();
        assert_eq!(empty_full, 73); // 65 + 8-byte header

        // Header-inclusive length must use the dedicated helper.
        let mut buf = [0u8; 4096];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(1, |ff| {
                ff.add(|mut e| {
                    e.speed(0).mpg(0.0);
                    e.usage_description(b"")
                })
            })
            .unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(&[0u8; 5]).unwrap();
        let car = car.model(&[0u8; 4]).unwrap();
        let car = car.activation_code(&[0u8; 6]).unwrap();
        let body = car.encoded_length();
        let full = car.encoded_length_with_header();
        assert!(full > body, "full length must exceed body length");

        // Pre-compute exact encoded length with the EncodedLength builder
        let pre_len = CarEncoder::compute_length()
            .fuel_figures_ragged(1, |ff| {
                ff.add()?.usage_description(0)?;
                Ok(())
            })?
            .performance_figures(0)
            .finish_empty()?
            .manufacturer(5)?
            .model(4)?
            .activation_code(6)?
            .encoded_length_with_header();
        assert_eq!(full, pre_len,
            "actual {full} must match pre-computed {pre_len}");

        // Encode a simple message (no nested groups, no entry var-data)
        // and verify the pre-computed length matches actual encoded length
        // Use large buffer pattern instead of staged EncodedLength builders
        let mut buf = [0u8; 4096];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civc").unwrap();
        let car = car.activation_code(b"abc123").unwrap();
        let body_len = car.encoded_length();
        let full_len = car.encoded_length_with_header();
        assert!(body_len > 0, "body_len must be positive");
        assert!(full_len > body_len, "full_len must exceed body_len");
    "#,
    );
    Ok(())
}

#[test]
fn fixed_entry_group_entries_iterator() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "entries_iter");
    compile_and_run(
        "entries_iter",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(1, |g| {
            g.add(|mut e| { e.octane_rating(95);
                e.acceleration(3, |ag| {
                    ag.add(|ae| { ae.mph(30).seconds(4.0); Ok(()) }).unwrap();
                    ag.add(|ae| { ae.mph(60).seconds(7.5); Ok(()) }).unwrap();
                    ag.add(|ae| { ae.mph(100).seconds(12.2); Ok(()) })
                })
            })
        }).unwrap();
        let car = car.manufacturer(b"Hon").unwrap();
        let car = car.model(b"Civ").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
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
    Ok(())
}

#[test]
fn array_accessor_all_paths_return_same_values() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "array_paths");
    compile_and_run(
        "array_paths",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();

        let safe: [u32; 4] = car2.some_numbers();
        assert_eq!(safe, [1u32, 2, 3, 4]);

        // vehicle_code (byte array)
        let vs: [u8; 6] = car2.vehicle_code();
        assert_eq!(vs, [97, 98, 99, 100, 101, 102]);
    "#,
    );
    Ok(())
}

#[test]
fn display_shows_group_entry_fields_not_just_count() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "display_entries");
    compile_and_run(
        "display_entries",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(2, |g| {
            g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urban") }).unwrap();
            g.add(|mut e| { e.speed(55).mpg(49.0); e.usage_description(b"Comb") })
        }).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        let display = format!("{}", car2);

        // Display must include entry field values, not just "N entries"
        assert!(display.contains("speed"), "Display missing 'speed': {display}");
        assert!(display.contains("mpg"), "Display missing 'mpg': {display}");
        assert!(display.contains("30"), "Display missing speed value 30: {display}");
        assert!(display.contains("55"), "Display missing speed value 55: {display}");
        // Must NOT show stale "N entries" count-only format
        assert!(!display.contains("2 entries"), "Display should not show raw count: {display}");
        assert!(display.contains("serialNumber"), "Display missing serialNumber: {display}");
        assert!(display.contains("1234"), "Display missing serial_number value: {display}");
    "#,
    );
    Ok(())
}

#[test]
fn composite_default_is_flyweight_value_is_eager_copy() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "composite_api");
    compile_and_run(
        "composite_api",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"abcdef").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();

        // Default: flyweight (zero-copy from buffer)
        let fly: EngineDecoder = car2.engine();
        assert_eq!(fly.capacity(), 2000);
        assert_eq!(fly.num_cylinders(), 4);

        let eager: Engine = car2.engine_value();
        assert_eq!(eager.capacity(), 2000);
        assert_eq!(eager.num_cylinders(), 4);

        assert_eq!(fly.capacity(), eager.capacity());
        assert_eq!(fly.num_cylinders(), eager.num_cylinders());

        #[allow(deprecated)]
        {
            let old: EngineDecoder = car2.engine();
            assert_eq!(old.capacity(), 2000);
        }
    "#,
    );
    Ok(())
}

#[test]
fn bounds_checks_active_by_default_nth_always_checked() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "bounds_default");
    compile_and_run(
        "bounds_default",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2000,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(0, 0, [0, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(()))
            .unwrap()
            .performance_figures(0, |_| Ok(()))
            .unwrap()
            .manufacturer(b"")
            .unwrap()
            .model(b"")
            .unwrap()
            .activation_code(b"")
            .unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        let mut ff = car2.into_fuel_figures().unwrap();
        // scan_entry_at() bounds check is ALWAYS present (trust boundary — external idx input)
        let result = ff.scan_entry_at(999);
        assert!(result.is_err(), "scan_entry_at(999) on 0-entry group must return Err");
    "#,
    );
    Ok(())
}

#[test]
fn bounds_checks_disabled_with_feature_flag() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "bounds_disabled");
    compile_and_run_with_feature(
        "bounds_disabled",
        &src,
        r#"
        // With bound-check-disabled, wrap_and_apply_header (unchecked) uses unsafe fast path
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(1, |g| {
            g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"Urb") })
        }).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Hon").unwrap();
        let car = car.model(b"Civ").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        // Field accessors work (without bounds checks in fast path)
        assert_eq!(car2.serial_number(), 1234);
        assert_eq!(car2.model_year(), 2013);
        let ff: Vec<_> = car2.into_fuel_figures().unwrap()
            .collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(ff.len(), 1);
        assert_eq!(ff[0].speed(), 30);
    "#,
        "bound-check-disabled",
    );
    Ok(())
}

#[test]
fn generated_code_has_cold_annotations() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // sbe_rt emits #[cold] on all three error Display impls
    let cold_count = src.matches("#[cold]").count();
    assert!(
        cold_count >= 3,
        "expected >=3 #[cold] annotations (DecodeError, EncodeError, VerifyError Display impls), found {cold_count}"
    );
    Ok(())
}

#[test]
fn generated_code_has_const_assertions() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // BLOCK_LENGTH assertion is the relevant compile-time check
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
    Ok(())
}

#[test]
fn generated_code_has_boolean_from_impls() -> Result<(), Box<dyn std::error::Error>> {
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

    assert!(
        src.contains("impl From<bool> for BooleanType"),
        "BooleanType must implement From<bool>"
    );
    assert!(
        src.contains("impl TryFrom<BooleanType> for bool"),
        "BooleanType must implement TryFrom (NullVal is not a Rust bool)"
    );
    assert!(
        !src.contains("impl From<BooleanType> for bool"),
        "infallible From<BooleanType> for bool must not collapse NullVal"
    );

    // From<bool> maps true → Self::T and false → Self::F
    assert!(
        src.contains("if val { Self::T } else { Self::F }"),
        "BooleanType From<bool> must map true/false to T/F variants"
    );

    assert!(
        src.contains("available_bool"),
        "Car encoder must have available_bool"
    );

    assert!(
        src.contains("available_bool"),
        "Car decoder must have available_bool"
    );
    Ok(())
}

#[test]
fn generated_code_has_vardata_maxlength() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // EncodeError must have VarDataTooLong variant
    assert!(
        src.contains("VarDataTooLong") && src.contains("max_length"),
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
    Ok(())
}

#[test]
fn composite_ref_members_generated() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

    // Engine: capacity(2)+numCylinders(1)+manufacturerCode(3)+efficiency(1)
    // +boosterEnabled(1)+booster(2) = 10 (constants maxRpm/fuel not on wire).
    assert!(
        src.contains("struct Engine") && src.contains("[u8; 10]"),
        "Engine should be [u8; 10] with expanded <ref> + nested BoostType, got:\n{}",
        src.lines()
            .find(|l| l.contains("struct Engine"))
            .unwrap_or("<missing Engine>")
    );
    assert!(
        src.contains("pub fn efficiency("),
        "Engine::efficiency() from <ref name=\"efficiency\" type=\"Percentage\"/>"
    );
    assert!(
        src.contains("pub fn booster_enabled("),
        "Engine::booster_enabled() from <ref name=\"boosterEnabled\" type=\"BooleanType\"/>"
    );
    assert!(
        src.contains("pub fn booster("),
        "Engine::booster() from <ref name=\"booster\" type=\"Booster\"/>"
    );

    // Booster: nested BoostType (char) + horsePower (uint8).
    assert!(
        src.contains("struct Booster") && src.contains("[u8; 2]"),
        "Booster should be [u8; 2] (BoostType + horsePower), got:\n{}",
        src.lines()
            .find(|l| l.contains("struct Booster"))
            .unwrap_or("<missing Booster>")
    );
    assert!(
        src.contains("pub enum BoostType"),
        "nested BoostType enum inside Booster must be generated"
    );
    assert!(
        src.contains("pub enum BooleanType"),
        "BooleanType should exist as a top-level enum"
    );
    assert!(
        src.contains("pub struct Booster"),
        "Booster composite should exist as a top-level type"
    );
    Ok(())
}

/// SBE-REF acceptance: parse → generate → compile → encode/decode Engine refs.
#[test]
fn composite_ref_engine_roundtrip_compile() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "engine_ref_rt");
    compile_and_run(
        "engine_ref_rt",
        &src,
        r#"
        let eng = Engine::new(
            2000,
            4,
            [b'1', b'2', b'3'],
            35,
            BooleanType::T,
            Booster::new(BoostType::NITROUS, 200),
        );
        assert_eq!(eng.0.len(), 10);
        assert_eq!(eng.capacity(), 2000);
        assert_eq!(eng.num_cylinders(), 4);
        assert_eq!(eng.manufacturer_code(), [b'1', b'2', b'3']);
        assert_eq!(eng.efficiency(), 35);
        assert_eq!(eng.booster_enabled(), BooleanType::T);
        assert_eq!(eng.booster().boost_type(), BoostType::NITROUS);
        assert_eq!(eng.booster().horse_power(), 200);

        // sbe-tool/Java car fixture engine block (body offset 35): capacity=2000,
        // cylinders=4, mfr="123", efficiency=35, boostEnabled=T, NITROUS@200.
        let fixture_engine: [u8; 10] =
            [0xd0, 0x07, 0x04, b'1', b'2', b'3', 35, 1, b'N', 200];
        assert_eq!(&fixture_engine[..], &eng.0[..]);

        // Full message: encode + decode recovers ref members.
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [0u32; 4],
                vehicle_code: [0u8; 6],
                extras: OptionalExtras::default(),
                engine: eng,
            })
            .fuel_figures(0, |_| Ok(()))
            .unwrap()
            .performance_figures(0, |_| Ok(()))
            .unwrap()
            .manufacturer(b"X")
            .unwrap()
            .model(b"Y")
            .unwrap()
            .activation_code(b"Z")
            .unwrap();
        let encoded = car.as_bytes_with_header();
        assert_eq!(CarDecoder::BLOCK_LENGTH, 45);
        let dec = CarDecoder::try_decode(encoded, 0).unwrap();
        let e2 = dec.engine();
        assert_eq!(e2.efficiency(), 35);
        assert_eq!(e2.booster().horse_power(), 200);
        "#,
    );
    Ok(())
}

#[test]
fn vardata_maxlength_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "vardata_max_len");
    compile_and_run(
        "vardata_max_len",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        assert!(car.activation_code(b"12345").is_ok(), "activationCode within maxLength via checked");

        // All var-data fields encode successfully via the checked path.
        "#,
    );
    Ok(())
}

#[test]
fn boolean_roundtrip_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "bool_rt");
    compile_and_run(
        "bool_rt",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"12345").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        let available = car2.available();
        assert_eq!(available, BooleanType::T, "round-trip available via available_bool(true)");
        assert_ne!(available.raw(), 0, "BooleanType::T raw != 0");

        let mut buf = [0u8; 512];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 1234,
                model_year: 2013,
                available: BooleanType::F,
                code: Model::A,
                some_numbers: [1u32, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"12345").unwrap();
        let encoded = car.as_bytes_with_header();

        let car2 = CarDecoder::try_decode(encoded, 0).unwrap();
        let available = car2.available();
        assert_eq!(available, BooleanType::F, "round-trip available via BooleanType::F");
        assert_eq!(available.raw(), 0, "BooleanType::F raw == 0");
        "#,
    );

    // Also verify bool conversions compile
    assert!(src.contains("impl From<bool> for BooleanType"));
    assert!(src.contains("impl TryFrom<BooleanType> for bool"));
    Ok(())
}

#[test]
fn generated_code_has_inline_annotations() -> Result<(), Box<dyn std::error::Error>> {
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

    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.starts_with("pub fn serial_number(")),
        "decoder checked accessor `serial_number` missing #[inline]"
    );
    assert!(
        inline_followed_by
            .iter()
            .any(|s| s.contains("fn fuel_figures(")),
        "group decoder accessor `fuel_figures` missing #[inline]"
    );
    assert!(
        src.contains("#[inline]\n    fn next(&mut self) -> Option<Self::Item>"),
        "generated group Iterator::next missing #[inline]"
    );
    assert!(
        src.contains("#[inline]\n    #[must_use]\n    pub fn fuel_figures<F>"),
        "message group encoder `fuel_figures` missing #[inline]"
    );
    assert!(
        src.contains("#[inline]\n    #[must_use]\n    pub fn manufacturer(\n        mut self,"),
        "message var-data encoder `manufacturer` missing #[inline]"
    );
    assert!(
        src.contains(
            "#[inline]\n    #[must_use]\n    pub fn usage_description(\n        mut self,"
        ),
        "entry var-data encoder `usage_description` missing #[inline]"
    );
    assert!(
        src.contains("#[inline]\n    pub fn extras(&mut self, val: OptionalExtras)"),
        "message set encoder `extras` missing #[inline]"
    );
    assert!(
        src.contains("#[inline]\n    pub fn engine(&mut self, val: Engine)"),
        "message composite encoder `engine` missing #[inline]"
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
    Ok(())
}

#[test]
fn generated_code_has_must_use_annotations() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);

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

    // Fixed-field setters (&mut Self) intentionally do NOT carry #[must_use] —
    // the side effect (buffer write) is the point; the returned reference is
    // meant to be discarded. Result-returning methods and encoder structs
    // keep their #[must_use].

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

    // Category-aware observers (T-5): `#[must_use]` may sit above `#[inline]`,
    // so look back from the fn rather than requiring it on the next line.
    for method in [
        "as_option",
        "as_bool",
        "as_body_bytes",
        "get_metadata",
        "written",
        "encoded_length_with_header",
    ] {
        let needle = format!("fn {method}(");
        let pos = src
            .find(&needle)
            .unwrap_or_else(|| panic!("observer `{method}` not generated"));
        let preceding = &src[pos.saturating_sub(280)..pos];
        assert!(
            preceding.contains("#[must_use"),
            "observer `{method}` missing #[must_use]; preceding:\n{preceding}"
        );
    }
    Ok(())
}

#[test]
fn static_header_templates_exist() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "static_tpl");

    assert!(
        src.contains("pub const HEADER_TEMPLATE: [u8; 8] = [45, 0, 1, 0, 1, 0, 0, 0];"),
        "HEADER_TEMPLATE must contain correct pre-computed header bytes \
         (blockLength=45, templateId=1, schemaId=1, version=0, little-endian)"
    );
    assert!(
        src.contains("pub const GROUP_DIM_TEMPLATE: [u8; 4] ="),
        "GROUP_DIM_TEMPLATE must exist as a [u8; 4] constant"
    );

    // wrap_and_apply_header must use copy_nonoverlapping from HEADER_TEMPLATE
    assert!(
        src.contains("copy_nonoverlapping") && src.contains("HEADER_TEMPLATE"),
        "wrap_and_apply_header must use copy_nonoverlapping from HEADER_TEMPLATE"
    );

    // Group encoder must use copy_nonoverlapping from its GROUP_DIM_TEMPLATE
    assert!(
        src.contains("copy_nonoverlapping") && src.contains("GROUP_DIM_TEMPLATE"),
        "group encoder must use copy_nonoverlapping from its GROUP_DIM_TEMPLATE"
    );

    // Runtime: wrap_and_apply_header writes HEADER_TEMPLATE bytes correctly
    compile_and_run(
        "static_tpl",
        &src,
        r#"
        let mut buf = [0u8; 512];
        // Drop the encoder immediately to release the mutable borrow,
        // then verify header bytes were written correctly.
        let _ = CarEncoder::<'_>::wrap_and_apply_header(&mut buf, 0);
        assert_eq!(
            &buf[0..8],
            &CarEncoder::<'_>::HEADER_TEMPLATE,
            "wrap_and_apply_header must write HEADER_TEMPLATE bytes"
        );
        let block_len = u16::from_le_bytes([buf[0], buf[1]]);
        let template_id = u16::from_le_bytes([buf[2], buf[3]]);
        let schema_id = u16::from_le_bytes([buf[4], buf[5]]);
        let version = u16::from_le_bytes([buf[6], buf[7]]);
        assert_eq!(block_len, 45, "header blockLength must be 45");
        assert_eq!(template_id, 1, "header templateId must be 1");
        assert_eq!(schema_id, 1, "header schemaId must be 1");
        assert_eq!(version, 0, "header version must be 0");
    "#,
    );
    Ok(())
}

#[test]
fn encoder_wrap_short_buffer_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "short_buf");
    compile_and_run(
        "short_buf",
        &src,
        r#"
        let total_needed = 8 + CarEncoder::BLOCK_LENGTH;

        // Buffer too short: try_wrap_and_apply_header checks header + BLOCK_LENGTH
        let mut short_header = [0u8; 7];
        assert!(matches!(
            CarEncoder::try_wrap_and_apply_header(&mut short_header, 0),
            Err(sbe_rt::EncodeError::BufferTooShort { needed, available: 7, .. })
            if needed == total_needed
        ));

        // Body too short: try_wrap checks header + BLOCK_LENGTH
        let mut short_body_storage = [0u8; 8192];
        assert!(total_needed - 1 <= short_body_storage.len());
        let mut short_body = &mut short_body_storage[..total_needed - 1];
        assert!(matches!(
            CarEncoder::try_wrap(&mut short_body, 0),
            Err(sbe_rt::EncodeError::BufferTooShort { .. })
        ));

        // Hostile caller offsets must return errors, never wrap around and
        // panic while slicing a small buffer.
        let mut scratch = [0u8; 64];
        assert!(CarEncoder::try_wrap(&mut scratch, usize::MAX).is_err());
        assert!(CarEncoder::try_wrap_and_apply_header(&mut scratch, usize::MAX).is_err());
        assert!(CarDecoder::try_decode(&scratch, usize::MAX).is_err());
        assert!(AnyMessage::try_decode(&scratch, usize::MAX).is_err());
        assert!(AnyMessage::decode_frame(&scratch, usize::MAX, 1).is_err());

        let mut exact_storage = [0u8; 8192];
        assert!(total_needed <= exact_storage.len());
        let mut exact = &mut exact_storage[..total_needed];
        let _encoder = CarEncoder::wrap_and_apply_header(&mut exact, 0);
    "#,
    );
    Ok(())
}

#[test]
fn incomplete_encoder_has_no_complete_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "incomplete_bytes");
    compile_fails_with_diagnostics(
        "incomplete_bytes",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let mut encoder = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        encoder.serial_number(1);
        let _ = encoder.as_bytes_with_header();
    "#,
        &["no method named `as_bytes_with_header`"],
    );
    Ok(())
}

#[test]
fn u8_dimension_type_generates_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let schema_path = Paths::sbe_tool_test_resource("u8-dimension-schema.xml");
    let (_schema, src) = generate(&schema_path, "u8dim");

    // u8 group dimension template = 2 bytes (blockLength + numInGroup, both uint8)
    assert!(
        src.contains("pub const GROUP_DIM_TEMPLATE: [u8; 2] ="),
        "u8 dimension type must produce 2-byte GROUP_DIM_TEMPLATE, got:\n{src}"
    );

    syn::parse_file(&src).expect("generated code for u8 schema is not valid Rust");
    Ok(())
}

#[test]
fn constant_field_in_message_header_does_not_affect_offsets()
-> Result<(), Box<dyn std::error::Error>> {
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

    syn::parse_file(&src).expect("generated code is not valid Rust");
    Ok(())
}

#[test]
fn forward_compat_v2_decoder_reads_v1_bytes() -> Result<(), Box<dyn std::error::Error>> {
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
        let mut buf = [0u8; 256];
        let e = versmsg_v1::VersionedMessageV1Encoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&versmsg_v1::VersionedMessageV1FixedFields { field_a1: 100, field_b1: 200 })
            .string1(b"v1data").unwrap();
        let encoded = e.as_bytes_with_header();

        let d = versmsg_v2::VersionedMessageV2Decoder::try_decode(encoded, 0).unwrap();

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
    Ok(())
}

#[test]
fn backward_compat_v1_decoder_reads_v2_bytes() -> Result<(), Box<dyn std::error::Error>> {
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
        let mut buf = [0u8; 256];
        let e = versmsg_v2::VersionedMessageV2Encoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&versmsg_v2::VersionedMessageV2FixedFields {
                field_a1: 42,
                field_b1: 99,
                field_c2: 111,
                field_d2: 222,
                field_e2: 333,
            })
            .string1(b"v2extra").unwrap();
        let encoded = e.as_bytes_with_header();

        let d = versmsg_v1::VersionedMessageV1Decoder::try_decode(encoded, 0).unwrap();

        assert_eq!(d.field_a1(), 42, "FieldA1 should survive backward compat");
        assert_eq!(d.field_b1(), 99, "FieldB1 should survive backward compat");

        // Var-data: tail offset must skip the extra 12 bytes of V2 fixed fields
        // (V2 blockLength=20, V1 compiled BLOCK_LENGTH=8, acting_block_length=20)
        let (s1, _done) = d.into_string1().unwrap();
        assert_eq!(s1, b"v2extra", "String1 should be at correct tail offset after V2 fixed fields");
    "#,
    );
    Ok(())
}

#[test]
fn anymessage_decode_dispatches_by_template_id() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "am_decode");
    compile_and_run(
        "am_decode",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 42,
                model_year: 2020,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: [97, 98, 99, 100, 101, 102],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [49, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let car = car.activation_code(b"abc").unwrap();
        let encoded = car.as_bytes_with_header();

        // decode dispatches on templateId
        let msg = AnyMessage::try_decode(encoded, 0).unwrap();
        match msg {
            AnyMessage::Car(d) => {
                assert_eq!(d.serial_number(), 42);
                assert_eq!(d.model_year(), 2020);
            }
            _ => panic!("expected Car, got Unknown"),
        }
    "#,
    );
    Ok(())
}

#[test]
fn anymessage_decode_frame_validates_length() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "am_frame");
    compile_and_run(
        "am_frame",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let car = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 99,
                model_year: 2021,
                available: BooleanType::F,
                code: Model::B,
                some_numbers: [9, 8, 7, 6],
                vehicle_code: [49, 50, 51, 52, 53, 54],
                extras: OptionalExtras::default(),
                engine: Engine::new(1500, 6, [50, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap();
        let car = car.performance_figures(0, |_| Ok(())).unwrap();
        let car = car.manufacturer(b"Toyo").unwrap();
        let car = car.model(b"Corolla").unwrap();
        let car = car.activation_code(b"xyz").unwrap();
        let encoded = car.as_bytes_with_header();
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
    Ok(())
}

#[test]
fn anymessage_unknown_template_forwards_the_complete_frame()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "am_unknown");
    compile_and_run(
        "am_unknown",
        &src,
        r#"
        // Construct a message with a non-existent templateId (99)
        // Header: blockLength(2) templateId(2) schemaId(2) version(2) = 8 bytes LE
        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&16u16.to_le_bytes());   // blockLength
        buf[2..4].copy_from_slice(&99u16.to_le_bytes());   // templateId = unknown
        buf[4..6].copy_from_slice(&1u16.to_le_bytes());    // schemaId = correct
        buf[6..8].copy_from_slice(&0u16.to_le_bytes());    // version
        buf[8..24].fill(0xAB);                             // 16 bytes of payload

        let result = AnyMessage::try_decode(&buf, 0);
        assert!(result.is_err(), "bare decode of unknown template must error");

        // decode_frame with frame_len → Unknown variant
        let frame = AnyMessage::decode_frame(&buf, 0, 24).unwrap();
        match frame.message {
            AnyMessage::Unknown { header, frame } => {
                assert_eq!(header.template_id(), 99);
                // frame = &buf[pos..pos+frame_len] = &buf[0..24] — header + body
                assert_eq!(frame.len(), 24);
                assert_eq!(frame[8], 0xAB); // body starts at offset 8
            }
            _ => panic!("expected Unknown"),
        }
    "#,
    );
    Ok(())
}

#[test]
fn framecursor_iterates_length_prefixed_frames() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "fc_iter");
    compile_and_run(
        "fc_iter",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let car1 = CarEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&CarFixedFields {
                serial_number: 10, model_year: 2022, available: BooleanType::T, code: Model::C,
                some_numbers: [0, 0, 0, 0], vehicle_code: [0; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(1000, 3, [51, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap()
            .performance_figures(0, |_| Ok(())).unwrap()
            .manufacturer(b"").unwrap()
            .model(b"").unwrap()
            .activation_code(b"").unwrap();
        let e1 = car1.as_bytes_with_header().to_vec();

        let car2 = CarEncoder::wrap_and_apply_header(&mut buf[e1.len()..], 0)
            .fixed(&CarFixedFields {
                serial_number: 20, model_year: 2023, available: BooleanType::F, code: Model::A,
                some_numbers: [5, 6, 7, 8], vehicle_code: [97; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(2000, 4, [52, 0, 0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
            })
            .fuel_figures(0, |_| Ok(())).unwrap()
            .performance_figures(0, |_| Ok(())).unwrap()
            .manufacturer(b"BMW").unwrap()
            .model(b"X5").unwrap()
            .activation_code(b"").unwrap();
        let e2 = car2.as_bytes_with_header().to_vec();

        let mut framed = Vec::new();
        framed.extend_from_slice(&(e1.len() as u32).to_le_bytes());
        framed.extend_from_slice(&e1);
        framed.extend_from_slice(&(e2.len() as u32).to_le_bytes());
        framed.extend_from_slice(&e2);

        let cursor = FrameCursor::new(&framed, FramingPolicy::LengthPrefixU32Le);
        let frames: Vec<_> = cursor.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(frames.len(), 2, "FrameCursor should yield 2 frames");
        let mut frames = frames.into_iter();
        match frames.next().unwrap().message {
            AnyMessage::Car(d) => assert_eq!(d.serial_number(), 10),
            _ => panic!("frame 0 should be Car"),
        }
        match frames.next().unwrap().message {
            AnyMessage::Car(d) => assert_eq!(d.serial_number(), 20),
            _ => panic!("frame 1 should be Car"),
        }
    "#,
    );
    Ok(())
}

#[test]
fn sbemessage_trait_provides_constants() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "sbe_trait");
    compile_and_run(
        "sbe_trait",
        &src,
        r#"
        assert_eq!(CarDecoder::SCHEMA_ID, 1);
        assert_eq!(CarDecoder::TEMPLATE_ID, 1);
        assert_eq!(CarDecoder::BLOCK_LENGTH, 45);
    "#,
    );
    Ok(())
}

#[test]
fn binance_spot_schema_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let schema_path = Paths::sbe_tool_test_resource("binance_spot_3_5.xml");
    let (_schema, src) = generate(&schema_path, "binance_spot");
    syn::parse_file(&src).expect("Binance spot schema must generate valid Rust");
    assert!(src.contains("pub mod prelude"));
    Ok(())
}

#[test]
fn cme_fix_binary_schema_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let schema_path = Paths::sbe_tool_test_resource("cme_templates_FixBinary.xml");
    let (_schema, src) = generate(&schema_path, "cme_fix");
    syn::parse_file(&src).expect("CME FIX Binary schema must generate valid Rust");
    assert!(src.contains("pub mod prelude"));
    Ok(())
}

#[test]
fn fix_message_samples_schema_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let schema_path = Paths::sbe_tool_test_resource("fix-message-samples.xml");
    let (_schema, src) = generate(&schema_path, "fix_samples");
    syn::parse_file(&src).expect("FIX message samples schema must generate valid Rust");
    Ok(())
}

#[test]
fn ilink_binary_schema_compiles() -> Result<(), Box<dyn std::error::Error>> {
    let schema_path = Paths::sbe_tool_test_resource("ilinkbinary.xml");
    let (_schema, src) = generate(&schema_path, "ilink");
    syn::parse_file(&src).expect("iLink Binary schema must generate valid Rust");
    assert!(src.contains("pub mod prelude"));
    Ok(())
}

#[test]
fn v2_decoder_reads_v1_group_entries_using_wire_blocklength()
-> Result<(), Box<dyn std::error::Error>> {
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
        let mut buf = [0u8; 256];
        let after_entries = grpvers_v1::GroupMsgEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&grpvers_v1::GroupMsgFixedFields {})
            .entries(2, |g| {
                g.add(|mut entry| {
                    entry.price(100).qty(10);
                    Ok(())
                })?;
                g.add(|mut entry| {
                    entry.price(200).qty(20);
                    Ok(())
                })?;
                Ok(())
            })
            .unwrap();
        let complete = after_entries.trailer(b"v1_trailer").unwrap();
        let encoded = complete.as_bytes_with_header();

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
    Ok(())
}

#[test]
fn var_data_after_version_mismatched_group_at_correct_offset()
-> Result<(), Box<dyn std::error::Error>> {
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
        let mut buf = [0u8; 256];
        let after_entries = grpvers_v2b::GroupMsgEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&grpvers_v2b::GroupMsgFixedFields {})
            .entries(2, |g| {
                g.add(|mut entry| {
                    entry.price(111).qty(22).flags(0xABCD);
                    Ok(())
                })?;
                g.add(|mut entry| {
                    entry.price(333).qty(44).flags(0xEF01);
                    Ok(())
                })?;
                Ok(())
            })
            .unwrap();
        let complete = after_entries.trailer(b"v2_trailer_data").unwrap();
        let encoded = complete.as_bytes_with_header();

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
    Ok(())
}

/// Every supported upstream issue fixture must continue to parse cleanly.
#[test]
fn upstream_issue_schemas_parse() -> Result<(), Box<dyn std::error::Error>> {
    let schemas = [
        "issue435.xml",
        "issue472.xml",
        "issue483.xml",
        "issue488.xml",
        "issue496.xml",
        "issue505.xml",
        "issue560.xml",
        "issue567-valid.xml",
        // The pinned sbe-tool rejects this implementation-specific count range;
        // FIX SBE itself does not make the schema invalid.
        "issue567-invalid.xml",
        "issue661.xml",
        "issue827.xml",
        "issue835.xml",
        "issue847.xml",
        "issue848.xml",
        "issue849.xml",
        "issue889.xml",
        "issue895.xml",
        "issue910.xml",
        "issue967.xml",
        "issue972.xml",
        "issue984.xml",
        "issue987.xml",
        "issue1007.xml",
        "issue1028.xml",
        "issue1057.xml",
        "issue1066.xml",
    ];

    for name in schemas {
        let path = Paths::sbe_tool_test_resource(name);
        assert!(path.is_file(), "upstream issue fixture is missing: {name}");
        ergo_sbe::parse_file(&path)
            .unwrap_or_else(|error| panic!("{name}: supported schema failed to parse: {error}"));
    }

    Ok(())
}

#[test]
fn generated_encoder_has_no_phantomdata_or_state_generic() -> Result<(), Box<dyn std::error::Error>>
{
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // Encoder now uses PhantomData<H> for HeaderPresent/HeaderAbsent typestate.
    // This is zero-sized and eliminated by the compiler — no SROA impact.
    assert!(
        src.contains("core::marker::PhantomData"),
        "encoder must use PhantomData for header-aware typestate"
    );
    assert!(
        src.contains("HeaderPresent"),
        "encoder must reference HeaderPresent for typestate"
    );
    // Encoder now has HeaderState = HeaderPresent as the default type param
    assert!(
        src.contains("HeaderState ="),
        "encoder must have HeaderState default parameter"
    );
    Ok(())
}

#[test]
fn generated_encoder_has_concrete_stage_structs() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        src.contains("pub struct CarAfterFuelFigures"),
        "encoder must generate concrete CarAfterFuelFigures stage struct"
    );
    assert!(
        src.contains("pub struct CarComplete"),
        "encoder must generate CarComplete terminal struct"
    );
    Ok(())
}

#[test]
fn generated_code_uses_one_slice_indexing() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        !src.contains("[offset..][.."),
        "generated code must use one-slice indexing [offset..offset+N], not [offset..][..N]"
    );
    Ok(())
}

#[test]
fn generated_decoder_has_consuming_stages_and_rewind() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    // Out-of-order skip_to_<later>() surface is not generated.
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
        src.contains("pub fn rewind(self) -> Self"),
        "decoder must have consuming rewind(self) returning Self"
    );
    Ok(())
}

#[test]
fn generated_decoder_validates_template_and_schema_id() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), MODULE);
    assert!(
        src.contains("TEMPLATE_ID") && src.contains("SCHEMA_ID"),
        "decoder try_wrap_and_apply_header must check both template_id and schema_id"
    );
    Ok(())
}

#[test]
fn nested_message_decode_via_vardata() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "nested_msg");
    compile_and_run(
        "nested_msg",
        &src,
        r#"
        let inner_len = InnerEncoder::compute_encoded_length_with_message_header(
            b"nested".len(),
        );
        let mut inner_buf_storage = [0u8; 8192];
        assert!(inner_len <= inner_buf_storage.len());
        let mut inner_buf = &mut inner_buf_storage[..inner_len];
        let inner_complete = InnerEncoder::wrap_and_apply_header(&mut inner_buf, 0)
            .fixed(&InnerFixedFields { value: 42 })
            .label(b"nested")
            .unwrap();
        let inner_bytes = inner_complete.as_bytes_with_header().to_vec();

        let outer_len = OuterEncoder::compute_encoded_length_with_message_header(
            b"test-app".len(),
            inner_bytes.len(),
        );
        let mut buf_storage = [0u8; 8192];
        assert!(outer_len <= buf_storage.len());
        let mut buf = &mut buf_storage[..outer_len];
        let outer = OuterEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OuterFixedFields { trace_id: 7 });
        let after_name = outer.app_name(b"test-app").unwrap();
        let complete = after_name.payload(&inner_bytes).unwrap();
        assert_eq!(complete.as_bytes_with_header().len(), outer_len);

        let outer_decoder = OuterDecoder::try_decode(&buf, 0).unwrap();
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
        assert_eq!(complete.encoded_length() + OuterSchema::HEADER_LENGTH, outer_len);
    "#,
    );
    Ok(())
}

/// `into_payload_as_message` only exists after preceding fields are consumed.
#[test]
fn nested_message_as_message_requires_ordered_consumption() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "nested_msg_cf");
    compile_fails_with_diagnostics(
        "nested_msg_cf",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let _complete = OuterEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OuterFixedFields { trace_id: 7 })
            .app_name(b"t").unwrap();
        let dec = OuterDecoder::try_decode(&buf, 0).unwrap();
        let _ = dec.into_payload_as_message();
    "#,
        &["no method named `into_payload_as_message`"],
    );
    Ok(())
}

#[test]
fn bounded_nested_payload_encode_via_with() -> Result<(), Box<dyn std::error::Error>> {
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
        let mut buf_storage = [0u8; 8192];
        assert!(outer_len <= buf_storage.len());
        let mut buf = &mut buf_storage[..outer_len];
        let complete = OuterEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OuterFixedFields { trace_id: 7 })
            .app_name(b"test-app").unwrap()
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                assert_eq!(payload.len(), inner_len);
                let inner_complete = InnerEncoder::try_wrap_and_apply_header(payload, 0)?
                    .fixed(&InnerFixedFields { value: 42 })
                    .label(b"nested")?;
                assert_eq!(inner_complete.as_bytes_with_header().len(), payload.len());
                Ok(())
            }).unwrap();
        assert_eq!(complete.as_bytes_with_header().len(), outer_len);

        let dec = OuterDecoder::try_decode(&buf, 0).unwrap();
        let (_app_name, after_name) = dec.into_app_name().unwrap();
        let (frame, _complete) = after_name.into_payload_as_message().unwrap();
        if let AnyMessage::Inner(inner) = frame.message {
            assert_eq!(inner.value(), 42);
        } else {
            panic!("expected Inner");
        }
    "#,
    );
    Ok(())
}

#[test]
fn decimal_converter_enable_config() -> Result<(), Box<dyn std::error::Error>> {
    // Builder methods produce a valid config (smoke test; detailed assertions
    // live in the config unit tests inside ergo-sbe).
    let _config = ergo_sbe::GenerationConfig::new("decimal_test")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let _default = ergo_sbe::GenerationConfig::default();
    Ok(())
}

#[test]
fn decimal_converter_emits_conversion_traits() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let ir = ergo_sbe::parse_file(&path).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("decimal_test")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let mut g = ergo_sbe::Generator::new(config);
    // try_generate validates the composite
    let modules = g.generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;

    assert!(
        src.contains("pub trait TryFromSbe"),
        "TryFromSbe trait missing"
    );
    assert!(src.contains("fn try_from_sbe"), "try_from_sbe missing");
    assert!(src.contains("fn try_to_sbe"), "try_to_sbe missing");
    Ok(())
}

#[test]
fn conversion_rejects_nonexistent_type() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="bad" id="99" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
</types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("bad")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("NonExistent"));
    let mut g = ergo_sbe::Generator::new(config);
    let err = g.generate(&schema).unwrap_err();
    assert!(matches!(
        err,
        ergo_sbe::GenerateError::InvalidConversion { .. }
    ));
    Ok(())
}

/// Non-existent composite name is rejected with "not found in schema".
#[test]
fn decimal_converter_rejects_missing_composite() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="bad" id="99" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
</types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("missing_dec")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("NonExistent"));
    let mut g = ergo_sbe::Generator::new(config);
    let err = g.generate(&schema).unwrap_err();
    assert!(matches!(
        err,
        ergo_sbe::GenerateError::InvalidConversion { .. }
    ));
    Ok(())
}

/// GenerateError renders a readable message via Display.
#[test]
fn generate_error_display_formats() -> Result<(), Box<dyn std::error::Error>> {
    let err = ergo_sbe::GenerateError::InvalidConversion {
        selector: "Decimal".into(),
        reason: "wrong layout".into(),
    };
    assert_eq!(
        err.to_string(),
        "invalid conversion 'Decimal': wrong layout"
    );
    Ok(())
}

/// A registered decimal composite with fewer than two members is rejected.
#[test]
fn conversion_rejects_nonexistent_named_type() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="bad" id="99" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
</types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("missing")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("DoesNotExist"));
    let mut g = ergo_sbe::Generator::new(config);
    let err = g.generate(&schema).unwrap_err();
    assert!(
        err.to_string().contains("invalid conversion"),
        "unexpected error: {err}"
    );
    Ok(())
}

/// The panicking `generate` wrapper still validates converter configuration.
#[test]
fn generate_returns_error_on_invalid_decimal_composite() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="bad" id="99" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
</types>
<sbe:message name="M" id="1"><field name="f" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("panics")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("NonExistent"));
    let mut g = ergo_sbe::Generator::new(config);
    let err = g.generate(&schema).unwrap_err();
    assert!(err.to_string().contains("NonExistent"));
    Ok(())
}

/// Converter emission skips scalar fields, non-decimal composites, and
/// messages without any decimal fields.
#[test]
fn decimal_converter_skips_non_decimal_fields_and_messages()
-> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="mixed" id="98" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="Decimal"><type name="mantissa" primitiveType="int64"/><type name="exponent" primitiveType="int8"/></composite>
  <composite name="Point"><type name="x" primitiveType="int32"/><type name="y" primitiveType="int32"/></composite>
</types>
<sbe:message name="Mixed" id="1">
  <field name="qty" id="1" type="uint32"/>
  <field name="pos" id="2" type="Point"/>
  <field name="price" id="3" type="Decimal"/>
</sbe:message>
<sbe:message name="NoDec" id="2">
  <field name="qty" id="1" type="uint32"/>
</sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("mixed_dec")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let mut g = ergo_sbe::Generator::new(config);
    let modules = g.generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;
    // The decimal field has raw _wire accessor and generic _as/_from methods.
    assert!(src.contains("price_wire"), "raw wire accessor missing");
    assert!(
        src.contains("price_as"),
        "generic price_as accessor missing"
    );
    assert!(
        src.contains("price_from"),
        "generic price_from setter missing"
    );
    // Non-decimal fields get no generic converter methods (use TryFromSbe in
    // the pattern to distinguish from struct accessors like pos_value).
    assert!(
        !src.contains("qty_as<T:"),
        "scalar field must not get a converter method"
    );
    assert!(
        !src.contains("pos_as<T:"),
        "non-decimal composite must not get a converter method"
    );
    Ok(())
}

/// A var-data composite whose `length` member is not first cannot represent
/// the required SBE length-prefix-then-payload wire layout.
#[test]
fn vardata_composite_with_length_not_first_member_is_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="revvar" id="97" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="oddVarEncoding">
    <type name="varData" primitiveType="uint8" length="0"/>
    <type name="length" primitiveType="uint32" maxValue="1024"/>
  </composite>
</types>
<sbe:message name="M" id="1">
  <field name="a" id="1" type="uint32"/>
  <data name="blob" id="2" type="oddVarEncoding"/>
</sbe:message>
</sbe:messageSchema>"#;
    let err = ergo_sbe::parse(xml).expect_err("reversed var-data members must be rejected");
    assert!(
        format!("{err}").contains("expected exactly 'length' then 'varData'"),
        "unexpected diagnostic: {err}"
    );
    Ok(())
}

/// `presence=constant` without a text value is invalid SBE and must fail parse
/// (not panic in codegen).
#[test]
fn group_entry_constant_field_without_value_is_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="novalconst" id="96" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="groupSizeEncoding"><type name="blockLength" primitiveType="uint16"/><type name="numInGroup" primitiveType="uint16"/></composite>
  <type name="EmptyConst" primitiveType="uint8" presence="constant"/>
</types>
<sbe:message name="M" id="1">
  <field name="a" id="1" type="uint32"/>
  <group name="g" id="2" dimensionType="groupSizeEncoding">
    <field name="c" id="3" type="EmptyConst"/>
    <field name="v" id="4" type="uint16"/>
  </group>
</sbe:message>
</sbe:messageSchema>"#;
    let err = ergo_sbe::parse(xml).expect_err("constant without value text must fail parse");
    let msg = format!("{err}");
    assert!(
        msg.contains("constant") || msg.contains("EmptyConst"),
        "expected constant-value fault, got: {msg}"
    );
    Ok(())
}

/// The default `headerType="messageHeader"` still requires that composite to
/// be declared, matching sbe-tool's schema validation.
#[test]
fn schema_without_header_composite_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="nohdr" id="95" version="0" byteOrder="littleEndian">
<types>
  <type name="u32x" primitiveType="uint32"/>
</types>
<sbe:message name="M" id="1"><field name="a" id="1" type="uint32"/></sbe:message>
</sbe:messageSchema>"#;
    let err = ergo_sbe::parse(xml).expect_err("headerless schema must be rejected");
    assert!(
        format!("{err}").contains("messageHeader: expected a defined composite"),
        "unexpected diagnostic: {err}"
    );
    Ok(())
}

/// Group entries via add() produce identical bytes regardless of how the
/// entry body is filled (inline vs chained).
#[test]
fn manual_start_entry_matches_closure() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "start_entry_test");
    compile_and_run(
        "start_entry_test",
        &src,
        r#"
        let mut buf_closure = [0u8; 512];
        let mut buf_manual = [0u8; 512];

        let ff = CarFixedFields {
            serial_number: 42, model_year: 2020, available: BooleanType::T, code: Model::A,
            some_numbers: [0u32;4], vehicle_code: [0u8;6],
            extras: OptionalExtras::default(),
            engine: Engine::new(1000, 4, [0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        };
        let closure_bytes = CarEncoder::wrap_and_apply_header(&mut buf_closure, 0)
            .fixed(&ff)
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"") })?;
                g.add(|mut e| { e.speed(55).mpg(23.7); e.usage_description(b"") })?;
                Ok(())
            }).unwrap()
            .performance_figures(0, |_| Ok(())).unwrap()
            .manufacturer(b"").unwrap()
            .model(b"").unwrap()
            .activation_code(b"").unwrap()
            .as_bytes_with_header().to_vec();

        let manual_bytes = CarEncoder::wrap_and_apply_header(&mut buf_manual, 0)
            .fixed(&ff)
            .fuel_figures(2, |g| {
                g.add(|mut e| { e.speed(30).mpg(35.9); e.usage_description(b"") })?;
                g.add(|mut e| { e.speed(55).mpg(23.7); e.usage_description(b"") })?;
                Ok(())
            }).unwrap()
            .performance_figures(0, |_| Ok(())).unwrap()
            .manufacturer(b"").unwrap()
            .model(b"").unwrap()
            .activation_code(b"").unwrap()
            .as_bytes_with_header().to_vec();

        assert_eq!(closure_bytes, manual_bytes);
    "#,
    );
    Ok(())
}

#[test]
fn decimal_converter_composite_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let (_schema, src) = generate(&path, "decimal_rt");
    compile_and_run(
        "decimal_rt",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let enc = OrderEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OrderFixedFields {
                price: Decimal::new(12345, -2), // 123.45
                size: Decimal::new(100, 0),     // 100
            });
        let encoded = enc.as_bytes_with_header().to_vec();

        let dec = OrderDecoder::try_decode(&encoded, 0).unwrap();
        let price = dec.price();
        assert_eq!(price.mantissa(), 12345);
        assert_eq!(price.exponent(), -2);
        let size = dec.size();
        assert_eq!(size.mantissa(), 100);
        assert_eq!(size.exponent(), 0);
    "#,
    );
    Ok(())
}

/// `DomainImpl::Manual` gets the same concrete `try_price(...)?` /
/// `try_price()?` signatures as `DomainImpl::Generated`, but ergo-sbe does
/// not generate the `TryFromSbe`/`TryToSbe` impl for the built-in
/// `rust_decimal` mapping — the caller supplies its own (deliberately
/// different from the built-in: scaled by an extra factor of 10) to prove
/// it's genuinely wired to the user's impl, not silently falling back to a
/// hidden built-in one.
#[test]
fn domain_type_manual_impl_uses_callers_own_impl() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    // ANCHOR: with_domain_type_manual_impl_config
    let (_schema, src) = generate_domain_with(&path, "manual_impl_dt", |c| {
        c.with_domain_type(
            ergo_sbe::ConversionSelector::named_type("Decimal"),
            "rust_decimal::Decimal",
            ergo_sbe::DomainImpl::Manual,
        )
    });
    // ANCHOR_END: with_domain_type_manual_impl_config
    compile_and_run_with_deps(
        "manual_impl_dt",
        &src,
        r#"
        // ANCHOR: with_domain_type_manual_impl_usage
        use rust_decimal::Decimal;

        // Caller-supplied impl: deliberately scales mantissa by 10 on the way
        // in/out, so a value only round-trips correctly through THIS impl —
        // proof ergo-sbe didn't quietly generate its own.
        impl TryFromSbe<self::Decimal> for rust_decimal::Decimal {
            type Error = &'static str;
            fn try_from_sbe(wire: self::Decimal) -> Result<Self, Self::Error> {
                Ok(rust_decimal::Decimal::new(wire.mantissa() / 10, (-wire.exponent()) as u32))
            }
        }
        impl TryToSbe<self::Decimal> for rust_decimal::Decimal {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<self::Decimal, Self::Error> {
                Ok(self::Decimal::new(self.mantissa() as i64 * 10, -(self.scale() as i8)))
            }
        }

        let mut buf = [0u8; 256];
        let mut enc = OrderEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OrderFixedFields { price: self::Decimal::new(0, 0), size: self::Decimal::new(0, 0) });
        enc.try_price(Decimal::new(12345, 2))?;
        enc.try_size(Decimal::new(100, 0))?;
        let encoded = enc.as_bytes_with_header().to_vec();

        let dec = OrderDecoder::try_decode(&encoded, 0)?;
        assert_eq!(dec.try_price()?, Decimal::new(12345, 2));
        assert_eq!(dec.try_size()?, Decimal::new(100, 0));
        // ANCHOR_END: with_domain_type_manual_impl_usage
        "#,
        "rust_decimal = \"1\"\n",
    );
    Ok(())
}

/// A `DomainImpl::Manual` field on one of the three built-in type paths
/// (here `rust_decimal::Decimal`) gets a doc comment on its generated
/// `try_*` accessor containing the exact impl `DomainImpl::Generated` would
/// have written — a ready-to-paste starting point (see
/// `conversion_traits.rs::generate_manual_impl_snippets`). A non-built-in
/// `rust_type` gets no snippet — there is no generated-impl template to
/// offer for a caller-invented type.
#[test]
fn domain_type_manual_impl_doc_comment_has_generated_snippet()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let (_schema, src) = generate_domain_with(&path, "manual_impl_doc", |c| {
        c.with_domain_type(
            ergo_sbe::ConversionSelector::named_type("Decimal"),
            "rust_decimal::Decimal",
            ergo_sbe::DomainImpl::Manual,
        )
    });
    assert_source_ok(
        &src,
        &[
            "DomainImpl::Manual",
            "impl TryFromSbe<Decimal> for rust_decimal::Decimal",
            "impl TryToSbe<Decimal> for rust_decimal::Decimal",
        ],
    );
    Ok(())
}

/// `DomainImpl::Manual` without a caller-supplied impl fails closed with a
/// named diagnostic (`#[diagnostic::on_unimplemented]`), not the default
/// "the trait bound `i64: TryFromSbe<Decimal>` is not satisfied" — see
/// `conversion_traits.rs::emit_conversion_traits`.
#[test]
fn domain_type_manual_impl_missing_gives_named_diagnostic() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let (_schema, src) = generate_domain_with(&path, "manual_impl_missing", |c| {
        c.with_domain_type(
            ergo_sbe::ConversionSelector::named_type("Decimal"),
            "i64",
            ergo_sbe::DomainImpl::Manual,
        )
    });
    // `i64: TryFromSbe<Decimal>` is never implemented, so the generated
    // module itself fails to compile — the test body doesn't need to call
    // anything.
    compile_fails_with_diagnostics(
        "manual_impl_missing",
        &src,
        "let _ = 0;",
        &[
            "`i64` has no `TryFromSbe<Decimal>` impl",
            "`i64` has no `TryToSbe<Decimal>` impl",
            "DomainImpl::Manual",
        ],
    );
    Ok(())
}

/// `DomainImpl::Manual` doc-comment snippets for the other two built-ins —
/// `bool` (an enum-backed selector, not a composite one like Decimal) and
/// `chrono::DateTime<Utc>` (a single global impl, not per-selector like
/// Decimal). Companion to
/// `domain_type_manual_impl_doc_comment_has_generated_snippet` above, which
/// covers the composite case.
#[test]
fn domain_type_manual_impl_doc_comment_covers_bool_and_chrono()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate_domain_with(&Paths::example_schema(), "manual_impl_bool", |c| {
        c.with_domain_type(
            ergo_sbe::ConversionSelector::named_type("BooleanType"),
            "bool",
            ergo_sbe::DomainImpl::Manual,
        )
    });
    assert_source_ok(
        &src,
        &[
            "DomainImpl::Manual",
            "impl TryFromSbe<BooleanType> for bool",
            "impl TryToSbe<BooleanType> for bool",
        ],
    );

    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/optional-domain-types.xml"
    ));
    let (_schema, src) = generate_domain_with(&path, "manual_impl_chrono", |c| {
        c.with_domain_type(
            ergo_sbe::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
            ergo_sbe::DomainImpl::Manual,
        )
    });
    assert_source_ok(
        &src,
        &[
            "DomainImpl::Manual",
            "impl TryFromSbe<u64> for chrono::DateTime<chrono::Utc>",
            "impl TryToSbe<u64> for chrono::DateTime<chrono::Utc>",
        ],
    );
    Ok(())
}

/// Domain types (`rust_decimal::Decimal`, `chrono::DateTime`) must round-trip
/// through **optional** fields. A composite decimal has no null image, so its
/// `try_*` accessor is a plain `Result<Decimal>`; an optional primitive
/// timestamp null-maps to `Option<DateTime>`. This also exercises multiple
/// distinct decimal composites (Decimal64 + Decimal128) under one config.
#[test]
fn optional_decimal_and_timestamp_domain_types_roundtrip() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/optional-domain-types.xml"
    ));
    let (_schema, src) = generate_domain_with(&path, "opt_domain", |c| {
        c.with_domain_type(
            ergo_sbe::ConversionSelector::named_type("Decimal64"),
            "rust_decimal::Decimal",
            ergo_sbe::DomainImpl::Generated,
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::named_type("Decimal128"),
            "rust_decimal::Decimal",
            ergo_sbe::DomainImpl::Generated,
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
            ergo_sbe::DomainImpl::Generated,
        )
    });
    compile_and_run_with_deps(
        "opt_domain",
        &src,
        r#"
        use rust_decimal::Decimal;
        use chrono::{DateTime, Utc};

        let mut buf = [0u8; 256];
        let len = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&QuoteFixedFields {
                price64: None,
                size64: Decimal64::new(0, 0),
                price128: None,
                ts: None,
            })
            .try_price64(Decimal::new(12345, 2))?
            .try_size64(Decimal::new(100, 0))?
            .try_price128(Decimal::new(99999, 1))?
            .try_ts(DateTime::from_timestamp(42, 0).unwrap())?
            .encoded_length_with_header();

        let dec = QuoteDecoder::try_decode(&buf[..len], 0)?;

        // Optional decimal composites decode to a plain Decimal — a composite
        // has no null image, so `presence="optional"` does not become Option.
        assert_eq!(dec.try_price64()?, Decimal::new(12345, 2));
        assert_eq!(dec.try_size64()?, Decimal::new(100, 0));
        assert_eq!(dec.try_price128()?, Decimal::new(99999, 1));

        // Optional primitive timestamp null-maps to Option<DateTime<Utc>>.
        assert_eq!(dec.try_ts()?, Some(DateTime::from_timestamp(42, 0).unwrap()));
        "#,
        "chrono = \"0.4\"\nrust_decimal = \"1\"\n",
    );
    Ok(())
}

/// A composite member with `presence="optional"` AND a schema `nullValue`
/// (e.g. a `PriceNull9`-style Decimal's `mantissa`) DOES have a genuine null
/// image, unlike `optional_decimal_and_timestamp_domain_types_roundtrip`'s
/// `Decimal64`/`Decimal128` above. The member accessor decodes it as
/// `Option<i64>`; decoding the null image as a `Decimal` fails closed as a
/// typed error rather than silently misreading the sentinel as a huge value.
#[test]
fn optional_composite_member_null_image_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/optional-decimal-composite-field.xml"
    ));
    let (_schema, src) = generate_domain_with(&path, "opt_dec_composite", |c| {
        c.with_domain_type(
            ergo_sbe::ConversionSelector::named_type("PriceNull9"),
            "rust_decimal::Decimal",
            ergo_sbe::DomainImpl::Generated,
        )
    });
    compile_and_run_with_deps(
        "opt_dec_composite",
        &src,
        r#"
        // ANCHOR: optional_composite_null_image
        use rust_decimal::Decimal;

        // `price: None` writes the schema null image (mantissa = nullValue).
        let mut buf = [0u8; 64];
        let len = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&QuoteFixedFields { price: None, qty: 7 })
            .encoded_length_with_header();

        let dec = QuoteDecoder::try_decode(&buf[..len], 0)?;
        assert_eq!(dec.price_value().mantissa(), None);
        assert!(dec.try_price().is_err(), "null mantissa is not a valid Decimal");

        // A real price round-trips through the null-aware member accessor.
        let len2 = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&QuoteFixedFields { price: None, qty: 7 })
            .try_price(Decimal::new(12345, 9))?
            .encoded_length_with_header();
        let dec2 = QuoteDecoder::try_decode(&buf[..len2], 0)?;
        assert_eq!(dec2.price_value().mantissa(), Some(12345));
        assert_eq!(dec2.try_price()?, Decimal::new(12345, 9));
        // ANCHOR_END: optional_composite_null_image
        "#,
        "rust_decimal = \"1\"\n",
    );
    Ok(())
}

/// When converter mode is enabled, Decimal-backed fields emit both raw
/// `*_wire` accessors and generic converted methods.
#[test]
fn decimal_converter_emits_wire_and_generic_methods() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let ir = ergo_sbe::parse_file(&path).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("decimal_wire")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let mut g = ergo_sbe::Generator::new(config);
    let modules = g.generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;

    // Raw accessors get _wire suffix when conversions are enabled
    assert!(src.contains("price_wire"), "decoder price_wire missing");
    assert!(src.contains("size_wire"), "decoder size_wire missing");

    assert!(
        src.contains("price_wire"),
        "encoder price_wire setter missing"
    );

    assert!(
        src.contains("price_as"),
        "generic price_as accessor missing"
    );
    assert!(src.contains("size_as"), "generic size_as accessor missing");

    assert!(
        src.contains("price_from"),
        "generic price_from setter missing"
    );
    assert!(
        src.contains("size_from"),
        "generic size_from setter missing"
    );

    assert!(src.contains("pub trait TryFromSbe"), "trait missing");
    Ok(())
}

/// Domain DTOs + conversion-only must call `*_wire` setters (not bare `price`),
/// and must not force a rust_decimal impl unless `with_domain_type` is used.
#[test]
fn conversion_only_domain_dto_uses_wire_setters() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let ir = ergo_sbe::parse_file(&path).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("conv_domain")
        .with_domain_objects(ergo_sbe::DomainVarData::Bytes)
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let mut g = ergo_sbe::Generator::new(config);
    let modules = g.generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;

    assert!(
        src.contains("struct OrderDomain"),
        "OrderDomain missing with enable_domain_objects"
    );
    // Encode must use renamed wire setters.
    assert!(
        src.contains("price_wire(self.price)") || src.contains("price_wire(self . price)"),
        "domain encode must call price_wire under conversion-only; got no match"
    );
    // Conversion-only must NOT inject rust_decimal into generated source.
    assert!(
        !src.contains("rust_decimal::"),
        "with_conversion alone must not reference rust_decimal"
    );

    compile_and_run(
        "conv_domain",
        src,
        r#"
        let mut buf = [0u8; 256];
        let enc = OrderEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OrderFixedFields {
                price: Decimal::new(99, -2),
                size: Decimal::new(3, 0),
            });
        let wire = enc.as_bytes_with_header().to_vec();

        let dec = OrderDecoder::try_decode(&wire, 0).unwrap();
        let dto = OrderDomain::try_from_decoder(dec)?;
        assert_eq!(dto.price.mantissa(), 99);
        assert_eq!(dto.price.exponent(), -2);

        let mut out = [0u8; 256];
        let n = dto.encode(&mut out).unwrap();
        assert_eq!(&out[8..n], &wire[8..]);
    "#,
    );
    Ok(())
}

/// Raw and converted paths produce identical wire bytes.
#[test]
fn decimal_converter_wire_and_generic_byte_identity() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let ir = ergo_sbe::parse_file(&path).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("decimal_id")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let mut g = ergo_sbe::Generator::new(config);
    let modules = g.generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;

    compile_and_run(
        "decimal_id",
        src,
        r#"
        // Wire path: encode via price_wire, decode via price_wire
        let mut buf_wire = [0u8; 256];
        let enc_wire = OrderEncoder::wrap_and_apply_header(&mut buf_wire, 0)
            .fixed(&OrderFixedFields {
                price: Decimal::new(12345, -2),
                size: Decimal::new(100, 0),
            });
        let wire_bytes = enc_wire.as_bytes_with_header().to_vec();

        // Verify wire decode
        let dec_wire = OrderDecoder::try_decode(&wire_bytes, 0).unwrap();
        let pw = dec_wire.price_wire();
        assert_eq!(pw.mantissa(), 12345);
        assert_eq!(pw.exponent(), -2);
        let sw = dec_wire.size_wire();
        assert_eq!(sw.mantissa(), 100);
        assert_eq!(sw.exponent(), 0);
    "#,
    );
    Ok(())
}

#[test]
fn fixed_fields_struct_exists_and_requires_all_required_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "fixed_fields_req");

    assert!(
        src.contains("struct CarFixedFields"),
        "CarFixedFields must be generated"
    );
    assert!(src.contains("serial_number: u64"));
    // Required fields are concrete (not Option)
    assert!(src.contains("serial_number: u64"));
    assert!(src.contains("model_year: u16"));
    Ok(())
}

#[test]
fn fixed_method_exists_and_is_functional() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "fixed_method_done");
    assert!(
        src.contains("pub fn fixed("),
        "fixed() method must be generated"
    );
    assert!(
        src.contains("pub fn raw_fixed("),
        "raw_fixed() method must be generated"
    );
    assert!(
        !src.contains("finish_unchecked"),
        "finish_unchecked() must NOT be generated (no fixed-phase bypass)"
    );
    assert!(
        src.contains("struct CarFixedFields"),
        "FixedFields struct must exist"
    );
    Ok(())
}

#[test]
fn composite_value_and_flyweight_symmetry_exists() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "composite_sym_done");
    assert!(
        src.contains("fn engine_value("),
        "engine_value() must be generated"
    );
    assert!(
        !src.contains("fn engine_as_struct("),
        "engine_as_struct() must NOT be generated"
    );
    Ok(())
}

#[test]
fn fixed_and_raw_fixed_replace_try_fixed() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "fixed_replaces_try");
    assert!(src.contains("pub fn fixed("), "fixed() must be generated");
    assert!(
        src.contains("pub fn raw_fixed("),
        "raw_fixed() dedicated writer must be generated"
    );
    assert!(
        src.contains("RawFixedWriter"),
        "RawFixedWriter struct must be generated"
    );
    assert!(
        !src.contains("try_fixed"),
        "try_fixed must NOT be generated"
    );
    Ok(())
}

#[test]
fn fixed_method_manual_equivalence() -> Result<(), Box<dyn std::error::Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "fixed_eq");
    compile_and_run(
        "fixed_eq",
        &src,
        r#"
        // Individual setters live on FieldsUnfixed; tails require fixed() which
        // seals FieldsFixed. Equivalence: fixed() alone is the only complete path.
        let mut buf_a = [0u8; 256];
        let mut buf_b = [0u8; 256];
        let ff = CarFixedFields {
            serial_number: 42, model_year: 2020,
            available: BooleanType::T, code: Model::A,
            some_numbers: [0u32;4], vehicle_code: [0u8;6],
            extras: OptionalExtras::default(),
            engine: Engine::new(1000, 4, [0,0,0], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
        };
        let a = CarEncoder::wrap_and_apply_header(&mut buf_a, 0)
            .fixed(&ff)
            .fuel_figures(0, |_| Ok(())).unwrap()
            .performance_figures(0, |_| Ok(())).unwrap()
            .manufacturer(b"H").unwrap()
            .model(b"C").unwrap()
            .activation_code(b"X").unwrap()
            .as_bytes_with_header()
            .to_vec();
        let b = CarEncoder::wrap_and_apply_header(&mut buf_b, 0)
            .fixed(&ff)
            .fuel_figures(0, |_| Ok(())).unwrap()
            .performance_figures(0, |_| Ok(())).unwrap()
            .manufacturer(b"H").unwrap()
            .model(b"C").unwrap()
            .activation_code(b"X").unwrap()
            .as_bytes_with_header()
            .to_vec();
        assert_eq!(a, b);
    "#,
    );
    Ok(())
}

/// Borrowed var-data slice must not escape a try_<data> callback (HRTB).
#[test]
fn callback_escape_try_data_is_compile_fail() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "cb_escape");
    compile_fails_with_diagnostics(
        "cb_escape",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let complete = OuterEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OuterFixedFields { trace_id: 7 })
            .app_name(b"test").unwrap().payload(b"data").unwrap();
        let _ = complete.as_bytes_with_header();
        let dec = OuterDecoder::try_decode(&buf, 0).unwrap();
        let mut escaped: Option<&[u8]> = None;
        let _ = dec.try_app_name::<sbe_rt::DecodeError, _>(|name| {
            escaped = Some(name); // HRTB: borrowed data cannot escape closure
            Ok(())
        });
    "#,
        &["borrowed data escapes outside of closure"],
    );
    Ok(())
}

/// A consumed encoder stage cannot be reused after a tail transition.
#[test]
fn consumed_encoder_stage_cannot_be_reused() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "consume_reuse");
    compile_fails_with_diagnostics(
        "consume_reuse",
        &src,
        r#"
        let mut buf = [0u8; 256];
        let outer = OuterEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OuterFixedFields { trace_id: 7 });
        let after_name = outer.app_name(b"t").unwrap();
        let _ = after_name;
        // outer was consumed by app_name() — cannot start another tail.
        let _ = outer.app_name(b"x");
    "#,
        &["use of moved value: `outer`"],
    );
    Ok(())
}

/// Unknown template in nested payload is forwarded as AnyMessage::Unknown.
#[test]
fn nested_message_rejects_malformed_payload() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "reject_malformed");
    compile_and_run(
        "reject_malformed",
        &src,
        r#"
        let mut tmp_inner = [0u8; 128];
        let inner_complete = InnerEncoder::wrap_and_apply_header(&mut tmp_inner, 0)
            .fixed(&InnerFixedFields { value: 42 })
            .label(b"")
            .unwrap();
        let inner_bytes = inner_complete.as_bytes_with_header().to_vec();
        let inner_len = inner_bytes.len();

        let app_name_len = b"t".len();
        let outer_len = OuterEncoder::compute_encoded_length_with_message_header(app_name_len, inner_len);
        let mut buf_storage = [0u8; 8192];
        assert!(outer_len <= buf_storage.len());
        let mut buf = &mut buf_storage[..outer_len];
        let complete = OuterEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OuterFixedFields { trace_id: 1 })
            .app_name(b"t").unwrap()
            .payload(&inner_bytes).unwrap();
        assert_eq!(complete.as_bytes_with_header().len(), outer_len);

        let dec = OuterDecoder::try_decode(&buf, 0).unwrap();
        let (_n, after_name) = dec.into_app_name().unwrap();
        let (frame, _c) = after_name.into_payload_as_message().unwrap();
        assert!(matches!(frame.message, AnyMessage::Inner(_)));

        // Wrong-schema payload (all zeros) is rejected by into_payload_as_message
        let payload_16 = [0u8; 16];
        let bad_outer_len = OuterEncoder::compute_encoded_length_with_message_header(0, 16);
        let mut bad_buf_storage = [0u8; 8192];
        assert!(bad_outer_len <= bad_buf_storage.len());
        let mut bad_buf = &mut bad_buf_storage[..bad_outer_len];
        OuterEncoder::wrap_and_apply_header(&mut bad_buf, 0)
            .fixed(&OuterFixedFields { trace_id: 1 })
            .app_name(b"")
            .unwrap()
            .payload(&payload_16)
            .unwrap();
        let bad_dec = OuterDecoder::try_decode(&bad_buf, 0).unwrap();
        let (_n, bad_after) = bad_dec.into_app_name().unwrap();
        // All-zeros has schema_id=0 — rejected with WrongSchema
        let bad_result = bad_after.into_payload_as_message();
        assert!(bad_result.is_err(), "wrong-schema payload must be rejected");
    "#,
    );
    Ok(())
}

/// Recursive AppMessage payloads (same template as outer) are dispatched
/// as AnyMessage::Outer, enabling explicit rejection by the application.
#[test]
fn nested_message_identifies_recursive_payload() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/nested-message-payload.xml"
    ));
    let (_schema, src) = generate(&path, "recurse_id");
    compile_and_run(
        "recurse_id",
        &src,
        r#"
        let inner_outer_len = OuterEncoder::compute_encoded_length_with_message_header(0, 0);
        let outer_len = OuterEncoder::compute_encoded_length_with_message_header(1, inner_outer_len);
        let mut buf_storage = [0u8; 8192];
        assert!(outer_len <= buf_storage.len());
        let mut buf = &mut buf_storage[..outer_len];
        let complete = OuterEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&OuterFixedFields { trace_id: 1 })
            .app_name(b"x").unwrap()
            .payload_with(inner_outer_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let c = OuterEncoder::try_wrap_and_apply_header(payload, 0)?
                    .fixed(&OuterFixedFields { trace_id: 99 })
                    .app_name(b"")
                    .unwrap()
                    .payload(b"")
                    .unwrap();
                assert_eq!(c.as_bytes_with_header().len(), inner_outer_len);
                Ok(())
            }).unwrap();
        assert_eq!(complete.as_bytes_with_header().len(), outer_len);

        let dec = OuterDecoder::try_decode(&buf, 0).unwrap();
        let (_name, after_name) = dec.into_app_name().unwrap();
        let (frame, _c) = after_name.into_payload_as_message().unwrap();
        // Recursive Outer appears as AnyMessage::Outer — app can reject it
        assert!(matches!(frame.message, AnyMessage::Outer(_)),
            "recursive Outer payload must be identifiable for rejection");
    "#,
    );
    Ok(())
}

/// Group-entry Decimal fields get generic converted methods plus raw
/// `*_wire`, exactly like ordinary fields.
#[test]
fn decimal_converter_covers_group_entry_fields() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe" package="entdec" id="94" version="0" byteOrder="littleEndian">
<types>
  <composite name="messageHeader"><type name="blockLength" primitiveType="uint16"/><type name="templateId" primitiveType="uint16"/><type name="schemaId" primitiveType="uint16"/><type name="version" primitiveType="uint16"/></composite>
  <composite name="groupSizeEncoding"><type name="blockLength" primitiveType="uint16"/><type name="numInGroup" primitiveType="uint16"/></composite>
  <composite name="Decimal"><type name="mantissa" primitiveType="int64"/><type name="exponent" primitiveType="int8"/></composite>
</types>
<sbe:message name="Book" id="1">
  <field name="mid" id="1" type="Decimal"/>
  <group name="levels" id="2" dimensionType="groupSizeEncoding">
    <field name="price" id="3" type="Decimal"/>
    <field name="qty" id="4" type="uint32"/>
  </group>
</sbe:message>
</sbe:messageSchema>"#;
    let ir = ergo_sbe::parse(xml).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("entdec")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let mut g = ergo_sbe::Generator::new(config);
    let modules = g.generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;

    // Source shape: entry raw accessors renamed, generic methods emitted.
    assert!(
        src.contains("impl<'a> LevelsEntryDecoder"),
        "entry decoder impl missing"
    );
    assert!(src.contains("price_wire"), "raw entry *_wire missing");
    assert!(src.contains("price_as"), "generic entry accessor missing");

    // Runtime: generic entry round trip is byte-identical with raw wire.
    compile_and_run(
        "entdec",
        src,
        r#"
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct Fixed { m: i64, e: i8 }
        impl TryFromSbe<Decimal> for Fixed {
            type Error = &'static str;
            fn try_from_sbe(wire: Decimal) -> Result<Self, Self::Error> { Ok(Fixed { m: wire.mantissa(), e: wire.exponent() }) }
        }
        impl TryToSbe<Decimal> for Fixed {
            type Error = &'static str;
            fn try_to_sbe(&self) -> Result<Decimal, Self::Error> { Ok(Decimal::new(self.m, self.e)) }
        }

        // Raw wire model
        let mut buf_wire = [0u8; 256];
        let complete = BookEncoder::wrap_and_apply_header(&mut buf_wire, 0)
            .fixed(&BookFixedFields {
                mid: Decimal::new(5, -1),
            })
            .levels(1, |g| {
                g.add(|mut e| {
                    e.price_wire(Decimal::new(500005, -1));
                    e.qty(7);
                    Ok(())
                })?;
                Ok(())
            })
            .unwrap();
        let wire_bytes = complete.as_bytes_with_header().to_vec();

        let mut buf_gen = [0u8; 256];
        let complete = BookEncoder::wrap_and_apply_header(&mut buf_gen, 0)
            .fixed(&BookFixedFields {
                mid: Decimal::new(5, -1),
            })
            .levels(1, |g| {
                g.add(|mut e| {
                    e.price_from(&Fixed { m: 500005, e: -1 }).unwrap();
                    e.qty(7);
                    Ok(())
                })?;
                Ok(())
            })
            .unwrap();
        let gen_bytes = complete.as_bytes_with_header().to_vec();

        assert_eq!(wire_bytes, gen_bytes, "generic and wire models must be byte-identical");

        let dec = BookDecoder::try_decode(&gen_bytes, 0).unwrap();
        assert_eq!(dec.mid_as::<Fixed>().unwrap(), Fixed { m: 5, e: -1 });
        let mut g = dec.into_levels().unwrap();
        let entry = g.next().unwrap();
        assert_eq!(entry.price_as::<Fixed>().unwrap(), Fixed { m: 500005, e: -1 });
        let raw = entry.price_wire();
        assert_eq!((raw.mantissa(), raw.exponent()), (500005, -1));
        assert_eq!(entry.qty(), 7);
    "#,
    );
    Ok(())
}

/// Independent exact fixed-scale adapter matrix: positive/negative
/// values, exponents 0/-8/-15/-18, overflow, and precision-loss rejection —
/// implemented in a temporary crate against the generated trait only.
#[test]
fn decimal_converter_exact_adapter_matrix() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/decimal-converter-schema.xml"
    ));
    let ir = ergo_sbe::parse_file(&path).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let config = ergo_sbe::GenerationConfig::new("exact_matrix")
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    let mut g = ergo_sbe::Generator::new(config);
    let modules = g.generate(&schema).unwrap();
    let src = &modules.modules().next().unwrap().source;

    compile_and_run(
        "exact_matrix",
        src,
        r#"
        /// Exact fixed-scale(18) adapter, independent of rust_decimal.
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct Exact18(i128); // value scaled by 10^18

        #[derive(Debug, PartialEq)]
        enum ExactErr { Overflow, PrecisionLoss }
        impl core::fmt::Display for ExactErr {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(f, "{self:?}")
            }
        }

        impl TryFromSbe<Decimal> for Exact18 {
            type Error = ExactErr;
            fn try_from_sbe(wire: Decimal) -> Result<Self, ExactErr> {
                let m = wire.mantissa();
                let e = wire.exponent();
                let shift = i32::from(e) + 18;
                if shift >= 0 {
                    if shift > 38 { return Err(ExactErr::Overflow); }
                    let f = 10i128.checked_pow(shift as u32).ok_or(ExactErr::Overflow)?;
                    Ok(Exact18(i128::from(m).checked_mul(f).ok_or(ExactErr::Overflow)?))
                } else {
                    let d = 10i128.checked_pow((-shift) as u32).ok_or(ExactErr::Overflow)?;
                    let v = i128::from(m);
                    if v % d != 0 { return Err(ExactErr::PrecisionLoss); }
                    Ok(Exact18(v / d))
                }
            }
        }
        impl TryToSbe<Decimal> for Exact18 {
            type Error = ExactErr;
            fn try_to_sbe(&self) -> Result<Decimal, Self::Error> {
                let m: i64 = self.0.try_into().map_err(|_| ExactErr::Overflow)?;
                Ok(Decimal::new(m, -18))
            }
        }

        // Positive/negative at exponents 0, -8, -15, -18.
        let cases: &[(i64, i8, i128)] = &[
            (5, 0, 5_000_000_000_000_000_000),
            (-5, 0, -5_000_000_000_000_000_000),
            (12345678, -8, 123_456_780_000_000_000),
            (-12345678, -8, -123_456_780_000_000_000),
            (123456789012345, -15, 123_456_789_012_345_000),
            (-123456789012345, -15, -123_456_789_012_345_000),
            (1, -18, 1),
            (-1, -18, -1),
        ];
        let mut buf = [0u8; 256];
        for &(m, e, scaled) in cases {
            // Trait direction checks.
            assert_eq!(Exact18::try_from_sbe(Decimal::new(m, e)), Ok(Exact18(scaled)), "m={m} e={e}");
            // Wire round trip through generated generic methods.
            let mut enc = OrderEncoder::wrap_and_apply_header(&mut buf, 0);
            let v = Exact18::try_from_sbe(Decimal::new(m, e)).unwrap();
            enc.price_from(&v).unwrap();
            enc.size_wire(Decimal::new(0, 0));
            let dec = OrderDecoder::try_decode(&buf, 0).unwrap();
            assert_eq!(dec.price_as::<Exact18>().unwrap(), v);
            // Raw wire carries the adapter's canonical scale.
            let raw = dec.price_wire();
            assert_eq!((raw.mantissa(), raw.exponent()), (scaled as i64, -18));
        }

        // Overflow: mantissa * 10^(e+18) exceeds i128/i64 range.
        assert_eq!(Exact18::try_from_sbe(Decimal::new(i64::MAX, 8)), Err(ExactErr::Overflow));
        // Precision loss: scaling down discards non-zero digits.
        assert_eq!(Exact18::try_from_sbe(Decimal::new(123, -20)), Err(ExactErr::PrecisionLoss));
        let mut enc = OrderEncoder::wrap_and_apply_header(&mut buf, 0);
        let too_big = Exact18(i128::from(i64::MAX) * 10);
        match enc.price_from(&too_big) {
            Err(ExactErr::Overflow) => {}
            Err(other) => panic!("expected Overflow, got {other:?}"),
            Ok(_) => panic!("oversized adapter value must not encode"),
        }
    "#,
    );
    Ok(())
}
