//! PLAN §5's temporary-table lifecycle: the clock-skew bound, the drop
//! predicate, the state machine, and the durable journal that lets cleanup
//! resume after a crash between its two DDL statements.

use ergo_clickhouse_persist::ingest::catalog::{Catalog, DropStep, LifecycleRecord};
use ergo_clickhouse_persist::ingest::lifecycle::{
    LifecycleState, MAX_FUTURE_SKEW_NS, OnInput, Retain, has_idle_policy, idle_clock_started,
    is_implausibly_future, may_drop, on_input, row_expiry_ns,
};

/// A fixed "now" so the assertions are arithmetic, not wall-clock dependent.
const NOW: u64 = 1_800_000_000_000_000_000;
const HOUR: u64 = 3_600_000_000_000;
const DAY: u64 = 24 * HOUR;

// --------------------------------------------------------------- clock skew

#[test]
fn an_implausible_future_capture_cannot_keep_a_table_alive() {
    // Without a bound, this row earns an expiry a century out and its table
    // can never go idle — one corrupt timestamp keeping storage forever, which
    // is exactly what §5 forbids.
    let bogus = NOW + 100 * 365 * DAY;
    assert!(is_implausibly_future(bogus, NOW));

    let expiry = row_expiry_ns(bogus, HOUR, NOW);
    assert_eq!(
        expiry,
        NOW + MAX_FUTURE_SKEW_NS + HOUR,
        "an implausible capture is clamped to the skew bound, then retention added"
    );
    assert!(
        expiry < bogus,
        "clamping must shorten the table's life, never extend it"
    );
}

#[test]
fn plausible_captures_are_not_clamped() {
    // Right at the bound is still plausible; a millisecond past it is not.
    let at_bound = NOW + MAX_FUTURE_SKEW_NS;
    assert!(!is_implausibly_future(at_bound, NOW));
    assert_eq!(row_expiry_ns(at_bound, HOUR, NOW), at_bound + HOUR);

    // Ordinary past and present captures pass through untouched.
    assert_eq!(row_expiry_ns(NOW - DAY, HOUR, NOW), NOW - DAY + HOUR);
    assert_eq!(row_expiry_ns(NOW, HOUR, NOW), NOW + HOUR);
    assert!(is_implausibly_future(at_bound + 1, NOW));
}

#[test]
fn a_corrupt_timestamp_is_bounded_rather_than_trusted() {
    // `u64::MAX` is clamped to the skew bound *before* retention is added, so
    // it reaches neither failure mode: it cannot wrap to a small value and
    // silently expire the row, and it cannot earn the table unlimited life.
    let expiry = row_expiry_ns(u64::MAX, DAY, NOW);
    assert_eq!(
        expiry,
        NOW + MAX_FUTURE_SKEW_NS + DAY,
        "bounded first, retention second"
    );
    assert!(expiry > NOW, "the row is not lost");
    assert!(expiry < u64::MAX, "and is not granted unlimited life");
}

// ----------------------------------------------------------- drop predicate

#[test]
fn all_three_conditions_are_required_before_a_table_may_drop() {
    let expired = NOW - HOUR;
    let idle_passed = NOW - HOUR;

    // Rows still live: refused, and the reason names the expiry.
    assert_eq!(
        may_drop(NOW + HOUR, idle_passed, NOW, 0, 0),
        Err(Retain::RowsLive {
            latest_expiry_ns: NOW + HOUR
        })
    );
    // Rows expired but the idle window has not elapsed.
    assert_eq!(
        may_drop(expired, NOW + HOUR, NOW, 0, 0),
        Err(Retain::Idle {
            until_ns: NOW + HOUR
        })
    );
    // Both elapsed, but the generation is still in use — by a writer, and
    // separately by a registration.
    assert_eq!(
        may_drop(expired, idle_passed, NOW, 1, 0),
        Err(Retain::InUse)
    );
    assert_eq!(
        may_drop(expired, idle_passed, NOW, 0, 1),
        Err(Retain::InUse)
    );
    // All three satisfied.
    assert_eq!(may_drop(expired, idle_passed, NOW, 0, 0), Ok(()));
}

#[test]
fn live_rows_are_reported_ahead_of_an_idle_timer() {
    // Both would retain; the more actionable reason must win, because "rows
    // are still live" and "wait for the idle window" call for different
    // responses from an operator.
    assert!(matches!(
        may_drop(NOW + HOUR, NOW + DAY, NOW, 0, 0),
        Err(Retain::RowsLive { .. })
    ));
}

// ------------------------------------------------------------ state machine

#[test]
fn fresh_input_cancels_an_uncommitted_drop_and_revives_a_dropped_one() {
    // §5: "Fresh input either cancels an uncommitted drop or creates the next
    // generation after a committed drop."
    assert_eq!(on_input(LifecycleState::Active, false), OnInput::Accept);
    assert_eq!(
        on_input(LifecycleState::DropPending, false),
        OnInput::CancelDrop
    );
    assert_eq!(
        on_input(LifecycleState::Dropped, false),
        OnInput::NewGeneration
    );
}

#[test]
fn already_expired_input_is_dropped_rather_than_recreating_storage() {
    // §5: "Already expired input is acknowledged as deliberately expired
    // without recreating storage." A replay of old records must not resurrect
    // a table that cleanup removed on purpose — in any state.
    for state in [
        LifecycleState::Active,
        LifecycleState::DropPending,
        LifecycleState::Dropped,
    ] {
        assert_eq!(on_input(state, true), OnInput::AlreadyExpired);
    }
}

