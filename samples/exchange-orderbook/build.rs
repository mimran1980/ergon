//! Build script: generate SBE code from Bitget + Binance Spot schemas.
//!
//! Uses `generate_multi()` with shared common types. The first schema
//! (common-types) provides shared SBE primitives; the exchange-specific
//! schemas import from it.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let schema_dir = PathBuf::from("schemas");

    // Load schemas
    let bitget_xml =
        fs::read_to_string(schema_dir.join("bitget-spot.xml")).expect("bitget-spot.xml not found");
    let binance_xml = fs::read_to_string(schema_dir.join("binance-spot.xml"))
        .expect("binance-spot.xml not found");

    // Parse IR
    let bitget_ir = ergosbe::parse(&bitget_xml).expect("failed to parse Bitget schema");
    let binance_ir = ergosbe::parse(&binance_xml).expect("failed to parse Binance schema");

    let bitget_schema = ergosbe::Schema::from_ir(bitget_ir);
    let binance_schema = ergosbe::Schema::from_ir(binance_ir);

    // Generate Rust source files
    let mut config = ergosbe::GenerationConfig::new("bitget_spot");
    config.domain_objects = true;
    let generator = ergosbe::Generator::new(config.clone());

    // Generate Bitget
    let modules = generator.generate(&bitget_schema);
    for m in modules.modules() {
        let dest = out_dir.join(&m.path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, &m.source).unwrap();
        println!("cargo:rerun-if-changed={}", dest.display());
    }

    // Generate Binance
    config.module_name = "binance_spot".into();
    let generator = ergosbe::Generator::new(config);
    let modules = generator.generate(&binance_schema);
    for m in modules.modules() {
        let dest = out_dir.join(&m.path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, &m.source).unwrap();
        println!("cargo:rerun-if-changed={}", dest.display());
    }

    // Generate normalized AppMessage schema with decimal converters
    let norm_xml = fs::read_to_string(schema_dir.join("normalized-app.xml"))
        .expect("normalized-app.xml not found");
    let norm_ir = ergosbe::parse(&norm_xml).expect("failed to parse normalized-app schema");
    let norm_schema = ergosbe::Schema::from_ir(norm_ir);
    let norm_config =
        ergosbe::GenerationConfig::new("normalized_app").enable_decimal_converters("Decimal");
    let norm_generator = ergosbe::Generator::new(norm_config);
    let norm_modules = norm_generator.generate(&norm_schema);
    for m in norm_modules.modules() {
        let dest = out_dir.join(&m.path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, &m.source).unwrap();
        println!("cargo:rerun-if-changed={}", dest.display());
    }

    // Tell cargo to re-run if schemas change
    println!("cargo:rerun-if-changed=schemas/bitget-spot.xml");
    println!("cargo:rerun-if-changed=schemas/binance-spot.xml");
    println!("cargo:rerun-if-changed=schemas/normalized-app.xml");
    // Generated codecs may reference cfg(feature = "serde").
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"serde\", \"bound-check-disabled\"))");
}
