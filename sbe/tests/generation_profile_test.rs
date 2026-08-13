//! GenerationProfile::Lean vs Full feature matrix + core consumer.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;

mod common;
use common::{Paths, compile_and_run};
use ergo_sbe::{GenerationConfig, GenerationProfile, Generator, Schema};

fn generate_with(
    path: &std::path::Path,
    module: &str,
    f: impl FnOnce(GenerationConfig) -> GenerationConfig,
) -> Result<String, Box<dyn Error>> {
    let ir = ergo_sbe::parse_file(path)?;
    let schema = Schema::from_ir(ir);
    let config = f(GenerationConfig::new(module));
    let modules = Generator::new(config).generate(&schema)?;
    Ok(modules.modules().next().ok_or("no module")?.source.clone())
}

#[test]
fn lean_omits_display_meta_and_dispatch() -> Result<(), Box<dyn Error>> {
    let lean = generate_with(&Paths::example_schema(), "prof_lean", |c| {
        c.profile(GenerationProfile::Lean)
    })?;
    assert!(
        !lean.contains("core::fmt::Display for CarDecoder"),
        "Lean must omit Display"
    );
    assert!(
        !lean.contains("enum AnyMessage"),
        "Lean must omit AnyMessage dispatch"
    );
    assert!(
        !lean.contains("fn serial_number_id") && !lean.contains("SERIAL_NUMBER_ID"),
        "Lean must omit field meta constants (id/offset)"
    );
    // Still has the hot path surface.
    assert!(lean.contains("pub fn wrap_and_apply_header"));
    assert!(lean.contains("pub fn decode"));
    assert!(lean.contains("pub struct CarEncoder"));
    Ok(())
}

#[test]
fn full_profile_includes_display_and_dispatch() -> Result<(), Box<dyn Error>> {
    let full = generate_with(&Paths::example_schema(), "prof_full", |c| {
        c.profile(GenerationProfile::Full)
    })?;
    assert!(
        full.contains("core::fmt::Display for CarDecoder")
            || full.contains("impl core::fmt::Display for CarDecoder"),
        "Full must emit Display for CarDecoder"
    );
    assert!(
        full.contains("enum AnyMessage") || full.contains("pub enum AnyMessage"),
        "Full must emit AnyMessage"
    );
    Ok(())
}

#[test]
fn lean_is_smaller_than_full() -> Result<(), Box<dyn Error>> {
    let lean = generate_with(&Paths::example_schema(), "prof_sz_lean", |c| {
        c.profile(GenerationProfile::Lean)
    })?;
    let full = generate_with(&Paths::example_schema(), "prof_sz_full", |c| {
        c.profile(GenerationProfile::Full)
    })?;
    assert!(
        lean.len() < full.len(),
        "lean {} should be < full {}",
        lean.len(),
        full.len()
    );
    // Document budget: lean should cut a meaningful slice of Full.
    let ratio = lean.len() as f64 / full.len() as f64;
    assert!(
        ratio < 0.95,
        "expected Lean < 95% of Full size, got ratio {ratio:.3}"
    );
    Ok(())
}

/// Core-only consumer: Lean Car encodes/decodes without Display/dispatch.
#[test]
fn lean_core_consumer_roundtrip() -> Result<(), Box<dyn Error>> {
    let src = generate_with(&Paths::example_schema(), "prof_core", |c| {
        c.profile(GenerationProfile::Lean)
    })?;
    compile_and_run(
        "prof_core",
        &src,
        r#"
        let mut buf = [0u8; 512];
        let done = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .fixed(&CarFixedFields {
                serial_number: 42,
                model_year: 2018,
                available: BooleanType::T,
                code: Model::A,
                some_numbers: [1, 2, 3, 4],
                vehicle_code: [b'A'; 6],
                extras: OptionalExtras::default(),
                engine: Engine::new(
                    2000, 4, [b'1', b'2', b'3'], 0i8, BooleanType::F,
                    Booster::new(BoostType::TURBO, 0),
                ),
            })
            .fuel_figures(0, |_| Ok(()))
            .unwrap()
            .performance_figures(0, |_| Ok(()))
            .unwrap()
            .manufacturer(b"Acme")
            .unwrap()
            .model(b"X")
            .unwrap()
            .activation_code(b"")
            .unwrap();
        let len = done.encoded_length_with_header();
        let dec = CarDecoder::try_decode(&buf[..len], 0).unwrap();
        assert_eq!(dec.serial_number(), 42);
        assert_eq!(dec.model_year(), 2018);
        // AnyMessage must not exist in Lean — ensure we didn't need it.
    "#,
    );
    Ok(())
}

/// Profile + knob override: Lean then re-enable dispatch only.
#[test]
fn profile_then_override_dispatch() -> Result<(), Box<dyn Error>> {
    let src = generate_with(&Paths::example_schema(), "prof_ov", |c| {
        c.profile(GenerationProfile::Lean).with_dispatch(true)
    })?;
    assert!(
        src.contains("enum AnyMessage") || src.contains("pub enum AnyMessage"),
        "with_dispatch(true) after Lean must restore AnyMessage"
    );
    assert!(
        !src.contains("core::fmt::Display for CarDecoder")
            && !src.contains("impl core::fmt::Display for CarDecoder"),
        "Display must stay off when only dispatch is re-enabled"
    );
    Ok(())
}
