//! Generate the ergon Car codec on-the-fly from the example schema so
//! benchmarks always measure the latest `codegen`. Also generates the
//! large-composite layout-access codec used by `layout_access_bench`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sbe_root = manifest
        .parent()
        .ok_or("benchmarks crate has no parent dir")?;

    let car_schema = sbe_root.join("tests/fixtures/schemas/example-schema.xml");
    // Benchmarks measure flyweights only — no domain objects.
    // Enable unchecked companions for single-binary checked vs unchecked comparison.
    ergo_sbe::generate_to_out_dir(
        &car_schema,
        ergo_sbe::GenerationConfig::new("car_bench").with_unchecked_companions(),
    )?;

    let large_schema = manifest.join("schemas/large-composite.xml");
    ergo_sbe::generate_to_out_dir(
        &large_schema,
        ergo_sbe::GenerationConfig::new("large_comp_bench"),
    )?;

    let large_be_schema = manifest.join("schemas/large-composite-be.xml");
    ergo_sbe::generate_to_out_dir(
        &large_be_schema,
        ergo_sbe::GenerationConfig::new("large_comp_be_bench"),
    )?;

    let matrix_schema = manifest.join("schemas/codec-matrix.xml");
    ergo_sbe::generate_to_out_dir(
        &matrix_schema,
        ergo_sbe::GenerationConfig::new("codec_matrix_bench")
            .enable_domain_objects(ergo_sbe::DomainVarData::Bytes),
    )?;

    let matrix_be_schema = manifest.join("schemas/codec-matrix-be.xml");
    ergo_sbe::generate_to_out_dir(
        &matrix_be_schema,
        ergo_sbe::GenerationConfig::new("codec_matrix_be_bench"),
    )?;

    let matrix_custom_header_schema = manifest.join("schemas/codec-matrix-custom-header.xml");
    ergo_sbe::generate_to_out_dir(
        &matrix_custom_header_schema,
        ergo_sbe::GenerationConfig::new("codec_matrix_custom_header_bench"),
    )?;

    println!("cargo:rerun-if-changed=../src/codegen");
    println!("cargo:rerun-if-changed=../src/codegen/mod.rs");
    println!("cargo:rerun-if-changed=../src/codegen/runtime.rs");
    println!("cargo:rerun-if-changed=../src/schema.rs");
    println!("cargo:rerun-if-changed=../src/ir.rs");
    println!("cargo:rerun-if-changed=../src/config.rs");
    println!("cargo:rerun-if-changed=schemas/large-composite.xml");
    println!("cargo:rerun-if-changed=schemas/large-composite-be.xml");
    println!("cargo:rerun-if-changed=schemas/codec-matrix.xml");
    println!("cargo:rerun-if-changed=schemas/codec-matrix-be.xml");
    let orderbook_schema = manifest.join("schemas/orderbook.xml");
    ergo_sbe::generate_to_out_dir(
        &orderbook_schema,
        ergo_sbe::GenerationConfig::new("orderbook_bench"),
    )?;

    println!("cargo:rerun-if-changed=schemas/codec-matrix-custom-header.xml");
    println!("cargo:rerun-if-changed=schemas/orderbook.xml");
    Ok(())
}
