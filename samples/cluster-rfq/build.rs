//! Generate RFQ codecs from vendored protocol-codecs.xml (schema 101).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ergo_sbe::generate_to_out_dir(
        "schemas/protocol-codecs.xml",
        ergo_sbe::GenerationConfig::new("rfq_codec"),
    )?;
    Ok(())
}
