use ergosbe::{GenerationConfig, Generator, Schema, parse_file};
use std::path::PathBuf;
fn main() {
    let path = PathBuf::from("sbe/tests/fixtures/schemas/l3-orderbook-schema.xml");
    let ir = parse_file(&path).expect("ok");
    let schema = Schema::from_ir(ir);
    let mut config = GenerationConfig::new("l3book");
    config.domain_objects = true;
    let generator = Generator::new(config);
    let ms = generator.generate(&schema);
    let src = &ms.modules().next().expect("ok").source;
    std::fs::write("/tmp/l3_full.rs", src).expect("ok");
    println!("Written {} bytes to /tmp/l3_full.rs", src.len());
}
