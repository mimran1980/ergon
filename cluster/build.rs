//! Build script: generate the cluster SBE codecs from the Aeron submodule
//! schemas using ErgoSBE, writing to OUT_DIR.
//!
//! Schemas (aeron submodule, pinned 1.52.2):
//!   aeron/aeron-cluster/src/main/resources/cluster/aeron-cluster-codecs.xml
//!   aeron/aeron-cluster/src/main/resources/cluster/aeron-cluster-mark-codecs.xml
//!
//! The generated files are `include!`d from `src/codecs/ergo_codecs.rs`. The
//! aeron submodule must be checked out (`git submodule update --init aeron`).

use std::fs;
use std::path::PathBuf;

fn main() {
    // Production codecs are ErgoSBE-only (OUT_DIR). Residual sbe-tool trees
    // under src/codecs/{cluster_codecs,rfq_codecs} are compile-time sources
    // for benches / RFQ only — no build.rs regeneration.

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let aeron = manifest_dir.join("..").join("aeron");
    let schema_dir = aeron.join("aeron-cluster/src/main/resources/cluster");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    for (xml, module) in [
        ("aeron-cluster-codecs.xml", "aeron_cluster_codecs"),
        ("aeron-cluster-mark-codecs.xml", "aeron_cluster_codecs_mark"),
    ] {
        let schema_path = schema_dir.join(xml);
        if !schema_path.exists() {
            panic!(
                "Aeron cluster schema not found at {}. \
                 Run `git submodule update --init aeron`.",
                schema_path.display()
            );
        }
        let xml_src =
            fs::read_to_string(&schema_path).unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
        let ir = ergosbe::parse(&xml_src).unwrap_or_else(|e| panic!("parse {}: {e}", schema_path.display()));
        let schema = ergosbe::Schema::from_ir(ir);
        let cfg = ergosbe::GenerationConfig::new(module);
        let generator = ergosbe::Generator::new(cfg);
        let modules = generator
            .try_generate(&schema)
            .unwrap_or_else(|e| panic!("generate {}: {e}", schema_path.display()));
        let m = modules
            .modules()
            .next()
            .unwrap_or_else(|| panic!("no module generated for {}", schema_path.display()));
        let out_path = out_dir.join(format!("{module}.rs"));
        fs::write(&out_path, &m.source).unwrap_or_else(|e| panic!("write {}: {e}", out_path.display()));
        println!("cargo::rerun-if-changed={}", schema_path.display());
    }

    println!("cargo::rerun-if-changed=../sbe/src/codegen.rs");
    println!("cargo::rerun-if-changed=../sbe/src/schema.rs");
    // The generated codecs reference `cfg(feature = "serde")`; declare it so
    // rustc's check-cfg does not warn (serde is an opt-in the cluster crate
    // does not enable).
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"serde\"))");
}
