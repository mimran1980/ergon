//! The engine's `clippy.toml` must reject a hardware clock read and a
//! hash-ordered map, and accept the same crate without them: a gate that
//! cannot fail is no gate.

use std::error::Error;
use std::path::Path;
use std::process::Command;

const CLIPPY_TOML: &str = include_str!("../clippy.toml");

/// `cargo clippy -D warnings` on a scratch crate whose `lib.rs` is `body`,
/// under the engine's `clippy.toml`. Ambient `RUSTFLAGS` are cleared: a
/// caller that caps lints would otherwise make every case pass.
fn clippy(dir: &Path, body: &str) -> Result<(bool, String), Box<dyn Error>> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"lint-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    )?;
    std::fs::write(dir.join("clippy.toml"), CLIPPY_TOML)?;
    std::fs::write(dir.join("src/lib.rs"), body)?;
    let out = Command::new(env!("CARGO"))
        .args(["clippy", "--quiet", "--", "-D", "warnings"])
        .current_dir(dir)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_RUSTFLAGS")
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .output()?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

#[test]
fn clock_reads_and_hash_maps_are_rejected_and_the_rest_is_not() -> Result<(), Box<dyn Error>> {
    let root = std::env::temp_dir().join(format!("engine-lint-{}", std::process::id()));
    let (ok, _) = clippy(
        &root.join("clean"),
        "pub fn f(m: &std::collections::BTreeMap<u8, u8>) -> usize { m.len() }\n",
    )?;
    assert!(ok, "the clean fixture must pass");
    for (name, body, needle) in [
        (
            "instant",
            "pub fn f() -> std::time::Instant { std::time::Instant::now() }\n",
            "Instant::now",
        ),
        (
            "system_time",
            "pub fn f() -> std::time::SystemTime { std::time::SystemTime::now() }\n",
            "SystemTime::now",
        ),
        (
            "hash_map",
            "pub fn f(m: &std::collections::HashMap<u8, u8>) -> usize { m.len() }\n",
            "HashMap",
        ),
    ] {
        let (ok, stderr) = clippy(&root.join(name), body)?;
        assert!(!ok, "{name}: clippy accepted it");
        assert!(
            stderr.contains("disallowed") && stderr.contains(needle),
            "{name}: rejected for another reason:\n{stderr}"
        );
    }
    Ok(())
}
