fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let schema = manifest_dir.join("../tests/fixtures/schemas/l3-orderbook-schema.xml");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    ergo_sbe::generate_to_dir(
        &schema,
        ergo_sbe::GenerationConfig::new("l3_codec"),
        &output,
    )?;
    println!("cargo::rerun-if-changed={}", schema.display());
    Ok(())
}
