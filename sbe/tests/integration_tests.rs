//! Integration tests for `ergon` code generation.

#![allow(missing_docs)]
#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(unused)]

mod common;
use common::{Paths, compile_and_run, generate};
use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
use std::fs;

#[test]
fn test_generate_car_example() -> Result<(), Box<dyn std::error::Error>> {
    let xml_path = Paths::example_schema();

    let xml_content = fs::read_to_string(&xml_path).expect("Failed to read example schema");

    let ir = parse(&xml_content).expect("Failed to parse SBE schema");
    let schema = Schema::from_ir(ir);

    let generator = Generator::new(GenerationConfig::new("car_example"));
    let module_set = generator.generate(&schema).unwrap();

    let module = module_set.modules().next().unwrap();
    assert_eq!(module.path, "car_example.rs");

    assert!(module.source.contains("pub struct CarDecoder"));
    assert!(module.source.contains("pub struct CarEncoder"));
    assert!(module.source.contains("pub struct MessageHeader"));
    assert!(module.source.contains("pub struct Booster"));
    assert!(module.source.contains("pub struct Engine"));
    assert!(module.source.contains("pub struct OptionalExtras"));
    assert!(module.source.contains("pub enum Model"));
    assert!(module.source.contains("pub enum BooleanType"));

    Ok(())
}
