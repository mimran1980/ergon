//! Owned domain-object generation.
//!
//! Same schema as the flyweight example, but with `enable_domain_objects()`.
//! Each message gets an owned `MsgDomain` struct with `From<MsgDecoder>`,
//! useful for persistence, cross-thread transfer, and serialization.
//!
//! Run with: `cargo run --example domain_objects`
#![allow(clippy::expect_used, clippy::unwrap_used, missing_docs)]

use ergo_sbe::{GenerationConfig, Generator, Schema, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema_xml = include_str!("../tests/fixtures/schemas/example-schema.xml");
    let ir = parse(schema_xml)?;
    let schema = Schema::from_ir(ir);

    let modules = Generator::new(GenerationConfig::new("car_codec").enable_domain_objects())
        .generate(&schema)?;

    println!(
        "Generated {} module(s) with domain objects:",
        modules.modules().len()
    );
    for m in modules.modules() {
        println!("  {} — {} bytes", m.path, m.source.len());
        // Show domain structs
        for line in m.source.lines() {
            if line.contains("Domain")
                && (line.starts_with("pub struct") || line.contains("impl From"))
            {
                println!("    {line}");
            }
        }
    }

    Ok(())
}
