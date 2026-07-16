//! Build script: generate persist SBE codecs (V1 + V2) into OUT_DIR.
//!
//! Both `sbe_schema.xml` (V1: DynamicSchema/DynamicRow) and
//! `sbe_schema_v2.xml` (V2: DynamicSchemaV2/DynamicRowV2 with Decimal
//! arrays) are generated at build time. Nothing is checked in.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn generate(out_dir: &Path, xml: &str, module: &str) {
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let path = schema_dir.join(xml);
    let xml_src = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {xml}: {e} (at {})", path.display()));
    let ir = ergosbe::parse(&xml_src).unwrap_or_else(|e| panic!("parse {xml}: {e}"));
    let schema = ergosbe::Schema::from_ir(ir);
    let config = ergosbe::GenerationConfig::new(module);
    let generator = ergosbe::Generator::new(config);
    let modules = generator
        .try_generate(&schema)
        .unwrap_or_else(|e| panic!("generate {xml}: {e}"));
    for m in modules.modules() {
        let dest = out_dir.join(&m.path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, &m.source).unwrap();
    }
    println!("cargo:rerun-if-changed={}", path.display());
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate(&out_dir, "sbe_schema.xml", "persist_sbe");
    generate(&out_dir, "sbe_schema_v2.xml", "persist_sbe_v2");
    println!("cargo:rerun-if-changed=build.rs");
}
