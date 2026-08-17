//! Regression coverage for the Criterion ratio gate.
//!
//! The gate is what makes a published head-to-head number mean anything, so its
//! own failure modes are pinned here:
//!
//! - it reads the estimator Criterion actually displays (regression slope), not
//!   the raw sample median;
//! - the SBE ceiling is literal — `1.0000` passes, anything above fails, and a
//!   caller cannot pass a tolerance that waves a regression through;
//! - a missing pair is a failure, never a silent skip;
//! - with `--run-id`, results that this run did not produce are refused:
//!   unstamped, mixed-run, stale, or an incomplete manifest all fail closed;
//! - the producer (`CRITERION_HOME`) and the consumer (the gate's directory
//!   argument) are the same path by construction.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
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
    // The gate builds its lookup key as `parity_${group//\//_}`, so the
    // `extended/...` groups land here as a single `parity_extended_...` dir.
    (
        "parity_extended_optional_enum_nullify",
        "ergo-sbe",
        "sbe-tool",
    ),
    ("parity_extended_group_with_data", "ergo-sbe", "sbe-tool"),
];

const RUN_ID: &str = "b20260807T000000Z-1234-abcdef123456";

struct TempCriterion(PathBuf);

impl TempCriterion {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // These tests run as threads in one process, so the pid is shared and a
        // timestamp alone can collide — two fixtures landing in one directory
        // would let one test's manifest satisfy another's gate invocation. The
        // counter makes the path unique regardless of clock granularity.
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "ergo-sbe-bench-gate-{}-{nonce}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
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

fn repository() -> Result<&'static Path, Box<dyn std::error::Error>> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "benchmark crate must be nested under the repository".into())
}

fn estimate_dir(root: &Path, group: &str, function: &str) -> PathBuf {
    root.join(group).join(function).join("new")
}

fn write_estimate(
    root: &Path,
    group: &str,
    function: &str,
    slope: f64,
    median: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = estimate_dir(root, group, function);
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("estimates.json"),
        format!(
            r#"{{"slope":{{"point_estimate":{slope}}},"median":{{"point_estimate":{median}}}}}"#
        ),
    )?;
    Ok(())
}

/// Every maintained pair, with ergon at `ergo` and sbe-tool at `reference`.
fn write_all_pairs(
    root: &Path,
    ergo: f64,
    reference: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    for (group, ergo_fn, ref_fn) in PAIRS {
        write_estimate(root, group, ergo_fn, ergo, ergo)?;
        write_estimate(root, group, ref_fn, reference, reference)?;
    }
    Ok(())
}

fn stamp_run_ids(root: &Path, run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    for (group, ergo_fn, ref_fn) in PAIRS {
        for function in [ergo_fn, ref_fn] {
            fs::write(
                estimate_dir(root, group, function).join("run-id.txt"),
                run_id,
            )?;
        }
    }
    Ok(())
}

fn write_manifest(root: &Path, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(root.join("run-manifest.json"), body)?;
    Ok(())
}

fn complete_manifest(run_id: &str) -> String {
    format!(
        r#"{{"run_id":"{run_id}","profile":"no-lto","commit":"abc","rustc":"rustc 1.0.0","target":"x86_64"}}"#
    )
}

const CLUSTER_PAIRS: &[(&str, &str, &str)] = &[
    (
        "cluster_encode_session_message_header",
        "ergo-sbe",
        "sbe-tool",
    ),
    ("cluster_encode_session_keep_alive", "ergo-sbe", "sbe-tool"),
    (
        "cluster_decode_session_message_header",
        "ergo-sbe",
        "sbe-tool",
    ),
    ("cluster_decode_session_event", "ergo-sbe", "sbe-tool"),
    (
        "cluster_encode_claim_shaped_header_plus_app",
        "ergo-sbe",
        "sbe-tool",
    ),
];

fn run_gate(dir: &Path, extra: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(
        Command::new(repository()?.join("scripts/check-bench-gate.sh"))
            .arg(dir)
            .args(["0", "sbe"])
            .args(extra)
            .output()?,
    )
}

