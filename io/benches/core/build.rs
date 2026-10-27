//! Hashes the crate's own sources (`Cargo.toml`, `build.rs`, every file under
//! `src/`, in path order) into `CORE_BUILD_HASH`, so a run manifest names the
//! exact measurement code behind its numbers and two arms built from
//! different cores are told apart.

use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let mut paths = vec![root.join("Cargo.toml"), root.join("build.rs")];
    let mut sources = Vec::new();
    files(&root.join("src"), &mut sources)?;
    sources.sort();
    paths.extend(sources);
    let mut hash = Sha256::new();
    for path in &paths {
        let name = path
            .strip_prefix(&root)?
            .to_string_lossy()
            .replace('\\', "/");
        let body = std::fs::read(path)?;
        // name, length, bytes: no two different trees hash alike by moving a
        // boundary between files
        hash.update(name.as_bytes());
        hash.update([0]);
        hash.update((body.len() as u64).to_le_bytes());
        hash.update(&body);
    }
    let hex = hash.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    println!("cargo:rustc-env=CORE_BUILD_HASH={hex}");
    Ok(())
}
