//! Generate the ergon Car codec on-the-fly from the example schema so
//! benchmarks always measure the latest `codegen`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("benchmarks crate has no parent dir")?
        .join("tests/fixtures/schemas/example-schema.xml");

    // Benchmarks measure flyweights only — no domain objects.
    // Enable unchecked companions for single-binary checked vs unchecked comparison.
    ergo_sbe::generate_to_out_dir(
        &schema_path,
        ergo_sbe::GenerationConfig::new("car_bench").with_unchecked_companions(),
    )?;

    println!("cargo:rerun-if-changed=../src/codegen.rs");
    println!("cargo:rerun-if-changed=../src/schema.rs");
    println!("cargo:rerun-if-changed=../src/ir.rs");
    println!("cargo:rerun-if-changed=../src/config.rs");
    Ok(())
}
