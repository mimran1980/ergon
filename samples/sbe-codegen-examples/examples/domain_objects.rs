//! Owned domain-object generation.
//!
//! Same schema as the flyweight example, but with `with_domain_objects(DomainVarData::LossyStrings)`.
//! Each message gets an owned `MsgDomain` struct with `From<MsgDecoder>`,
//! useful for persistence, cross-thread transfer, and serialization.
//!
//! Run with: `cargo run --example domain_objects`

#![allow(clippy::expect_used, clippy::unwrap_used)]

use ergo_sbe::{DomainVarData, GenerationConfig, Generator, Schema, parse};

const SCHEMA: &str = include_str!("../schemas/car-schema.xml");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    let schema = Schema::from_ir(ir);

    let config = GenerationConfig::new("car_codec").with_domain_objects(DomainVarData::LossyStrings);
    let modules = Generator::new(config).generate(&schema)?;

    println!(
        "Generated {} module(s) with domain objects:",
        modules.modules().len()
    );
    for m in modules.modules() {
        println!("  {} — {} bytes", m.path, m.source.len());
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
