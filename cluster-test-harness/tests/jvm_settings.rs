#![allow(missing_docs, dead_code)]
//! The recommended JVM options (`jvm/aeron.jvm`) and latency properties
//! (`jvm/low-latency.properties`) the book shows: the `java` the harness runs
//! accepts every option, every property is one Aeron reads, and a cluster
//! node started with both serves a client.

mod common;

use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use ergo_aeron_cluster_test_harness::{TestCluster, jvm};
use serial_test::serial;

use common::{Capture, await_echo};

/// A directory of the test's own, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Result<Self, Box<dyn Error>> {
        let dir = std::env::temp_dir().join(format!("ergon-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn settings(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("jvm")
        .join(name)
}

/// `jvm/aeron.jvm` with its log moved from /dev/shm, which only Linux has,
/// into `dir`: the argument file `java @<it>` reads.
fn options_file(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let text = std::fs::read_to_string(settings("aeron.jvm"))?;
    if !text.contains("file=/dev/shm/") {
        return Err("aeron.jvm no longer logs to /dev/shm: update this test".into());
    }
    let logs = dir.to_str().ok_or("scratch dir is not valid UTF-8")?;
    let path = dir.join("aeron.jvm");
    std::fs::write(
        &path,
        text.replace("file=/dev/shm/", &format!("file={logs}/")),
    )?;
    Ok(path)
}

/// `jvm/low-latency.properties` as `(name, value)` pairs.
fn properties() -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let text = std::fs::read_to_string(settings("low-latency.properties"))?;
    let mut properties = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line
            .split_once('=')
            .ok_or_else(|| format!("low-latency.properties: not name=value: {line}"))?;
        properties.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    Ok(properties)
}

/// `jvm/low-latency.properties` as `-D<name>=<value>` flags.
fn property_flags() -> Result<Vec<OsString>, Box<dyn Error>> {
    Ok(properties()?
        .into_iter()
        .map(|(name, value)| OsString::from(format!("-D{name}={value}")))
        .collect())
}

/// The Aeron commit ergon pins: the `aeron` gitlink in the repository's HEAD.
/// Not the checkout, which the Aeron 1.46.8 compatibility job moves to an
/// older release than the one `low-latency.properties` is written for.
fn pinned_aeron(root: &Path) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-tree", "HEAD", "aeron"])
        .output()?;
    let listing = String::from_utf8(output.stdout)?;
    // `160000 commit <sha>\taeron`
    match listing.split_whitespace().collect::<Vec<_>>().as_slice() {
        [_, "commit", sha, "aeron"] if output.status.success() => Ok((*sha).to_owned()),
        _ => Err(format!(
            "no aeron gitlink in HEAD ({}): {listing}{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into()),
    }
}

/// Aeron ignores a property it does not read, so a misspelt name in
/// `low-latency.properties` would change nothing and fail nothing: each name
/// must be a string literal in the main sources of the Aeron commit ergon
/// pins.
#[test]
fn every_latency_property_is_one_aeron_reads() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let commit = pinned_aeron(&root)?;
    let properties = properties()?;
    assert!(
        !properties.is_empty(),
        "low-latency.properties sets nothing"
    );

    let mut grep = Command::new("git");
    grep.arg("-C")
        .arg(root.join("aeron"))
        .args(["grep", "--no-color", "-h", "-o", "-F"]);
    for (name, _) in &properties {
        grep.arg("-e").arg(format!("\"{name}\""));
    }
    grep.arg(&commit).arg("--").args([
        "aeron-client/src/main/java",
        "aeron-driver/src/main/java",
        "aeron-archive/src/main/java",
        "aeron-cluster/src/main/java",
    ]);
    let output = grep.output()?;
    // git grep exits 1 when nothing matches, and above 1 on an error.
    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(format!(
            "git grep in aeron at {commit} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let found = String::from_utf8(output.stdout)?;
    let found: BTreeSet<&str> = found.lines().collect();
    for (name, _) in &properties {
        assert!(
            found.contains(format!("\"{name}\"").as_str()),
            "low-latency.properties sets {name}, which no main source of Aeron {commit} names"
        );
    }
    Ok(())
}

/// The JVM log `aeron.jvm` asks for, from the one process that wrote to `dir`.
fn jvm_log(dir: &Path) -> Result<String, Box<dyn Error>> {
    let mut logs = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("aeron-jvm-")
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("log"))
        {
            logs.push(path);
        }
    }
    match logs.as_slice() {
        [log] => Ok(std::fs::read_to_string(log)?),
        _ => Err(format!(
            "expected one aeron-jvm-<pid>.log in {}, found {logs:?}",
            dir.display()
        )
        .into()),
    }
}

#[test]
fn every_recommended_jvm_option_is_accepted() -> Result<(), Box<dyn Error>> {
    let scratch = Scratch::new("jvm-options")?;
    let options = options_file(&scratch.0)?;
    let output = jvm::try_java()?
        .arg(format!("@{}", options.display()))
        // The file's 2 GiB heap, every page touched at start, is not needed
        // to see whether the JVM takes the options.
        .args(["-Xms64m", "-Xmx64m", "-version"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "java @aeron.jvm -version failed ({}):\n{stderr}",
        output.status
    );
    for complaint in [
        "unrecognized",
        "ignoring option",
        "warning",
        "error",
        "obsolete",
    ] {
        assert!(
            !stderr.to_lowercase().contains(complaint),
            "java @aeron.jvm -version said {complaint:?}:\n{stderr}"
        );
    }
    let log = jvm_log(&scratch.0)?;
    assert!(
        log.contains("Using Parallel"),
        "no Parallel GC in the JVM log:\n{log}"
    );
    Ok(())
}

#[test]
#[serial]
fn a_cluster_node_with_the_recommended_settings_serves_a_client() -> Result<(), Box<dyn Error>> {
    let scratch = Scratch::new("jvm-cluster")?;
    let mut args = vec![OsString::from(format!(
        "@{}",
        options_file(&scratch.0)?.display()
    ))];
    // Later flags win: a heap the size of the harness's own.
    args.extend(["-Xms256m", "-Xmx256m"].map(OsString::from));
    args.extend(property_flags()?);

    let cluster = TestCluster::single_node_with_jvm_args(&args)?;
    let dir = cluster
        .aeron_dir()
        .to_str()
        .ok_or("aeron dir is not valid UTF-8")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel(cluster.egress_channel.clone())?
        .message_timeout(Duration::from_secs(5))?;
    let mut client = AeronCluster::connect(&builder, dir)?;
    let mut listener = Capture::new();
    await_echo(
        &mut client,
        &mut listener,
        b"tuned-1",
        Duration::from_secs(10),
    )?;
    await_echo(
        &mut client,
        &mut listener,
        b"tuned-2",
        Duration::from_secs(10),
    )?;
    client.close()?;
    drop(cluster);

    // The node ran with the file's options: its JVM log says which
    // collector it used. A property Aeron rejects stops the node before it
    // reports ready.
    let log = jvm_log(&scratch.0)?;
    assert!(
        log.contains("Using Parallel"),
        "no Parallel GC in the JVM log:\n{log}"
    );
    Ok(())
}
