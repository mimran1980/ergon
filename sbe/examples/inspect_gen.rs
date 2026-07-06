use ergosbe::{GenerationConfig, Generator, Schema, parse_file};
use std::fs;

fn main() {
    let cwd = std::env::current_dir().unwrap();
    let workspace = cwd.ancestors()
        .find(|a| a.join("Cargo.toml").exists() && a.join("sbe").exists())
        .unwrap_or_else(|| &cwd);

    let xml_path = workspace.join("sbe").join("tests").join("fixtures").join("schemas").join("example-schema.xml");
    let ir = parse_file(&xml_path).expect("parse schema");
    let schema = Schema::from_ir(ir);
    let generator = Generator::new(GenerationConfig::new("car_example"));
    let module_set = generator.generate(&schema);
    let module = module_set.modules().next().unwrap();

    let out_path = workspace.join("tmp").join("generated_car_example.rs");
    fs::create_dir_all(workspace.join("tmp")).unwrap();
    fs::write(&out_path, &module.source).expect("write");
    println!("Generated code written to: {:?}", out_path);
}
