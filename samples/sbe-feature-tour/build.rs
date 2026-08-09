//! Generate codecs into `src/generated/` (gitignored) for IDE go-to-definition.
//! After `cargo build`, open `src/generated/feature_tour.rs`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: build_rs_example
    let generated_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    let config = ergo_sbe::GenerationConfig::new("feature_tour")
        .with_domain_objects(ergo_sbe::DomainVarData::Strings)
        .with_domain_type(
            ergo_sbe::ConversionSelector::named_type("BooleanType"),
            "bool",
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        )
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));

    ergo_sbe::generate_to_dir("schemas/feature-tour.xml", config, &generated_dir)?;
    // ANCHOR_END: build_rs_example
    Ok(())
}
