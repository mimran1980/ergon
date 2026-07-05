//! Integration tests for ErgoSBE code generation.

#![allow(missing_docs)]

use std::fs;
use ergosbe::{parse, Generator, GenerationConfig, Schema};

#[test]
fn test_generate_car_example() {
    let xml_path = if fs::metadata("simple-binary-encoding/sbe-samples/src/main/resources/example-schema.xml").is_ok() {
        "simple-binary-encoding/sbe-samples/src/main/resources/example-schema.xml"
    } else {
        "../simple-binary-encoding/sbe-samples/src/main/resources/example-schema.xml"
    };

    let xml_content = fs::read_to_string(xml_path)
        .expect("Failed to read example schema");

    let ir = parse(&xml_content).expect("Failed to parse SBE schema");
    let schema = Schema::from_ir(ir);

    let generator = Generator::new(GenerationConfig::low_latency("car_example"));
    let module_set = generator.generate(&schema);

    let module = module_set.modules().next().unwrap();
    assert_eq!(module.path, "car_example.rs");

    // Check that expected generated components exist in the source code
    assert!(module.source.contains("pub struct CarDecoder"));
    assert!(module.source.contains("pub struct CarEncoder"));
    assert!(module.source.contains("pub struct MessageHeader"));
    assert!(module.source.contains("pub struct Booster"));
    assert!(module.source.contains("pub struct Engine"));
    assert!(module.source.contains("pub struct OptionalExtras"));
    assert!(module.source.contains("pub struct Model"));
    assert!(module.source.contains("pub enum ModelKind"));
    assert!(module.source.contains("pub struct BooleanType"));
    assert!(module.source.contains("pub enum BooleanTypeKind"));
}
