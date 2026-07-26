//! Helpers for Cargo `build.rs` scripts.
//!
//! Prefer these over hand-rolling parse → generate → write → `rerun-if-changed`.
//!
//! ```rust,ignore
//! // build.rs
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     ergo_sbe::generate_to_out_dir(
//!         "schemas/messages.xml",
//!         ergo_sbe::GenerationConfig::new("messages"),
//!     )?;
//!     Ok(())
//! }
//!
//! // lib.rs / main.rs
//! ergo_sbe::sbe_mod!(messages);
//! use messages::*;
//! ```

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::{GenerateError, GeneratedModuleSet, Generator};
use crate::config::GenerationConfig;
use crate::schema::Schema;
use crate::xml::{ParseError, parse, parse_file};

/// Errors from [`generate_to_out_dir`] / [`generate_str_to_out_dir`].
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Schema XML could not be parsed or resolved.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Code generation failed (e.g. invalid conversion config).
    #[error(transparent)]
    Generate(#[from] GenerateError),
    /// `OUT_DIR` is unset — this helper is meant for Cargo `build.rs` only.
    #[error("OUT_DIR is not set (run from a Cargo build.rs script)")]
    MissingOutDir,
    /// Failed to write a generated file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Generator produced no modules.
    #[error("schema generated no modules")]
    Empty,
}

/// Parse a schema **file**, generate codecs, write every module under `OUT_DIR`.
///
/// Also prints `cargo::rerun-if-changed=<schema_path>` and
/// `cargo::warning=…` for non-fatal generation warnings.
///
/// `config.module_name` becomes `{module_name}.rs` (e.g. `"messages"` →
/// `$OUT_DIR/messages.rs`).
///
/// # Errors
///
/// Parse, generate, missing `OUT_DIR`, or I/O failures.
///
/// # Example
///
/// ```rust,ignore
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     ergo_sbe::generate_to_out_dir(
///         "schemas/messages.xml",
///         ergo_sbe::GenerationConfig::new("messages")
///             .enable_domain_objects(),
///     )?;
///     Ok(())
/// }
/// ```
pub fn generate_to_out_dir(
    schema_path: impl AsRef<Path>,
    config: GenerationConfig,
) -> Result<GeneratedModuleSet, BuildError> {
    let schema_path = schema_path.as_ref();
    let ir = parse_file(schema_path)?;
    let modules = write_generated(Schema::from_ir(ir), config, &out_dir()?)?;
    println!("cargo::rerun-if-changed={}", schema_path.display());
    Ok(modules)
}

/// Like [`generate_to_out_dir`], but from an XML string (e.g. `include_str!`).
///
/// Does **not** emit `rerun-if-changed` (no file path). Prefer
/// [`generate_to_out_dir`] when the schema lives on disk so Cargo rebuilds
/// when it changes. If you use `include_str!`, add your own
/// `cargo::rerun-if-changed` for that path.
///
/// # Errors
///
/// Parse, generate, missing `OUT_DIR`, or I/O failures.
pub fn generate_str_to_out_dir(
    schema_xml: &str,
    config: GenerationConfig,
) -> Result<GeneratedModuleSet, BuildError> {
    generate_str_to_dir(schema_xml, config, &out_dir()?)
}

/// Parse schema XML, generate codecs, write every module under `out_dir`.
///
/// Same as [`generate_str_to_out_dir`] but with an explicit output directory
/// (useful in tests or non-Cargo drivers). Does not emit `rerun-if-changed`.
///
/// # Errors
///
/// Parse, generate, or I/O failures.
pub fn generate_str_to_dir(
    schema_xml: &str,
    config: GenerationConfig,
    out_dir: &Path,
) -> Result<GeneratedModuleSet, BuildError> {
    let ir = parse(schema_xml)?;
    write_generated(Schema::from_ir(ir), config, out_dir)
}

/// Absolute path to Cargo's `OUT_DIR` (build scripts only).
///
/// # Errors
///
/// [`BuildError::MissingOutDir`] when not running under Cargo.
pub fn out_dir() -> Result<PathBuf, BuildError> {
    env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or(BuildError::MissingOutDir)
}

fn write_generated(
    schema: Schema,
    config: GenerationConfig,
    out: &Path,
) -> Result<GeneratedModuleSet, BuildError> {
    let modules = Generator::new(config).generate(&schema)?;
    if modules.modules().len() == 0 {
        return Err(BuildError::Empty);
    }
    for m in modules.modules() {
        let dest = out.join(&m.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, &m.source)?;
    }
    for w in modules.warnings() {
        println!("cargo::warning={w}");
    }
    Ok(modules)
}

/// Include a module written by [`generate_to_out_dir`] / [`generate_str_to_out_dir`].
///
/// ```ignore
/// // After generate_to_out_dir(..., GenerationConfig::new("messages")):
/// ergo_sbe::include_sbe!("messages");
/// ```
///
/// Expands to `include!(concat!(env!("OUT_DIR"), "/messages.rs"))`.
#[macro_export]
macro_rules! include_sbe {
    ($module:literal) => {
        include!(concat!(env!("OUT_DIR"), "/", $module, ".rs"));
    };
    ($module:ident) => {
        include!(concat!(env!("OUT_DIR"), "/", stringify!($module), ".rs"));
    };
}

/// Declare a module that includes generated SBE codecs from `OUT_DIR`.
///
/// Applies the usual `allow`s for generated code (snake/camel, unused, …).
///
/// ```ignore
/// // build.rs wrote $OUT_DIR/messages.rs
/// ergo_sbe::sbe_mod!(messages);
/// // → mod messages { #![allow(...)] include!(.../messages.rs); }
///
/// ergo_sbe::sbe_mod!(pub codecs); // public module `codecs` → codecs.rs
///
/// // Module name differs from the generated file stem:
/// ergo_sbe::sbe_mod!(pub ergo_car = "car_bench"); // → car_bench.rs
/// ```
#[macro_export]
macro_rules! sbe_mod {
    ($name:ident) => {
        mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                clippy::all
            )]
            include!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".rs"));
        }
    };
    ($vis:vis $name:ident) => {
        $vis mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                clippy::all
            )]
            include!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".rs"));
        }
    };
    ($name:ident = $file:literal) => {
        mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                clippy::all
            )]
            include!(concat!(env!("OUT_DIR"), "/", $file, ".rs"));
        }
    };
    ($vis:vis $name:ident = $file:literal) => {
        $vis mod $name {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                unused_mut,
                unused_assignments,
                unused_must_use,
                unused_comparisons,
                non_camel_case_types,
                non_snake_case,
                unexpected_cfgs,
                clippy::all
            )]
            include!(concat!(env!("OUT_DIR"), "/", $file, ".rs"));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_schema() -> &'static str {
        r#"<?xml version="1.0"?>
        <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
          </types>
          <message name="Ping" id="1">
            <field name="seq" id="1" type="uint32" offset="0"/>
          </message>
        </messageSchema>"#
    }

    #[test]
    fn generate_str_to_dir_writes_module() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile_dir()?;
        let set = generate_str_to_dir(minimal_schema(), GenerationConfig::new("ping"), &dir)?;
        assert_eq!(set.modules().len(), 1);
        let path = dir.join("ping.rs");
        assert!(path.is_file(), "expected {}", path.display());
        let src = fs::read_to_string(&path)?;
        assert!(src.contains("PingEncoder"), "{src}");
        assert!(src.contains("PingDecoder"), "{src}");
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    fn tempfile_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dir = env::temp_dir().join(format!(
            "ergo_sbe_build_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}
