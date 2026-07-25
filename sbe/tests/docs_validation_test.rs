//! Validates README + rustdoc claims against real codegen and compilable snippets.
//!
//! - Extracts ```rust fences from `sbe/README.md` (skips ```rust,ignore)
//! - Compiles each fence as a tiny crate depending on path `ergo-sbe`
//! - Generates a representative schema and asserts documented API surfaces
//! - Smoke-runs encode/decode patterns described in crate docs

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ergo_sbe::{
    GenerationConfig, Generator, Schema, parse, validate_against_sbe_xsd, SBE_XSD,
};

fn header_and_types() -> &'static str {
    r#"
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <type name="Nums" primitiveType="uint32" length="4"/>
    <type name="Code" primitiveType="char" length="6" characterEncoding="ASCII"/>
    <type name="qty" primitiveType="uint32" minValue="1" maxValue="1000"
          epoch="unix" timeUnit="nanosecond" semanticType="UTCTimestamp"/>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
"#
}

fn docs_schema_xml() -> String {
    format!(
        r#"<?xml version="1.0"?>
        <messageSchema package="docs" id="9" version="0" byteOrder="littleEndian">
        {types}
        </types>
        <message name="Quote" id="1" blockLength="30">
          <field name="seq" id="1" type="uint32" offset="0"/>
          <field name="someNumbers" id="2" type="Nums" offset="4"/>
          <field name="vehicleCode" id="3" type="Code" offset="20"/>
          <field name="qty" id="4" type="qty" offset="26"/>
          <group name="legs" id="10" dimensionType="groupSizeEncoding">
            <field name="value" id="11" type="uint32" offset="0"/>
          </group>
          <data name="note" id="20" type="varStringEncoding"/>
        </message>
        <message name="Heartbeat" id="2" blockLength="4">
          <field name="seq" id="1" type="uint32" offset="0"/>
        </message>
        </messageSchema>"#,
        types = header_and_types()
    )
}

fn extract_rust_fences(md: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut rest = md;
    let mut line_base = 1usize;
    while let Some(start) = rest.find("```") {
        let before = &rest[..start];
        line_base += before.matches('\n').count();
        rest = &rest[start + 3..];
        let nl = rest.find('\n').unwrap_or(rest.len());
        let lang = rest[..nl].trim().to_string();
        rest = &rest[nl + 1..];
        let Some(end) = rest.find("```") else { break };
        let body = rest[..end].to_string();
        rest = &rest[end + 3..];
        line_base += body.matches('\n').count() + 1; // fence close
        // Runnable fences: bare `rust` only (not rust,ignore / rust,no_run).
        if lang == "rust" || lang == "rs" {
            out.push((line_base, body));
        }
    }
    out
}

fn wrap_snippet(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.contains("fn main") {
        // Allow snippets that already form a program.
        format!(
            "#![allow(dead_code, unused_imports, unused_variables, unused_mut)]\n{trimmed}\n"
        )
    } else {
        format!(
            "#![allow(dead_code, unused_imports, unused_variables, unused_mut)]\n\
             fn main() -> Result<(), Box<dyn std::error::Error>> {{\n\
             {trimmed}\n\
             Ok(())\n\
             }}\n"
        )
    }
}

