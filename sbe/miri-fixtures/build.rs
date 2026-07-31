fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    for (schema, module) in [
        ("schemas/little-endian.xml", "little_endian"),
        ("schemas/big-endian.xml", "big_endian"),
        ("schemas/nested.xml", "nested"),
    ] {
        let schema = manifest_dir.join(schema);
        ergo_sbe::generate_to_dir(
            &schema,
            ergo_sbe::GenerationConfig::new(module),
            &output,
        )?;
        println!("cargo::rerun-if-changed={}", schema.display());
    }
    Ok(())
}
