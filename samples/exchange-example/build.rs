//! Generate SBE codecs for all sample schemas.
//! Pure SBE — no JSON, no REST, no external protocol translation.

fn generate_schema(
    xml_path: &str,
    module_name: &str,
    decimal: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(xml_path).exists() {
        println!("cargo:warning=schema not found: {xml_path}");
        return Ok(());
    }
    // Flyweight codecs. Decimal uses with_conversion only (generic price_as /
    // price_from + app TryFromSbe in src/decimal.rs) — not with_domain_type.
    let mut config = ergo_sbe::GenerationConfig::new(module_name);
    if decimal {
        config = config.with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));
    }
    ergo_sbe::generate_to_out_dir(xml_path, config)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_schema("schemas/normalized-app.xml", "normalized_app", true)?;
    generate_schema("schemas/bitget-spot.xml", "bitget_spot", false)?;
    generate_schema("schemas/binance-spot.xml", "binance_spot", false)?;
    Ok(())
}
