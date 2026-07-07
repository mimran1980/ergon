//! Build script: generate SBE code for DynamicSchema + DynamicRow messages.
//!
//! Uses ergosbe codegen pipeline. Strips inner attributes from generated source
//! for edition 2024 compatibility (inner attributes after `include!()` are E0753).

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let schema_path = PathBuf::from("src/sbe_schema.xml");
    let schema_xml = fs::read_to_string(&schema_path).expect("sbe_schema.xml not found in src/");

    let ir = ergosbe::parse(&schema_xml).expect("failed to parse sbe_schema.xml");
    let schema = ergosbe::Schema::from_ir(ir);

    let config = ergosbe::GenerationConfig::new("persist_sbe");
    let generator = ergosbe::Generator::new(config);

    let modules = generator.generate(&schema);
    for m in modules.modules() {
        // Strip inner attributes (module doc comment + #![allow(...)] lines) from
        // the generated source.  These are edition-2024-incompatible when the file
        // is included via `include!()` from a wrapper module that provides its own
        // allow lints.
        let cleaned = strip_inner_attrs(&m.source);
        let dest = out_dir.join(&m.path);
        fs::write(&dest, &cleaned).unwrap();
        println!("cargo:rerun-if-changed={}", dest.display());
    }

    // Tell cargo to re-run if the schema changes
    println!("cargo:rerun-if-changed=src/sbe_schema.xml");
}

/// Remove leading `//!` doc-comment lines and `#![...]` inner-attribute lines
/// from the generated source so the caller's `include!()` wrapper doesn't hit
/// E0753 in edition 2024.
fn strip_inner_attrs(src: &str) -> String {
    let mut lines = src.lines().peekable();
    // Skip leading blank lines
    while let Some(line) = lines.peek() {
        if line.trim().is_empty() {
            lines.next();
        } else {
            break;
        }
    }
    // Skip inner-attribute lines (//! and #![...])
    let mut body = Vec::new();
    let mut in_preamble = true;
    for line in lines {
        if in_preamble {
            let trimmed = line.trim();
            if trimmed.starts_with("//!") || trimmed.starts_with("#![") || trimmed.is_empty() {
                continue;
            }
            in_preamble = false;
        }
        body.push(line);
    }
    body.join("\n")
}
