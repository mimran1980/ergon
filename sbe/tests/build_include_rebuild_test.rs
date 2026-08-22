//! Cargo rebuilds generated codecs when only an included schema file changes.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_types(path: &std::path::Path, primitive: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        path,
        format!(
            r#"<?xml version="1.0"?>
<types>
  <composite name="messageHeader">
    <type name="blockLength" primitiveType="uint16"/>
    <type name="templateId" primitiveType="uint16"/>
    <type name="schemaId" primitiveType="uint16"/>
    <type name="version" primitiveType="uint16"/>
  </composite>
  <type name="Seq" primitiveType="{primitive}"/>
</types>
"#
        ),
    )?;
    Ok(())
}

#[test]
fn cargo_rebuilds_generated_source_after_include_only_edit()
-> Result<(), Box<dyn std::error::Error>> {
    let sbe_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "ergo_sbe_include_rebuild_{}_{stamp}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("schemas"))?;
    fs::create_dir_all(dir.join("src/generated"))?;

    write_types(&dir.join("schemas/types.xml"), "uint32")?;
    fs::write(
        dir.join("schemas/root.xml"),
        r#"<?xml version="1.0"?>
<messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
  <include href="types.xml"/>
  <types/>
  <message name="Ping" id="1">
    <field name="seq" id="1" type="Seq"/>
  </message>
</messageSchema>
"#,
    )?;
    fs::write(
        dir.join("build.rs"),
        r#"fn main() -> ergo_sbe::miette::Result<()> {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    ergo_sbe::generate_to_dir(
        "schemas/root.xml",
        ergo_sbe::GenerationConfig::new("ping"),
        &out,
    )?;
    Ok(())
}
"#,
    )?;
    fs::write(
        dir.join("src/lib.rs"),
        r#"#[path = "generated/ping.rs"]
mod ping;
pub use ping::*;
"#,
    )?;
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "include_rebuild"
version = "0.1.0"
edition = "2024"
build = "build.rs"

[dependencies]
ergo-sbe = {{ path = "{sbe}" }}

[build-dependencies]
ergo-sbe = {{ path = "{sbe}" }}
"#,
            sbe = sbe_dir.display()
        ),
    )?;

    let target = dir.join("target_ci");
    let cargo = |dir: &std::path::Path, target: &std::path::Path| {
        Command::new("cargo")
            .args(["build", "--offline"])
            .current_dir(dir)
            .env("CARGO_TARGET_DIR", target)
            .env("CARGO_NET_OFFLINE", "true")
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .output()
    };

    let first = cargo(&dir, &target)?;
    assert!(
        first.status.success(),
        "first cargo build failed:\n{}\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let generated = dir.join("src/generated/ping.rs");
    let before = fs::read_to_string(&generated)?;
    assert!(
        before.contains("u32"),
        "first generate must use uint32 Seq:\n{before}"
    );

    write_types(&dir.join("schemas/types.xml"), "uint64")?;
    let second = cargo(&dir, &target)?;
    assert!(
        second.status.success(),
        "second cargo build failed:\n{}\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    let after = fs::read_to_string(&generated)?;
    assert!(
        after.contains("u64"),
        "include-only edit must rerun generate_to_dir and emit uint64 Seq:\n{after}"
    );

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}
