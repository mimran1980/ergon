use ergosbe::{GenerationConfig, Generator, Schema, parse_file};
use std::fs;
use std::path::PathBuf;

fn main() {
    let cwd = std::env::current_dir().unwrap();
    let workspace = cwd.ancestors()
        .find(|a| a.join("Cargo.toml").exists() && a.join("sbe").exists())
        .unwrap_or_else(|| &cwd);

    let xml_path = workspace.join("sbe").join("tests").join("fixtures").join("schemas").join("example-schema.xml");
    println!("Loading schema from: {:?}", xml_path);

    let ir = parse_file(&xml_path).expect("parse schema");
    let schema = Schema::from_ir(ir);
    let generator = Generator::new(GenerationConfig::new("car_example"));
    let module_set = generator.generate(&schema);
    let module = module_set.modules().next().unwrap();

    let golden_path = workspace.join("sbe").join("tests").join("golden").join("car_example.rs");
    println!("Writing golden file: {:?}", golden_path);
    fs::write(&golden_path, &module.source).expect("write golden");

    let bench_path = workspace.join("sbe").join("benches").join("generated").join("car_patched.rs");
    println!("Writing bench file: {:?}", bench_path);
    fs::write(&bench_path, &module.source).expect("write bench");

    println!("Golden files regenerated successfully!");
}
