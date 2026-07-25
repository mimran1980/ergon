//! Generate the feature-tour codecs from `schemas/feature-tour.xml`.
//!
//! Config enables:
//! - domain objects (owned DTOs, e.g. `CarDomain`)
//! - `BooleanType` → `bool` domain conversion
//! - UTCTimestamp semantic type → `chrono::DateTime<Utc>` on Heartbeat

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
        .with_domain_type(
            ergo_sbe::ConversionSelector::named_type("BooleanType"),
            "bool",
        )
        .with_domain_type(
            ergo_sbe::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        );

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
