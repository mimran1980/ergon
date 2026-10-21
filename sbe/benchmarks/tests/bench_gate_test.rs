//! Regression coverage for the Criterion ratio gate.
//!
//! The gate is what makes a published head-to-head number mean anything, so its
//! own failure modes are pinned here:
//!
//! - it reads the estimator Criterion actually displays (regression slope when
//!   one exists, otherwise the **mean** — `Estimates::typical()`, criterion
//!   0.8.2 `estimate.rs:116`), never the raw sample median;
//! - the SBE ceiling is literal — `1.0000` passes, anything above fails, and a
//!   caller cannot pass a tolerance that waves a regression through;
//! - a missing pair is a failure, never a silent skip;
//! - with `--run-id`, results that this run did not produce are refused:
//!   unstamped, mixed-run, stale, or an incomplete manifest all fail closed;
//! - the producer (`CRITERION_HOME`) and the consumer (the gate's directory
//!   argument) are the same path by construction;
//! - the harness's noise is **repeated**, not read off one control: identical
//!   code walks between invocations by more than a within-run confidence
//!   interval, so a pair graded on one invocation claims resolution it does
//!   not have;
//! - a repeat snapshot carries only estimates its own invocation produced —
//!   the Criterion home is shared with other bench binaries, and leftovers
//!   from one would widen this run's envelope with foreign evidence.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    ("parity_decode_full_message", "ergo-sbe_ordered", "sbe-tool"),
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
            r#"{{"slope":{{"point_estimate":{slope}}},"mean":{{"point_estimate":{slope}}},"median":{{"point_estimate":{median}}}}}"#
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
    manifest_for(run_id, "no-lto")
}

fn manifest_for(run_id: &str, profile: &str) -> String {
    format!(
        r#"{{"run_id":"{run_id}","profile":"{profile}","commit":"abc","rustc":"rustc 1.0.0","target":"x86_64","host":"Test CPU"}}"#
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

/// Criterion writes `slope` as JSON `null` for flat-sampling benchmarks: it
/// runs no regression, so the key is present with no value. The gate's reader
/// used `dict.get('slope', default)`, which returns that null rather than the
/// default — the pair then reported "missing estimates" and one run failed
/// thirteen of them at once. Flat sampling is how the suite keeps each sample
/// to a few milliseconds.
///
/// Criterion reports **the mean** when it has no slope, not the median:
/// `Estimates::typical()` is `self.slope.as_ref().unwrap_or(&self.mean)`
/// (criterion 0.8.2 `src/estimate.rs:116`), and the console `time:` line is
/// printed from `typical()` (`src/report.rs:564,576`). Both are written here
/// so a fixture can make them disagree.
fn write_estimate_flat(
    root: &Path,
    group: &str,
    function: &str,
    mean: f64,
    median: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = estimate_dir(root, group, function);
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("estimates.json"),
        format!(
            r#"{{"slope":null,"mean":{{"point_estimate":{mean}}},"median":{{"point_estimate":{median}}}}}"#
        ),
    )?;
    Ok(())
}

/// An estimate carrying a confidence interval, at an explicit arm directory.
/// The A/A null band and every ratio CI are derived from these, so a fixture
/// that has to exercise the "no established winner" rule must write them.
fn write_estimate_into(
    dir: &Path,
    slope: f64,
    ci_lo: f64,
    ci_hi: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join("estimates.json"),
        format!(
            r#"{{"slope":{{"point_estimate":{slope},"confidence_interval":{{"lower_bound":{ci_lo},"upper_bound":{ci_hi}}}}},"mean":{{"point_estimate":{slope},"confidence_interval":{{"lower_bound":{ci_lo},"upper_bound":{ci_hi}}}}},"median":{{"point_estimate":{slope}}}}}"#
        ),
    )?;
    Ok(())
}

fn write_estimate_ci(
    root: &Path,
    group: &str,
    function: &str,
    slope: f64,
    ci_lo: f64,
    ci_hi: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    write_estimate_into(&estimate_dir(root, group, function), slope, ci_lo, ci_hi)
}

/// A raw `estimates.json` body at an explicit arm directory, for the cases a
/// well-formed writer cannot produce: truncated JSON, a `NaN` the run never
/// measured, a negative count.
fn write_estimate_body(dir: &Path, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("estimates.json"), body)?;
    Ok(())
}

/// `repeats/<k>/<group>/<function>`: one arm of one independent invocation.
fn repeat_arm(root: &Path, k: usize, group: &str, function: &str) -> PathBuf {
    root.join("repeats")
        .join(k.to_string())
        .join(group)
        .join(function)
}

/// `n` independent invocations, every maintained pair in every one of them,
/// each arm at `slope` with the same confidence half-width `ci`.
fn write_repeats(
    root: &Path,
    n: usize,
    ergo: f64,
    reference: f64,
    ci: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    for k in 1..=n {
        for (group, ergo_fn, ref_fn) in PAIRS {
            write_estimate_into(
                &repeat_arm(root, k, group, ergo_fn),
                ergo,
                ergo - ci,
                ergo + ci,
            )?;
            write_estimate_into(
                &repeat_arm(root, k, group, ref_fn),
                reference,
                reference - ci,
                reference + ci,
            )?;
        }
    }
    Ok(())
}

/// Stamp every arm directory that holds estimates, control included.
fn stamp_everything(root: &Path, run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    fn stamp_arms(dir: &Path, run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            if path.join("estimates.json").is_file() {
                fs::write(path.join("run-id.txt"), run_id)?;
            }
            stamp_arms(&path, run_id)?;
        }
        Ok(())
    }
    stamp_arms(root, run_id)
}

/// The SBE suite's harness control: two arms running identical work. Their
/// ratio is the harness's own noise.
const SBE_AA_GROUP: &str = "parity_control_aa_encode_throughput";

