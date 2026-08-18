//! Integration tests for the feature-tour sample (same demos as `run_all`).

#[test]
fn feature_tour_run_all() -> Result<(), Box<dyn std::error::Error>> {
    sbe_feature_tour::run_all()
}

#[test]
fn car_length_matches_encode() -> Result<(), Box<dyn std::error::Error>> {
    let wire = sbe_feature_tour::demo_car_size_and_encode()?;
    assert!(!wire.is_empty());
    sbe_feature_tour::demo_car_decode_stages(&wire)?;
    sbe_feature_tour::demo_car_domain_dto(&wire)?;
    Ok(())
}

#[test]
fn heartbeat_and_any_message() -> Result<(), Box<dyn std::error::Error>> {
    let _ = sbe_feature_tour::demo_fixed_heartbeat()?;
    sbe_feature_tour::demo_any_message()?;
    Ok(())
}

#[test]
fn bulk_add_acceleration_encodes() -> Result<(), Box<dyn std::error::Error>> {
    let wire = sbe_feature_tour::demo_bulk_add()?;
    assert!(!wire.is_empty());
    Ok(())
}

/// `with_conversion(Decimal)` alone: generic `price_as` / `price_from`, not
/// concrete `price() -> rust_decimal::Decimal` (that requires `with_domain_type`).
#[test]
fn conversion_only_not_redundant_with_domain_type() -> Result<(), Box<dyn std::error::Error>> {
    let wire = sbe_feature_tour::demo_conversion_only()?;
    assert_eq!(wire.len(), sbe_feature_tour::QuoteEncoder::ENCODED_LENGTH);
    let dec = sbe_feature_tour::QuoteDecoder::try_from(wire.as_slice())?;
    let price: rust_decimal::Decimal = dec.price_as()?;
    assert_eq!(price, rust_decimal::Decimal::new(12345, 2));
    let fixed: sbe_feature_tour::FixedPrice = dec.price_as()?;
    assert_eq!(fixed.mantissa, 12345);
    assert_eq!(fixed.exponent, -2);
    Ok(())
}
