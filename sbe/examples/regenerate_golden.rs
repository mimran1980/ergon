//! Regenerate the checked-in car codec used by stability and parity tests.

use std::path::PathBuf;

use ergo_sbe::{DomainVarData, GenerationConfig, Generator, Schema, parse_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: regenerate_golden OUTPUT_PATH")?;
    let schema_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/schemas/example-schema.xml");
    let schema = Schema::from_ir(parse_file(&schema_path)?);
    let config = GenerationConfig::new("car_example").enable_domain_objects(DomainVarData::Bytes);
    let generator = Generator::new(config);
    let modules = generator.generate(&schema)?;
    let source = modules
        .modules()
        .next()
        .ok_or("schema generated no module")?
        .source
        .as_bytes();

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, source)?;
    println!("{}", output_path.display());
    Ok(())
}
