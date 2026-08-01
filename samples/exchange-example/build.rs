//! Generate SBE codecs into `src/generated/` (gitignored) for IDE go-to-definition.

fn generate_schema(
    xml_path: &str,
    module_name: &str,
    decimal: bool,
    out: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(xml_path).exists() {
        println!("cargo:warning=schema not found: {xml_path}");
        return Ok(());
    }
    // ANCHOR: with_conversion_config
    let mut config = ergo_sbe::GenerationConfig::new(module_name);
    if decimal {
        config = config.with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    }
    // ANCHOR_END: with_conversion_config
    ergo_sbe::generate_to_dir(xml_path, config, out)?;
    Ok(())
}

// ANCHOR: build_with_conversion
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generated_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    generate_schema(
        "schemas/normalized-app.xml",
        "normalized_app",
        true,
        &generated_dir,
    )?;
    generate_schema(
        "schemas/bitget-spot.xml",
        "bitget_spot",
        false,
        &generated_dir,
    )?;
    generate_schema(
        "schemas/binance-spot.xml",
        "binance_spot",
        false,
        &generated_dir,
    )?;
    Ok(())
}
// ANCHOR_END: build_with_conversion
