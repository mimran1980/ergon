//! Dump the full generated source for the car schema to stdout.
//!
//! Useful for inspecting exactly what the generator produces — every struct,
//! impl, trait, and method. Pipe to a file or `less` for browsing:
//!
//! ```sh
//! cargo run --example dump_gen > car_codec.rs
//! ```
//!
//! Run with: `cargo run --example dump_gen`

#![allow(clippy::expect_used, clippy::unwrap_used)]

use ergo_sbe::{GenerationConfig, Generator, Schema, parse};

const SCHEMA: &str = include_str!("../schemas/car-schema.xml");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    let schema = Schema::from_ir(ir);
    let modules = Generator::new(GenerationConfig::new("car_codec")).generate(&schema)?;

    for m in modules.modules() {
        eprintln!("// ── {} ({} bytes) ──", m.path, m.source.len());
        print!("{}", m.source);
    }

    Ok(())
}
