fn main() {
    let xml_path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/schemas/baseline-schema.xml"
    ));
    let ir = ergo_sbe::parse_file(&xml_path).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let mut config = ergo_sbe::GenerationConfig::new("car_example");
    let config = config.enable_domain_objects();
    let g = ergo_sbe::Generator::new(config);
    let output = g
        .generate(&schema)
        .unwrap()
        .modules()
        .next()
        .unwrap()
        .source
        .clone();
    let golden_path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/car_example.rs"
    ));
    std::fs::write(&golden_path, &output).unwrap();
    eprintln!("Updated golden file at {}", golden_path.display());
}
