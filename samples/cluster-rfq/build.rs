//! Build script: generate RFQ codecs from vendored protocol-codecs.xml
//! (schema 101) using ergo-sbe, writing to OUT_DIR.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let schema_path = manifest_dir.join("schemas").join("protocol-codecs.xml");

    if !schema_path.exists() {
        panic!("RFQ schema not found at {}", schema_path.display());
    }

    let xml_src = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let ir = ergo_sbe::parse(&xml_src)
        .unwrap_or_else(|e| panic!("parse {}: {e}", schema_path.display()));
    let schema = ergo_sbe::Schema::from_ir(ir);
    let cfg = ergo_sbe::GenerationConfig::new("rfq_codec");
    let generator = ergo_sbe::Generator::new(cfg);
    let modules = generator
        .generate(&schema)
        .unwrap_or_else(|e| panic!("generate {}: {e}", schema_path.display()));
    let m = modules
        .modules()
        .next()
        .unwrap_or_else(|| panic!("no module generated for {}", schema_path.display()));

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out_path = out_dir.join("rfq_codec.rs");
    fs::write(&out_path, &m.source).unwrap_or_else(|e| panic!("write {}: {e}", out_path.display()));

    println!("cargo::rerun-if-changed={}", schema_path.display());
}
