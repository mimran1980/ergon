//! Generate L3 book codecs from `schemas/l3-book.xml`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Concrete app types (implies conversion). For generic price_as::<T> only,
    // see samples/exchange-example and samples/sbe-feature-tour.
    let config = ergo_sbe::GenerationConfig::new("l3_codec")
        .enable_domain_objects()
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
    ergo_sbe::generate_to_out_dir("schemas/l3-book.xml", config)?;
    Ok(())
}
