//! `scripts/extract_work_counts.py`, the extractor behind the tracked
//! `work-counts.md`, over fixture runs. The counts' contract is `Work` and
//! `SendSchedule` in `src/stats.rs`: a count a run reported is read or named
//! — never dropped, never a minus count — and a run that reported none of a
//! schedule is left out as an honest hole. Harness as `host_scripts.rs`: the
//! script on a fixture directory, asserting on its exit code and its output.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type R = Result<(), Box<dyn std::error::Error>>;

fn extractor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("extract_work_counts.py")
}

fn extract(run_dir: &Path) -> Result<Output, std::io::Error> {
    Command::new("python3")
        .arg(extractor())
        .arg(run_dir)
        .output()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A run with one work table (`work_rows`) and one point's `meta.txt`.
fn fixture(
    tag: &str,
    work_rows: &[&str],
    meta: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("extract-work-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("res/p1"))?;
    let mut tables = String::from(
        "## r1-base-0\n\n### Work behind the percentiles\n\n\
         | | scheduled | issued | completed | late | rejected | censored |\n\
         |---|---|---|---|---|---|---|\n",
    );
    for row in work_rows {
        tables.push_str(row);
        tables.push('\n');
    }
    std::fs::write(dir.join("tables.md"), tables)?;
    std::fs::write(dir.join("res/p1/meta.txt"), meta)?;
    Ok(dir)
}

fn counts(run_dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(run_dir.join("work-counts.md"))?)
}

/// Present but partial is malformed, not an honest hole: a run that wrote
/// `sent` and `batches` alone cannot say whether the traffic was the planned
/// traffic, so the point is not reported and the extractor stops. The missing
/// key is named, as `SendSchedule::from_meta` names it.
#[test]
fn a_partial_sender_schedule_fails_naming_the_missing_key() -> R {
    let dir = fixture(
        "partial-send",
        &["| p1 | 10 | 8 | 7 | 2 | 0 | 1 |"],
        "send.sent=100\nsend.batches=1000\n",
    )?;
    let o = extract(&dir)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = stderr(&o);
    assert!(err.contains("send.late_100us"), "{err}");
    assert!(
        err.contains("is missing: a sender schedule is all or none"),
        "{err}"
    );
    assert!(
        !dir.join("work-counts.md").exists(),
        "malformed evidence still wrote the table"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// A minus count is malformed metadata, named key and value — not a figure
/// that reads as a clean run with a negative share of late batches.
#[test]
fn a_negative_sender_count_fails_naming_key_and_value() -> R {
    let dir = fixture(
        "neg-send",
        &["| p1 | 10 | 8 | 7 | 2 | 0 | 1 |"],
        "send.sent=100\nsend.batches=1000\nsend.late_100us=-80\nsend.late_max_us=178\n",
    )?;
    let o = extract(&dir)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = stderr(&o);
    assert!(err.contains("send.late_100us=-80"), "{err}");
    assert!(err.contains("is not a count"), "{err}");
    assert!(
        !dir.join("work-counts.md").exists(),
        "malformed evidence still wrote the table"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// Unreadable outranks incomplete: a run that wrote `send.batches=lots` and
/// forgot `send.sent` is lying about a number, which outranks the hole. The
/// unreadable key is named first, as `SendSchedule::from_meta` reads it.
#[test]
fn unreadable_sender_evidence_is_named_before_incomplete() -> R {
    let dir = fixture("unreadable-first", &[], "send.batches=lots\n")?;
    let o = extract(&dir)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = stderr(&o);
    assert!(err.contains("send.batches=lots"), "{err}");
    assert!(!err.contains("is missing"), "{err}");
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// A run that wrote none of the keys has no sender account: receive-only, or
/// recorded before the counts were. The point is left out rather than shown
/// as zeros (an honest hole) and the extractor does not fail.
#[test]
fn a_point_with_no_sender_account_stays_an_honest_hole() -> R {
    let dir = fixture(
        "no-send",
        &["| p1 | 10 | 8 | 7 | 2 | 0 | 1 |"],
        "mode=lab\n",
    )?;
    let o = extract(&dir)?;
    assert!(o.status.success(), "{o:?}");
    let out = counts(&dir)?;
    assert!(out.contains("| p1 | 10 | 8 | 7 | 2 | 0 | 1 |"), "{out}");
    assert!(!out.contains("Sender schedule"), "{out}");
    assert!(!out.contains("| p1 | 0 | 0 |"), "{out}");
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// A complete account is reported, with the share of late batches and the
/// verdict the run logs use. A zero-batch run is judged on nothing and keeps
/// "not yet measured" — never a zero percent that reads as clean.
#[test]
fn a_complete_sender_schedule_is_reported_and_a_zero_batch_run_is_not_yet_measured() -> R {
    let dir = fixture(
        "send-verdicts",
        &[],
        "send.sent=3060455\nsend.batches=1586374\nsend.late_100us=39464\nsend.late_max_us=178\n",
    )?;
    let o = extract(&dir)?;
    assert!(o.status.success(), "{o:?}");
    let out = counts(&dir)?;
    assert!(
        out.contains(
            "| p1 | 3060455 | 1586374 | 39464 (2.49%) | 178 | over 1%: not the planned traffic |"
        ),
        "{out}"
    );
    assert!(
        out.contains("1 of 1 points over 1% of batches late: `p1`."),
        "{out}"
    );
    std::fs::remove_dir_all(&dir)?;

    let dir = fixture(
        "zero-batches",
        &[],
        "send.sent=0\nsend.batches=0\nsend.late_100us=0\nsend.late_max_us=0\n",
    )?;
    let o = extract(&dir)?;
    assert!(o.status.success(), "{o:?}");
    let out = counts(&dir)?;
    assert!(
        out.contains("| p1 | 0 | 0 | 0 | 0 | not yet measured |"),
        "{out}"
    );
    assert!(
        out.contains("all 1 points within 1% of batches late."),
        "{out}"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// A work count a run reported but that is not a number — a minus count
/// included — is malformed metadata, named key and value, as `Work::from_meta`
/// names it. Not copied into the tracked table beside the percentiles.
#[test]
fn a_negative_work_count_fails_naming_key_and_value() -> R {
    let dir = fixture("neg-work", &["| p1 | 10 | 8 | -80 | 2 | 0 | 1 |"], "")?;
    let o = extract(&dir)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = stderr(&o);
    assert!(err.contains("work.completed=-80"), "{err}");
    assert!(err.contains("is not a count"), "{err}");
    assert!(
        !dir.join("work-counts.md").exists(),
        "malformed evidence still wrote the table"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// A row that recorded only some of the six counts is kept as recorded: the
/// rest stay "not yet measured" (`Work::from_meta` keeps a partial account).
/// Never dropped, never filled in with zeros.
#[test]
fn a_partial_work_row_stays_not_measured() -> R {
    let row = "| p1 | not yet measured | not yet measured | 3056399 | not yet measured | \
               not yet measured | 0 |";
    let dir = fixture("partial-work", &[row], "")?;
    let o = extract(&dir)?;
    assert!(o.status.success(), "{o:?}");
    let out = counts(&dir)?;
    assert!(out.contains(row), "{out}");
    assert!(!out.contains("| p1 | 0 | 0 | 3056399"), "{out}");
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// Counts that contradict each other are reported, not accepted
/// (`Work::check`), and the reason names the `work.*` keys so it can be
/// pasted under the table.
#[test]
fn contradictory_work_counts_are_reported_not_accepted() -> R {
    let dir = fixture("bad-sum", &["| p1 | 10 | 8 | 7 | 2 | 0 | 5 |"], "")?;
    let o = extract(&dir)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = stderr(&o);
    assert!(
        err.contains("work.issued 8 is not work.completed 7 + work.censored 5"),
        "{err}"
    );
    assert!(!dir.join("work-counts.md").exists(), "{err}");
    std::fs::remove_dir_all(&dir)?;

    let dir = fixture("bad-sched", &["| p1 | 1 | 8 | 7 | 2 | 0 | 1 |"], "")?;
    let o = extract(&dir)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let err = stderr(&o);
    assert!(
        err.contains("work.scheduled 1 is below work.issued 8"),
        "{err}"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// `latest µs` is the worst batch of the whole run, so a figure orders of
/// magnitude past the rest of the column is a stall and not a ranking. The
/// extractor names those points beside the table and points at the
/// investigation, so the column cannot be read as a like-for-like ordering.
/// Without an outlier the note is not printed at all.
#[test]
fn a_sender_max_far_past_the_rest_of_the_run_is_named_not_ranked() -> R {
    let normal = "send.sent=3000\nsend.batches=1500\nsend.late_100us=1\nsend.late_max_us=120\n";
    let stalled = "send.sent=3000\nsend.batches=1500\nsend.late_100us=1\nsend.late_max_us=40180\n";
    let dir = fixture("stall", &[], normal)?;
    std::fs::create_dir_all(dir.join("res/p2"))?;
    std::fs::write(dir.join("res/p2/meta.txt"), normal)?;
    std::fs::create_dir_all(dir.join("res/p3"))?;
    std::fs::write(dir.join("res/p3/meta.txt"), stalled)?;
    let o = extract(&dir)?;
    assert!(o.status.success(), "{o:?}");
    let out = counts(&dir)?;
    assert!(
        out.contains("`p3` at 40,180"),
        "the stall is named with its maximum: {out}"
    );
    assert!(
        out.contains("over 100× the median of the column (120)"),
        "the comparison that makes it an outlier is shown: {out}"
    );
    assert!(
        out.contains("Do not rank a row on `latest µs`"),
        "the column is not offered as a ranking: {out}"
    );
    assert!(
        out.contains("io/benches/BENCHMARK.md"),
        "the investigation is pointed at, not restated: {out}"
    );
    std::fs::remove_dir_all(&dir)?;

    let dir = fixture("no-stall", &[], normal)?;
    std::fs::create_dir_all(dir.join("res/p2"))?;
    std::fs::write(dir.join("res/p2/meta.txt"), normal)?;
    let o = extract(&dir)?;
    assert!(o.status.success(), "{o:?}");
    let out = counts(&dir)?;
    assert!(
        !out.contains("Do not rank a row on `latest µs`"),
        "no outlier, no note: {out}"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}
