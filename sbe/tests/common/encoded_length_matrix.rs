//! One-crate many-test runner for encoded-length conformance matrices.
//!
//! Writes the generated codec source once, then emits each test as a
//! separately named `#[test] fn`, compiles, and runs them all in one
//! `cargo test` invocation.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// One generated Rust test body.
#[derive(Clone, Debug)]
pub struct GeneratedRustTest {
    /// Snake-case test function name.
    pub name: String,
    /// Full test body (without `fn` wrapper).
    pub body: String,
}

/// Write generated source + tests into a temp crate, compile, and run.
///
/// Returns `Ok(())` if all tests pass, `Err` with combined stdout/stderr
/// on any failure.
pub fn compile_and_run_generated_tests(
    crate_name: &str,
    generated_source: &str,
    tests: &[GeneratedRustTest],
) -> Result<(), Box<dyn std::error::Error>> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tmp = base
        .join("..")
        .join("target")
        .join("encoded_length_matrix")
        .join(crate_name);

    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("src"))?;

    let mut seen = std::collections::HashSet::new();
    for t in tests {
        if t.name.is_empty() {
            return Err("test name must not be empty".into());
        }
        if !t.name.chars().next().unwrap().is_ascii_lowercase() && !t.name.starts_with('_') {
            return Err(format!(
                "invalid test name '{}': must start with lowercase or _",
                t.name
            )
            .into());
        }
        if !seen.insert(&t.name) {
            return Err(format!("duplicate test name '{}'", t.name).into());
        }
    }

    // Cargo.toml — CARGO_MANIFEST_DIR is the sbe crate root
    let cargo_toml = format!(
        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
ergo-sbe = {{ path = "{}", features = ["compact_str", "smol_str", "bytes"] }}

[lib]
path = "src/lib.rs"
"#,
        base.display(),
    );
    fs::write(tmp.join("Cargo.toml"), cargo_toml)?;

    // lib.rs: include generated source + all tests
    let mut lib_rs = String::new();
    lib_rs.push_str("#![allow(dead_code, unused_imports, unused_variables, clippy::all, clippy::pedantic, clippy::restriction, unused)]\n");
    lib_rs.push_str("mod codec {\n");
    lib_rs.push_str(generated_source);
    lib_rs.push_str("\n}\n");
    lib_rs.push_str("pub use codec::*;\n\n");

    for t in tests {
        lib_rs.push_str(&format!(
            "#[test]\nfn {}() -> Result<(), Box<dyn std::error::Error>> {{\n{}\n    Ok(())\n}}\n\n",
            t.name, t.body
        ));
    }

    fs::write(tmp.join("src").join("lib.rs"), lib_rs)?;

    let output = Command::new("cargo")
        .args(["test", "--", "--test-threads=1"])
        .current_dir(&tmp)
        .output()
        .map_err(|e| format!("failed to run cargo test: {e}"))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!(
            "matrix tests failed for {crate_name}:\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
        ).into());
    }

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_names() {
        let tests = vec![
            GeneratedRustTest {
                name: "a".into(),
                body: "assert!(true);".into(),
            },
            GeneratedRustTest {
                name: "a".into(),
                body: "assert!(true);".into(),
            },
        ];
        let result = compile_and_run_generated_tests("dup_test", "// empty", &tests);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn rejects_empty_name() {
        let tests = vec![GeneratedRustTest {
            name: "".into(),
            body: "".into(),
        }];
        let result = compile_and_run_generated_tests("empty_test", "// empty", &tests);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_identifier() {
        let tests = vec![GeneratedRustTest {
            name: "1Bad".into(),
            body: "".into(),
        }];
        let result = compile_and_run_generated_tests("invalid_test", "// empty", &tests);
        assert!(result.is_err());
    }
}
