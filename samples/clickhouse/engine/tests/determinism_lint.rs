//! The engine's `clippy.toml` must reject a hardware clock read, a
//! hash-ordered map, and state shared across threads or a thread started
//! (the client ban), and accept the same crate without them: a gate that
//! cannot fail is no gate.

use std::error::Error;
use std::path::Path;
use std::process::Command;

const CLIPPY_TOML: &str = include_str!("../clippy.toml");

/// `cargo clippy -D warnings` on a scratch crate whose `lib.rs` is `body`,
/// under the engine's `clippy.toml`. Ambient `RUSTFLAGS` are cleared: a
/// caller that caps lints would otherwise make every case pass. Colour is
/// off, as CI forces it on and the diagnostics are searched as text.
fn clippy(dir: &Path, body: &str) -> Result<(bool, String), Box<dyn Error>> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"lint-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    )?;
    std::fs::write(dir.join("clippy.toml"), CLIPPY_TOML)?;
    std::fs::write(dir.join("src/lib.rs"), body)?;
    let out = Command::new(env!("CARGO"))
        .args(["clippy", "--quiet", "--color", "never", "--"])
        .args(["-D", "warnings"])
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
fn clock_reads_hash_maps_and_threads_alone_are_rejected() -> Result<(), Box<dyn Error>> {
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
        (
            "arc",
            "pub fn f(x: &std::sync::Arc<u8>) -> u8 { **x }\n",
            "std::sync::Arc",
        ),
        (
            "spawn",
            "pub fn f() { let _ = std::thread::spawn(|| ()); }\n",
            "std::thread::spawn",
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
