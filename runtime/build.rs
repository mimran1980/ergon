//! Generate the codec for `schema/events.xml`, the event rows' own schema.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=schema/events.xml");
    ergo_sbe::generate_to_out_dir(
        "schema/events.xml",
        ergo_sbe::GenerationConfig::new("events"),
    )?;
    Ok(())
}
