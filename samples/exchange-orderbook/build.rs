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
    let mut config = ergosbe::{ let mut c = GenerationConfig::new("bitget_spot"); c.domain_objects = true; c };
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

    // Tell cargo to re-run if schemas change
    println!("cargo:rerun-if-changed=schemas/bitget-spot.xml");
    println!("cargo:rerun-if-changed=schemas/binance-spot.xml");
}
