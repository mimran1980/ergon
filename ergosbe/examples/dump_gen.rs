use ergosbe::{GenerationConfig, Generator, Schema, parse};
use std::fs;

fn main() {
    let xml_path = "simple-binary-encoding/sbe-samples/src/main/resources/example-schema.xml";
    let xml_content = fs::read_to_string(xml_path).expect("read");
    let ir = parse(&xml_content).expect("parse");
    let schema = Schema::from_ir(ir);
    let generator = Generator::new(GenerationConfig::low_latency("car_example"));
    let module_set = generator.generate(&schema);
    let module = module_set.modules().next().unwrap();
    println!("{}", module.source);
}
