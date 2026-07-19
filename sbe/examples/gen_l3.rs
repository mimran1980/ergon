//! Generate L3 orderbook code for inspection.
#![allow(clippy::expect_used, clippy::unwrap_used, missing_docs)]
use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("sbe/tests/fixtures/schemas/l3-orderbook-schema.xml");
    let ir = parse_file(&path).expect("parse");
    let schema = Schema::from_ir(ir);
    let mut config = GenerationConfig::new("l3book");
    config.domain_objects = true;
    let generator = Generator::new(config);
    let ms = generator.generate(&schema);
    let src = &ms.modules().next().expect("module").source;
    std::fs::write("/tmp/l3_full.rs", src).expect("write");
    println!("Written {} bytes to /tmp/l3_full.rs", src.len());

    Ok(())
}
