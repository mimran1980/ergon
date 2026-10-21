//! The work accounting beside the percentiles: scheduled, issued, completed,
//! late, rejected and censored, recorded together so a sample that is short of
//! the run's work is never read as a complete one.

use io_bench::matrix;
use io_bench::record::{Record, write_records};
use io_bench::stalls::{StallSummary, format_stalls};
use io_bench::stats::{Work, work_row};

type R = Result<(), Box<dyn std::error::Error>>;

const SEC: i64 = 1_000_000_000;
const T0: i64 = 1_792_000_800 * SEC;

#[test]
fn issued_is_completed_plus_censored_and_scheduled_is_never_below_issued() {
    let w = Work::paced(T0 + 5 * SEC, T0, SEC / 10, 8, 6, 2);
    w.check().expect("the identity holds");
    assert_eq!(
        (
            w.scheduled,
            w.issued,
            w.completed,
            w.late,
            w.rejected,
            w.censored
        ),
        (Some(51), Some(8), Some(6), Some(2), Some(0), Some(2)),
        "censored is the issued work with no sample: 8 issued, 6 recorded"
    );
    assert_eq!(Work::paced(T0, T0, 0, 8, 6, 0).check(), Ok(()));
    // A run whose counts do not add up is named, not silently reported.
    let bad = Work::round_trip(10, 8, 6, 0, 5);
    assert!(bad.check().is_err(), "{bad:?}");
    let behind = Work::round_trip(3, 8, 8, 0, 0);
    assert!(behind.check().is_err(), "{behind:?}");
}

#[test]
fn a_paced_run_counts_every_slot_the_schedule_owed() {
    // 100/s for five seconds is 501 slots from the first due time; the client
    // issued 8 and two of those went out after their slot.
    let w = Work::paced(T0 + 5 * SEC, T0, SEC / 100, 8, 8, 2);
    assert_eq!(w.scheduled, Some(501));
    assert_eq!(
        w.scheduled.zip(w.issued).map(|(s, i)| s - i),
        Some(493),
        "the slots never filled are unsent work"
    );
    // A stall that never caught up: the schedule never slips, so every slot
    // after the stall is a send still owed.
    let stalled = Work::paced(T0 + SEC, T0, SEC / 1000, 10, 10, 10);
    assert_eq!(stalled.scheduled, Some(1001));
    assert_eq!(stalled.late, Some(10));
    assert!(stalled.check().is_ok());
}

#[test]
fn an_unpaced_run_has_no_schedule_beyond_its_replies() {
    let w = Work::paced(T0 + 5 * SEC, 0, 0, 8, 6, 3);
    assert_eq!(w.scheduled, Some(8), "scheduled is the sends made");
    assert_eq!(
        w.late,
        Some(3),
        "kept as recorded, though no slot defines it"
    );
    w.check().expect("the identity holds");
}

#[test]
fn a_send_is_late_only_when_the_run_owed_the_next_slot_too() {
    // On time: past its own slot by the loop's detection latency, but the next
    // slot is not yet due. Counting these would report `late` as `issued`.
    assert!(!Work::is_late(T0 + 200, T0, SEC / 10));
    assert!(!Work::is_late(T0 + SEC / 10 - 1, T0, SEC / 10));
    // The boundary: the next slot is due exactly now.
    assert!(Work::is_late(T0 + SEC / 10, T0, SEC / 10));
    // A catch-up send after a stall, and one further behind again.
    assert!(Work::is_late(T0 + SEC / 10 + 1, T0, SEC / 10));
    assert!(Work::is_late(T0 + SEC, T0, SEC / 10));
    // An unpaced run has no slots, so nothing is late.
    assert!(!Work::is_late(T0 + SEC, T0, 0));
    // A stall that never caught up: every one of its sends is late.
    let stalled = Work::paced(T0 + SEC, T0, SEC / 1000, 10, 10, 10);
    assert_eq!(stalled.late, Some(10));
}

