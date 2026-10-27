//! Generate RFQ codecs into `src/generated/` (gitignored) for IDE navigation.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generated_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    ergo_sbe::generate(
        ergo_sbe::SchemaSource::file("schemas/protocol-codecs.xml"),
        ergo_sbe::GenerationConfig::new("rfq_codec"),
    )?
    .write_to(&generated_dir)?;
    Ok(())
}