fn write_sbe_aa_control(
    root: &Path,
    arm_a: (f64, f64, f64),
    arm_b: (f64, f64, f64),
) -> Result<(), Box<dyn std::error::Error>> {
    write_estimate_ci(root, SBE_AA_GROUP, "arm_a", arm_a.0, arm_a.1, arm_a.2)?;
    write_estimate_ci(root, SBE_AA_GROUP, "arm_b", arm_b.0, arm_b.1, arm_b.2)?;
    Ok(())
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

#[test]
fn bench_gate_falls_back_to_the_mean_when_slope_is_null() -> Result<(), Box<dyn std::error::Error>>
{
    let criterion = TempCriterion::new()?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_flat(&criterion.0, group, ergo, 90.0, 90.0)?;
        write_estimate_flat(&criterion.0, group, reference, 100.0, 100.0)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        output.status.success(),
        "gate failed on flat-sampled estimates (slope: null) instead of falling \
         back to the mean:\n{}",
        describe(&output)
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("missing estimates"),
        "gate reported missing estimates for a flat-sampled run that has means:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn bench_gate_fails_a_mean_breach_when_slope_is_null() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_flat(&criterion.0, group, ergo, 102.0, 102.0)?;
        write_estimate_flat(&criterion.0, group, reference, 100.0, 100.0)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "a flat-sampled mean ratio of 1.02 must fail the 1.00 ceiling:\n{}",
        describe(&output)
    );
    Ok(())
}

/// The estimator the gate reads has to be the estimator Criterion displays.
/// Criterion's `Estimates::typical()` is `slope.unwrap_or(mean)` — never the
/// median — so a fixture whose means say 2.0 while its medians say 1.0 must
/// fail. Reading the median would grade the wrong statistic and let a real
/// regression through: these samples are right-skewed (17% high outliers is
/// ordinary for this suite), so the mean sits well above the median.
#[test]
fn bench_gate_reads_the_mean_not_the_median_when_slope_is_null()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    for (group, ergo, reference) in PAIRS {
        // Means say ergo is 2.0x the reference; medians say they are equal.
        write_estimate_flat(&criterion.0, group, ergo, 200.0, 100.0)?;
        write_estimate_flat(&criterion.0, group, reference, 100.0, 100.0)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "the gate graded the median (1.0) instead of the mean (2.0) that \
         Criterion reports, and passed a 2.0x regression:\n{}",
        describe(&output)
    );
    Ok(())
}

/// A calculation that cannot run is not a pass. The caller invokes
/// `check_ratio ... || failures=$((failures + 1))`, which suppresses `set -e`
/// around the call, so a Python traceback left `status` empty and fell through
/// to `return 0` — five ratios at 2.0x printed tracebacks and the run still
/// exited 0 with PASS. Fail closed instead.
#[test]
fn bench_gate_fails_closed_when_the_ratio_calculation_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_flat(&criterion.0, group, ergo, 100.0, 100.0)?;
        write_estimate_flat(&criterion.0, group, reference, 100.0, 100.0)?;
    }
    // Malformed point estimates: `float()` raises, the script prints a
    // traceback, and no `@STATUS` line is produced.
    for (group, ergo, reference) in PAIRS {
        let dir = estimate_dir(&criterion.0, group, ergo);
        fs::write(
            dir.join("estimates.json"),
            r#"{"slope":null,"mean":{"point_estimate":"not-a-number"},"median":{"point_estimate":1.0}}"#,
        )?;
        let dir = estimate_dir(&criterion.0, group, reference);
        fs::write(
            dir.join("estimates.json"),
            r#"{"slope":null,"mean":{"point_estimate":100.0},"median":{"point_estimate":100.0}}"#,
        )?;
    }

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "a failed ratio calculation reported PASS. A gate that cannot compute \
         a ratio must not certify it:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── A/A null band ──────────────────────────────────────────────────────────
//
// The A/A control is two arms running identical work, so its ratio is the
// harness's own noise. `check-bench-gate.sh` states the contract twice — in
// `aa_band` and in `report_aa_null`: "a pair inside it has no established
// winner". These tests hold the reader to that contract. They are not a way
// to pass a breach: a pair outside a measured band, or a pair with no
// measured control at all, still fails closed.

#[test]
fn a_pair_inside_a_measured_aa_null_band_establishes_no_winner()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    // Identical arms, each ±1 %: the ratio CI the gate derives spans about
    // 0.98–1.02, which is the harness's resolution for this loop.
    write_sbe_aa_control(&criterion.0, (100.0, 99.0, 101.0), (100.0, 99.0, 101.0))?;
    for (group, ergo, reference) in PAIRS {
        // A 2 % ratio with tight CIs: above the ceiling as a point estimate,
        // but inside what the harness produces for identical code.
        write_estimate_ci(&criterion.0, group, ergo, 102.0, 101.5, 102.5)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 99.5, 100.5)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "a pair inside a measured A/A null band has no established winner and must \
         not be graded as a breach:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("no winner"),
        "the gate must say no winner is established, not silently pass:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("A/A null"),
        "the gate must report the A/A null it graded against:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_breach_outside_the_aa_null_band_still_fails() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    // A tight control: the harness resolves these arms to within 0.02 %.
    write_sbe_aa_control(&criterion.0, (100.0, 99.99, 100.01), (100.0, 99.99, 100.01))?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_ci(&criterion.0, group, ergo, 105.0, 104.9, 105.1)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 99.9, 100.1)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "a 5 % breach outside a tight A/A null band must still fail:\n{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("outside A/A null band"),
        "the gate must show the breach is outside the null band, so it cannot be \
         waved away as harness noise:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_breach_without_a_measured_aa_control_still_fails() -> Result<(), Box<dyn std::error::Error>> {
    // Fail closed: with no control in the run there is no measured noise band,
    // so nothing may excuse a ratio above its ceiling.
    let criterion = TempCriterion::new()?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_ci(&criterion.0, group, ergo, 102.0, 101.9, 102.1)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 99.9, 100.1)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "a breach with no A/A control measured must fail closed:\n{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("A/A null not measured"),
        "the gate must say the null was not measured rather than imply the pair \
         was inside a band:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_pair_below_its_ceiling_is_not_made_inconclusive_by_a_wide_aa_band()
