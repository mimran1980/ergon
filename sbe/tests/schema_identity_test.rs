//! T-100: schema identity is one `Ir`; old public fields do not compile.

use std::error::Error;
use std::fs;
use std::process::Command;

fn compile_snippet(name: &str, main_rs: &str) -> Result<(bool, String), Box<dyn Error>> {
    let sbe_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("ergo_sbe_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src"))?;
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
ergo-sbe = {{ path = "{}" }}
"#,
            sbe_dir.display()
        ),
    )?;
    fs::write(dir.join("src/main.rs"), main_rs)?;
    let out = Command::new("cargo")
        .args(["build", "--offline"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", dir.join("target_ci"))
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()?;
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = fs::remove_dir_all(&dir);
    Ok((out.status.success(), stderr))
}

#[test]
fn old_schema_field_access_does_not_compile() -> Result<(), Box<dyn Error>> {
    let (ok, stderr) = compile_snippet(
        "schema_fields_removed",
        r##"
fn main() {
    let ir = ergo_sbe::parse(r#"<?xml version="1.0"?>
        <messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
          </types>
        </messageSchema>"#).unwrap();
    let schema = ergo_sbe::Schema::from_ir(ir);
    let _ = schema.id;
    let _ = schema.package;
    let _ = schema.version;
    let _ = schema.ir;
}
"##,
    )?;
    assert!(!ok, "old Schema field access must fail, stderr:\n{stderr}");
    assert!(
        stderr.contains("no field")
            || stderr.contains("has no field")
            || stderr.contains("schema.id"),
        "expected field-access diagnostic, stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn old_schema_struct_literal_does_not_compile() -> Result<(), Box<dyn Error>> {
    let (ok, stderr) = compile_snippet(
        "schema_literal_removed",
        r#"
fn main() {
    let _ = ergo_sbe::Schema {
        package: "t".into(),
        id: 1,
        version: 0,
        ir: unimplemented!(),
    };
}
"#,
    )?;
    assert!(!ok, "Schema struct literal must fail, stderr:\n{stderr}");
    assert!(
        stderr.contains("package")
            || stderr.contains("private")
            || stderr.contains("cannot initialize"),
        "expected private-field diagnostic, stderr:\n{stderr}"
    );
    Ok(())
}
