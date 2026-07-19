//! Build script: generate the cluster SBE codecs from the Aeron submodule
//! schemas using ErgoSBE, writing to OUT_DIR.
//!
//! Schemas (aeron submodule, pinned 1.52.2):
//!   aeron/aeron-cluster/src/main/resources/cluster/aeron-cluster-codecs.xml
//!   aeron/aeron-cluster/src/main/resources/cluster/aeron-cluster-mark-codecs.xml
//!
//! RFQ (vendored cookbook schema 101):
//!   cluster/schemas/protocol-codecs.xml
//!
//! The generated files are `include!`d from `src/codecs/mod.rs`. The
//! aeron submodule must be checked out (`git submodule update --init aeron`).

use std::fs;
use std::path::PathBuf;

fn generate_schema(schema_path: &std::path::Path, module: &str, out_dir: &std::path::Path) {
    if !schema_path.exists() {
        panic!(
            "SBE schema not found at {}. \
             For Aeron schemas run `git submodule update --init aeron`.",
            schema_path.display()
        );
    }
    let xml_src = fs::read_to_string(schema_path).unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let ir = ergo_sbe::parse(&xml_src).unwrap_or_else(|e| panic!("parse {}: {e}", schema_path.display()));
    let schema = ergo_sbe::Schema::from_ir(ir);
    let cfg = ergo_sbe::GenerationConfig::new(module);
    let generator = ergo_sbe::Generator::new(cfg);
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

fn main() {
    // Production codecs are ErgoSBE-only (OUT_DIR). Residual sbe-tool trees
    // under src/codecs/{cluster_codecs,rfq_codecs} remain for head-to-head
    // benches only — no sbe-tool regeneration here.

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let aeron = manifest_dir.join("..").join("aeron");
    let schema_dir = aeron.join("aeron-cluster/src/main/resources/cluster");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    for (xml, module) in [
        ("aeron-cluster-codecs.xml", "aeron_cluster_codecs"),
        ("aeron-cluster-mark-codecs.xml", "aeron_cluster_codecs_mark"),
    ] {
        generate_schema(&schema_dir.join(xml), module, &out_dir);
    }

    // Cookbook RFQ protocol (schema 101) — production path is ErgoSBE.
    generate_schema(
        &manifest_dir.join("schemas/protocol-codecs.xml"),
        "aeron_rfq_codecs",
        &out_dir,
    );

    println!("cargo::rerun-if-changed=../sbe/src/codegen.rs");
    println!("cargo::rerun-if-changed=../sbe/src/schema.rs");
    // The generated codecs reference `cfg(feature = "serde")`; declare it so
    // rustc's check-cfg does not warn (serde is an opt-in the cluster crate
    // does not enable).
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"serde\"))");
}
