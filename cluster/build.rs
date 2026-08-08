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

use std::fs;
use std::path::{Path, PathBuf};

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
    println!("cargo::rustc-check-cfg=cfg(feature, values(\"test-harness\"))");

    // Java ClusterLauncher only when building with `--features test-harness`.
    if std::env::var_os("CARGO_FEATURE_TEST_HARNESS").is_some() {
        let aeron = manifest_dir.join("..").join("aeron");
        let java_out = PathBuf::from(std::env::var("OUT_DIR")?).join("test-harness-java");
        compile_test_harness_java(&manifest_dir, &aeron, &java_out);
    }
    Ok(())
}

/// Build Aeron jars if missing and compile `ClusterLauncher` into isolated Cargo output.
fn compile_test_harness_java(manifest_dir: &Path, aeron_dir: &Path, java_out: &Path) {
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
    let cp = std::env::join_paths([jar_dir.join("*"), cluster_jar.join("*")])
        .expect("Java classpath entries must be valid paths");
    eprintln!("Compiling ClusterLauncher into {}", java_out.display());
    let _ = fs::create_dir_all(java_out);
    // Force UTF-8 so source comments with non-ASCII bytes never trip US-ASCII
    // defaults on some macOS/JDK installs.
    let status = std::process::Command::new("javac")
        .args(["-encoding", "UTF-8", "-cp"])
        .arg(&cp)
        .arg("-d")
        .arg(java_out)
        .arg(&java_src)
        .status()
        .expect("failed to run javac — is Java 17+ installed?");
    if !status.success() {
        panic!("javac failed to compile ClusterLauncher");
    }

    println!("cargo::rerun-if-changed=src/test_support/java/ClusterLauncher.java");
    println!("cargo::rerun-if-changed=../aeron/aeron-all/build/libs");
}
