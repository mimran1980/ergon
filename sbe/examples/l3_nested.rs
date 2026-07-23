//! L3 nested-group showcase — 3 levels of repeating groups.
//!
//! Uses the Car schema (fuelFigures → performanceFigures → acceleration) to
//! demonstrate the nested-group encoder/decoder API. Same pattern as an L3
//! order book: bids → individual orders, asks → individual orders.
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
    println!("Schema: example-schema.xml (Car with 3 levels of nested groups)\n");
    println!(
        "Structure:  Car → fuelFigures (L1) → performanceFigures (L2) → acceleration (L3)"
    );
    println!();
    println!("Key types generated:\n");

    for m in modules.modules() {
        println!("  ── {} ({}) ──", m.path, human_size(m.source.len()));
        for line in m.source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub struct CarFixedFields {")
                || trimmed.starts_with("pub struct CarEncoder<")
                || trimmed.starts_with("pub struct CarDecoder<")
                || trimmed.starts_with("pub struct FuelFiguresDecoder<")
                || trimmed.starts_with("pub struct FuelFiguresEncoder<")
                || trimmed.starts_with("pub struct FuelFiguresEntryEncoder<")
                || trimmed.starts_with("pub struct PerformanceFiguresDecoder<")
                || trimmed.starts_with("pub struct PerformanceFiguresEncoder<")
                || trimmed.starts_with("pub struct PerformanceFiguresEntryEncoder<")
                || trimmed.starts_with("pub struct AccelerationDecoder<")
                || trimmed.starts_with("pub struct AccelerationEncoder<")
                || trimmed.starts_with("pub struct AccelerationEntryEncoder<")
                || trimmed.starts_with("pub fn acceleration(")
                || trimmed.starts_with("pub fn into_acceleration(")
                || trimmed.starts_with("pub fn performance_figures(")
                || trimmed.starts_with("pub fn into_performance_figures(")
                || trimmed.starts_with("pub fn fuel_figures(")
                || trimmed.starts_with("pub fn into_fuel_figures(")
                || trimmed.starts_with("pub fn compute_encoded_length_with_message_header")
                || trimmed.starts_with("pub fn wrap_and_apply_header")
                || trimmed.starts_with("pub fn try_from(")
                || trimmed.starts_with("pub fn finish(")
                || trimmed.starts_with("pub fn fixed(")
                || trimmed.starts_with("pub fn as_bytes(")
                || trimmed.starts_with("pub fn encoded_length(")
                || trimmed.starts_with("pub fn next(")
                || trimmed.starts_with("pub fn add(")
            {
                println!("    {trimmed}");
            }
        }
        println!();
    }

    println!("── Usage sketch (L1 → L2 → L3 encode) ──\n");
    println!("// Encode: 2 fuel figures, each 1 perf figure, each 2 accel entries");
    println!("let len = CarEncoder::compute_encoded_length_with_message_header(2, 12);");
    println!("let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;");
    println!("enc.fixed(&CarFixedFields {{ serial_number: 775, .. }});");
    println!();
    println!("// L1: fuelFigures (2 entries)");
    println!("enc.fuel_figures(2, |g| {{");
    println!("    g.add(|e| {{");
    println!("        e.speed(220); e.mpg(35);");
    println!("        // L2: performanceFigures (1 entry) inside fuel figure");
    println!("        e.performance_figures(1, |pg| {{");
    println!("            pg.add(|pe| {{");
    println!("                pe.octane_rating(98);");
    println!("                // L3: acceleration (2 entries) inside perf figure");
    println!("                pe.acceleration(2, |ag| {{");
    println!("                    ag.add(|ae| {{ ae.mph(30); ae.seconds(2.5); Ok(()) }})?;");
    println!("                    ag.add(|ae| {{ ae.mph(60); ae.seconds(6.1); Ok(()) }})?;");
    println!("                    Ok(())");
    println!("                }})?;");
    println!("                Ok(())");
    println!("            }})?;");
    println!("            Ok(())");
    println!("        }})?;");
    println!("        Ok(())");
    println!("    }})?;");
    println!("    Ok(())");
    println!("}})?;");
    println!();
    println!("// Decode: walk all 3 levels");
    println!("let dec = CarDecoder::try_from(&buf)?;");
    println!("let fuel = dec.into_fuel_figures()?;             // L1");
    println!("while let Some(entry) = fuel.next() {{");
    println!("    let perf = entry.into_performance_figures()?; // L2");
    println!("    while let Some(pe) = perf.next() {{");
    println!("        let accel = pe.into_acceleration()?;       // L3");
    println!("        while let Some(ae) = accel.next() {{");
    println!("            println!(\"0-{{}}: {{}}s\", ae.mph(), ae.seconds());");
    println!("        }}");
    println!("        let _ = accel.finish()?;                   // back to L2 entry");
    println!("    }}");
    println!("    let _ = perf.finish()?;                       // back to L1 entry");
    println!("}}");
    println!("let after_fuel = fuel.finish()?;                  // back to Car tail");

    Ok(())
}

fn human_size(bytes: usize) -> String {
    if bytes >= 1024 {
        format!("{} KiB", bytes / 1024)
    } else {
        format!("{bytes} B")
    }
}
