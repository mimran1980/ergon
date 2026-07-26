//! Generate normalized AppMessage/L2Book codecs for the HA sample.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ergo_sbe::generate_to_out_dir(
        "schemas/normalized-app.xml",
        ergo_sbe::GenerationConfig::new("normalized_app")
            .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal")),
    )?;
    Ok(())
}
