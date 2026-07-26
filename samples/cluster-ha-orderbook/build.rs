//! Generate normalized AppMessage/L2Book codecs for the HA sample.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generated codecs gate serde behind cfg(feature = "serde"); declare so
    // rustc check-cfg does not warn when the sample does not enable it.
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"bound-check-disabled\", \"serde\"))");

    ergo_sbe::generate_to_out_dir(
        "schemas/normalized-app.xml",
        ergo_sbe::GenerationConfig::new("normalized_app")
            .with_conversion(ergo_sbe::ConversionSelector::named_type("Decimal")),
    )?;
    Ok(())
}