#[test]
fn a_receive_only_run_reports_the_send_side_as_not_yet_measured() {
    let w = Work::received(6, 2);
    assert_eq!((w.completed, w.censored), (Some(6), Some(2)));
    // Never a zero: the sender's schedule is not visible from here.
    assert_eq!(
        (w.scheduled, w.issued, w.late, w.rejected),
        (None, None, None, None)
    );
    let cells = w.cells();
    assert_eq!(cells.matches("not yet measured").count(), 4, "{cells}");
    assert!(!cells.contains("| 0 |"), "{cells}");
    w.check().expect("nothing to contradict");
}

#[test]
fn meta_round_trips_the_six_counts_and_a_missing_one_stays_missing() {
    let w = Work::round_trip(10, 8, 6, 2, 2);
    let mut m = String::new();
    w.write_meta(&mut m);
    assert_eq!(
        m,
        "work.scheduled=10\nwork.issued=8\nwork.completed=6\nwork.late=2\nwork.rejected=0\nwork.censored=2\n"
    );
    let map: std::collections::HashMap<String, String> = m
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    assert_eq!(Work::from_meta(&map), Ok(Some(w)));

    // A partial write is a partial count: the rest is "not yet measured".
    let mut partial = String::new();
    Work::received(6, 2).write_meta(&mut partial);
    let map: std::collections::HashMap<String, String> = partial
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let got = Work::from_meta(&map)
        .expect("meta parses")
        .expect("two counts are present");
    assert_eq!((got.completed, got.censored), (Some(6), Some(2)));
    assert_eq!(got.issued, None);

    // A run recorded before the counts were has no row, and is not reported as
    // six zeros.
    let empty = std::collections::HashMap::new();
    assert_eq!(Work::from_meta(&empty), Ok(None));
}