-> Result<(), Box<dyn std::error::Error>> {
    // A wide control must not turn a clean win into a shrug. Only a ratio that
    // actually exceeds its ceiling can be inconclusive.
    let criterion = TempCriterion::new()?;
    write_sbe_aa_control(&criterion.0, (100.0, 95.0, 105.0), (100.0, 95.0, 105.0))?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_ci(&criterion.0, group, ergo, 90.0, 89.5, 90.5)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 99.5, 100.5)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "a ratio of 0.90 is within every ceiling and must pass:\n{}",
        describe(&output)
    );
    assert!(
        !stdout.contains("no winner established"),
        "a ratio well below its ceiling is resolved, not inconclusive:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("resolved"),
        "the gate must report the ratio as resolved:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── Uncertainty is not a shrug ─────────────────────────────────────────────
//
// "Inconclusive" is the harness saying it cannot separate two arms at its own
// resolution. It is only that when a **measured** A/A null band reaches as far
// as the pair. An over-ceiling pair the measurement simply cannot resolve —
// a wide interval, no control, a foreign control — is unproven, and unproven
// is not a pass.

#[test]
fn an_over_ceiling_pair_with_a_wide_interval_and_no_control_fails()
-> Result<(), Box<dyn std::error::Error>> {
    // A 2x ratio whose interval is so wide it straddles the ceiling. With no
    // control there is no band to excuse it, so nothing establishes the
    // ceiling was met — and a 2x point estimate is not "no winner".
    let criterion = TempCriterion::new()?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_ci(&criterion.0, group, ergo, 200.0, 50.0, 400.0)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 90.0, 110.0)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "a 2x ratio with a wide interval and no A/A control must fail — the ceiling \
         is not demonstrated:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("A/A null not measured"),
        "the gate must say the null was not measured:\n{}",
        describe(&output)
    );
    assert!(
        !stdout.contains("inconclusive"),
        "an unproven pair must not be reported as inconclusive:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn an_over_ceiling_pair_inside_a_measured_band_establishes_no_winner()
-> Result<(), Box<dyn std::error::Error>> {
    // The one case that is legitimately inconclusive: the measured harness
    // noise for this loop really reaches as far as the pair, so identical code
    // would look the same.
    let criterion = TempCriterion::new()?;
    write_sbe_aa_control(&criterion.0, (100.0, 50.0, 200.0), (100.0, 50.0, 200.0))?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_ci(&criterion.0, group, ergo, 200.0, 50.0, 400.0)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 90.0, 110.0)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "a pair the measured A/A null band reaches is inconclusive, not a breach:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("no winner"),
        "the gate must say no winner is established:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("inside A/A null band"),
        "the gate must show the pair was graded against a measured band:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_foreign_control_cannot_excuse_a_breach() -> Result<(), Box<dyn std::error::Error>> {
    // The control is evidence like any other. A band read off estimates this
    // run did not produce would turn any regression into "harness noise".
    // The pairs are this run's; only the control is foreign.
    let criterion = TempCriterion::new()?;
    write_sbe_aa_control(&criterion.0, (100.0, 50.0, 200.0), (100.0, 50.0, 200.0))?;
    for (group, ergo, reference) in PAIRS {
        write_estimate_ci(&criterion.0, group, ergo, 200.0, 50.0, 400.0)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 90.0, 110.0)?;
    }
    write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;
    for (group, ergo_fn, ref_fn) in PAIRS {
        for function in [ergo_fn, ref_fn] {
            fs::write(
                estimate_dir(&criterion.0, group, function).join("run-id.txt"),
                RUN_ID,
            )?;
        }
    }
    for function in ["arm_a", "arm_b"] {
        fs::write(
            estimate_dir(&criterion.0, SBE_AA_GROUP, function).join("run-id.txt"),
            "foreign",
        )?;
    }

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "a control from another run must not excuse a 2x ratio:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("mixed-run") || stdout.contains("run id"),
        "the gate must name the provenance failure:\n{}",
        describe(&output)
    );
    assert!(
        !stdout.contains("inconclusive"),
        "a foreign band must not be reported as inconclusive:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── Repetition completeness ────────────────────────────────────────────────
//
// The envelope is only the envelope of the run if every declared invocation is
// in it. A pair graded on a subset of the invocations claims a resolution the
// run does not have.

#[test]
fn a_malformed_repetition_is_refused_not_dropped() -> Result<(), Box<dyn std::error::Error>> {
    // Valid first and third invocations, truncated JSON in the second of **both**
    // arms of one pair. Reading only what parses grades that pair on two of the
    // three invocations the run declared — and two identical arms look like a
    // clean pass. The gate must refuse the pair and name the invocation.
    let criterion = TempCriterion::new()?;
    write_repeats(&criterion.0, 3, 1.0, 1.0, 0.001)?;
    for function in ["ergo-sbe", "sbe-tool"] {
        write_estimate_body(
            &repeat_arm(&criterion.0, 2, "parity_decode_scalar", function),
            r#"{"slope":{"point_estimate":1.0,"confidence_inter"#,
        )?;
    }
    write_manifest(&criterion.0, &manifest_with_repeats(RUN_ID, "no-lto", 3))?;
    stamp_everything(&criterion.0, RUN_ID)?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "a malformed repetition must fail the pair, not vanish:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("malformed")
            || stdout.contains("unreadable")
            || stdout.contains("repetition"),
        "the gate must name the unreadable invocation:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_pair_missing_one_declared_repetition_is_incomplete_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    // Three invocations declared, both arms of one pair absent from the
    // second. Every other pair is complete, so the run-level count still
    // reads 3 — but this pair has 2, and grading it on 2 while the run claims
    // 3 is exactly the thinner envelope that overstates resolution.
    let criterion = TempCriterion::new()?;
    write_repeats(&criterion.0, 3, 1.0, 1.0, 0.001)?;
    for (group, ergo_fn, ref_fn) in PAIRS {
        if *group == "parity_decode_scalar" {
            let _ = fs::remove_dir_all(repeat_arm(&criterion.0, 2, group, ergo_fn));
            let _ = fs::remove_dir_all(repeat_arm(&criterion.0, 2, group, ref_fn));
        }
    }
    write_manifest(&criterion.0, &manifest_with_repeats(RUN_ID, "no-lto", 3))?;
    stamp_everything(&criterion.0, RUN_ID)?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "a pair graded on fewer invocations than the run declared must fail:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("incomplete evidence") || stdout.contains("repetition"),
        "the gate must name the missing invocation:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn a_non_finite_or_non_positive_estimate_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    // `NaN` and a negative estimate are not measurements. Python's JSON reader
    // accepts both, and a reader that trusts them grades a ratio on them.
    for body in [
        r#"{"slope":{"point_estimate":NaN},"mean":{"point_estimate":NaN}}"#,
        r#"{"slope":{"point_estimate":-1.0},"mean":{"point_estimate":-1.0}}"#,
        r#"{"slope":{"point_estimate":0.0},"mean":{"point_estimate":0.0}}"#,
    ] {
        let criterion = TempCriterion::new()?;
        for (group, ergo, reference) in PAIRS {
            write_estimate_ci(&criterion.0, group, ergo, 1.0, 1.0, 1.0)?;
            write_estimate_ci(&criterion.0, group, reference, 1.0, 1.0, 1.0)?;
        }
        write_estimate_body(
            &estimate_dir(&criterion.0, "parity_decode_scalar", "ergo-sbe"),
            body,
        )?;
        write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;
        stamp_everything(&criterion.0, RUN_ID)?;

        let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
        assert!(
            !output.status.success(),
            "an estimate of {body} is not a measurement and must fail:\n{}",
            describe(&output)
        );
    }
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

/// Pair-table rows are quoted `"…|ceiling"` strings. The last `|` field is
/// the numeric ceiling.
fn parse_maintained_ceilings(script: &str) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    for line in script.lines() {
        let trimmed = line.trim().trim_end_matches(',').trim();
        if !trimmed.starts_with('"') || !trimmed.ends_with('"') {
            continue;
        }
        let inner = &trimmed[1..trimmed.len() - 1];
        let Some((head, ceil)) = inner.rsplit_once('|') else {
            continue;
        };
        let Ok(value) = ceil.parse::<f64>() else {
            continue;
        };
        let label = head.split('|').next().unwrap_or(head);
        out.push((label.to_string(), value));
    }
    out
}

#[test]
fn a_fixture_script_containing_one_thousandth_over_one_is_detected() {
    let snippet = r#"
    pairs=(
        "decode_scalar|decode_scalar|ergo-sbe|sbe-tool|1.001"
    )
    "#;
    let ceilings = parse_maintained_ceilings(snippet);
    assert!(
        ceilings.iter().any(|(_, c)| *c > 1.00),
        "parser must treat |1.001 as above 1.00, got {ceilings:?}"
    );
}

// Documented, narrow noise-floor exceptions — see the matching comment in
// scripts/check-bench-gate.sh. 2026-08-21: 9 consecutive bench-cluster runs,
// including one on a genuinely quiet machine, measured these two pairs
// straddling 1.00 without converging. Adding a label here is not a rubber
// stamp — it requires the same empirical evidence and rationale comment as
// these two, not just a build that happened to fail once.
const NOISE_FLOOR_CEILING_EXCEPTIONS: &[&str] = &[
    "cluster_decode_session_message_header",
    // also carries a no-LTO-only override; see PROFILE_SCOPED_OVERRIDE_MAX
    "cluster_decode_session_event",
];

/// Documented maximum for each profile-scoped override, per label. Each entry
/// records a measured tie, so the bound is per-label rather than global — one
/// allowance must never widen the bound the others are held to.
const PROFILE_SCOPED_OVERRIDE_MAX: &[(&str, f64)] = &[
    // A genuine tie: both arms read the same five scalars plus `detail_slice`,
    // already batched 10k, so there is no unread member to amplify with. Five
    // isolated runs on an idle machine measured 1.0038 / 0.9997 / 0.9994 /
    // 0.9994 / 0.9988 — mean 1.0002, 0.5% spread. A failure here usually means
    // the machine was loaded: a mid-release run of this same pair measured
    // 1.0104 while other work was in flight. Re-run on a quiet machine before
    // believing it.
    ("cluster_decode_session_event", 1.05),
];

/// Ceilings raised for a single profile are applied by a conditional in the
/// gate script, not by the pairs table, so `parse_maintained_ceilings` cannot
/// see them. Audit them directly: every such override must name an allowlisted
/// label and stay within its documented maximum in `PROFILE_SCOPED_OVERRIDE_MAX`,
/// or a ceiling could creep up unnoticed — which is precisely what the ceiling
/// audit exists to prevent.
#[test]
fn profile_scoped_ceiling_overrides_are_allowlisted_and_bounded()
-> Result<(), Box<dyn std::error::Error>> {
    let script = include_str!("../../../scripts/check-bench-gate.sh");
    let mut seen = 0usize;
    for (i, line) in script.lines().enumerate() {
        let trimmed = line.trim();
        // The SBE loop names its variable `$label`, the cluster loop `$group`.
        // Match both — an override the audit cannot see is an unbounded one.
        if !trimmed.starts_with("if [ \"$profile\" =") {
            continue;
        }
        let var = ["$label", "$group"]
            .into_iter()
            .find(|v| trimmed.contains(*v))
            .ok_or_else(|| {
                format!(
                    "line {}: profile-scoped override keys on no known variable",
                    i + 1
                )
            })?;
        seen += 1;
        let label = trimmed
            .split(&format!("\"{var}\" = \""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .ok_or_else(|| format!("line {}: cannot read the override label", i + 1))?;
        assert!(
            NOISE_FLOOR_CEILING_EXCEPTIONS.contains(&label),
            "line {}: {label} has a profile-scoped ceiling override but is not in \
             NOISE_FLOOR_CEILING_EXCEPTIONS",
            i + 1
        );
        let max = PROFILE_SCOPED_OVERRIDE_MAX
            .iter()
            .find(|(l, _)| *l == label)
            .map(|(_, m)| *m)
            .ok_or_else(|| format!("line {}: {label} has no documented maximum", i + 1))?;
        let ceiling: f64 = script
            .lines()
            .skip(i + 1)
            .find_map(|l| l.trim().strip_prefix("ceiling=\""))
            .and_then(|rest| rest.split('"').next())
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("line {}: cannot read the override ceiling", i + 1))?;
        assert!(
            ceiling <= max,
            "line {}: {label} override ceiling {ceiling} exceeds its documented maximum {max}",
            i + 1
        );
    }
    assert!(
        seen > 0,
        "expected at least one profile-scoped ceiling override to audit; if the last \
         one was removed, delete this test and its allowlist entry"
    );
    Ok(())
}

#[test]
fn no_maintained_ceiling_exceeds_one() {
    let script = include_str!("../../../scripts/check-bench-gate.sh");
    let ceilings = parse_maintained_ceilings(script);
    assert!(
        !ceilings.is_empty(),
        "maintained pair tables must declare numeric ceilings"
    );
    for (label, ceiling) in &ceilings {
        if NOISE_FLOOR_CEILING_EXCEPTIONS.contains(&label.as_str()) {
            assert!(
                *ceiling <= 1.01,
                "{label} is an allowlisted noise-floor exception, but {ceiling} exceeds even 1.01"
            );
            continue;
        }
        assert!(*ceiling <= 1.00, "{label} ceiling {ceiling} exceeds 1.00");
    }
}

/// Build a tree where one pair sits at `ratio`, under `profile`.
fn tree_with_ratio(
    profile: &str,
    group: &str,
    ratio: f64,
) -> Result<TempCriterion, Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    write_estimate(
        &criterion.0,
        group,
        "ergo-sbe",
        100.0 * ratio,
        100.0 * ratio,
    )?;
    write_manifest(&criterion.0, &manifest_for(RUN_ID, profile))?;
    Ok(criterion)
}

const NULLIFY: &str = "parity_extended_optional_enum_nullify";

/// `extended_optional_enum_nullify` is literal 1.00 in both profiles. The
/// arms share four independent totals of the same three reads, so a ratio
/// above 1.00 is a real loss, not the old single-total placement tie.
#[test]
fn nullify_above_one_fails_under_no_lto() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = tree_with_ratio("no-lto", NULLIFY, 1.005)?;
    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "nullify at 1.005 must fail the 1.00 no-LTO ceiling:\n{}",
        describe(&output)
    );
    Ok(())
}

