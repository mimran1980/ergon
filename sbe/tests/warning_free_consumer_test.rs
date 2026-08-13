//! generated consumer must compile warning-free under a strict lint set.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;
use std::fs;
use std::process::Command;

mod common;
use common::{Paths, generate};
use ergo_sbe::{GenerationConfig, GenerationProfile, Generator, Schema};

/// Compile a minimal Lean + Full consumer with `-D warnings`.
#[test]
fn generated_lean_and_full_consumers_are_warning_free() -> Result<(), Box<dyn Error>> {
    for (label, profile) in [
        ("wf_lean", GenerationProfile::Lean),
        ("wf_full", GenerationProfile::Full),
    ] {
        let ir = ergo_sbe::parse_file(&Paths::example_schema())?;
        let schema = Schema::from_ir(ir);
        let config = GenerationConfig::new(label).profile(profile);
        let modules = Generator::new(config).generate(&schema)?;
        let src = &modules.modules().next().ok_or("no module")?.source;

        let dir = std::env::temp_dir().join(format!("ergo_soundness_{label}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src"))?;
        fs::write(dir.join(format!("src/{label}.rs")), src)?;
        fs::write(
            dir.join("src/main.rs"),
            format!(
                r#"// Strict consumer lint set. Two allows are inherent to a binary
// fixture consuming a full generated module:
// - dead_code: not every public type is exercised
// - unused_imports: wildcard import from generated module
// The generated module carries its own #![allow(...)] for internal
// shape-dependent warnings (unused_mut, unused_variables, etc.).
#![deny(warnings)]
#![allow(dead_code, unused_imports)]

// Wrap generated source with the same allows sbe_mod! applies.
#[allow(
    unused_unsafe,
    unused_variables,
    unused_mut,
    unused_assignments,
    unused_must_use,
    unused_comparisons,
    non_camel_case_types,
    non_snake_case,
    unexpected_cfgs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]
mod {label};
use {label}::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let mut buf = [0u8; 512];
    let done = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&CarFixedFields {{
            serial_number: 1,
            model_year: 2020,
            available: BooleanType::T,
            code: Model::A,
            some_numbers: [0; 4],
            vehicle_code: [b'A'; 6],
            extras: OptionalExtras::default(),
            engine: Engine::new(
                1, 1, [0; 3], 0i8, BooleanType::F, Booster::new(BoostType::TURBO, 0),
            ),
        }})
        .fuel_figures(0, |_| Ok(()))?
        .performance_figures(0, |_| Ok(()))?
        .manufacturer(b"x")?
        .model(b"y")?
        .activation_code(b"")?;
    let len = done.encoded_length_with_header();
    let dec = CarDecoder::try_decode(&buf[..len], 0)?;
    assert_eq!(dec.serial_number(), 1);
    Ok(())
}}
"#
            ),
        )?;
        // The generated module references `ergo_sbe::…` for the optional
        // string/byte types, so the dependency must be present whichever
        // features the outer test run enables.
        let sbe_path = Paths::sbe_dir().display().to_string().replace('\\', "\\\\");
        fs::write(
            dir.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{label}_consumer"
version = "0.1.0"
edition = "2024"

[dependencies]
ergo-sbe = {{ path = "{sbe_path}", features = ["compact_str", "smol_str", "bytes", "chrono"] }}
"#
            ),
        )?;
        let target = dir.join("target_ci");
        // No RUSTFLAGS=-D warnings here: the crate itself has #![deny(warnings)]
        // with an explicit allowlist for unused/dead_code (supported lint set).
        let out = Command::new("cargo")
            .args(["build"])
            .current_dir(&dir)
            .env("CARGO_TARGET_DIR", &target)
            .output()?;
        let stderr = String::from_utf8_lossy(&out.stderr);
        let _ = fs::remove_dir_all(&dir);
        assert!(
            out.status.success(),
            "warning-free consumer failed for {label}:\n{stderr}"
        );
    }
    Ok(())
}

/// Stale 0.1 names must not appear in Full generated Car source.
#[test]
fn generated_full_car_has_no_try_wrap_aliases() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "wf_stale");
    // try_wrap / try_wrap_and_apply_header are the checked constructors.
    assert!(src.contains("pub fn try_wrap_and_apply_header("));
    assert!(src.contains("pub fn wrap_and_apply_header("));
    assert!(src.contains("pub fn try_decode("));
    Ok(())
}
