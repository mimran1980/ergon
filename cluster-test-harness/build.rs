//! Compile `ClusterLauncher` against locally built Aeron jars.
#![allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]

use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let aeron = manifest_dir.join("..").join("aeron");
    let java_out = PathBuf::from(std::env::var("OUT_DIR")?).join("test-harness-java");
    compile_test_harness_java(&manifest_dir, &aeron, &java_out);
    Ok(())
}

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
