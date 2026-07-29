//! Regression coverage for the Criterion ratio-gate estimator.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const PAIRS: &[(&str, &str, &str)] = &[
    ("parity_decode_scalar", "ergo-sbe", "sbe-tool"),
    ("parity_decode_array", "ergo-sbe", "sbe-tool"),
    (
        "parity_decode_composite",
        "ergo-sbe_engine",
        "sbe-tool_engine",
    ),
    (
        "parity_decode_full_message",
        "ergo-sbe_consuming",
        "sbe-tool",
    ),
    (
        "parity_decode_entry_point",
        "ergo-sbe_wrap",
        "sbe-tool_wrap",
    ),
    (
        "parity_encode_scalar",
        "ergo-sbe_header_and_body",
        "sbe-tool_header_and_body",
    ),
    (
        "parity_encode_scalar",
        "ergo-sbe_body_only",
        "sbe-tool_body_only",
    ),
    ("parity_encode_throughput_10k", "ergo-sbe", "sbe-tool"),
    ("parity_throughput_batch_10k", "ergo-sbe", "sbe-tool"),
    ("parity_wire_parity_encode_full", "ergo-sbe", "sbe-tool"),
];

struct TempCriterion(PathBuf);

impl TempCriterion {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "ergo-sbe-bench-gate-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TempCriterion {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_estimate(
    root: &Path,
    group: &str,
    function: &str,
    slope: f64,
    median: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = root.join(group).join(function).join("new");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("estimates.json"),
        format!(
            r#"{{"slope":{{"point_estimate":{slope}}},"median":{{"point_estimate":{median}}}}}"#
        ),
    )?;
    Ok(())
}

#[test]
fn bench_gate_uses_criterion_displayed_regression_estimate()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    for (group, ergo, reference) in PAIRS {
        // The displayed Criterion regression says Ergo is faster, while the
        // raw sample median says the opposite. The gate must not mix them.
        write_estimate(&criterion.0, group, ergo, 90.0, 110.0)?;
        write_estimate(&criterion.0, group, reference, 100.0, 100.0)?;
    }

    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("benchmark crate must be nested under the repository")?;
    let output = Command::new(repository.join("scripts/check-bench-gate.sh"))
        .arg(&criterion.0)
        .args(["0", "sbe"])
        .output()?;

    assert!(
        output.status.success(),
        "gate did not use Criterion's displayed regression estimate:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
