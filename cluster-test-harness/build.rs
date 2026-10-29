//! Compile `ClusterLauncher` against locally built Aeron jars.
//!
//! The JDK is `$JAVA_HOME` when set, else `javac` on `PATH`, as for Gradle.
//! `ClusterLauncher` is compiled for Java 17 (`--release 17`), as Aeron's own
//! classes are, so it runs on any JDK from 17 up whichever JDK compiled it.
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
            .unwrap_or_else(|e| panic!("failed to run {}: {e}", gradle.display()));
        if !status.success() {
            panic!(
                "Gradle jar build failed ({status}). Aeron's Gradle wrapper (9.8.2 at Aeron 1.54.0) \
                 runs on JDK 17 to 27, not 28 or newer: set JAVA_HOME to a JDK 25 and run \
                 `just build-aeron-jars`"
            );
        }
    }

    let java_src = manifest_dir
        .join("src")
        .join("test_support")
        .join("java")
        .join("ClusterLauncher.java");
    let jar_dir = aeron_dir.join("aeron-all").join("build").join("libs");
    let cluster_jar = aeron_dir.join("aeron-cluster").join("build").join("libs");
    let samples_jar = aeron_dir.join("aeron-samples").join("build").join("libs");
    let cp = std::env::join_paths([
        jar_dir.join("*"),
        cluster_jar.join("*"),
        samples_jar.join("*"),
    ])
    .expect("Java classpath entries must be valid paths");
    eprintln!("Compiling ClusterLauncher into {}", java_out.display());
    let _ = fs::create_dir_all(java_out);
    let javac = match std::env::var_os("JAVA_HOME") {
        Some(home) if !home.is_empty() => PathBuf::from(home).join("bin").join("javac"),
        _ => PathBuf::from("javac"),
    };
    let status = std::process::Command::new(&javac)
        .args(["--release", "17", "-encoding", "UTF-8", "-cp"])
        .arg(&cp)
        .arg("-d")
        .arg(java_out)
        .arg(&java_src)
        .status()
        .unwrap_or_else(|e| {
            panic!(
                "failed to run {}: {e}: set JAVA_HOME to a JDK 25 (17 or newer)",
                javac.display()
            )
        });
    if !status.success() {
        panic!(
            "{} failed to compile ClusterLauncher ({status}); `release version 17 not \
             supported` above means a JDK older than 17: set JAVA_HOME to a JDK 25 (17 or newer)",
            javac.display()
        );
    }

    println!("cargo::rerun-if-changed=src/test_support/java/ClusterLauncher.java");
    println!("cargo::rerun-if-env-changed=JAVA_HOME");
    println!("cargo::rerun-if-changed=../aeron/aeron-all/build/libs");
}
