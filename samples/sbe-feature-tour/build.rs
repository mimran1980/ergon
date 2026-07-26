//! Generate codecs from `schemas/feature-tour.xml`.
//!
//! Conversion (two different styles on purpose):
//! - `with_domain_type` → concrete `available() -> bool`, `timestamp() -> DateTime`
//! - `with_conversion`  → generic `price_as::<T>` / `price_from` on Quote (see
//!   `demo_conversion_only` in `src/lib.rs`)
//!
//! Do not call both for the same selector.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ergo_sbe::GenerationConfig::new("feature_tour")
        .enable_domain_objects(ergo_sbe::DomainVarData::LossyStrings)
        .with_domain_type(
            ergo_sbe::ConversionSelector::named_type("BooleanType"),
            "bool",
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        )
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));

    ergo_sbe::generate_to_out_dir("schemas/feature-tour.xml", config)?;
    Ok(())
}
