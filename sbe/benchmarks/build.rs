//! Build script: generates `ergon` Car codec on-the-fly from the example
//! schema so benchmarks always measure the latest `codegen`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_car_codec(&out_dir);
}

fn generate_car_codec(out_dir: &Path) {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join("schemas")
        .join("example-schema.xml");

    let ir = ergo_sbe::parse_file(&schema_path)
        .unwrap_or_else(|e| panic!("Failed to parse schema at {}: {e}", schema_path.display()));

    let schema = ergo_sbe::Schema::from_ir(ir);
    // Benchmarks measure flyweights only — no domain objects.
    // Enable unchecked companions for single-binary checked vs unchecked comparison.
    let config = ergo_sbe::GenerationConfig::new("car_bench").with_unchecked_companions();
    let generator = ergo_sbe::Generator::new(config);
    let module_set = generator
        .generate(&schema)
        .unwrap_or_else(|e| panic!("SBE generation failed for {}: {e}", schema_path.display()));
    let src = &module_set
        .modules()
        .next()
        .expect("GeneratedModuleSet has no modules")
        .source;

    let out_path = out_dir.join("car_bench.rs");
    fs::write(&out_path, src).unwrap_or_else(|e| {
        panic!(
            "Failed to write generated code to {}: {e}",
            out_path.display()
        )
    });

    // Rerun if schema or codegen sources change
    println!("cargo:rerun-if-changed={}", schema_path.display());
    println!("cargo:rerun-if-changed=../src/codegen.rs");
    println!("cargo:rerun-if-changed=../src/schema.rs");
    println!("cargo:rerun-if-changed=../src/ir.rs");
    println!("cargo:rerun-if-changed=../src/config.rs");
}
