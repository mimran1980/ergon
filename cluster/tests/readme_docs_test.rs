#![allow(missing_docs)]
//! Compile-check every `rust` / `rust,no_run` fence in `cluster/README.md`.
//! `rust,ignore` fences are rejected — use `rust,no_run` for Aeron-dependent
//! examples that cannot execute in the test harness.

// Shared scratch-crate target dir (tests/ is excluded from the package).
// Formatted with sbe/'s rustfmt settings, not this crate's.
#[rustfmt::skip]
#[path = "../../sbe/tests/common/scratch.rs"]
mod scratch;

use scratch::ScratchCrate;
use std::path::Path;

/// Extract `rust` and `rust,no_run` fences from a markdown file.
/// Returns `(fence_content, fence_info_tag)` for each fence.
fn extract_fences(md: &str) -> Vec<(String, String)> {
    let mut fences = Vec::new();
    let mut in_fence = false;
    let mut fence_info = String::new();
    let mut fence_content = String::new();

    for line in md.lines() {
        if line.starts_with("```") && !in_fence {
            in_fence = true;
            fence_info = line.trim_start_matches("```").trim().to_string();
            fence_content.clear();
        } else if line.starts_with("```") && in_fence {
            in_fence = false;
            let info_lower = fence_info.to_lowercase();
            if info_lower == "rust" || info_lower.starts_with("rust,") {
                let tag = if info_lower.contains("ignore") {
                    "rust,ignore"
                } else {
                    "rust"
                };
                fences.push((fence_content.clone(), tag.to_string()));
            }
        } else if in_fence {
            fence_content.push_str(line);
            fence_content.push('\n');
        }
    }
    fences
}

#[test]
fn readme_fences_compile() -> Result<(), Box<dyn std::error::Error>> {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let md = std::fs::read_to_string(&readme_path)?;
    let fences = extract_fences(&md);

    assert!(!fences.is_empty(), "README.md must contain at least one Rust fence");

    // Reject rust,ignore — use rust,no_run for non-executable examples
    assert!(
        !fences.iter().any(|(_, tag)| tag == "rust,ignore"),
        "README.md contains rust,ignore fence(s); use rust,no_run instead"
    );

    let to_compile: Vec<_> = fences.iter().filter(|(_, tag)| tag == "rust").collect();

    for (i, (content, _)) in to_compile.iter().enumerate() {
        // Compile as a temporary crate
        let krate = ScratchCrate::new(&format!("readme_test_{i}"), false);
        let tmp = std::env::temp_dir().join(krate.name());
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src"))?;

        let cluster_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        std::fs::write(
            tmp.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
ergo-aeron-cluster = {{ path = "{}" }}
"#,
                krate.name(),
                cluster_dir.display(),
            ),
        )?;
        krate.write_lockfile(&tmp);

        // Wrap no_run fences in a main function if they don't have one
        let src = if content.contains("fn main()") {
            content.clone()
        } else {
            format!("fn main() {{\n{content}}}\n")
        };
        std::fs::write(tmp.join("src/main.rs"), src)?;

        let output = krate
            .cargo()
            .args(["check", "--quiet", "--manifest-path"])
            .arg(tmp.join("Cargo.toml"))
            .output()?;
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(
            output.status.success(),
            "README.md fence #{} failed to compile:\n{}\n--- stderr ---\n{}",
            i + 1,
            content,
            String::from_utf8_lossy(&output.stderr),
        );
    }

    Ok(())
}