/// The accounting a fresh executable once reported without complaint:
/// `1 scheduled, 8 issued, 7 completed, 5 censored`. It violates both
/// identities — `scheduled < issued`, and `issued != completed + censored` —
/// so it is malformed metadata and must be reported, not accepted.
#[test]
fn contradictory_counts_are_reported_not_accepted() {
    let map: std::collections::HashMap<String, String> = [
        ("work.scheduled", "1"),
        ("work.issued", "8"),
        ("work.completed", "7"),
        ("work.censored", "5"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
    .collect();
    let err = Work::from_meta(&map).expect_err("the counts contradict each other");
    assert!(
        err.contains("work.issued 8 is not work.completed 7 + work.censored 5"),
        "the report names the broken identity, got: {err}"
    );
}

/// `scheduled` below `issued` is a contradiction on its own.
#[test]
fn more_issued_than_scheduled_is_reported() {
    let map: std::collections::HashMap<String, String> = [
        ("work.scheduled", "3"),
        ("work.issued", "8"),
        ("work.completed", "8"),
        ("work.censored", "0"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
    .collect();
    let err = Work::from_meta(&map).expect_err("the counts contradict each other");
    assert!(
        err.contains("work.scheduled 3 is below work.issued 8"),
        "the report names the broken identity, got: {err}"
    );
}

/// An absent count is "not yet measured" — a path that never produced it. A
/// count that is present but not a number is malformed metadata, and must not
/// collapse into the same gap: `work.scheduled=garbage` read as "not yet
/// measured" would sit beside the percentiles as an honest hole while the run
/// had in fact reported something unreadable.
#[test]
fn an_unreadable_count_is_malformed_not_missing() {
    let map: std::collections::HashMap<String, String> = [
        ("work.scheduled", "garbage"),
        ("work.issued", "8"),
        ("work.completed", "6"),
        ("work.censored", "2"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
    .collect();
    let err = Work::from_meta(&map).expect_err("a count that is not a number is malformed");
    assert!(
        err.contains("work.scheduled") && err.contains("garbage"),
        "the report names the key and the value it could not read, got: {err}"
    );

    // A key that is never written stays "not yet measured".
    let partial: std::collections::HashMap<String, String> =
        [("work.completed", "6"), ("work.censored", "2")]
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
    let got = Work::from_meta(&partial)
        .expect("present counts parse")
        .expect("two counts are present");
    assert_eq!((got.completed, got.censored), (Some(6), Some(2)));
    assert_eq!(
        got.scheduled, None,
        "an absent key is still not yet measured"
    );
}

/// The one loader the report paths share: the map's counts, checked against
/// the records the run actually wrote, before any warm-up is dropped.
#[test]
fn load_reconciles_completions_with_the_records_on_disk() {
    let mut m = String::new();
    Work::round_trip(10, 8, 7, 2, 1).write_meta(&mut m);
    let map: std::collections::HashMap<String, String> = m
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    assert!(Work::load(&map, 7).is_ok(), "7 completions, 7 records");
    let err = Work::load(&map, 6).expect_err("more completions than records");
    assert!(
        err.contains("work.completed") && err.contains("records"),
        "the reason names both sides, got: {err}"
    );
}

#[test]
fn the_work_row_puts_the_counts_next_to_the_percentiles() {
    let w = Work::round_trip(10, 8, 6, 2, 2);
    assert_eq!(
        work_row("r1-base-0", &w),
        "| r1-base-0 | 10 | 8 | 6 | 2 | 0 | 2 |"
    );
}

/// A run: `mode`, and records sent at t, decoded `user_us` after sending. The
/// first record sets the run's start; the rest fall after the one-second
/// round-trip warm-up, so the tables report `n` = 6 of the 7 recorded.
fn run(dir: &std::path::Path, mode: &str, user_us: i64, work: &str) -> R {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("meta.txt"),
        format!("mode={mode}\nduration_s=60\nrate=10000\n{work}"),
    )?;
    let records: Vec<Record> = (0..7)
        .map(|i| {
            let t = if i == 0 {
                T0
            } else {
                T0 + SEC + 100_000 + i * 100_000
            };
            Record {
                id: i + 1,
                src_ns: t,
                src2_ns: if mode == "pingpong" { t - 1_000 } else { 0 },
                user_ns: t + user_us * 1_000,
                ..Record::default()
            }
        })
        .collect();
    write_records(&dir.join("records.bin"), &records)?;
    let sum = StallSummary {
        cpu: 2,
        ..StallSummary::default()
    };
    std::fs::write(dir.join("stalls-0.txt"), format_stalls(&sum, &[]))?;
    Ok(())
}

#[test]
fn a_sample_short_of_the_runs_work_reports_what_it_leaves_out() -> R {
    let dir = std::env::temp_dir().join(format!("io-bench-work-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let run_dir = dir.join("r1-base-0").join("pp-recv-busy-10000");
    // 8 sends out of 10 scheduled slots, 7 of them recorded: 1 censored, 2
    // late. The percentiles then drop the warm-up, so their `n` is 6 — one
    // less than `completed` — and a reader needs both to see it.
    run(
        &run_dir,
        "pingpong",
        200,
        "work.scheduled=10\nwork.issued=8\nwork.completed=7\nwork.late=2\nwork.rejected=0\nwork.censored=1\n",
    )?;
    let (tags, _) = matrix::load(&dir, 10)?;
    assert_eq!(tags.len(), 1);
    let got = tags[0]
        .runs
        .get("pp-recv-busy-10000")
        .expect("the run loads")
        .clone();
    assert_eq!(got.latency.as_ref().map(|s| s.n), Some(6));
    assert_eq!(
        got.work,
        Some(Work::round_trip(10, 8, 7, 2, 1)),
        "the counts reach the report from meta.txt"
    );
    got.work.expect("the counts").check().expect("the identity");

    let report = matrix::report(&tags, &[]);
    assert!(
        report.contains("| r1-base-0 | 10 | 8 | 7 | 2 | 0 | 1 |"),
        "the work counts sit beside the percentiles:\n{report}"
    );
    assert!(
        report.contains("### Work behind the percentiles"),
        "{report}"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// A run whose `meta.txt` claims work that cannot have happened: 8 sends out
/// of 1 scheduled slot, 7 recorded and 5 of them censored. The loader must
/// name the contradiction instead of reporting the counts.
#[test]
fn contradictory_work_counts_are_rejected_not_reported() -> R {
    let dir = std::env::temp_dir().join(format!("io-bench-badwork-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let run_dir = dir.join("r1-base-0").join("pp-recv-busy-10000");
    run(
        &run_dir,
        "pingpong",
        200,
        "work.scheduled=1\nwork.issued=8\nwork.completed=7\nwork.late=0\nwork.rejected=0\nwork.censored=5\n",
    )?;
    match matrix::load_run(&run_dir, 10) {
        Err(why) => {
            assert!(
                why.contains("work.issued") && why.contains("work.censored"),
                "the reason names the counts that disagree: {why}"
            );
        }
        Ok(got) => panic!("a contradictory run loaded as {got:?}"),
    }
    // And through `load`, so the report lists it beside the runs that failed.
    let (tags, skipped) = matrix::load(&dir, 10)?;
    assert!(tags.is_empty(), "the run is not reported: {tags:?}");
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(
        skipped[0].0.ends_with("pp-recv-busy-10000") && skipped[0].1.contains("work."),
        "{skipped:?}"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// `work.completed` is the sends with a recorded sample before the warm-up is
/// dropped, so it must equal the records on disk. A run that claims more
/// completions than it recorded is lying about its own sample.
#[test]
fn completed_must_match_the_records_on_disk() -> R {
    let dir = std::env::temp_dir().join(format!("io-bench-shortrec-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let run_dir = dir.join("r1-base-0").join("pp-recv-busy-10000");
    run(
        &run_dir,
        "pingpong",
        200,
        "work.scheduled=10\nwork.issued=9\nwork.completed=9\nwork.late=0\nwork.rejected=0\nwork.censored=0\n",
    )?;
    // `run` writes 7 records; the meta claims 9 completions.
    match matrix::load_run(&run_dir, 10) {
        Err(why) => assert!(
            why.contains("work.completed") && why.contains("records"),
            "the reason names both sides: {why}"
        ),
        Ok(got) => panic!("a run claiming more completions than records loaded as {got:?}"),
    }
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn a_run_without_the_counts_is_not_reported_as_six_zeros() -> R {
    let dir = std::env::temp_dir().join(format!("io-bench-nowork-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let run_dir = dir.join("r1-base-0").join("pp-recv-busy-10000");
    run(&run_dir, "pingpong", 200, "")?;
    let (tags, _) = matrix::load(&dir, 10)?;
    let got = tags[0]
        .runs
        .get("pp-recv-busy-10000")
        .expect("the run loads");
    assert_eq!(got.work, None, "a run recorded before the counts were");
    let report = matrix::report(&tags, &[]);
    assert!(
        !report.contains("### Work behind the percentiles"),
        "no invented counts:\n{report}"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// The sender's batch schedule is a different population from [`Work::late`].
/// `Work` counts the paced order slots: `late` is a slot filled when the next
/// one was already due. The sender counts **batches** across all of its
/// traffic — the order connection and the feeds together, the first message
/// of each due time — and `late_100us` is a batch that began over 100 µs
/// late. A run can show `work.late` = 2 and 39 464 late batches at once, and
/// only the second says the offered traffic was not the planned traffic.
mod send_schedule {
    use io_bench::stats::SendSchedule;
    use std::collections::HashMap;

    type R = Result<(), Box<dyn std::error::Error>>;

    fn meta(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn it_is_the_send_side_of_the_traffic_not_the_order_slots() {
        let w = io_bench::stats::Work::round_trip(10, 8, 7, 2, 1);
        assert_eq!(w.late, Some(2), "two order slots were filled a slot late");
        let s = SendSchedule {
            sent: Some(3_060_455),
            batches: Some(1_586_374),
            late_100us: Some(39_464),
            late_max_us: Some(178),
        };
        assert_eq!(
            s.late_100us,
            Some(39_464),
            "39 464 batches began over 100 µs late"
        );
        assert_ne!(
            s.late_100us, w.late,
            "the two count different things and must never be printed as one"
        );
    }

    #[test]
    fn an_absent_sender_schedule_stays_not_measured() -> R {
        assert_eq!(SendSchedule::from_meta(&meta(&[]))?, None);
        // A partial one is kept as recorded: `late_max_us` is the only field
        // the sender can have when no batch was late.
        let s = SendSchedule::from_meta(&meta(&[
            ("send.sent", "3060455"),
            ("send.batches", "1586374"),
            ("send.late_100us", "0"),
            ("send.late_max_us", "0"),
        ]))?
        .expect("present counts load");
        assert_eq!(s.late_100us, Some(0));
        assert_eq!(s.cells(), "3060455 | 1586374 | 0 | 0 | ok");
        Ok(())
    }

    #[test]
    fn a_present_but_unreadable_count_is_malformed_not_missing() {
        let err = SendSchedule::from_meta(&meta(&[
            ("send.batches", "1586374"),
            ("send.late_100us", "lots"),
        ]))
        .expect_err("garbage is not an honest hole");
        assert!(err.contains("send.late_100us"), "{err}");
    }

    #[test]
    fn batches_without_late_counts_is_malformed() {
        // `sent` and `batches` alone cannot say whether the traffic was the
        // planned traffic, so they are not a usable schedule account.
        let err = SendSchedule::from_meta(&meta(&[
            ("send.sent", "3060455"),
            ("send.batches", "1586374"),
        ]))
        .expect_err("a schedule that cannot be judged is not reported");
        assert!(err.contains("send.late_100us"), "{err}");
    }

    #[test]
    fn a_sender_over_one_percent_of_batches_late_is_flagged() {
        // The threshold the harness warns at: more than 1% of batches.
        let ok = SendSchedule {
            sent: Some(100),
            batches: Some(10_000),
            late_100us: Some(99),
            late_max_us: Some(150),
        };
        assert_eq!(ok.verdict(), "ok");
        assert_eq!(ok.late_pct(), Some(0.99));
        let slipped = SendSchedule {
            sent: Some(100),
            batches: Some(10_000),
            late_100us: Some(101),
            late_max_us: Some(178),
        };
        assert_eq!(slipped.verdict(), "over 1%: not the planned traffic");
        assert_eq!(slipped.late_pct(), Some(1.01));
    }

    #[test]
    fn a_zero_batch_run_is_judged_on_nothing() {
        let none = SendSchedule {
            sent: Some(0),
            batches: Some(0),
            late_100us: Some(0),
            late_max_us: Some(0),
        };
        assert_eq!(none.verdict(), "not yet measured");
        assert_eq!(none.late_pct(), None);
    }

    #[test]
    fn the_verdict_sits_next_to_the_counts() {
        let s = SendSchedule {
            sent: Some(3_060_455),
            batches: Some(1_586_374),
            late_100us: Some(39_464),
            late_max_us: Some(178),
        };
        assert_eq!(
            io_bench::stats::send_schedule_row("rt-uring-spin-n4 @ 10000/s", &s),
            "| rt-uring-spin-n4 @ 10000/s | 3060455 | 1586374 | 39464 (2.49%) | 178 | over 1%: not the planned traffic |"
        );
        assert_eq!(
            io_bench::stats::SEND_SCHEDULE_HEADER,
            "| | sent | batches | late >100 µs | latest µs | verdict |\n\
             |---|---|---|---|---|---|"
        );
    }
}
