//! Generate the recording-protocol codecs into `src/generated/` (gitignored,
//! IDE-visible).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    println!("cargo:rerun-if-changed=schemas/recording.xml");
    ergo_sbe::generate_to_dir(
        "schemas/recording.xml",
        ergo_sbe::GenerationConfig::new("recording"),
        &out,
    )?;
    Ok(())
}
