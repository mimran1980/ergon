//! Flyweight codec generation (zero-copy, no heap allocation).
//!
//! Parses the canonical car example schema and generates Rust encoder/decoder
//! types. The generated code uses consuming tail stages, borrows `&[u8]` /
//! `&mut [u8]`, and allocates no heap memory on the hot path.
//!
//! Run with: `cargo run --example flyweight`
//!
//! ```sh
//! cd samples/sbe-codegen-examples && cargo run --example flyweight
//! ```

#![allow(clippy::expect_used, clippy::unwrap_used)]

use ergo_sbe::{GenerationConfig, Generator, Schema, parse};

const SCHEMA: &str = include_str!("../schemas/car-schema.xml");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    let schema = Schema::from_ir(ir);

    let modules = Generator::new(GenerationConfig::new("car_codec")).generate(&schema)?;

    println!(
        "Generated {} module(s) from car example schema:",
        modules.modules().len()
    );
    for m in modules.modules() {
        println!("  {} — {} bytes", m.path, m.source.len());
        for line in m.source.lines() {
            if line.starts_with("pub struct") || line.starts_with("pub fn") {
                println!("    {line}");
            }
        }
    }

    Ok(())
}