/// A wider miss fails the same way.
#[test]
fn nullify_further_above_one_fails_under_no_lto() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = tree_with_ratio("no-lto", NULLIFY, 1.02)?;
    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "nullify at 1.02 must fail the 1.00 no-LTO ceiling:\n{}",
        describe(&output)
    );
    Ok(())
}

/// LTO uses the same 1.00 ceiling as no-LTO.
#[test]
fn nullify_above_one_fails_under_lto() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = tree_with_ratio("lto", NULLIFY, 1.005)?;
    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "nullify at 1.005 must fail the 1.00 LTO ceiling:\n{}",
        describe(&output)
    );
    Ok(())
}

/// A wider LTO miss fails the same way.
#[test]
fn nullify_further_above_one_fails_under_lto() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = tree_with_ratio("lto", NULLIFY, 1.02)?;
    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "nullify at 1.02 must fail the 1.00 LTO ceiling:\n{}",
        describe(&output)
    );
    Ok(())
}

/// The allowance is for one *pair* only: every other comparison stays literal
/// 1.00 in both profiles.
#[test]
fn another_pair_barely_above_one_still_fails_under_no_lto() -> Result<(), Box<dyn std::error::Error>>
{
    let criterion = tree_with_ratio("no-lto", "parity_decode_scalar", 1.005)?;
    let output = run_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "the nullify allowance must not apply to any other pair:\n{}",
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

#[test]
fn a_manifest_without_the_host_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    // `target` is `aarch64-apple-darwin` on an M1 and on an M4. Figures taken
    // on one are not figures from the other, so a manifest that cannot say
    // which machine produced them cannot support a comparison.
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_run_ids(&criterion.0, RUN_ID)?;
    write_manifest(
        &criterion.0,
        &format!(
            r#"{{"run_id":"{RUN_ID}","profile":"no-lto","commit":"abc","rustc":"rustc 1.0.0","target":"aarch64-apple-darwin"}}"#
        ),
    )?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        !output.status.success(),
        "a manifest that cannot say which machine produced the numbers must be refused:\n{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("host"),
        "the refusal must name the missing host:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn the_provenance_line_names_the_host() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_pairs(&criterion.0, 100.0, 100.0)?;
    stamp_run_ids(&criterion.0, RUN_ID)?;
    write_manifest(&criterion.0, &complete_manifest(RUN_ID))?;

    let output = run_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        output.status.success(),
        "correctly stamped results must pass:\n{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("host Test CPU"),
        "the gate must print the machine the numbers came from, so a reader can \
         tell an M1 run from an M4 one:\n{}",
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

#[test]
fn cluster_decode_ratio_barely_above_one_fails() -> Result<(), Box<dyn std::error::Error>> {
    // These two pairs carry a documented 1.01 noise-floor exception (see
    // check-bench-gate.sh) — 1.005 legitimately passes now; assert the gate
    // still catches something just over the *widened* ceiling instead.
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    for (group, ergo_fn) in [
        ("cluster_decode_session_message_header", "ergo-sbe"),
        ("cluster_decode_session_event", "ergo-sbe"),
    ] {
        write_estimate(&criterion.0, group, ergo_fn, 101.5, 101.5)?;
    }

    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "cluster decode at 1.015 must fail even the widened 1.01 ceiling:\n{}",
        describe(&output)
    );
    Ok(())
}

