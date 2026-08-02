//! HFT-010: reject stale 0.1 dual-lane teaching in generated code AND inventory docs.
//!
//! Generated sources: no public `try_wrap*` / safe raw unchecked helpers.
//! Inventory surfaces: forbid teaching phrases that reintroduce the 0.1
//! try/trusted story. Migration / changelog / release-spec prose is allowlisted.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

mod common;
use common::{Paths, generate};

const STALE_GENERATED: &[&str] = &[
    "pub fn try_wrap(",
    "pub fn try_wrap_and_apply_header(",
    "pub fn read_bytes_unchecked",
    "pub fn write_bytes_unchecked",
];

/// Substrings that must not appear in inventory docs (case-sensitive where noted).
const STALE_DOC_SUBSTRINGS: &[&str] = &[
    "try_* untrusted",
    "try vs trusted",
    "try/trusted",
    "try_* trust boundary",
    "trusted wrap",
    "trusted direct wraps",
    "for trusted buffers",
    "Infallible wrap for trusted",
    "infallible wrap for trusted",
    "skips the header",
    "read garbage",
    "garbage rather than returning an error",
    "car.verify()?",
    "car.verify()",
    "try_wrap for untrusted",
    "checked try_* entry",
    "wrap trusted",
    "`try_*` untrusted",
];

fn allowlisted(path: &Path) -> bool {
    let s = path.to_string_lossy().replace('\\', "/");
    s.contains("MIGRATION_0_1_TO_0_2")
        || s.contains("CHANGELOG.md")
        || s.contains("SBE_HFT_0_2_RELEASE_SPEC")
        || s.contains("docs/research/")
        || s.contains("sbe-hft-architecture-primary-sources")
        || s.ends_with("hft_stale_interface_test.rs")
        || s.ends_with("hft_001_soundness_test.rs")
        || s.ends_with("hft_005_warning_free_consumer_test.rs")
        || s.ends_with("fix_sbe_conformance_test.rs")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sbe parent")
        .to_path_buf()
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "generated"
        {
            continue;
        }
        if p.is_dir() {
            walk_files(&p, out);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "md" | "rs") {
                out.push(p);
            }
        }
    }
}

fn inventory_paths(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for r in [
        root.join("book/src"),
        root.join("samples"),
        root.join("sbe/benchmarks"),
        root.join("sbe/src"),
        root.join("docs/evidence"),
    ] {
        if r.is_dir() {
            walk_files(&r, &mut files);
        }
    }
    for f in [
        root.join("sbe/README.md"),
        root.join("sbe/BENCHMARKS.md"),
        root.join("SECURITY.md"),
        root.join("AI-ASSISTANCE.md"),
        root.join("docs/SBE_COMPATIBILITY.md"),
        root.join("samples/README.md"),
    ] {
        if f.is_file() {
            files.push(f);
        }
    }
    files
}

fn line_is_negation(line: &str) -> bool {
    let l = line.to_lowercase();
    l.contains("removed")
        || l.contains("no public")
        || l.contains("there is **no**")
        || l.contains("there is no public")
        || l.contains("forbid")
        || l.contains("reject")
        || l.contains("aliases are")
        || l.contains("must not")
        || l.contains("do not retain")
        || l.contains("keep=false")
        || l.contains("module-private")
        || l.contains("not `car.verify")
        || l.contains("rather than `car.verify")
        || l.contains("associated `decoder::verify")
        || l.contains("associated decoder::verify")
        || l.contains("use associated")
        || l.contains("hft-010")
            && (l.contains("reject") || l.contains("forbid") || l.contains("stale"))
}

#[test]
fn car_generated_source_rejects_stale_0_1_names() -> Result<(), Box<dyn Error>> {
    let (_schema, src) = generate(&Paths::example_schema(), "stale_iface");
    for needle in STALE_GENERATED {
        assert!(
            !src.contains(needle),
            "stale interface still generated: {needle}"
        );
    }
    assert!(src.contains("pub fn wrap("), "checked wrap missing");
    assert!(
        src.contains("pub fn wrap_and_apply_header("),
        "checked wah missing"
    );
    assert!(src.contains("pub fn decode("), "decode missing");
    assert!(
        src.contains("pub unsafe fn wrap_and_apply_header_unchecked")
            || src.contains("unsafe fn wrap_and_apply_header_unchecked"),
        "unchecked twin missing"
    );
    Ok(())
}

#[test]
fn inventory_docs_reject_0_1_dual_lane_teaching() -> Result<(), Box<dyn Error>> {
    let root = workspace_root();
    let files = inventory_paths(&root);
    let mut failures: Vec<String> = Vec::new();

    for path in &files {
        if allowlisted(path) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if line_is_negation(line) {
                continue;
            }
            for phrase in STALE_DOC_SUBSTRINGS {
                if line.contains(phrase) {
                    failures.push(format!(
                        "{}:{}: `{phrase}` → {line}",
                        path.strip_prefix(&root).unwrap_or(path).display(),
                        i + 1
                    ));
                }
            }
            // Special: infallible wrap schematic for trusted buffers
            let l = line.to_lowercase();
            if l.contains("wrap(")
                && l.contains("-> self")
                && (l.contains("trusted") || l.contains("infallible"))
            {
                failures.push(format!(
                    "{}:{}: infallible/trusted wrap → {line}",
                    path.strip_prefix(&root).unwrap_or(path).display(),
                    i + 1
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "HFT-010 inventory still teaches 0.1 dual-lane / wrong APIs ({} hits):\n{}",
        failures.len(),
        failures.join("\n")
    );
    Ok(())
}
