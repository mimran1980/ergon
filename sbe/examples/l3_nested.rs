//! L3 nested-group showcase — the full API surface for 3 levels of nesting.
//!
//! Uses the Car schema (fuelFigures → performanceFigures → acceleration) to
//! show every type generated for nested groups. Same pattern as an L3 order
//! book: `bids → individual orders`, `asks → individual orders`.
//!
//! Run with: `cargo run --example l3_nested`
#![allow(clippy::expect_used, clippy::unwrap_used, missing_docs)]

use ergo_sbe::{GenerationConfig, Generator, Schema, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema_xml = include_str!("../tests/fixtures/schemas/example-schema.xml");
    let ir = parse(schema_xml)?;
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
            } else if t.starts_with("pub struct CarAfterFuelFigures") {
                Some("after L1 — leads to var-data")
            } else if t.starts_with("pub struct CarDecoderAfterFuelFigures") {
                Some("after L1 decode — leads to var-data")
            } else if t.starts_with("pub struct FuelFiguresEncoder") {
                Some("L1 group encoder")
            } else if t.starts_with("pub struct FuelFiguresDecoder") {
                Some("L1 group decoder")
            } else if t.starts_with("pub struct FuelFiguresEntryEncoder") {
                Some("L1 entry — leads to L2")
            } else if t.starts_with("pub struct FuelFiguresEntryDecoder") {
                Some("L1 entry — leads to L2")
            } else if t.starts_with("pub struct PerformanceFiguresEncoder") {
                Some("L2 group encoder")
            } else if t.starts_with("pub struct PerformanceFiguresDecoder") {
                Some("L2 group decoder")
            } else if t.starts_with("pub struct PerformanceFiguresEntryEncoder") {
                Some("L2 entry — leads to L3")
            } else if t.starts_with("pub struct PerformanceFiguresEntryDecoder") {
                Some("L2 entry — leads to L3")
            } else if t.starts_with("pub struct AccelerationEncoder") {
                Some("L3 group encoder")
            } else if t.starts_with("pub struct AccelerationDecoder") {
                Some("L3 group decoder")
            } else if t.starts_with("pub struct AccelerationEntryEncoder") {
                Some("L3 entry (leaf)")
            } else if t.starts_with("pub fn compute_encoded_length_with_message_header") {
                Some("exact buffer sizing")
            } else if t.starts_with("pub fn wrap_and_apply_header") {
                Some("write header → fixed field stage")
            } else if t.starts_with("pub fn try_from(") {
                Some("decode + verify header")
            } else if t.starts_with("pub fn fixed(") {
                Some("write fixed fields from struct")
            } else if t.starts_with("pub fn raw_fixed(") {
                Some("individual fixed-field setters")
            } else if t.starts_with("pub fn finish(") {
                Some("ascend one nesting level")
            } else if t.starts_with("pub fn next(") {
                Some("next entry in group")
            } else if t.starts_with("pub fn add(") {
                Some("add entry to group")
            } else if t.starts_with("pub fn as_bytes(") {
                Some("view complete encoded bytes")
            } else if t.starts_with("pub fn encoded_length(") {
                Some("prove exact fit")
            } else if t.starts_with("impl.*Display for.*Decoder") {
                Some("Display impl — debug/log output")
            } else {
                None
            };

            if let Some(desc) = tag {
                println!("    {:<12} {}", desc, t);
            }
        }
        println!();
    }

    println!("── L3 walk (decoder) ──\n");
    println!("let dec = CarDecoder::try_from(bytes)?;                     // entry point");
    println!("let fuel = dec.into_fuel_figures()?;                       // L1 group");
    println!("while let Some(entry) = fuel.next() {{                      // L1 entries");
    println!("    let perf = entry.into_performance_figures()?;           // L2 group");
    println!("    while let Some(pe) = perf.next() {{                     // L2 entries");
    println!("        let accel = pe.into_acceleration()?;                // L3 group");
    println!("        while let Some(ae) = accel.next() {{                // L3 entries (leaf)");
    println!("            println!(\"0-{{}}: {{}}s\", ae.mph(), ae.seconds());");
    println!("        }}");
    println!("        let _ = accel.finish()?;                            // ascend to L2");
    println!("    }}");
    println!("    let _ = perf.finish()?;                                 // ascend to L1");
    println!("}}");
    println!("let after = fuel.finish()?;                                 // ascend to tail");
    println!("let (mfr, complete) = after.into_manufacturer_as_str()?;   // var-data");

    Ok(())
}

fn human_size(bytes: usize) -> String {
    if bytes >= 1024 { format!("{} KiB", bytes / 1024) } else { format!("{bytes} B") }
}
