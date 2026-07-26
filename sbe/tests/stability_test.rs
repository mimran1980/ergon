//! Regression tests ensuring generated output is deterministic.

#![allow(clippy::panic)]
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

use std::fs;

mod common;
use common::Paths;
use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};

fn generate_with_domain(xml_path: &std::path::Path, module_name: &str) -> String {
    let ir = parse_file(xml_path).unwrap();
    let schema = Schema::from_ir(ir);
    let mut config = GenerationConfig::new(module_name);
    let config = config.enable_domain_objects(false);
    let g = Generator::new(config);
    g.generate(&schema)
        .unwrap()
        .modules()
        .next()
        .unwrap()
        .source
        .clone()
}

#[test]
fn generated_output_matches_golden() -> Result<(), Box<dyn std::error::Error>> {
    let output = generate_with_domain(&Paths::example_schema(), "car_example");
    let golden_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/car_example.rs");

    let golden = fs::read_to_string(golden_path).unwrap_or_else(|e| {
        panic!(
            "Golden file not found at {golden_path}: {e}\n\
             Run `cargo test update_golden -- --ignored` to generate it."
        )
    });

    assert_eq!(
        output, golden,
        "\nGenerated output differs from golden file at {golden_path}.\n\
         Run `cargo test update_golden -- --ignored` to regenerate.\n"
    );

    Ok(())
}

#[test]
#[ignore = "run this manually to regenerate the golden file"]
fn update_golden() -> Result<(), Box<dyn std::error::Error>> {
    let output = generate_with_domain(&Paths::example_schema(), "car_example");
    let golden_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/car_example.rs");
    let _ = fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"));
    fs::write(golden_path, &output)
        .unwrap_or_else(|e| panic!("Failed to write golden file at {golden_path}: {e}"));
    eprintln!("Updated golden file at {golden_path}");

    Ok(())
}
