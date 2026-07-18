//! Generate normalized AppMessage/L2Book codecs for the HA sample.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn generate_schema(out_dir: &Path, xml_path: &str, module_name: &str) {
    let path = PathBuf::from(xml_path);
    let xml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {xml_path}: {e}"));
    let ir = ergosbe::parse(&xml).unwrap_or_else(|e| panic!("parse {xml_path}: {e}"));
    let schema = ergosbe::Schema::from_ir(ir);
    let config = ergosbe::GenerationConfig::new(module_name).enable_decimal_converters("Decimal");
    let generator = ergosbe::Generator::new(config);
    let modules = generator
        .try_generate(&schema)
        .unwrap_or_else(|e| panic!("generate {xml_path}: {e}"));
    for m in modules.modules() {
        let dest = out_dir.join(&m.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&dest, &m.source).unwrap();
    }
    println!("cargo:rerun-if-changed={xml_path}");
}

fn main() {
    // Generated codecs gate serde behind cfg(feature = "serde"); declare so
    // rustc check-cfg does not warn when the sample does not enable it.
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"bound-check-disabled\", \"serde\"))");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_schema(&out_dir, "schemas/normalized-app.xml", "normalized_app");
}
