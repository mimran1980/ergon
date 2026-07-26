//! L3 nested-group showcase — the full API surface for 3 levels of nesting.
//!
//! Uses the Car schema (fuelFigures → performanceFigures → acceleration) to
//! show every type generated for nested groups. Same pattern as an L3 order
//! book: `bids → individual orders`, `asks → individual orders`.
//!
//! Run with: `cargo run --example l3_nested`

#![allow(clippy::expect_used, clippy::unwrap_used)]

use ergo_sbe::{GenerationConfig, Generator, Schema, parse};

const SCHEMA: &str = include_str!("../schemas/car-schema.xml");

fn human_size(n: usize) -> String {
    if n >= 1024 {
        format!("{} KiB", n / 1024)
    } else {
        format!("{} B", n)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(SCHEMA)?;
    let schema = Schema::from_ir(ir);
    let modules = Generator::new(GenerationConfig::new("car_codec")).generate(&schema)?;

    println!("=== L3 nested-group API surface ===\n");
    println!("Schema nesting:  Car → fuelFigures (L1) → performanceFigures (L2)");
    println!("                 performanceFigures → acceleration (L3)\n");

    for m in modules.modules() {
        println!("  ── {} ({}) ──\n", m.path, human_size(m.source.len()));

        for line in m.source.lines() {
            let t = line.trim();

            let tag = if t.starts_with("pub struct CarFixedFields") {
                Some("fixed-fields struct")
            } else if t.starts_with("pub struct CarEncoder") {
                Some("encoder entry point")
            } else if t.starts_with("pub struct CarDecoder") {
                Some("decoder entry point")
            } else if t.starts_with("pub struct FuelFiguresEncoder") {
                Some("L1 group encoder")
            } else if t.starts_with("pub struct FuelFiguresEntryEncoder") {
                Some("L1 entry — leads to L2")
            } else if t.starts_with("pub struct PerformanceFiguresEncoder") {
                Some("L2 group encoder")
            } else if t.starts_with("pub struct PerformanceFiguresEntryEncoder") {
                Some("L2 entry — leads to L3")
            } else if t.starts_with("pub struct AccelerationEncoder") {
                Some("L3 group encoder")
            } else if t.starts_with("pub struct AccelerationEntryEncoder") {
                Some("L3 entry (leaf)")
            } else {
                None
            };

            if let Some(tag) = tag {
                println!("    {t:55}  ← {tag}");
            }
        }
    }

    Ok(())
}
