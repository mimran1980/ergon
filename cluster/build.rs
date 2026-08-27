//! Generate cluster SBE codecs from vendored schemas into `OUT_DIR`.
//!
//! Schemas (vendored under cluster/schemas/):
//!   cluster/schemas/aeron-cluster-codecs.xml
//!   cluster/schemas/aeron-cluster-mark-codecs.xml
//!
//! Build scripts are allowed to panic/unwrap — they run at compile time and
//! a failure should stop the build immediately.
#![allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]
//!
//! The generated files are `include!`d from `src/codecs/mod.rs` as public
//! modules `session` and `mark`.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Production codecs are ergo-sbe-only (OUT_DIR). Residual sbe-tool trees
    // under src/codecs/{cluster_codecs,rfq_codecs} remain for head-to-head
    // benches only — no sbe-tool regeneration here.

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let schema_dir = manifest_dir.join("schemas");

    for (xml, module) in [
        ("aeron-cluster-codecs.xml", "session"),
        ("aeron-cluster-mark-codecs.xml", "mark"),
    ] {
        let schema_path = schema_dir.join(xml);
        if !schema_path.exists() {
            panic!(
                "SBE schema not found at {}. \
                 For Aeron schemas run `git submodule update --init aeron`.",
                schema_path.display()
            );
        }
        ergo_sbe::generate_to_out_dir(&schema_path, ergo_sbe::GenerationConfig::new(module))?;
    }

    println!("cargo::rerun-if-changed=../sbe/src/codegen.rs");
    println!("cargo::rerun-if-changed=../sbe/src/schema.rs");
    Ok(())
}
