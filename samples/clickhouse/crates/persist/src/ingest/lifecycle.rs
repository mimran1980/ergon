//! PLAN §5's temporary-table lifecycle: `Active → DropPending → Dropped`.
//!
//! A temporary table's storage is created on first use and must eventually be
//! removed once its rows have expired and it has gone idle. Removing it is not
//! one DDL statement — the public view and the backing table are dropped
//! separately — so a crash between them must resume rather than leave a
//! half-removed table. Transitions are therefore journalled in the durable
//! catalog (see [`crate::ingest::catalog`]), and each cleanup step is
//! idempotent, so replaying the sequence is safe.
//!
//! Everything here is pure: the policy decisions are separated from the DDL so
//! they can be tested without a server, and so the ingester has exactly one
//! place that decides whether a table may go.

/// How far into the future a capture timestamp may be before it is treated as
/// implausible.
///
/// PLAN §5 requires a *documented* bound. Without one, a single record carrying
/// a bogus far-future timestamp sets a far-future expiry, and the table it
/// belongs to can never go idle — one bad row keeps storage alive forever. A
/// minute is generous for clock skew between recording hosts and the ingester,
/// and small enough that a corrupt timestamp is still caught.
pub const MAX_FUTURE_SKEW_NS: u64 = 60 * 1_000_000_000;

/// A row's immutable expiry in Unix nanoseconds.
///
/// The capture time is clamped to `now + MAX_FUTURE_SKEW_NS` *before* the
/// retention is added. That direction matters: an implausible future timestamp
/// then shortens the table's life rather than extending it indefinitely, which
/// is the property §5 asks for. Saturating arithmetic keeps a corrupt
/// timestamp from wrapping into a small (already-expired) value.
#[must_use]
pub fn row_expiry_ns(captured_at_ns: u64, row_ttl_ns: u64, now_ns: u64) -> u64 {
    let bound = now_ns.saturating_add(MAX_FUTURE_SKEW_NS);
    captured_at_ns.min(bound).saturating_add(row_ttl_ns)
}

/// Whether a capture timestamp is beyond the documented skew bound.
#[must_use]
pub fn is_implausibly_future(captured_at_ns: u64, now_ns: u64) -> bool {
    captured_at_ns > now_ns.saturating_add(MAX_FUTURE_SKEW_NS)
}

/// Lifecycle state of one temporary table generation.
///
/// `DropPending` is deliberately not terminal. §5 requires that fresh input
/// either cancels an uncommitted drop or, once the drop has committed, creates
/// the next generation — so a table can move back to `Active` from
/// `DropPending`, and only `Dropped` is final.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleState {
    /// Rows are being accepted; the generation may still be written to.
    Active,
    /// Cleanup has decided this generation should go, but nothing has been
    /// removed yet. Fresh input cancels it.
    DropPending,
    /// The generation's storage was removed. Terminal — a later record creates
    /// the next generation instead of reviving this one.
    Dropped,
}

impl LifecycleState {
    /// Spelling used in the durable journal.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::DropPending => "drop_pending",
            Self::Dropped => "dropped",
        }
    }

    /// Parse the spelling used in the durable journal.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "drop_pending" => Some(Self::DropPending),
            "dropped" => Some(Self::Dropped),
            _ => None,
        }
    }
}

/// Why a temporary table may not be dropped yet.
///
/// Returned rather than a bare `bool` so the ingester can report *which*
/// condition is holding the table, which is what an operator needs when a
/// table they expect to disappear does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retain {
    /// Rows have not all expired; dropping now would lose live data.
    RowsLive {
        /// The latest expiry in this generation.
        latest_expiry_ns: u64,
    },
    /// All rows have expired, but the idle window has not elapsed.
    Idle {
        /// When the idle window ends.
        until_ns: u64,
    },
    /// Writers or registrations still hold this generation.
    InUse,
}

/// Whether a temporary table was given an idle window at all.
///
/// The wire encodes `row_ttl_ns`/`idle_ttl_ns` as "0 = permanent/default"
/// (see `protocol::PolicyDeclaration`), so a zero window means the producer
/// declared *no* retention policy — not an immediate expiry. Cleanup must not
/// read it as an already-elapsed window: doing so drops every temporary table
/// whose producer declared nothing, including while it is still being written
/// to. A table with no idle policy is never eligible.
#[must_use]
pub const fn has_idle_policy(idle_ttl_ns: u64) -> bool {
    idle_ttl_ns > 0
}

/// Whether a generation's idle clock has actually started.
///
/// A zero `last_input_ns` means no input was ever recorded for this generation,
/// which is *not* the same as "idle since the epoch". Read as an elapsed window
/// it makes the idle deadline a 1970 timestamp, so the very first cleanup pass
/// drops a table that was never idle at all — observed live, on a table the
/// fixture had just declared and never wrote to.
#[must_use]
pub const fn idle_clock_started(last_input_ns: u64) -> bool {
    last_input_ns > 0
}

