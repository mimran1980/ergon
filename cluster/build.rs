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
use std::path::{Path, PathBuf};

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
    let mut cfg = ergo_sbe::GenerationConfig::new(module);
    if cfg!(feature = "unchecked-companions") {
        cfg = cfg.with_unchecked_companions();
    }
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
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"serde\", \"test-harness\"))");

    // Java ClusterLauncher only when building with `--features test-harness`.
    if std::env::var_os("CARGO_FEATURE_TEST_HARNESS").is_some() {
        compile_test_harness_java(&manifest_dir, &aeron);
    }
}

/// Build Aeron jars if missing and compile `ClusterLauncher` onto the samples classpath.
fn compile_test_harness_java(manifest_dir: &Path, aeron_dir: &Path) {
    let libs_dir = aeron_dir.join("aeron-all").join("build").join("libs");
    if !libs_dir.exists() {
        let gradle = if cfg!(target_os = "windows") {
            aeron_dir.join("gradlew.bat")
        } else {
            aeron_dir.join("gradlew")
        };
        eprintln!("Building aeron jars via Gradle in {}", aeron_dir.display());
        let status = std::process::Command::new(&gradle)
            .current_dir(aeron_dir)
            .args([
                ":aeron-cluster:jar",
                ":aeron-archive:jar",
                ":aeron-all:jar",
                ":aeron-samples:jar",
            ])
            .status()
            .expect("failed to run Gradle — is Java 17+ installed?");
        if !status.success() {
            panic!("Gradle jar build failed with exit code: {status}");
        }
    }

    let java_src = manifest_dir
        .join("src")
        .join("test_support")
        .join("java")
        .join("ClusterLauncher.java");
    let jar_dir = aeron_dir.join("aeron-all").join("build").join("libs");
    let cluster_jar = aeron_dir.join("aeron-cluster").join("build").join("libs");
    let samples_classes = aeron_dir
        .join("aeron-samples")
        .join("build")
        .join("classes")
        .join("java")
        .join("main");

    let cp = format!("{}/*:{}/*", jar_dir.display(), cluster_jar.display());
    eprintln!("Compiling ClusterLauncher into {}", samples_classes.display());
    let _ = fs::create_dir_all(&samples_classes);
    let status = std::process::Command::new("javac")
        .args(["-cp", &cp, "-d"])
        .arg(&samples_classes)
        .arg(&java_src)
        .status()
        .expect("failed to run javac — is Java 17+ installed?");
    if !status.success() {
        panic!("javac failed to compile ClusterLauncher");
    }

    println!("cargo::rerun-if-changed=src/test_support/java/ClusterLauncher.java");
    println!("cargo::rerun-if-changed=../aeron/aeron-all/build/libs");
}
