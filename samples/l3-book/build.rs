use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let schema_path = manifest_dir.join("schemas").join("l3-book.xml");

    let xml = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let ir = ergo_sbe::parse(&xml)
        .unwrap_or_else(|e| panic!("parse {}: {e}", schema_path.display()));
    let schema = ergo_sbe::Schema::from_ir(ir);
    // ponytail: converters configured but current codegen emits generic *_as::<T>()
    // rather than direct rust_decimal/bool accessors. Re-enable when converter
    // codegen supports non-generic domain-type accessors.
    let config = ergo_sbe::GenerationConfig::new("l3_codec");
    let generator = ergo_sbe::Generator::new(config);
    let modules = generator
        .generate(&schema)
        .unwrap_or_else(|e| panic!("generate {}: {e}", schema_path.display()));
    let m = modules
        .modules()
        .next()
        .unwrap_or_else(|| panic!("no module generated"));

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("l3_codec.rs"), &m.source)
        .unwrap_or_else(|e| panic!("write: {e}"));

    println!("cargo::rerun-if-changed={}", schema_path.display());
}
