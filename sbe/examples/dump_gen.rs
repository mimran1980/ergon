#![allow(clippy::all, clippy::pedantic, clippy::restriction, missing_docs)]
use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};
use std::path::PathBuf;

fn main() {
    let xml_path = PathBuf::from("sbe/tests/fixtures/schemas/unit-attribute-test-schema.xml");
    let ir = parse_file(&xml_path).unwrap();
    let schema = Schema::from_ir(ir);
    let generator = Generator::new(GenerationConfig::new("unit_attr_test"));
    let ms = generator.generate(&schema);
    let module = ms.modules().next().unwrap();

    // Search for _MIN, _MAX, _NULL
    let mut found = false;
    for line in module.source.lines() {
        if line.contains("_MIN") || line.contains("_MAX") || line.contains("_NULL") {
            println!("FOUND: {}", line.trim());
            found = true;
        }
    }
    if !found {
        println!("NO _MIN/_MAX/_NULL constants found in entire generated source");
        // Also search for PRICE specifically
        for line in module.source.lines() {
            if line.contains("PRICE") {
                println!("PRICE line: {}", line.trim());
            }
        }
    }
}