/// Whether an idle temporary table may begin dropping.
///
/// PLAN §5: "The ingester drops an idle temporary table only after all retained
/// rows have expired, its idle threshold has passed, and there are no pending
/// writes or registrations using its lifecycle generation."
///
/// The three conditions are checked in that order so the reported reason is the
/// most actionable one: live rows outrank an idle timer, and both outrank a
/// bare "in use".
pub fn may_drop(
    latest_expiry_ns: u64,
    idle_deadline_ns: u64,
    now_ns: u64,
    pending_writes: u64,
    registrations: u64,
) -> Result<(), Retain> {
    if latest_expiry_ns > now_ns {
        return Err(Retain::RowsLive { latest_expiry_ns });
    }
    if idle_deadline_ns > now_ns {
        return Err(Retain::Idle {
            until_ns: idle_deadline_ns,
        });
    }
    if pending_writes > 0 || registrations > 0 {
        return Err(Retain::InUse);
    }
    Ok(())
}

/// Drive a pending drop as far as it can, journalling each removal.
///
/// The view is dropped before the backing table because the failure modes are
/// not symmetric: a query against a surviving view whose backing table is gone
/// errors, whereas a backing table behind a dropped view is merely invisible
/// and can still be cleaned up. Each step is journalled immediately after it
/// succeeds, so a crash between the two resumes at the statement that had not
/// run. Both statements are `IF EXISTS`, so re-running a completed step is
/// harmless — the journal is what makes the *resume point* reliable, not the
/// statements themselves.
///
/// A record that is not `DropPending` is left alone: cleanup only ever acts on
/// a transition that [`may_drop`] already approved.
pub fn advance_drop(
    ch: &crate::ingest::clickhouse::ClickHouse,
    catalog: &crate::ingest::catalog::Catalog,
    record: &mut crate::ingest::catalog::LifecycleRecord,
    view: &str,
    backing: &str,
) -> Result<(), crate::ingest::clickhouse::ChError> {
    if record.state != LifecycleState::DropPending {
        return Ok(());
    }
    match record.next_drop_step() {
        Some(crate::ingest::catalog::DropStep::View) => {
            ch.exec(&format!("DROP VIEW IF EXISTS `{view}`"))?;
            record.view_dropped = true;
            journal(catalog, record);
        }
        Some(crate::ingest::catalog::DropStep::Backing) => {
            ch.exec(&format!("DROP TABLE IF EXISTS `{backing}`"))?;
            record.backing_dropped = true;
            record.state = LifecycleState::Dropped;
            journal(catalog, record);
        }
        None => {
            // Both removals already ran — a crash after the second statement
            // but before the final journal write. Record the terminal state
            // rather than leaving the record stuck in `DropPending`.
            record.state = LifecycleState::Dropped;
            journal(catalog, record);
        }
    }
    Ok(())
}

/// Persist a journal step, reporting rather than swallowing a failure.
///
/// A failed journal write must not be silent: the DDL has already run, so the
/// record is now the *only* thing that knows the drop is half-done, and losing
/// that turns a resumable drop into a table that leaks forever.
fn journal(
    catalog: &crate::ingest::catalog::Catalog,
    record: &crate::ingest::catalog::LifecycleRecord,
) {
    if let Err(e) = catalog.put_lifecycle(record) {
        eprintln!(
            "lifecycle: journal write failed for {} gen {}: {e}",
            record.table, record.generation
        );
    }
}

/// What to do with an incoming record for a temporary table, given the
/// generation's journalled state.
///
/// This is §5's "fresh input either cancels an uncommitted drop or creates the
/// next generation after a committed drop", plus "already expired input is
/// acknowledged as deliberately expired without recreating storage".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnInput {
    /// Write the row to the existing generation.
    Accept,
    /// The drop was still uncommitted; cancel it and write the row.
    CancelDrop,
    /// The generation is gone; open the next one and write the row.
    NewGeneration,
    /// The row's own expiry has already passed, so it is deliberately dropped
    /// rather than recreated. §5: replay of already-expired records must not
    /// recreate an empty table.
    AlreadyExpired,
}

/// Decide what an incoming record means for its table's lifecycle.
///
/// `row_expired` is the record's expiry already having passed at `now_ns`,
/// which is what stops a replay of old records from resurrecting a table that
/// cleanup deliberately removed.
#[must_use]
pub fn on_input(state: LifecycleState, row_expired: bool) -> OnInput {
    if row_expired {
        return OnInput::AlreadyExpired;
    }
    match state {
        LifecycleState::Active => OnInput::Accept,
        LifecycleState::DropPending => OnInput::CancelDrop,
        LifecycleState::Dropped => OnInput::NewGeneration,
    }
}