/// `cluster_decode_session_event` carries a **no-LTO only** override above its
/// 1.01 table ceiling: a memory-bound decode whose no-LTO arm walks either side
/// of parity (observed to 1.0444) while LTO stays ahead at 0.99. See the
/// rationale block in `scripts/check-bench-gate.sh`.
#[test]
fn cluster_session_event_within_the_no_lto_override_passes()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    write_estimate(
        &criterion.0,
        "cluster_decode_session_event",
        "ergo-sbe",
        104.0,
        104.0,
    )?;
    write_manifest(&criterion.0, &manifest_for(RUN_ID, "no-lto"))?;

    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        output.status.success(),
        "1.04 is inside the documented 1.05 no-LTO override:\n{}",
        describe(&output)
    );
    Ok(())
}

/// The override is bounded — past 1.05 it is a regression, not a tie.
#[test]
fn cluster_session_event_beyond_the_no_lto_override_fails() -> Result<(), Box<dyn std::error::Error>>
{
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    write_estimate(
        &criterion.0,
        "cluster_decode_session_event",
        "ergo-sbe",
        106.0,
        106.0,
    )?;
    write_manifest(&criterion.0, &manifest_for(RUN_ID, "no-lto"))?;

    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "1.06 exceeds the 1.05 no-LTO override and must fail:\n{}",
        describe(&output)
    );
    Ok(())
}

/// LTO keeps the 1.01 table ceiling — the no-LTO override must not leak.
#[test]
fn cluster_session_event_override_does_not_leak_into_lto() -> Result<(), Box<dyn std::error::Error>>
{
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    write_estimate(
        &criterion.0,
        "cluster_decode_session_event",
        "ergo-sbe",
        104.0,
        104.0,
    )?;
    write_manifest(&criterion.0, &manifest_for(RUN_ID, "lto"))?;

    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "the no-LTO override must not apply under LTO:\n{}",
        describe(&output)
    );
    Ok(())
}

