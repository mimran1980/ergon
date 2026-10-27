//! Generate normalized AppMessage/L2Book codecs into `src/generated/` (gitignored).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generated_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    ergo_sbe::generate(
        ergo_sbe::SchemaSource::file("schemas/normalized-app.xml"),
        ergo_sbe::GenerationConfig::new("normalized_app")
            .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal")),
    )?
    .write_to(&generated_dir)?;
    Ok(())
}
