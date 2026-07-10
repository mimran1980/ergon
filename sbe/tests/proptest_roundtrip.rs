//! Property-based fuzz round-trip tests for the generated Car SBE codec.
//!
//! Strategy: generate random schematically-valid field values, encode with the
//! generated encoder, decode with the generated decoder, assert every field
//! round-trips to the logical value that was encoded.
//!
//! Because the generated code exists only at test-run time (it is produced by
//! the ErgoSBE code generator at test time), the actual proptest evaluation
//! happens inside a temporary crate that depends on `proptest`.  This file
//! orchestrates the whole process: generate, patch, compile, run, clean up.

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, generate, patch_source};
use std::fs;
use std::process::Command;

// ── Helper: compile generated code and run proptest in a temp crate ─────

/// Like `compile_and_run` but creates a proper crate with `proptest` as a
/// dev-dependency so that the test code can use the `proptest!` macro.
///
/// `test_label`  — unique label for the temp directory (prevents parallel-test collisions).
/// `module_name` — the Rust module name for the generated types.
/// `source`      — the raw generated Rust source (will be patched).
/// `test_code`   — a complete `.rs` file placed in `tests/` of the temp crate.
fn compile_and_run_proptest(test_label: &str, module_name: &str, source: &str, test_code: &str) {
    let dir = std::env::temp_dir().join(format!("ergo_prop_{test_label}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Write the generated + patched module
    let patched = patch_source(source);
    let module_path = format!("{module_name}.rs");
    fs::write(src_dir.join(&module_path), &patched)
        .unwrap_or_else(|e| panic!("write {module_path}: {e}"));

    // lib.rs — re-export everything
    let lib = format!("mod {module_name};\npub use {module_name}::*;\n");
    fs::write(src_dir.join("lib.rs"), &lib).unwrap();

    // Test file (integration test, compiled with proptest dev-dep)
    let tests_dir = dir.join("tests");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(tests_dir.join("roundtrip.rs"), test_code).unwrap();

    // Cargo.toml – crate depends on the generated module (no extra deps);
    // proptest is only a dev-dep.
    let cargo_toml = format!(
        "[package]\n\
         name = \"prop_{module_name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         \n\
         [dev-dependencies]\n\
         proptest = \"1\"\n"
    );
    fs::write(dir.join("Cargo.toml"), &cargo_toml).unwrap();

    let target_dir = dir.join("target_ci");
    let out = Command::new("cargo")
        .args(["test"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .unwrap_or_else(|e| panic!("cargo test on temp crate {module_name}: {e}"));

    let _ = fs::remove_dir_all(&dir);

    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr);
        let o = String::from_utf8_lossy(&out.stdout);
        panic!(
            "proptest roundtrip '{}' FAILED\n--- stdout ---\n{o}\n--- stderr ---\n{e}",
            module_name
        );
    }
}

// ── Round-trip tests ───────────────────────────────────────────────────

/// Test 1 — random scalars + Engine composite.
///
/// Generates every scalar field (u64, u16, enums, fixed-size arrays, bit-set)
/// and the Engine composite, writes an otherwise-empty message, decodes, and
/// asserts field-by-field equality.
#[test]
fn roundtrip_scalar_and_engine() {
    let (_schema, src) = generate(&Paths::example_schema(), "car_example");

    let test_code = r##"
use prop_car_example::*;
use proptest::prelude::*;

fn arb_boolean_type() -> impl Strategy<Value = BooleanType> {
    prop_oneof![Just(BooleanType::F), Just(BooleanType::T)]
}

fn arb_model() -> impl Strategy<Value = Model> {
    prop_oneof![Just(Model::A), Just(Model::B), Just(Model::C)]
}

proptest! {
    #[test]
    fn roundtrip_scalars(
        serial_number in any::<u64>(),
        model_year in any::<u16>(),
        available in arb_boolean_type(),
        code in arb_model(),
        some_numbers in prop::array::uniform4(any::<u32>()),
        vehicle_code in prop::array::uniform6(any::<u8>()),
        extras_raw in any::<u8>(),
        capacity in any::<u16>(),
        num_cylinders in any::<u8>(),
        mc in prop::array::uniform3(any::<u8>()),
    ) {
        let extras = OptionalExtras::from(extras_raw);
        let engine = Engine::new(capacity, num_cylinders, mc);

        let mut buf = vec![0u8; 4096];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(serial_number);
        car.model_year(model_year);
        car.available(available);
        car.code(code);
        car.some_numbers(some_numbers);
        car.vehicle_code(vehicle_code);
        car.extras(extras);
        car.engine(engine);

        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();

        let encoded = car.as_bytes();
        let decoded = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();

        prop_assert_eq!(serial_number, decoded.serial_number());
        prop_assert_eq!(model_year, decoded.model_year());
        prop_assert_eq!(available, decoded.available());
        prop_assert_eq!(code, decoded.code());
        prop_assert_eq!(some_numbers, decoded.some_numbers());
        prop_assert_eq!(vehicle_code, decoded.vehicle_code());
        prop_assert_eq!(extras, decoded.extras());

        let de = decoded.engine();
        prop_assert_eq!(engine.capacity(), de.capacity());
        prop_assert_eq!(engine.num_cylinders(), de.num_cylinders());
        prop_assert_eq!(engine.manufacturer_code(), de.manufacturer_code());

        // Constant fields always round-trip
        prop_assert_eq!(9000_u16, de.max_rpm());
        prop_assert_eq!("Petrol", de.fuel());
        prop_assert_eq!(Model::C, decoded.discounted_model());
    }
}
"##;

    compile_and_run_proptest("scalar", "car_example", &src, test_code);
}

/// Test 2 — var-data (string) round-trip: manufacturer, model, activationCode.
#[test]
fn roundtrip_strings() {
    let (_schema, src) = generate(&Paths::example_schema(), "car_example");

    let test_code = r##"
use prop_car_example::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_strings(
        manufacturer in proptest::collection::vec(32u8..=126, 0..100),
        model in proptest::collection::vec(32u8..=126, 0..100),
        activation in proptest::collection::vec(32u8..=126, 0..100),
    ) {
        let mut buf = vec![0u8; 4096];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(0);
        car.model_year(2000);
        car.available(BooleanType::F);
        car.code(Model::A);
        car.some_numbers([0u32; 4]);
        car.vehicle_code([0u8; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(1000, 4, [0, 0, 0]));
        let car = car.fuel_figures(0, |_| {}).unwrap();
        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(&manufacturer).unwrap();
        let car = car.model(&model).unwrap();
        let car = car.activation_code(&activation).unwrap();

        let encoded = car.as_bytes();
        let decoded = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();
        let after_perf = decoded
            .into_fuel_figures()
            .unwrap()
            .finish()
            .unwrap()
            .into_performance_figures()
            .unwrap()
            .finish()
            .unwrap();
        let (dec_mfr, s1) = after_perf.into_manufacturer().unwrap();
        let (dec_model, s2) = s1.into_model().unwrap();
        let (dec_activation, _done) = s2.into_activation_code().unwrap();

        prop_assert_eq!(&manufacturer[..], dec_mfr);
        prop_assert_eq!(&model[..], dec_model);
        prop_assert_eq!(&activation[..], dec_activation);
    }
}
"##;

    compile_and_run_proptest("strings", "car_example", &src, test_code);
}

/// Test 3 — group round-trip: fuel figures with random entries.
///
/// Generates 0–8 entries with random speed, mpg, and usage description.
#[test]
fn roundtrip_groups() {
    let (_schema, src) = generate(&Paths::example_schema(), "car_example");

    let test_code = r##"
use prop_car_example::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_fuel_figures(
        entries in proptest::collection::vec(
            (any::<u16>(), any::<f32>(), proptest::collection::vec(32u8..=126, 0..80)),
            0..=8,
        ),
    ) {
        let mut buf = vec![0u8; 4096];
        let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
        car.serial_number(0);
        car.model_year(2000);
        car.available(BooleanType::F);
        car.code(Model::A);
        car.some_numbers([0u32; 4]);
        car.vehicle_code([0u8; 6]);
        car.extras(OptionalExtras::default());
        car.engine(Engine::new(1000, 4, [0, 0, 0]));

        let count = entries.len() as u16;
        let car = car.fuel_figures(count, |g| {
            for (speed, mpg, usage) in &entries {
                g.add(|e| {
                    e.speed(*speed);
                    e.mpg(*mpg);
                    e.usage_description(usage).unwrap();
                }).unwrap();
            }
        }).unwrap();

        let car = car.performance_figures(0, |_| {}).unwrap();
        let car = car.manufacturer(b"").unwrap();
        let car = car.model(b"").unwrap();
        let car = car.activation_code(b"").unwrap();

        let encoded = car.as_bytes();
        let decoded = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();

        let mut fuel_iter = decoded.into_fuel_figures().unwrap();
        let fuel: Vec<_> = fuel_iter
            .by_ref()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        prop_assert_eq!(entries.len(), fuel.len(), "fuel figures count");

        for (i, (speed, mpg, usage)) in entries.iter().enumerate() {
            prop_assert_eq!(*speed, fuel[i].speed(), "ff[{}].speed", i);
            let mpg_diff = (mpg - fuel[i].mpg()).abs();
            prop_assert!(mpg_diff < f32::EPSILON, "ff[{}].mpg diff={}", i, mpg_diff);
            prop_assert_eq!(&usage[..], fuel[i].usage_description().unwrap(), "ff[{}].usage", i);
        }
    }
}
"##;

    compile_and_run_proptest("groups", "car_example", &src, test_code);
}

/// Test 4 — zero-length edge cases: empty groups, empty var-data.
#[test]
fn roundtrip_zero_length() {
    let (_schema, src) = generate(&Paths::example_schema(), "car_example");

    let test_code = r##"
use prop_car_example::*;

#[test]
fn zero_length_roundtrip() {
    let mut buf = vec![0u8; 512];
    let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
    car.serial_number(0);
    car.model_year(0);
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
    let decoded = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();

    assert_eq!(0, decoded.serial_number());
    let fuel = decoded.into_fuel_figures().unwrap();
    assert!(fuel.is_empty(), "fuel figures not empty");
    let perf = fuel.finish().unwrap().into_performance_figures().unwrap();
    assert!(perf.is_empty(), "perf figures not empty");
    let after_perf = perf.finish().unwrap();
    let (mfr, a1) = after_perf.into_manufacturer().unwrap();
    assert_eq!(b"", mfr, "manufacturer");
    let (model, a2) = a1.into_model().unwrap();
    assert_eq!(b"", model, "model");
    let (activation, _done) = a2.into_activation_code().unwrap();
    assert_eq!(b"", activation, "activationCode");
}
"##;

    compile_and_run_proptest("zero_len", "car_example", &src, test_code);
}

/// Test 5 — boundary values: minimum and maximum integer values, all bits set.
#[test]
fn roundtrip_boundary_values() {
    let (_schema, src) = generate(&Paths::example_schema(), "car_example");

    let test_code = r##"
use prop_car_example::*;

#[test]
fn boundary_values() {
    let mut buf = vec![0u8; 512];
    let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
    car.serial_number(u64::MAX);
    car.model_year(u16::MAX);
    car.available(BooleanType::T);
    car.code(Model::C);
    car.some_numbers([u32::MAX, u32::MAX, u32::MAX, u32::MAX]);
    car.vehicle_code([u8::MAX; 6]);
    let mut extras = OptionalExtras::default();
    extras.set_sun_roof(true);
    extras.set_sports_pack(true);
    extras.set_cruise_control(true);
    car.extras(extras);
    car.engine(Engine::new(u16::MAX, u8::MAX, [u8::MAX; 3]));

    let car = car.fuel_figures(1, |g| {
        g.add(|e| { e.speed(u16::MAX); e.mpg(f32::MAX); e.usage_description(b"").unwrap(); }).unwrap();
    }).unwrap();
    let car = car.performance_figures(0, |_| {}).unwrap();
    let car = car.manufacturer(b"MAX").unwrap();
    let car = car.model(b"MAX").unwrap();
    let car = car.activation_code(b"MAX").unwrap();

    let encoded = car.as_bytes();
    let decoded = CarDecoder::wrap_and_apply_header(encoded, 0).unwrap();

    assert_eq!(u64::MAX, decoded.serial_number());
    assert_eq!(u16::MAX, decoded.model_year());
    assert_eq!(BooleanType::T, decoded.available());
    assert_eq!(Model::C, decoded.code());
    assert_eq!([u32::MAX; 4], decoded.some_numbers());
    assert_eq!([u8::MAX; 6], decoded.vehicle_code());
    let extras2 = decoded.extras();
    assert!(extras2.sun_roof());
    assert!(extras2.sports_pack());
    assert!(extras2.cruise_control());

    let de = decoded.engine();
    assert_eq!(u16::MAX, de.capacity());
    assert_eq!(u8::MAX, de.num_cylinders());

    let mut fuel_iter = decoded.into_fuel_figures().unwrap();
    let ff: Vec<_> = fuel_iter
        .by_ref()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(1, ff.len());
    assert_eq!(u16::MAX, ff[0].speed());
    // f32::MAX is the largest finite f32; check round-trip within epsilon
    assert!((f32::MAX - ff[0].mpg()).abs() < 1.0, "ff[0].mpg");

    let after_perf = fuel_iter
        .finish()
        .unwrap()
        .into_performance_figures()
        .unwrap()
        .finish()
        .unwrap();
    let (mfr, a1) = after_perf.into_manufacturer().unwrap();
    assert_eq!(b"MAX", mfr);
    let (model, a2) = a1.into_model().unwrap();
    assert_eq!(b"MAX", model);
    let (activation, _done) = a2.into_activation_code().unwrap();
    assert_eq!(b"MAX", activation);
}
"##;

    compile_and_run_proptest("boundary", "car_example", &src, test_code);
}