fn run_cluster_gate(dir: &Path, extra: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(
        Command::new(repository()?.join("scripts/check-bench-gate.sh"))
            .arg(dir)
            .args(["0.5", "cluster"])
            .args(extra)
            .output()?,
    )
}

fn write_all_cluster_pairs(
    root: &Path,
    ergo: f64,
    reference: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    for (group, ergo_fn, ref_fn) in CLUSTER_PAIRS {
        write_estimate(root, group, ergo_fn, ergo, ergo)?;
        write_estimate(root, group, ref_fn, reference, reference)?;
    }
    Ok(())
}

fn stamp_cluster_run_ids(root: &Path, run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    for (group, ergo_fn, ref_fn) in CLUSTER_PAIRS {
        for function in [ergo_fn, ref_fn] {
            fs::write(
                estimate_dir(root, group, function).join("run-id.txt"),
                run_id,
            )?;
        }
    }
    Ok(())
}

fn describe(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ── Estimator ──────────────────────────────────────────────────────────────

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

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        output.status.success(),
        "gate did not use Criterion's displayed regression estimate:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── Literal 1.00 ceiling ───────────────────────────────────────────────────

#[test]
fn exactly_equal_performance_passes() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        output.status.success(),
        "a ratio of exactly 1.0000 is at the ceiling and must pass:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_ratio_barely_above_one_fails() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    // 1.0001 — the kind of margin a noise tolerance used to absorb.
    write_all_pairs(&criterion.0, 100.01, 100.0)?;

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "the SBE ceiling is literal — anything above 1.00 must fail:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_caller_supplied_tolerance_cannot_loosen_the_sbe_gate() -> Result<(), Box<dyn std::error::Error>>
{
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.4, 100.0)?; // ratio 1.004

    // A generous tolerance in argument position must be ignored for SBE.
    let output = Command::new(repository()?.join("scripts/check-bench-gate.sh"))
        .arg(&criterion.0)
        .args(["0.5", "sbe"])
        .output()?;
    assert!(
        !output.status.success(),
        "SBE runs at zero tolerance regardless of the tolerance argument:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── Noise-floor ceiling for `optional_enum_nullify` ───────────────────────
//
// That scenario decodes two raw 1-byte enums — memory-bound, already optimal
// in both crates, so under LTO it is a tie (~775ns, 0.06% apart inside
// Criterion CI). A 1.00 ceiling there is a coin-flip noise decides. It carries
// a documented 1.01 ceiling instead (see check-bench-gate.sh); these tests pin
// that boundary so a silent revert to 1.00 is caught.

#[test]
fn nullify_tie_passes_at_one_percent_but_no_more() -> Result<(), Box<dyn std::error::Error>> {
    // Ratio 1.005 — above every 1.00 ceiling, below nullify's 1.01. Every
    // other pair sits at 1.00, so the gate's verdict hinges on nullify alone.
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    write_estimate(
        &criterion.0,
        "parity_extended_optional_enum_nullify",
        "ergo-sbe",
        100.5,
        100.5,
    )?;

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        output.status.success(),
        "nullify at 1.005 is a documented tie and must pass under its 1.01 ceiling:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn nullify_still_fails_above_its_noise_floor() -> Result<(), Box<dyn std::error::Error>> {
    // Ratio 1.015 — above nullify's 1.01 ceiling, so it must still fail.
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    write_estimate(
        &criterion.0,
        "parity_extended_optional_enum_nullify",
        "ergo-sbe",
        101.5,
        101.5,
    )?;

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "nullify at 1.015 exceeds its 1.01 ceiling and must fail:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_missing_pair_fails_rather_than_silently_passing() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    // Drop one arm of one maintained pair.
    let (group, ergo_fn, _) = PAIRS[0];
    fs::remove_dir_all(estimate_dir(&criterion.0, group, ergo_fn))?;

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "an absent maintained scenario must fail, never be skipped:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── Provenance ─────────────────────────────────────────────────────────────

#[test]
fn stamped_results_from_the_expected_run_pass() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_run_ids(&criterion.0, RUN_ID)?;
    write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        output.status.success(),
        "correctly stamped results from this run must pass:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn unstamped_results_are_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;
    // No per-estimate run-id.txt: results predate the provenance mechanism or
    // came from somewhere else entirely.

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        !output.status.success(),
        "estimates without a run id must be refused:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_missing_manifest_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_run_ids(&criterion.0, RUN_ID)?;
    // Estimates are stamped, but nothing records what produced the directory.

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        !output.status.success(),
        "a profile directory with no run manifest must be refused:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn an_incomplete_manifest_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_run_ids(&criterion.0, RUN_ID)?;
    // Right run id, but nothing identifying the toolchain or commit — the
    // fields that make a recorded number reproducible.
    write_manifest(&criterion.0, &format!(r#"{{"run_id":"{RUN_ID}"}}"#))?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        !output.status.success(),
        "a manifest missing commit/rustc/target must be refused:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_stale_manifest_run_id_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_run_ids(&criterion.0, RUN_ID)?;
    write_manifest(&criterion.0, &complete_manifest("b20260101T000000Z-1-old"))?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        !output.status.success(),
        "a manifest from an earlier run must be refused:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn mixed_run_ids_across_estimates_are_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_run_ids(&criterion.0, RUN_ID)?;
    write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;
    // One arm re-stamped from a different run: the classic way a flattering
    // ratio survives into a later report.
    let (group, _, ref_fn) = PAIRS[0];
    fs::write(
        estimate_dir(&criterion.0, group, ref_fn).join("run-id.txt"),
        "b20260101T000000Z-1-old",
    )?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        !output.status.success(),
        "a ratio built from two different runs must be refused:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── Producer / consumer path identity ──────────────────────────────────────

#[test]
fn the_runner_gates_exactly_the_directory_it_produces() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(repository()?.join("scripts/run-sbe-bench.sh"))
        .args(["--print-plan", RUN_ID])
        .output()?;
    assert!(
        output.status.success(),
        "the runner must be able to print its plan:\n{}",
        describe(&output)
    );

    let plan = String::from_utf8(output.stdout)?;
    let mut profiles = Vec::new();
    for line in plan.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split('\t');
        let profile = parts.next().ok_or("plan line has no profile")?;
        let producer = parts
            .next()
            .and_then(|f| f.strip_prefix("producer="))
            .ok_or("plan line has no producer path")?;
        let gate = parts
            .next()
            .and_then(|f| f.strip_prefix("gate="))
            .ok_or("plan line has no gate path")?;
        assert_eq!(
            producer, gate,
            "{profile}: the benchmark writes to {producer} but the gate would read {gate} — \
             a gate reading a different directory proves nothing about this run"
        );
        assert!(
            producer.contains(RUN_ID),
            "{profile}: results must live under the run id, got {producer}"
        );
        profiles.push(profile.to_string());
    }

    assert_eq!(
        profiles,
        vec!["no-lto".to_string(), "lto".to_string()],
        "both optimisation profiles are blocking, so both must appear in the plan"
    );
    Ok(())
}

// ── Cluster provenance + literal 1.00 ──────────────────────────────────────

#[test]
fn cluster_stamped_results_from_the_expected_run_pass() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_cluster_run_ids(&criterion.0, RUN_ID)?;
    write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;

    let output = run_cluster_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        output.status.success(),
        "correctly stamped cluster results from this run must pass:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn cluster_wrong_run_id_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_cluster_run_ids(&criterion.0, RUN_ID)?;
    write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;

    let output = run_cluster_gate(&criterion.0, &["--run-id", "deliberately-wrong"])?;
    assert!(
        !output.status.success(),
        "a wrong --run-id must fail the cluster gate instead of scoring a stale tree:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn cluster_a_ratio_barely_above_one_fails_even_with_caller_tolerance()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.4, 100.0)?;

    // 0.5 is passed as the tolerance argument — cluster must ignore it.
    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "cluster runs at zero tolerance regardless of the tolerance argument:\n{}",
        describe(&output)
    );
    Ok(())
}
