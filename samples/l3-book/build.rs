//! Generate L3 book codecs into `src/generated/` (gitignored) for IDE navigation.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generated_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    let config = ergo_sbe::GenerationConfig::new("l3_codec")
        .enable_domain_objects(ergo_sbe::DomainVarData::Bytes)
        .with_unchecked_companions()
        .with_domain_type(
            ergo_sbe::ConversionSelector::named_type("Decimal"),
            "rust_decimal::Decimal",
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::named_type("BooleanType"),
            "bool",
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        );
    ergo_sbe::generate_to_dir("schemas/l3-book.xml", config, &generated_dir)?;
    Ok(())
}
