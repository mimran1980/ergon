//! Generate the feature-tour codecs from `schemas/feature-tour.xml`.
//!
//! # `with_conversion` vs `with_domain_type` (not redundant)
//!
//! | API | What you get |
//! |-----|----------------|
//! | **`with_conversion(sel)`** | Wire accessors stay primary (`price_value()` / `price_wire()`). Adds **generic** `price_as::<T>()` / `price_from(&T)` using `TryFromSbe` / `TryToSbe`. Caller picks `T` (e.g. `rust_decimal::Decimal`). |
//! | **`with_domain_type(sel, "path::Type")`** | Implies conversion for `sel`, **and** emits **concrete** methods `price() -> path::Type` / `price(Type)` that hard-wire that type. Domain DTOs also store that type when `enable_domain_objects()` is on. |
//!
//! Prefer `with_domain_type` when the app has one canonical Rust type.
//! Prefer `with_conversion` alone when you want pluggable converters or to keep
//! the flyweight API wire-typed.
//!
//! This sample uses **both**:
//! - domain types for `BooleanType` → `bool` and `UTCTimestamp` → `DateTime<Utc>`
//! - conversion-only for `Decimal` → generic `*_as` / `*_from` (see `demo_conversion_only`)

use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let schema_path = manifest_dir.join("schemas").join("feature-tour.xml");

    let xml = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let ir = ergo_sbe::parse(&xml).unwrap_or_else(|e| panic!("parse schema: {e}"));
    let schema = ergo_sbe::Schema::from_ir(ir);

    let config = ergo_sbe::GenerationConfig::new("feature_tour")
        .enable_domain_objects()
        // Concrete domain methods: available() -> bool, timestamp() -> DateTime<Utc>
        .with_domain_type(
            ergo_sbe::ConversionSelector::named_type("BooleanType"),
            "bool",
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        )
        // Conversion-only: Quote.price_as::<rust_decimal::Decimal>(), not price() -> Decimal
        .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal"));

    let modules = ergo_sbe::Generator::new(config)
        .generate(&schema)
        .unwrap_or_else(|e| panic!("generate: {e}"));
    let module = modules
        .modules()
        .next()
        .unwrap_or_else(|| panic!("no module generated"));

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("feature_tour.rs"), &module.source)
        .unwrap_or_else(|e| panic!("write OUT_DIR/feature_tour.rs: {e}"));

    println!("cargo::rerun-if-changed={}", schema_path.display());
    // Generated codecs may reference cfg(feature = "serde"); declare it for check-cfg.
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"serde\"))");
}