/// And it must not leak to the sibling pair that shares the 1.01 exception.
#[test]
fn cluster_session_message_header_keeps_its_own_ceiling_under_no_lto()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    write_estimate(
        &criterion.0,
        "cluster_decode_session_message_header",
        "ergo-sbe",
        104.0,
        104.0,
    )?;
    write_manifest(&criterion.0, &manifest_for(RUN_ID, "no-lto"))?;

    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "the session_event override must not apply to session_message_header:\n{}",
        describe(&output)
    );
    Ok(())
}

#[test]
fn cluster_decode_ratio_within_the_documented_exception_passes()
-> Result<(), Box<dyn std::error::Error>> {
    // The flip side of the test above: 1.005 sits inside the documented
    // noise-floor exception for these two pairs and must pass, proving the
    // allowlist in check-bench-gate.sh and its matching self-test in
    // no_maintained_ceiling_exceeds_one actually take effect end to end.
    let criterion = TempCriterion::new()?;
    write_all_cluster_pairs(&criterion.0, 100.0, 100.0)?;
    for (group, ergo_fn) in [
        ("cluster_decode_session_message_header", "ergo-sbe"),
        ("cluster_decode_session_event", "ergo-sbe"),
    ] {
        write_estimate(&criterion.0, group, ergo_fn, 100.5, 100.5)?;
    }

    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        output.status.success(),
        "cluster decode at 1.005 must pass the documented 1.01 noise-floor exception:\n{}",
        describe(&output)
    );
    Ok(())
}

// ── Repeated runs ──────────────────────────────────────────────────────────
//
// A within-run confidence interval is not the harness's resolution. On
// 2026-10-20 three invocations of *unchanged* cluster code produced
// `cluster_decode_session_message_header` ratios of 1.0236, 0.9916 and 0.9887
// — a 3.5 % swing where each run's own ratio CI was ±0.5 %. The identity of
// the "resolved breach" rotated through the pairs on every run. Grading one
// invocation therefore claims a resolution the harness does not have.
//
// The runner repeats the measurement; the gate grades the **envelope** of the
// repeats and the envelope of a repeated A/A control. A pair separated from
// its ceiling across every repeat and from the control's envelope across every
// repeat is resolved. Anything else establishes no winner.

/// One arm's estimate in one independent invocation: `(point, ci_lo, ci_hi)`.
type Arm = (f64, f64, f64);

/// Where repeat `k` of a key's estimates lives, and where the single-run
/// layout puts it. Both are read by the gate; the repeat tree wins when it is
/// present, so a fixture that writes only one of them pins the fallback.
fn repeat_estimate_dir(root: &Path, repeat: usize, group: &str, function: &str) -> PathBuf {
    root.join("repeats")
        .join(repeat.to_string())
        .join(group)
        .join(function)
}

fn write_arm(dir: &Path, arm: Arm) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    let (slope, ci_lo, ci_hi) = arm;
    fs::write(
        dir.join("estimates.json"),
        format!(
            r#"{{"slope":{{"point_estimate":{slope},"confidence_interval":{{"lower_bound":{ci_lo},"upper_bound":{ci_hi}}}}},"mean":{{"point_estimate":{slope},"confidence_interval":{{"lower_bound":{ci_lo},"upper_bound":{ci_hi}}}}},"median":{{"point_estimate":{slope}}}}}"#
        ),
    )?;
    Ok(())
}

fn write_repeat_arm(
    root: &Path,
    repeat: usize,
    group: &str,
    function: &str,
    arm: Arm,
) -> Result<(), Box<dyn std::error::Error>> {
    write_arm(&repeat_estimate_dir(root, repeat, group, function), arm)
}

/// A pair across `ergo.len()` independent invocations, plus the single-run
/// snapshot the un-repeated reader would grade. That snapshot is `at`, an
/// index into `ergo`: the runner leaves the last invocation in `new/`, and on
/// this host it was the *last* of three that carried the false breach.
fn write_pair_repeats(
    root: &Path,
    group: &str,
    ergo_fn: &str,
    ref_fn: &str,
    ergo: &[Arm],
    reference: &[Arm],
    at: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for (k, (e, r)) in ergo.iter().zip(reference.iter()).enumerate() {
        write_repeat_arm(root, k, group, ergo_fn, *e)?;
        write_repeat_arm(root, k, group, ref_fn, *r)?;
    }
    write_arm(&estimate_dir(root, group, ergo_fn), ergo[at])?;
    write_arm(&estimate_dir(root, group, ref_fn), reference[at])?;
    Ok(())
}

/// The cluster suite's harness controls, repeated. Each is two arms running
/// identical work, so their ratio is the harness's own noise at that place in
/// the run; the gate takes the widest envelope over all of them.
const CLUSTER_AA_GROUPS: &[&str] = &[
    "cluster_control_aa_encode_keep_alive",
    "cluster_control_aa_encode_claim_shaped",
];

fn write_cluster_aa_repeats(root: &Path, arms: &[Arm]) -> Result<(), Box<dyn std::error::Error>> {
    for group in CLUSTER_AA_GROUPS {
        for (k, arm) in arms.iter().enumerate() {
            write_repeat_arm(root, k, group, "arm_a", *arm)?;
            write_repeat_arm(root, k, group, "arm_b", *arm)?;
        }
        write_arm(&estimate_dir(root, group, "arm_a"), arms[arms.len() - 1])?;
        write_arm(&estimate_dir(root, group, "arm_b"), arms[arms.len() - 1])?;
    }
    Ok(())
}

/// The cluster pairs the fixture is not exercising: held identical in every
/// invocation, so only the pair under test can move the verdict.
fn write_cluster_stable_repeats(
    root: &Path,
    repeats: usize,
    skip: &str,
    ergo: Arm,
    reference: Arm,
) -> Result<(), Box<dyn std::error::Error>> {
    for (group, ergo_fn, ref_fn) in CLUSTER_PAIRS {
        if *group == skip {
            continue;
        }
        for k in 0..repeats {
            write_repeat_arm(root, k, group, ergo_fn, ergo)?;
            write_repeat_arm(root, k, group, ref_fn, reference)?;
        }
        write_arm(&estimate_dir(root, group, ergo_fn), ergo)?;
        write_arm(&estimate_dir(root, group, ref_fn), reference)?;
    }
    Ok(())
}

fn manifest_with_repeats(run_id: &str, profile: &str, repeats: usize) -> String {
    format!(
        r#"{{"run_id":"{run_id}","profile":"{profile}","commit":"abc","rustc":"rustc 1.0.0","target":"x86_64","host":"Test CPU","repeats":{repeats}}}"#
    )
}

