//! Regression tests ensuring generated output is deterministic.

#![allow(clippy::panic)]
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

mod common;
use common::Paths;
use ergo_sbe::{DomainVarData, GenerationConfig, Generator, Schema, parse_file};

fn generate_with_domain(xml_path: &std::path::Path, module_name: &str) -> String {
    let ir = parse_file(xml_path).unwrap();
    let schema = Schema::from_ir(ir);
    let mut config = GenerationConfig::new(module_name);
    let config = config.enable_domain_objects(DomainVarData::Bytes);
    let g = Generator::new(config);
    g.generate(&schema)
        .unwrap()
        .modules()
        .next()
        .unwrap()
        .source
        .clone()
}

fn canonical_rust(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let _ = syn::parse_file(source)?;
    let mut child = Command::new("rustfmt")
        .args(["--emit", "stdout", "--edition", "2024"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("rustfmt stdin unavailable")?
        .write_all(source.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "rustfmt failed while canonicalizing generated Rust:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn mismatch_context<'a>(left: &'a str, right: &'a str) -> (&'a str, &'a str, usize) {
    let mismatch = left
        .bytes()
        .zip(right.bytes())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| left.len().min(right.len()));
    let mut start = mismatch.saturating_sub(80);
    let mut left_end = mismatch.saturating_add(80).min(left.len());
    let mut right_end = mismatch.saturating_add(80).min(right.len());
    while !left.is_char_boundary(start) || !right.is_char_boundary(start) {
        start = start.saturating_sub(1);
    }
    while !left.is_char_boundary(left_end) {
        left_end = left_end.saturating_sub(1);
    }
    while !right.is_char_boundary(right_end) {
        right_end = right_end.saturating_sub(1);
    }
    (&left[start..left_end], &right[start..right_end], mismatch)
}

#[test]
fn canonical_rust_ignores_formatter_only_rewrites() -> Result<(), Box<dyn std::error::Error>> {
    let compact = "enum Example { Item { first: u32, second: u64 } }";
    let rustfmt =
        "enum Example {\n    Item {\n        first: u32,\n        second: u64,\n    },\n}";
    assert_eq!(canonical_rust(compact)?, canonical_rust(rustfmt)?);
    assert_ne!(
        canonical_rust(compact)?,
        canonical_rust("enum Example { Item { first: u32, changed: u64 } }")?
    );
    assert_eq!(
        canonical_rust("fn example() { let _ = value.map_err(|_| { Error::Invalid }); }")?,
        canonical_rust("fn example() { let _ = value.map_err(|_| Error::Invalid); }")?
    );
    Ok(())
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

    let output = canonical_rust(&output)?;
    let golden = canonical_rust(&golden)?;
    if output != golden {
        let (output_context, golden_context, mismatch) = mismatch_context(&output, &golden);
        panic!(
            "Generated output differs from golden file at {golden_path} \
             (first canonical byte mismatch at {mismatch}).\n\
             generated: {output_context:?}\n\
             golden:    {golden_context:?}\n\
             Run `cargo test update_golden -- --ignored` to regenerate."
        );
    }

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
