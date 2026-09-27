//! Generate the SBE codecs for `schema/market.xml` and `schema/trading.xml`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../schema/market.xml");
    println!("cargo:rerun-if-changed=../schema/trading.xml");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    let decimal9 = ergo_sbe::ConversionSelector::named_type("Decimal9");
    ergo_sbe::generate_to_dir(
        "../schema/market.xml",
        // Generates `TryToSbe<Decimal9>` and `TryToSbe<Rate>` for
        // `rust_decimal::Decimal` (used by `d9` and `rate`).
        ergo_sbe::GenerationConfig::new("market")
            .with_domain_type(decimal9.clone(), "rust_decimal::Decimal")
            .with_domain_type(
                ergo_sbe::ConversionSelector::named_type("Rate"),
                "rust_decimal::Decimal",
            ),
        &out,
    )?;
    ergo_sbe::generate_to_dir(
        "../schema/trading.xml",
        ergo_sbe::GenerationConfig::new("trading")
            .with_domain_type(decimal9, "rust_decimal::Decimal"),
        &out,
    )?;
    Ok(())
}
