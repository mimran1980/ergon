//! Build script: generate normalized AppMessage codec with decimal converters.
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let schema_dir = PathBuf::from("schemas");

    // Copy the normalized-app schema from the exchange-orderbook sample
    let norm_path = schema_dir.join("normalized-app.xml");
    if norm_path.exists() {
        let xml = fs::read_to_string(&norm_path).expect("read normalized-app.xml");
        let ir = ergosbe::parse(&xml).expect("parse normalized-app.xml");
        let schema = ergosbe::Schema::from_ir(ir);
        let config = ergosbe::GenerationConfig::new("normalized_app")
            .enable_decimal_converters("Decimal");
        let generator = ergosbe::Generator::new(config);
        let modules = generator.generate(&schema);
        for m in modules.modules() {
            let dest = out_dir.join(&m.path);
            fs::create_dir_all(dest.parent().unwrap()).unwrap();
            fs::write(&dest, &m.source).unwrap();
        }
        println!("cargo:rerun-if-changed={}", norm_path.display());
    }
}
