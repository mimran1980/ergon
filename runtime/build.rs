//! Generate the codecs for `schema/events.xml`, the event rows' own schema,
//! and `schema/frames.xml`, raw feed frames for backtests.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=schema/events.xml");
    println!("cargo:rerun-if-changed=schema/frames.xml");
    ergo_sbe::generate_to_out_dir(
        "schema/events.xml",
        ergo_sbe::GenerationConfig::new("events"),
    )?;
    ergo_sbe::generate_to_out_dir(
        "schema/frames.xml",
        ergo_sbe::GenerationConfig::new("frames"),
    )?;
    println!("cargo:rerun-if-changed=schema/input.xml");
    ergo_sbe::generate_to_out_dir("schema/input.xml", ergo_sbe::GenerationConfig::new("input"))?;
    Ok(())
}
