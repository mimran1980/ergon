//! Generate RFQ codecs into `src/generated/` (gitignored) for IDE navigation.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generated_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    ergo_sbe::generate_to_dir(
        "schemas/protocol-codecs.xml",
        ergo_sbe::GenerationConfig::new("rfq_codec"),
        &generated_dir,
    )?;
    Ok(())
}