/// Identical code walked 3.5 % across three invocations of the same binary on
/// 2026-10-20, while each invocation's own ratio CI was ±0.5 %. The run that
/// landed last carried 1.0236 and was graded a "resolved breach"; the two
/// either side of it measured 0.9916 and 0.9887 for the same instructions.
/// Grading the envelope of the five reproduces that walk and must establish
/// no winner — the gate cannot separate the arms at this resolution, and a
/// pair it cannot separate is not a pair it has beaten.
#[test]
fn a_pair_that_walks_between_repeats_establishes_no_winner()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    // The walk, as measured: under the ceiling four times, over it once.
    let ergo: Vec<Arm> = [
        (99.2, 98.9, 99.5),
        (98.9, 98.6, 99.2),
        (99.1, 98.8, 99.4),
        (99.3, 99.0, 99.6),
        (102.4, 101.6, 103.1),
    ]
    .into_iter()
    .collect();
    let reference: Vec<Arm> = [
        (100.0, 99.7, 100.3),
        (100.0, 99.7, 100.3),
        (100.0, 99.7, 100.3),
        (100.0, 99.7, 100.3),
        (100.0, 99.7, 100.3),
    ]
    .into_iter()
    .collect();
    write_cluster_aa_repeats(
        &criterion.0,
        &[
            (100.0, 99.7, 100.3),
            (100.0, 99.7, 100.3),
            (100.2, 99.9, 100.5),
            (100.1, 99.8, 100.4),
            (100.4, 100.1, 100.7),
        ],
    )?;
    write_cluster_stable_repeats(
        &criterion.0,
        5,
        "cluster_decode_session_message_header",
        (100.0, 99.7, 100.3),
        (100.0, 99.7, 100.3),
    )?;
    write_pair_repeats(
        &criterion.0,
        "cluster_decode_session_message_header",
        "ergo-sbe",
        "sbe-tool",
        &ergo,
        &reference,
        4,
    )?;

    let output = run_cluster_gate(&criterion.0, &[])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "a pair whose ratio walked 0.989 to 1.024 across five invocations of unchanged \
         code establishes no winner and must not fail:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("no winner"),
        "the gate must say no winner is established, not pass silently:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("5 repeats"),
        "the gate must report how many invocations its uncertainty spans, so a reader \
         can see the verdict is not one run's confidence interval:\n{}",
        describe(&output)
    );
    Ok(())
}

/// The other half of the contract: repetition must not excuse a real
/// regression. Five invocations that all agree the arm is 5 % over its ceiling,
/// against a control that never moves, is a resolved breach and must fail.
#[test]
fn a_breach_stable_across_repeats_still_fails() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_cluster_aa_repeats(&criterion.0, &[(100.0, 99.99, 100.01); 5])?;
    write_cluster_stable_repeats(
        &criterion.0,
        5,
        "cluster_encode_session_message_header",
        (100.0, 99.99, 100.01),
        (100.0, 99.99, 100.01),
    )?;
    let ergo = [(105.0, 104.9, 105.1); 5];
    let reference = [(100.0, 99.99, 100.01); 5];
    write_pair_repeats(
        &criterion.0,
        "cluster_encode_session_message_header",
        "ergo-sbe",
        "sbe-tool",
        &ergo,
        &reference,
        4,
    )?;

    let output = run_cluster_gate(&criterion.0, &[])?;
    assert!(
        !output.status.success(),
        "a 5 % breach repeated in every invocation must still fail:\n{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("resolved breach"),
        "the gate must resolve the breach rather than shrug at it:\n{}",
        describe(&output)
    );
    Ok(())
}

/// A repeated control is not decoration. The disposition already records that
/// one control understates placement sensitivity — the same ergon loop
/// measured 2.7 % apart in two groups of one run. When the control's own
/// invocations reach as far as the pair's offset, the harness has shown it
/// cannot separate the arms, and the pair is inconclusive even though it sits
/// above its ceiling every time.
#[test]
fn a_pair_overlapping_a_repeated_aa_control_establishes_no_winner()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    // The pair is stably 2 % over; the control reaches past that on its own.
    write_sbe_aa_control(&criterion.0, (100.0, 99.99, 100.01), (100.0, 99.99, 100.01))?;
    for repeat in 0..5_usize {
        let (a, b) = match repeat {
            0 => ((100.0, 99.7, 100.3), (101.0, 100.7, 101.3)),
            1 => ((100.0, 99.7, 100.3), (100.2, 99.9, 100.5)),
            2 => ((100.0, 99.7, 100.3), (98.5, 98.2, 98.8)),
            3 => ((100.0, 99.7, 100.3), (97.8, 97.5, 98.1)),
            _ => ((100.0, 99.7, 100.3), (97.2, 96.9, 97.5)),
        };
        write_repeat_arm(&criterion.0, repeat, SBE_AA_GROUP, "arm_a", a)?;
        write_repeat_arm(&criterion.0, repeat, SBE_AA_GROUP, "arm_b", b)?;
    }
    for (group, ergo, reference) in PAIRS {
        for repeat in 0..5_usize {
            write_repeat_arm(&criterion.0, repeat, group, ergo, (102.0, 101.9, 102.1))?;
            write_repeat_arm(&criterion.0, repeat, group, reference, (100.0, 99.9, 100.1))?;
        }
        write_estimate_ci(&criterion.0, group, ergo, 102.0, 101.9, 102.1)?;
        write_estimate_ci(&criterion.0, group, reference, 100.0, 99.9, 100.1)?;
    }

    let output = run_gate(&criterion.0, &[])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "a pair whose 2 % offset lies inside what a repeated control shows identical \
         code does on its own establishes no winner and must not fail:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("no winner"),
        "the gate must say no winner is established:\n{}",
        describe(&output)
    );
    assert!(
        stdout.contains("A/A null"),
        "the gate must report the null it graded against:\n{}",
        describe(&output)
    );
    Ok(())
}

