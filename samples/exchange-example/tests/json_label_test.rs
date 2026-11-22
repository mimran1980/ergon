//! The build.rs `jsonValue` extension hook: enums whose validValues all
//! carry a `jsonValue` schema mark get a `json_label()` accessor. Enums
//! with partial or no coverage are skipped by the hook — and the other two
//! schemas (no marks at all) generate byte-identical code, which the crate
//! building at all already proves.

#[test]
fn fully_labelled_enum_exposes_json_labels() {
    use exchange_example::binance_spot::AccountType;

    assert_eq!(AccountType::Spot.json_label(), "SPOT");
    assert_eq!(AccountType::Unknown.json_label(), "UNKNOWN");
    assert_eq!(
        AccountType::NonRepresentable.json_label(),
        "NON_REPRESENTABLE"
    );
    // The generated `NullVal` variant falls through the wildcard arm.
    assert_eq!(AccountType::NullVal.json_label(), "UNKNOWN");
}

#[test]
fn hook_fires_per_enum_not_once_per_schema() {
    use exchange_example::binance_spot::OrderStatus;

    assert_eq!(OrderStatus::New.json_label(), "NEW");
    assert_eq!(
        OrderStatus::PartiallyFilled.json_label(),
        "PARTIALLY_FILLED"
    );
}
