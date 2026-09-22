//! Generate market-data and diagnostics codecs with persist hooks.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    println!("cargo:rerun-if-changed=schemas/market-data.xml");
    println!("cargo:rerun-if-changed=schemas/diagnostics.xml");

    let persist = ergo_clickhouse_persist::codegen::persist_hook();

    let market = {
        let hook = std::sync::Arc::clone(&persist);
        ergo_sbe::GenerationConfig::new("market_data")
            .with_domain_objects(ergo_sbe::DomainVarData::Bytes)
            .with_hook(move |ctx| hook(ctx))
    };
    ergo_sbe::generate_to_dir("schemas/market-data.xml", market, &out)?;

    let diagnostics = {
        let hook = std::sync::Arc::clone(&persist);
        ergo_sbe::GenerationConfig::new("diagnostics")
            .with_domain_objects(ergo_sbe::DomainVarData::Bytes)
            .with_hook(move |ctx| hook(ctx))
    };
    ergo_sbe::generate_to_dir("schemas/diagnostics.xml", diagnostics, &out)?;
    Ok(())
}
