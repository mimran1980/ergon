fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let l3_schema = manifest_dir.join("../tests/fixtures/schemas/l3-orderbook-schema.xml");
    ergo_sbe::generate_to_dir(
        &l3_schema,
        ergo_sbe::GenerationConfig::new("l3_codec"),
        &output,
    )?;
    println!("cargo::rerun-if-changed={}", l3_schema.display());

    let ob_schema = manifest_dir.join("../benchmarks/schemas/orderbook.xml");
    ergo_sbe::generate_to_dir(
        &ob_schema,
        ergo_sbe::GenerationConfig::new("orderbook_codec"),
        &output,
    )?;
    println!("cargo::rerun-if-changed={}", ob_schema.display());

    Ok(())
}