fn compile_snippet(tmp_root: &Path, name: &str, body: &str) -> Result<(), String> {
    let crate_dir = tmp_root.join(name);
    fs::create_dir_all(crate_dir.join("src")).map_err(|e| e.to_string())?;
    let ergo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.0.0"
edition = "2021"
[dependencies]
ergo-sbe = {{ path = "{ergo}" }}
"#,
        name = name,
        ergo = ergo_path.display()
    );
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml).map_err(|e| e.to_string())?;
    fs::write(crate_dir.join("src/main.rs"), wrap_snippet(body)).map_err(|e| e.to_string())?;

    let target_dir = tmp_root.join("target");
    let out = Command::new("cargo")
        .args([
            "check",
            "--quiet",
            "--manifest-path",
        ])
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .output()
        .map_err(|e| format!("spawn cargo: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo check failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

#[test]
fn readme_rust_fences_compile() -> Result<(), Box<dyn std::error::Error>> {
    let readme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let md = fs::read_to_string(&readme_path)?;
    let fences = extract_rust_fences(&md);
    assert!(
        !fences.is_empty(),
        "expected at least one ```rust fence in README.md"
    );

    let tmp = tempfile::tempdir()?;
    for (i, (line, body)) in fences.iter().enumerate() {
        // Skip pure include! shape that needs OUT_DIR — those should be rust,ignore.
        if body.contains("include!(concat!(env!(\"OUT_DIR\")") {
            continue;
        }
        let name = format!("readme_snip_{i}");
        compile_snippet(tmp.path(), &name, body).map_err(|e| {
            format!(
                "README.md rust fence near line {line} failed to compile:\n{e}\n--- body ---\n{body}"
            )
        })?;
    }
    Ok(())
}

#[test]
fn documented_generated_surface_strings() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse(&docs_schema_xml())?;
    let schema = Schema::from_ir(ir);
    let cfg = GenerationConfig::new("docs_codec")
        .enable_domain_objects()
        .with_keyword_append_token("_");
    let src = Generator::new(cfg)
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no module")?
        .source
        .clone();

    // Crate docs / README claims
    for needle in [
        "QuoteDecoder",
        "QuoteEncoder",
        "HeartbeatDecoder",
        "AnyMessage",
        "FrameCursor",
        "SEQ_ID",
        "SEQ_ENCODING_OFFSET",
        "SEQ_ENCODING_LENGTH",
        "seq_meta_attribute",
        "MetaAttribute",
        "put_some_numbers",
        "vehicle_code_str",
        "copy_vehicle_code",
        "FixedArrayTooLong",
        "QuoteDomain",
        "ValueOutOfRange",
        "try_wrap_and_apply_header",
        "ENCODED_LENGTH",
    ] {
        assert!(
            src.contains(needle),
            "documented surface missing {needle:?} in generated module"
        );
    }
    Ok(())
}

#[test]
fn documented_encode_decode_smoke() -> Result<(), Box<dyn std::error::Error>> {
    // Compile generated module + exercise APIs described in crate rustdocs.
    let ir = parse(&docs_schema_xml())?;
    let schema = Schema::from_ir(ir);
    let src = Generator::new(GenerationConfig::new("docs_run").enable_domain_objects())
        .generate(&schema)?
        .modules()
        .next()
        .ok_or("no module")?
        .source
        .clone();

    let tmp = tempfile::tempdir()?;
    let crate_dir = tmp.path().join("docs_run");
    fs::create_dir_all(crate_dir.join("src"))?;
    let ergo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "docs_run"
version = "0.0.0"
edition = "2021"
[dependencies]
ergo-sbe = {{ path = "{}" }}
"#,
            ergo.display()
        ),
    )?;
    fs::write(crate_dir.join("src/gen.rs"), src)?;
    fs::write(
        crate_dir.join("src/main.rs"),
        r#"
#![allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all)]
mod gen;
use gen::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fixed length + try wrap (docs: safe decode/encode)
    let mut buf = vec![0u8; HeartbeatEncoder::ENCODED_LENGTH];
    {
        let mut enc = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
        enc.seq(7);
    }
    let dec = HeartbeatDecoder::try_from(buf.as_slice())?;
    assert_eq!(dec.seq(), 7);
    assert_eq!(HeartbeatDecoder::SEQ_ID, 1);
    assert_eq!(
        HeartbeatDecoder::seq_meta_attribute(sbe_rt::MetaAttribute::Presence),
        Some("required")
    );

    // Bulk array helpers + group/var-data tail (docs)
    let mut qbuf = vec![0u8; 512];
    let written = {
        let mut enc = QuoteEncoder::try_wrap_and_apply_header(&mut qbuf, 0)?;
        enc.seq(1);
        enc.put_some_numbers(1, 2, 3, 4);
        enc.vehicle_code_str("ABCDEF")?;
        enc.qty(10);
        let enc = enc.legs(1, |g| {
            g.add(|e| {
                e.value(99);
                Ok(())
            })?;
            Ok(())
        })?;
        let done = enc.note(b"hi")?;
        done.encoded_length_with_header()
    };
    let q = QuoteDecoder::try_from(&qbuf[..written])?;
    assert_eq!(q.seq(), 1);
    assert_eq!(q.some_numbers(), [1, 2, 3, 4]);
    let mut dst = [0u8; 6];
    assert_eq!(q.copy_vehicle_code(&mut dst), 6);
    assert_eq!(&dst, b"ABCDEF");

    // Domain object (docs)
    let dto = QuoteDomain::from(q);
    let mut out = vec![0u8; 512];
    let n = dto.encode(&mut out)?;
    assert!(n > 0);

    // AnyMessage dispatch (crate docs)
    match AnyMessage::decode(buf.as_slice(), 0)? {
        AnyMessage::Heartbeat(h) => assert_eq!(h.seq(), 7),
        AnyMessage::Quote(_) | AnyMessage::Unknown { .. } => {
            panic!("expected Heartbeat")
        }
    }

    Ok(())
}
"#,
    )?;

    let status = Command::new("cargo")
        .args(["run", "--quiet", "--manifest-path"])
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(tmp.path().join("target"))
        .status()?;
    assert!(status.success(), "docs_run smoke failed");
    Ok(())
}

#[test]
fn xsd_constant_and_validate_align_with_docs() -> Result<(), Box<dyn std::error::Error>> {
    assert!(SBE_XSD.contains("messageSchema"));
    validate_against_sbe_xsd(&docs_schema_xml())?;
    Ok(())
}

// Prefer std tempfile without adding a dep if not present — use cargo's temp.
// Check if tempfile is available via dev-deps.
mod tempfile {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    pub struct TempDir(PathBuf);
    impl TempDir {
        pub fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    pub fn tempdir() -> std::io::Result<TempDir> {
        let id = N.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ergo_sbe_docs_{}_{}", std::process::id(), id));
        std::fs::create_dir_all(&p)?;
        Ok(TempDir(p))
    }
}
