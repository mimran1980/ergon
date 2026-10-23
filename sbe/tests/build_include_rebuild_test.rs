//! Cargo rebuilds generated codecs when only an included schema file changes.

#[path = "common/scratch.rs"]
mod scratch;

use scratch::ScratchCrate;
use std::fs;
use std::path::PathBuf;

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
    // A unique package in the shared scratch target dir: its first build is a
    // fresh build of this crate, so the second proves the include edit alone
    // reruns the build script.
    let krate = ScratchCrate::new("include_rebuild", true);
    let dir = std::env::temp_dir().join(krate.name());
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
name = "{name}"
version = "0.1.0"
edition = "2024"
build = "build.rs"

[dependencies]
ergo-sbe = {{ path = "{sbe}" }}

[build-dependencies]
ergo-sbe = {{ path = "{sbe}" }}
"#,
            name = krate.name(),
            sbe = sbe_dir.display()
        ),
    )?;
    krate.write_lockfile(&dir);

    let cargo = |dir: &std::path::Path| krate.cargo().current_dir(dir).args(["build"]).output();

    let first = cargo(&dir)?;
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
    let second = cargo(&dir)?;
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