/// The runner declares how many invocations it produced. A manifest claiming
/// five with two on disk is incomplete evidence, and the gate must refuse it
/// rather than grade the thinner envelope it happens to find.
#[test]
fn fewer_repeats_than_the_manifest_declares_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_cluster_aa_repeats(&criterion.0, &[(100.0, 99.7, 100.3); 2])?;
    write_cluster_stable_repeats(
        &criterion.0,
        2,
        "",
        (100.0, 99.7, 100.3),
        (100.0, 99.7, 100.3),
    )?;
    write_manifest(&criterion.0, &manifest_with_repeats(RUN_ID, "lto", 5))?;
    stamp_cluster_run_ids(&criterion.0, RUN_ID)?;

    let output = run_cluster_gate(&criterion.0, &["--run-id", RUN_ID])?;
    assert!(
        !output.status.success(),
        "a manifest declaring 5 invocations with 2 on disk must be refused:\n{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("repeats"),
        "the refusal must name the missing repeats, not fail for an unrelated reason:\n{}",
        describe(&output)
    );
    Ok(())
}

/// The verdict a pair is graded on is the envelope of its invocations, so the
/// figure on the graded line has to be that envelope. The headline used to
/// print the *last* invocation's ratio while the ok / above-ceiling label was
/// decided by the maximum across all of them, which on this host produced
/// `0.9941 (max 1.00) above ceiling` beside `over 5 repeats 0.9941–1.0046`:
/// a reader sees a passing number marked as a breach.
#[test]
fn the_headline_figure_is_the_one_the_verdict_uses() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    // Last invocation under the ceiling, earlier ones over it: the two figures
    // the old line could disagree about.
    let ergo: Vec<Arm> = [
        (100.4, 100.3, 100.5),
        (100.3, 100.2, 100.4),
        (100.2, 100.1, 100.3),
        (100.1, 100.0, 100.2),
        (99.8, 99.7, 99.9),
    ]
    .into_iter()
    .collect();
    let reference: Vec<Arm> = [(100.0, 99.9, 100.1); 5].into_iter().collect();
    write_cluster_aa_repeats(&criterion.0, &[(100.0, 99.99, 100.01); 5])?;
    write_cluster_stable_repeats(
        &criterion.0,
        5,
        "cluster_encode_session_message_header",
        (100.0, 99.99, 100.01),
        (100.0, 99.99, 100.01),
    )?;
    write_pair_repeats(
        &criterion.0,
        "cluster_encode_session_message_header",
        "ergo-sbe",
        "sbe-tool",
        &ergo,
        &reference,
        4,
    )?;

    let output = run_cluster_gate(&criterion.0, &[])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let headline = stdout
        .lines()
        .find(|line| line.contains("cluster_encode_session_message_header") && line.contains("max"))
        .ok_or_else(|| format!("no graded line for the pair:\n{}", describe(&output)))?;
    assert!(
        headline.contains("1.0040"),
        "the graded line must show the figure the verdict uses (the highest invocation) \
         so a reader can see why it is labelled as it is, not only the last one:\n  {headline}\n{}",
        describe(&output)
    );
    assert!(
        headline.contains("0.9980"),
        "the graded line must also show the low end of the span — a single number \
         hides that the invocations disagree:\n  {headline}\n{}",
        describe(&output)
    );
    Ok(())
}

fn write_stale_estimate(
    root: &Path,
    group: &str,
    function: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    write_estimate(root, group, function, 100.0, 100.0)
}

fn set_mtime(path: &Path, when: SystemTime) -> Result<(), Box<dyn std::error::Error>> {
    let handle = fs::File::options().write(true).open(path)?;
    handle.set_times(fs::FileTimes::new().set_accessed(when).set_modified(when))?;
    Ok(())
}

fn run_snapshot(dir: &Path, k: usize) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(
        Command::new(repository()?.join("scripts/snapshot-bench-repeat.sh"))
            .arg(dir)
            .arg(k.to_string())
            .output()?,
    )
}

/// `target/criterion` is shared: `ergon-clock-bench` writes `clock_*` into the
/// same Criterion home, and Criterion leaves `new/` in place for groups a run
/// does not touch. A snapshot of "everything under the home" therefore folds
/// another binary's estimates into this run's envelope and widens it with
/// evidence the invocation never produced. The runner marks the start of the
/// invocation; only estimates at or after that mark belong to it.
#[test]
fn a_snapshot_keeps_only_estimates_this_invocation_produced()
-> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    let now = SystemTime::now();
    write_stale_estimate(&criterion.0, "clock_mono_read", "ergon-clock")?;
    write_stale_estimate(
        &criterion.0,
        "cluster_encode_session_message_header",
        "ergo-sbe",
    )?;
    let marker = criterion.0.join(".bench-since");
    fs::write(&marker, b"")?;
    // The foreign key is from an earlier run of a different bench binary; the
    // cluster key is what this invocation is producing right now.
    set_mtime(
        &criterion
            .0
            .join("clock_mono_read/ergon-clock/new/estimates.json"),
        now - Duration::from_secs(600),
    )?;
    set_mtime(
        &criterion
            .0
            .join("cluster_encode_session_message_header/ergo-sbe/new/estimates.json"),
        now + Duration::from_secs(600),
    )?;
    set_mtime(&marker, now)?;

    let output = run_snapshot(&criterion.0, 1)?;
    assert!(
        output.status.success(),
        "the snapshot of one invocation must succeed:\n{}",
        describe(&output)
    );
    assert!(
        criterion
            .0
            .join("repeats/1/cluster_encode_session_message_header/ergo-sbe/estimates.json")
            .is_file(),
        "the group this invocation produced must be kept:\n{}",
        describe(&output)
    );
    assert!(
        !criterion.0.join("repeats/1/clock_mono_read").exists(),
        "a group left behind by a different bench binary must not enter this run's \
         envelope — it is not evidence this invocation produced:\n{}",
        describe(&output)
    );
    Ok(())
}

/// Without the marker there is no way to tell this invocation's estimates from
/// another binary's leftovers, so the snapshot is refused rather than taken.
#[test]
fn a_snapshot_without_a_producer_marker_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let criterion = TempCriterion::new()?;
    write_stale_estimate(
        &criterion.0,
        "cluster_encode_session_message_header",
        "ergo-sbe",
    )?;

    let output = run_snapshot(&criterion.0, 1)?;
    assert!(
        !output.status.success(),
        "a snapshot that cannot prove which estimates this invocation produced \
         must be refused:\n{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("marker"),
        "the refusal must name the missing marker:\n{}",
        describe(&output)
    );
    assert!(
        !criterion.0.join("repeats").exists(),
        "a refused snapshot must write nothing:\n{}",
        describe(&output)
    );
    Ok(())
}