#[test]
fn lifecycle_state_round_trips_through_its_spelling() {
    for state in [
        LifecycleState::Active,
        LifecycleState::DropPending,
        LifecycleState::Dropped,
    ] {
        assert_eq!(LifecycleState::parse(state.as_str()), Some(state));
    }
    assert_eq!(LifecycleState::parse("nonsense"), None);
}

// --------------------------------------------------------- durable journal

fn temp_catalog(tag: &str) -> Result<(Catalog, std::path::PathBuf), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("ergo-lifecycle-{tag}-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let catalog = Catalog::open(path.to_str().ok_or("non-utf8 temp path")?)?;
    Ok((catalog, path))
}

#[test]
fn lifecycle_survives_a_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let (catalog, path) = temp_catalog("reopen")?;
    let record = LifecycleRecord {
        run_id: 7,
        layout_id: 3,
        table: "book_debug".into(),
        generation: 2,
        state: LifecycleState::DropPending,
        latest_expiry_ns: NOW + DAY,
        last_input_ns: NOW,
        view_dropped: false,
        backing_dropped: false,
    };
    catalog.put_lifecycle(&record)?;
    assert_eq!(catalog.lifecycle(7, 3), Some(record.clone()));

    // A restart must see the transition, not the state before it.
    drop(catalog);
    let reopened = Catalog::open(path.to_str().ok_or("non-utf8 temp path")?)?;
    assert_eq!(reopened.lifecycle(7, 3), Some(record));
    assert_eq!(
        reopened.lifecycle(7, 4),
        None,
        "unknown layout has no record"
    );
    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn a_crash_between_the_two_removals_resumes_at_the_second() -> Result<(), Box<dyn std::error::Error>>
{
    // The view is dropped before the backing table, so a crash after the first
    // statement must resume at the second rather than repeat the first or,
    // worse, consider the drop finished.
    let (catalog, path) = temp_catalog("resume")?;
    let mut record = LifecycleRecord {
        run_id: 1,
        layout_id: 1,
        table: "book_debug".into(),
        generation: 1,
        state: LifecycleState::DropPending,
        latest_expiry_ns: NOW,
        last_input_ns: NOW,
        view_dropped: false,
        backing_dropped: false,
    };
    assert_eq!(record.next_drop_step(), Some(DropStep::View));

    // The view removal ran; the process then died before the backing table.
    record.view_dropped = true;
    catalog.put_lifecycle(&record)?;
    drop(catalog);

    let reopened = Catalog::open(path.to_str().ok_or("non-utf8 temp path")?)?;
    let resumed = reopened.lifecycle(1, 1).ok_or("record must survive")?;
    assert_eq!(
        resumed.next_drop_step(),
        Some(DropStep::Backing),
        "resumes at the statement that had not run"
    );
    assert_eq!(resumed.state, LifecycleState::DropPending);

    // And once both have run, there is nothing left to do.
    let done = LifecycleRecord {
        view_dropped: true,
        backing_dropped: true,
        state: LifecycleState::Dropped,
        ..resumed
    };
    assert_eq!(done.next_drop_step(), None);
    let _ = std::fs::remove_file(&path);
    Ok(())
}

// ------------------------------------------------- zero means "no policy"

#[test]
fn a_zero_idle_window_means_no_policy_not_expire_now() {
    // The wire spells "no retention" as 0. Read as an *elapsed* window it
    // would satisfy `may_drop`'s idle test immediately, and cleanup would drop
    // every temporary table whose producer declared nothing — including while
    // it is still being written to. Hence the guard, asserted here.
    assert!(!has_idle_policy(0));
    assert!(has_idle_policy(1));

    // The hazard itself: a zero window would otherwise look droppable.
    let last_input = NOW - DAY;
    let zero_window_deadline = last_input; // last_input + 0
    assert_eq!(
        may_drop(NOW - HOUR, zero_window_deadline, NOW, 0, 0),
        Ok(()),
        "a zero window reads as already elapsed — which is exactly why the \
         ingester must consult `has_idle_policy` before calling this"
    );
}

#[test]
fn a_generation_whose_idle_clock_never_started_is_not_idle() {
    // Found live, not by inspection: a freshly declared temporary table was
    // dropped on the first cleanup pass. `apply_declaration` seeded
    // `last_input_ns` with 0, so the idle deadline computed as `0 + 7d` — a
    // 1970 timestamp — and `may_drop` saw a window that had elapsed 56 years
    // ago. A table the fixture had just declared, and deliberately never wrote
    // to, was removed seconds later.
    assert!(!idle_clock_started(0));
    assert!(idle_clock_started(1));

    const WEEK: u64 = 7 * 24 * 3_600_000_000_000;
    let now = NOW;
    // What the unseeded record produced: deadline = 0 + WEEK.
    let bogus_deadline = 0u64.saturating_add(WEEK);
    assert!(
        may_drop(0, bogus_deadline, now, 0, 0).is_ok(),
        "this is the bug: an unstarted clock reads as already elapsed"
    );
    // What the seeded record produces: the window runs from creation.
    let seeded = now;
    assert_eq!(
        may_drop(seeded, seeded.saturating_add(WEEK), now, 0, 0),
        Err(Retain::Idle {
            until_ns: seeded + WEEK
        }),
        "a table created now is retained for the full idle window"
    );
}
