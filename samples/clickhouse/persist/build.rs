//! Generate SBE codecs for the test schemas (used only by `tests/`).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for v in ["v1", "v2"] {
        let schema = format!("tests/schemas/shapes_{v}.xml");
        ergo_sbe::generate_to_out_dir(
            &schema,
            ergo_sbe::GenerationConfig::new(format!("shapes_{v}")),
        )?;
    }
    Ok(())
}
