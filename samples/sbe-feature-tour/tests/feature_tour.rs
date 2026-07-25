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
