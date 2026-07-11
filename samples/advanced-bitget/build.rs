//! Build script: generate SBE codecs for all sample schemas.
//! Pure SBE — no JSON, no REST, no external protocol translation.
use std::env;
use std::fs;
use std::path::PathBuf;

fn generate_schema(out_dir: &PathBuf, xml_path: &str, module_name: &str, decimal: bool) {
    let path = PathBuf::from(xml_path);
    if !path.exists() {
        println!("cargo:warning=schema not found: {xml_path}");
        return;
    }
    let xml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {xml_path}: {e}"));
    let ir = ergosbe::parse(&xml).unwrap_or_else(|e| panic!("parse {xml_path}: {e}"));
    let schema = ergosbe::Schema::from_ir(ir);

    let mut config = ergosbe::GenerationConfig::new(module_name);
    if decimal {
        config = config.enable_decimal_converters("Decimal");
    }
    let generator = ergosbe::Generator::new(config);
    let modules = generator.generate(&schema);
    for m in modules.modules() {
        let dest = out_dir.join(&m.path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, &m.source).unwrap();
    }
    println!("cargo:rerun-if-changed={xml_path}");
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // AppMessage/L2Book/Trade — the normalized internal SBE schema
    generate_schema(&out_dir, "schemas/normalized-app.xml", "normalized_app", true);

    // Bitget spot SBE schema — exchange-native format
    generate_schema(&out_dir, "schemas/bitget-spot.xml", "bitget_spot", false);

    // Binance spot SBE schema — exchange-native format
    generate_schema(&out_dir, "schemas/binance-spot.xml", "binance_spot", false);
}
