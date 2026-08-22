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
    let (modules, _warnings) = Generator::new(config).generate(&schema)?.into_parts();
    Ok(modules.into_iter().next().ok_or("no module")?.source)
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

// ── Config surface that had no callers anywhere ───────────────────────────
//
// The four APIs below were public but exercised by nothing: `lean`,
// `with_module_name`, `with_error_from_impls`, and
// `ConversionSelector::field_path`. Untested public API is how the coverage
// ratchet drifted below its baseline; these assert behaviour, not mere calls.

/// `lean()` is shorthand for `new(..).profile(Lean)` — it must produce the same
/// source as spelling the profile out.
#[test]
fn lean_shorthand_matches_explicit_lean_profile() -> Result<(), Box<dyn Error>> {
    let shorthand = generate_with(&Paths::example_schema(), "cfg_lean_a", |_| {
        GenerationConfig::lean("cfg_lean_a")
    })?;
    let explicit = generate_with(&Paths::example_schema(), "cfg_lean_a", |c| {
        c.profile(GenerationProfile::Lean)
    })?;
    assert_eq!(
        shorthand, explicit,
        "lean() must be exactly new(..).profile(Lean)"
    );
    Ok(())
}

/// `with_module_name` renames the emitted module.
#[test]
fn with_module_name_renames_the_generated_module() -> Result<(), Box<dyn Error>> {
    let src = generate_with(&Paths::example_schema(), "cfg_before", |c| {
        c.with_module_name("cfg_renamed")
    })?;
    // The module name reaches generated output via the sbe_rt path/doc header.
    assert!(
        !src.is_empty(),
        "renamed module must still generate a source module"
    );
    let ir = ergo_sbe::parse_file(&Paths::example_schema())?;
    let schema = Schema::from_ir(ir);
    let modules =
        Generator::new(GenerationConfig::new("cfg_before").with_module_name("cfg_renamed"))
            .generate(&schema)?;
    let m = modules.modules().next().ok_or("no module")?;
    assert_eq!(
        m.path, "cfg_renamed.rs",
        "with_module_name must win over the name passed to new()"
    );
    Ok(())
}

/// `with_error_from_impls` emits `From<EncodeError>` / `From<DecodeError>` for
/// the caller's error type.
#[test]
#[allow(deprecated)]
fn with_error_from_impls_emits_conversions() -> Result<(), Box<dyn Error>> {
    let src = generate_with(&Paths::example_schema(), "cfg_errfrom", |c| {
        c.with_error_from_impls("crate::MyError")
    })?;
    assert!(
        src.contains("impl From<sbe_rt::EncodeError> for crate::MyError"),
        "encode error conversion must be generated"
    );
    assert!(
        src.contains("impl From<sbe_rt::DecodeError> for crate::MyError"),
        "decode error conversion must be generated"
    );
    Ok(())
}

/// A malformed error path is rejected at generation time rather than emitting
/// source that cannot compile.
#[test]
#[allow(deprecated)]
fn with_error_from_impls_rejects_a_non_type_path() -> Result<(), Box<dyn Error>> {
    let ir = ergo_sbe::parse_file(&Paths::example_schema())?;
    let schema = Schema::from_ir(ir);
    let err = Generator::new(
        GenerationConfig::new("cfg_errbad").with_error_from_impls("not a rust type!"),
    )
    .generate(&schema)
    .expect_err("a non-type error path must fail generation");
    let msg = err.to_string();
    assert!(
        msg.contains("error-from path") || msg.contains("error_from_path"),
        "error must name the offending option, got: {msg}"
    );
    Ok(())
}

/// Field-preserving `From<EncodeError>` / `From<DecodeError>` compiles against
/// the generated `sbe_rt` types (the 1.0 replacement for the lossy bridge).
#[test]
fn typed_error_from_impls_preserve_buffer_fields() -> Result<(), Box<dyn Error>> {
    let src = generate_with(&Paths::example_schema(), "cfg_errtyped", |c| c)?;
    let src = format!(
        "{src}\n\
         #[derive(Debug)]\n\
         pub enum AppError {{\n\
             Encode(sbe_rt::EncodeError),\n\
             Decode(sbe_rt::DecodeError),\n\
         }}\n\
         impl From<sbe_rt::EncodeError> for AppError {{\n\
             fn from(error: sbe_rt::EncodeError) -> Self {{\n\
                 Self::Encode(error)\n\
             }}\n\
         }}\n\
         impl From<sbe_rt::DecodeError> for AppError {{\n\
             fn from(error: sbe_rt::DecodeError) -> Self {{\n\
                 Self::Decode(error)\n\
             }}\n\
         }}\n"
    );
    compile_and_run(
        "cfg_errtyped",
        &src,
        r#"
        use cfg_errtyped::sbe_rt;
        let err: AppError = sbe_rt::EncodeError::BufferTooShort {
            field: "seq",
            needed: 8,
            available: 2,
        }
        .into();
        match err {
            AppError::Encode(sbe_rt::EncodeError::BufferTooShort { needed, available, .. }) => {
                assert_eq!(needed, 8);
                assert_eq!(available, 2);
            }
            other => panic!("fields lost: {other:?}"),
        }
        "#,
    );
    Ok(())
}

/// Callers that `#![deny(deprecated)]` must migrate off `with_error_from_impls`.
#[test]
fn with_error_from_impls_deprecation_fires() -> Result<(), Box<dyn Error>> {
    use std::fs;
    use std::process::Command;

    let sbe_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!(
        "ergo_sbe_errfrom_deprecated_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src"))?;
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "errfrom_deprecated"
version = "0.1.0"
edition = "2024"

[dependencies]
ergo-sbe = {{ path = "{}" }}
"#,
            sbe_dir.display()
        ),
    )?;
    fs::write(
        dir.join("src/main.rs"),
        r#"#![deny(deprecated)]
fn main() {
    let _ = ergo_sbe::GenerationConfig::new("x").with_error_from_impls("crate::E");
}
"#,
    )?;
    let out = Command::new("cargo")
        .args(["build", "--offline"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", dir.join("target_ci"))
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        !out.status.success(),
        "deny(deprecated) must fail, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("deprecated") && stderr.contains("with_error_from_impls"),
        "expected deprecation diagnostic, stderr:\n{stderr}"
    );
    Ok(())
}

/// `ConversionSelector::field_path` builds the `FieldPath` variant.
#[test]
fn conversion_selector_field_path_constructor() {
    use ergo_sbe::ConversionSelector;
    assert_eq!(
        ConversionSelector::field_path("Car.price"),
        ConversionSelector::FieldPath("Car.price".to_string()),
        "field_path must construct the FieldPath variant"
    );
}

#[test]
fn changelog_names_manual_domain_mapping() {
    let log = include_str!("../../CHANGELOG.md");
    assert!(
        log.contains("with_manual_domain_type"),
        "CHANGELOG must record additive Manual domain mapping"
    );
}

#[test]
fn two_argument_domain_mapping_is_repeatable() -> Result<(), Box<dyn Error>> {
    use ergo_sbe::ConversionSelector;
    let src_a = generate_with(&Paths::example_schema(), "dom_a", |c| {
        c.with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
    })?;
    let src_b = generate_with(&Paths::example_schema(), "dom_a", |c| {
        c.with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
    })?;
    assert_eq!(
        src_a, src_b,
        "two-argument with_domain_type must be deterministic"
    );
    Ok(())
}
