//! Generate the SBE codecs for `schema/market.xml`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../schema/market.xml");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    ergo_sbe::generate_to_dir(
        "../schema/market.xml",
        // Generates `TryToSbe<Decimal9> for rust_decimal::Decimal` (used by `d9`).
        ergo_sbe::GenerationConfig::new("market").with_domain_type(
            ergo_sbe::ConversionSelector::named_type("Decimal9"),
            "rust_decimal::Decimal",
        ),
        &out,
    )?;
    Ok(())
}
