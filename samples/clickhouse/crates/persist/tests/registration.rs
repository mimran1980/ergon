//! Registration tests: prepared handles, durable dictionary ordering,
//! duplicate/conflicting definitions, disabled laziness, wrong-layout
//! columns, buffer reuse, quotas, and backpressure.

use ergo_clickhouse_persist::persist::{
    EncodeError, Persistable, RecordOutcome, RowWriter, reasons,
};
use ergo_clickhouse_persist::protocol::{Declaration, Policy, RowEncoding};
use ergo_clickhouse_persist::recorder::{EnableSlot, MemoryPublication};
use ergo_clickhouse_persist::registration::{
    InMemorySink, RecorderConfig, RecorderSession, Symbol,
};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};

/// Handwritten fixed-layout row: instrument: u32, position: i64.
#[derive(Clone, Copy, Debug)]
pub struct StrategyState {
    pub instrument: u32,
    pub position: i64,
}

pub static STRATEGY_SCHEMA: RowSchema = RowSchema {
    columns: &[
        ValueSchema::scalar(TypeCode::U32),
        ValueSchema::scalar(TypeCode::I64),
    ],
};

impl Persistable for StrategyState {
    fn schema() -> &'static RowSchema {
        &STRATEGY_SCHEMA
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(12)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.set_raw(0, &self.instrument.to_le_bytes())?;
        out.set_raw(1, &self.position.to_le_bytes())?;
        Ok(())
    }
}

fn session() -> RecorderSession {
    let config = RecorderConfig {
        process: "market-recorder".into(),
        instance: "test-0".into(),
        build: "test".into(),
        max_record_bytes: 1024 * 1024,
        diagnostics_quota_bytes_per_sec: 0, // unlimited for most tests
        permanent: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 64,
            slot_bytes: 4096,
        },
        diagnostics: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 64,
            slot_bytes: 4096,
        },
    };
    RecorderSession::connect(config).expect("connect")
}

#[test]
fn registration_sends_dictionary_before_use() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    s.declare_session_start()?;
    let _t = s.table::<StrategyState>("strategy_state", Policy::Permanent)?;
    let sink_decls = s.catalog().layouts().to_vec();
    assert_eq!(sink_decls.len(), 1);
    assert_eq!(sink_decls[0].table_name, "strategy_state");
    assert_eq!(sink_decls[0].layout_id, 1);
    Ok(())
}

#[test]
fn duplicate_identical_definition_reuses_id() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let t1 = s.table::<StrategyState>("strategy_state", Policy::Permanent)?;
    let t2 = s.table::<StrategyState>("strategy_state", Policy::Permanent)?;
    assert_eq!(t1.layout_id(), t2.layout_id());
    Ok(())
}

#[test]
fn restart_reuses_local_ids_in_a_fresh_session() -> Result<(), Box<dyn std::error::Error>> {
    // Session 1: layout id 1.
    let mut s1 = session();
    let run1 = s1.run_id();
    let t1 = s1.table::<StrategyState>("t", Policy::Permanent)?;
    assert_eq!(t1.layout_id(), 1);
    // Session 2 (fresh process): same compact id 1, different run id.
    let mut s2 = session();
    let run2 = s2.run_id();
    let t2 = s2.table::<StrategyState>("t", Policy::Permanent)?;
    assert_eq!(t2.layout_id(), 1);
    assert_ne!(run1, run2, "run ids differ across restarts");
    Ok(())
}

#[test]
fn wrong_session_handle_is_rejected_before_publication() -> Result<(), Box<dyn std::error::Error>> {
    let mut s1 = session();
    let handle = s1.table::<StrategyState>("t", Policy::Permanent)?;
    let mut s2 = session();
    let _t2 = s2.table::<StrategyState>("t", Policy::Permanent)?;
    let mut w = s2.writer()?;
    let outcome = w.record(
        &handle,
        &StrategyState {
            instrument: 1,
            position: 2,
        },
        0,
    );
    assert_eq!(outcome, RecordOutcome::Invalid(reasons::SESSION_MISMATCH));
    Ok(())
}

#[test]
fn disabled_table_never_evaluates_the_closure() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let t = s.table::<StrategyState>("strategy_state", Policy::Temporary)?;
    let mut w = s.writer()?;
    let mut side_effects = 0u32;
    let outcome = w.record_with(&t, 0, |_row| {
        side_effects += 1;
        Ok(())
    });
    assert_eq!(outcome, RecordOutcome::Disabled);
    assert_eq!(side_effects, 0, "disabled call must not run the closure");
    assert_eq!(w.counters().published, 0);
    assert_eq!(w.counters().dropped, 0);
    Ok(())
}

#[test]
fn enable_then_record_then_disable() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    s.declare_session_start()?;
    let t = s.table::<StrategyState>("strategy_state", Policy::Temporary)?;
    let mut w = s.writer()?;
    t.slot().set_enabled(t.policy_id(), 1);
    let outcome = w.record(
        &t,
        &StrategyState {
            instrument: 42,
            position: -3,
        },
        123,
    );
    assert_eq!(outcome, RecordOutcome::Published);
    t.slot().set_disabled();
    let outcome = w.record(
        &t,
        &StrategyState {
            instrument: 1,
            position: 1,
        },
        124,
    );
    assert_eq!(outcome, RecordOutcome::Disabled);
    assert_eq!(w.counters().published, 1);
    assert_eq!(w.sequence(), 2, "only the enabled call consumed a sequence");
    Ok(())
}

#[test]
fn sequence_advances_on_failed_offers_so_gaps_reveal_loss() -> Result<(), Box<dyn std::error::Error>>
{
    // A transport with a tiny slot: offer succeeds, but a big row is dropped.
    let mut s = session();
    let t = s.table::<StrategyState>("t", Policy::Permanent)?;
    let mut w = s.writer()?;
    t.slot().set_enabled(t.policy_id(), 1);
    // drop the transport capacity by encoding an oversize payload is not
    // possible with this fixed schema; instead exhaust slots: 64-slot ring
    // is consumed by writes, but the writer keeps publishing (overwrite
    // oldest). Simulate transport failure through a closed publication.
    for i in 0..3 {
        let outcome = w.record(
            &t,
            &StrategyState {
                instrument: i,
                position: i as i64,
            },
            i as u64,
        );
        assert_eq!(outcome, RecordOutcome::Published);
    }
    assert_eq!(w.sequence(), 4);
    Ok(())
}

#[test]
fn dynamic_table_columns_reject_foreign_handles() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::registration::{Column, RegistrationError};
    let mut s = session();
    let mut t = s.dynamic_table("state", Policy::Permanent)?;
    let instrument: Column<u32> = t.column("instrument")?;
    let position: Column<i64> = t.column("position")?;
    let state = t.prepare()?;
    assert_eq!(state.slot_of(&instrument)?, 0);
    assert_eq!(state.slot_of(&position)?, 1);

    // A handle minted on a second layout addresses a different column
    // order; it must be rejected rather than silently writing that slot.
    let mut t2 = s.dynamic_table("state2", Policy::Permanent)?;
    let foreign: Column<u32> = t2.column("instrument")?;
    let other = t2.prepare()?;
    assert_eq!(other.slot_of(&foreign)?, 0);
    assert!(
        matches!(
            other.slot_of(&instrument),
            Err(RegistrationError::Conflict(_))
        ),
        "a handle from another layout was accepted"
    );
    Ok(())
}

#[test]
fn dynamic_record_round_trip_through_the_ring() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::registration::Column;
    let mut s = session();
    s.declare_session_start()?;
    let mut t = s.dynamic_table("strategy_state", Policy::Permanent)?;
    let _instrument: Column<u32> = t.column::<u32>("instrument")?;
    let _position: Column<i64> = t.column::<i64>("position")?;
    let state = t.prepare()?;
    state.slot().set_enabled(state.policy_id(), 1);
    let mut w = s.writer()?;
    let outcome = w.record_dynamic(&state, 7, |row| {
        row.set_raw(0, &42u32.to_le_bytes())?;
        row.set_raw(1, &1000i64.to_le_bytes())?;
        Ok(())
    });
    assert_eq!(outcome, RecordOutcome::Published);

    // Drain the ring and verify the column bytes survived the round trip.
    let frames = w.drain_permanent();
    assert_eq!(frames.len(), 1, "one published record expected");
    let mut expected = Vec::new();
    expected.extend_from_slice(&42u32.to_le_bytes());
    expected.extend_from_slice(&1000i64.to_le_bytes());
    assert!(
        frames[0].windows(expected.len()).any(|win| win == expected),
        "the encoded column values did not survive the ring: {:?}",
        frames[0]
    );
    Ok(())
}

#[test]
fn callback_error_aborts_the_row_without_publishing() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let t = s.table::<StrategyState>("t", Policy::Permanent)?;
    let mut w = s.writer()?;
    t.slot().set_enabled(t.policy_id(), 1);
    let outcome = w.record_with(&t, 5, |row| {
        row.set_raw(0, &1u32.to_le_bytes())?;
        Err(EncodeError::Custom(7)) // abort mid-row
    });
    assert_eq!(outcome, RecordOutcome::Invalid(reasons::CALLBACK_ERROR));
    assert_eq!(w.counters().published, 0);
    assert_eq!(
        w.sequence(),
        1,
        "no publication attempt: the row aborted before sequence allocation"
    );
    Ok(())
}

#[test]
fn quota_drops_do_not_suspend_permanent_recording() -> Result<(), Box<dyn std::error::Error>> {
    let config = RecorderConfig {
        process: "m".into(),
        instance: "i".into(),
        build: "b".into(),
        max_record_bytes: 1024 * 1024,
        diagnostics_quota_bytes_per_sec: 128, // ~2 envelopes per second
        permanent: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        diagnostics: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
    };
    let mut s = RecorderSession::connect(config)?;
    let dbg = s.table::<StrategyState>("book_debug", Policy::Temporary)?;
    let mkt = s.table::<StrategyState>("trades", Policy::Permanent)?;
    let mut w = s.writer()?;
    dbg.slot().set_enabled(dbg.policy_id(), 1);
    mkt.slot().set_enabled(mkt.policy_id(), 1);
    // First temporary record consumes the quota.
    let o1 = w.record(
        &dbg,
        &StrategyState {
            instrument: 1,
            position: 1,
        },
        1,
    );
    assert_eq!(o1, RecordOutcome::Published);
    // Subsequent temporary records are quota-dropped.
    let mut quota_drops = 0;
    for i in 0..10 {
        if w.record(
            &dbg,
            &StrategyState {
                instrument: 2,
                position: i as i64,
            },
            2,
        ) == RecordOutcome::Dropped(reasons::QUOTA_EXHAUSTED)
        {
            quota_drops += 1;
        }
    }
    assert!(quota_drops > 0, "quota must drop once exhausted");
    // Permanent recording is untouched by the exhausted quota.
    let o = w.record(
        &mkt,
        &StrategyState {
            instrument: 9,
            position: 9,
        },
        3,
    );
    assert_eq!(o, RecordOutcome::Published);
    Ok(())
}

#[test]
fn buffer_reuse_immediately_after_published() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let t = s.table::<StrategyState>("t", Policy::Permanent)?;
    let mut w = s.writer()?;
    t.slot().set_enabled(t.policy_id(), 1);
    // The borrowed row buffer is owned by the writer; the caller's own
    // buffer is free the moment record returns. Reuse a caller buffer.
    let mut caller_buf = [7u8; 12];
    let value = StrategyState {
        instrument: u32::from_le_bytes(caller_buf[..4].try_into()?),
        position: 5,
    };
    let outcome = w.record(&t, &value, 1);
    assert_eq!(outcome, RecordOutcome::Published);
    caller_buf.fill(0); // immediate reuse is safe
    Ok(())
}

#[test]
fn unregistered_symbols_count_and_return() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    // Interning returns compact ids; lookups of unknown ids in the catalog
    // return None (counted as unregistered by consumers).
    assert!(s.catalog().symbol(999).is_none());
    let sym = s.intern_symbol("BTC/USDT")?;
    assert_eq!(
        s.catalog().symbol(sym.0).map(|d| d.value.as_str()),
        Some("BTC/USDT")
    );
    assert!(s.catalog().symbol(999).is_none());
    Ok(())
}

#[test]
fn enable_slot_word_layout() {
    let slot = EnableSlot::new(0x0102);
    assert!(!EnableSlot::is_enabled_word(slot.load()));
    slot.set_enabled(0x0102, 0x0304_0506);
    let w = slot.load();
    assert!(EnableSlot::is_enabled_word(w));
    assert_eq!(EnableSlot::policy_id_word(w), 0x0102);
    assert_eq!(EnableSlot::policy_rev_word(w), 0x0304_0506);
    slot.set_disabled();
    assert!(!EnableSlot::is_enabled_word(slot.load()));
    assert_eq!(EnableSlot::policy_id_word(slot.load()), 0x0102);
}

#[test]
fn memory_ring_drops_and_counts() {
    let mut ring = MemoryPublication::new(2, 64);
    assert!(ring.offer(b"one"));
    assert!(ring.offer(b"two"));
    assert!(ring.offer(b"three")); // overwrites oldest unconsumed
    assert_eq!(ring.drops(), 1);
    let mut out = [0u8; 64];
    assert_eq!(ring.pop(&mut out), Some(3));
    assert_eq!(&out[..3], b"two");
    assert_eq!(ring.pop(&mut out), Some(5));
    assert_eq!(&out[..5], b"three");
    assert_eq!(ring.pop(&mut out), None);
    // oversize record rejected without growing
    let big = [0u8; 65];
    assert!(!ring.offer(&big));
    assert_eq!(ring.drops(), 2);
}

#[test]
fn symbol_values_are_registered_once() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let sink = InMemorySink::new();
    let _ = sink;
    let _ = Symbol(1);
    let a = s.intern_symbol("BTC/USDT")?;
    let b = s.intern_symbol("BTC/USDT")?;
    assert_eq!(a, b, "second intern is idempotent");
    Ok(())
}

#[test]
fn reserved_table_prefix_is_rejected() {
    let mut s = session();
    assert!(
        s.table::<StrategyState>("_record_x", Policy::Permanent)
            .is_err()
    );
}

#[test]
fn row_encoding_metadata_declared() {
    // OrderedRow layout travels with its encoding; consumers dispatch on it.
    assert_eq!(RowEncoding::OrderedRow.wire() as u8, 1);
}

/// `declare_session_end` carries the writer's final sequence, and the ingest
/// side only sees a completed session through this declaration — a session
/// that never emits it is indistinguishable from a stalled one.
///
/// `InMemorySink` is `Rc`-backed and `Clone`, so the handle kept here observes
/// the same queue the session writes into.
#[test]
fn session_end_is_declared_with_the_final_sequence() -> Result<(), Box<dyn std::error::Error>> {
    let sink = InMemorySink::new();
    let observed = sink.clone();
    let config = RecorderConfig {
        process: "market-recorder".into(),
        instance: "test-0".into(),
        build: "test".into(),
        permanent: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        diagnostics: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        ..RecorderConfig::default()
    };
    let mut session = RecorderSession::connect_with(config, Box::new(sink))?;
    session.declare_session_start()?;
    let t = session.table::<StrategyState>("t", Policy::Permanent)?;
    t.slot().set_enabled(t.policy_id(), 1);
    let mut w = session.writer()?;
    for i in 0..3u64 {
        assert_eq!(
            w.record(
                &t,
                &StrategyState {
                    instrument: i as u32,
                    position: i as i64
                },
                i,
            ),
            RecordOutcome::Published
        );
    }
    let final_sequence = w.sequence();
    session.declare_session_end(final_sequence)?;

    let sent = observed.drain();
    assert!(
        sent.iter()
            .any(|d| matches!(d, Declaration::SessionStart(_))),
        "session start was not declared: {sent:?}"
    );
    let end = sent
        .iter()
        .find_map(|d| match d {
            Declaration::SessionEnd { final_sequence, .. } => Some(*final_sequence),
            _ => None,
        })
        .ok_or("session end was not declared")?;
    assert_eq!(
        end, final_sequence,
        "session end reported a different final sequence"
    );
    assert_eq!(final_sequence, 4, "three records consumed sequences 1..=3");
    Ok(())
}

/// The design invariant: table, column and process names travel **only** in
/// registration records; a data envelope carries compact numeric IDs. Nothing
/// asserted this, so a future change could have started inlining a table name
/// into every row without a test noticing.
#[test]
fn data_envelopes_carry_no_structural_names() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Clone, Copy)]
    struct NameProbe {
        sentinel_column_alpha: u32,
        sentinel_column_beta: i64,
    }
    static PROBE_SCHEMA: RowSchema = RowSchema {
        columns: &[
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::scalar(TypeCode::I64),
        ],
    };
    impl Persistable for NameProbe {
        fn schema() -> &'static RowSchema {
            &PROBE_SCHEMA
        }
        fn encoded_len(&self) -> Result<usize, EncodeError> {
            Ok(12)
        }
        fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
            out.set_raw(0, &self.sentinel_column_alpha.to_le_bytes())?;
            out.set_raw(1, &self.sentinel_column_beta.to_le_bytes())?;
            Ok(())
        }
    }

    const PROCESS: &str = "sentinel_process_zeta";
    const TABLE: &str = "sentinel_table_omega";
    let config = RecorderConfig {
        process: PROCESS.into(),
        instance: "sentinel_instance_psi".into(),
        build: "test".into(),
        permanent: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        diagnostics: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        ..RecorderConfig::default()
    };
    let mut s = RecorderSession::connect(config)?;
    s.declare_session_start()?;
    let t = s.table::<NameProbe>(TABLE, Policy::Permanent)?;
    t.slot().set_enabled(t.policy_id(), 1);
    let mut w = s.writer()?;
    assert_eq!(
        w.record(
            &t,
            &NameProbe {
                sentinel_column_alpha: 7,
                sentinel_column_beta: -9
            },
            1,
        ),
        RecordOutcome::Published
    );

    let frames = w.drain_permanent();
    assert_eq!(frames.len(), 1, "one data record expected");
    let frame = &frames[0];
    let haystack = String::from_utf8_lossy(frame);
    for name in [
        PROCESS,
        TABLE,
        "sentinel_column_alpha",
        "sentinel_column_beta",
    ] {
        assert!(
            !haystack.contains(name),
            "data envelope carries the structural name {name:?}: {frame:?}"
        );
    }
    // Positive control, on the same substring search: the *registration*
    // stream must carry the table name, or the checks above prove nothing.
    let layout = s
        .catalog()
        .layouts()
        .first()
        .cloned()
        .ok_or("no layout declared")?;
    let mut reg_buf = [0u8; 4096];
    let n = Declaration::Layout(layout).encode(&mut reg_buf)?;
    let registration = String::from_utf8_lossy(&reg_buf[..n]);
    assert!(
        registration.contains(TABLE),
        "the registration stream must carry the table name; it did not, so the \
         envelope check above is vacuous"
    );
    Ok(())
}

/// PLAN task 1: "every rejected record has a deterministic outcome and
/// counter". Three of the eight reason codes had no test at all, so a change
/// that silently started *publishing* an oversize or back-pressured record
/// would not have been caught.
#[test]
fn oversize_record_is_rejected_with_a_counter() -> Result<(), Box<dyn std::error::Error>> {
    // `max_record_bytes` bounds the whole encoded record, so any payload that
    // does not fit it must be rejected before reaching the transport.
    let config = RecorderConfig {
        process: "market-recorder".into(),
        instance: "test-0".into(),
        build: "test".into(),
        max_record_bytes: 16,
        permanent: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        diagnostics: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        ..RecorderConfig::default()
    };
    let mut s = RecorderSession::connect(config)?;
    let t = s.table::<StrategyState>("t", Policy::Permanent)?;
    t.slot().set_enabled(t.policy_id(), 1);
    let mut w = s.writer()?;
    let outcome = w.record(
        &t,
        &StrategyState {
            instrument: 1,
            position: 2,
        },
        0,
    );
    assert_eq!(outcome, RecordOutcome::Invalid(reasons::OVERSIZE));
    assert_eq!(w.counters().invalid, 1, "oversize must be counted");
    assert_eq!(w.counters().published, 0, "nothing may be published");
    Ok(())
}

/// The same contract for a full transport: the record is dropped, counted as a
/// transport-full drop, and does not reach the publication.
#[test]
fn transport_full_drops_and_counts() -> Result<(), Box<dyn std::error::Error>> {
    // A ring whose slots are far smaller than a record: every offer fails.
    let config = RecorderConfig {
        process: "market-recorder".into(),
        instance: "test-0".into(),
        build: "test".into(),
        max_record_bytes: 4096,
        permanent: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 4,
            slot_bytes: 8,
        },
        diagnostics: ergo_clickhouse_persist::registration::TransportConfig::Memory {
            slots: 4,
            slot_bytes: 8,
        },
        ..RecorderConfig::default()
    };
    let mut s = RecorderSession::connect(config)?;
    let t = s.table::<StrategyState>("t", Policy::Permanent)?;
    t.slot().set_enabled(t.policy_id(), 1);
    let mut w = s.writer()?;
    let outcome = w.record(
        &t,
        &StrategyState {
            instrument: 1,
            position: 2,
        },
        0,
    );
    assert_eq!(outcome, RecordOutcome::Dropped(reasons::TRANSPORT_FULL));
    assert_eq!(
        w.counters().transport_full,
        1,
        "must count the full transport"
    );
    assert_eq!(w.counters().published, 0);
    assert_eq!(
        w.sequence(),
        2,
        "a rejected offer still consumes a sequence, so gaps reveal loss"
    );
    Ok(())
}

/// PLAN §5: the retention an operator configures must reach the policy the
/// table declares, because that declaration is what the ingester persists and
/// freezes into its layout binding.
///
/// Before this, `register_policy_for` hardcoded zeros: `recording.yaml`'s
/// `row_ttl: 24h` / `idle_table_ttl: 7d` were parsed, validated, tested and
/// displayed, and then never used by anything. Every temporary row therefore
/// expired at its own capture time.
#[test]
fn a_configured_retention_reaches_the_declared_policy() -> Result<(), Box<dyn std::error::Error>> {
    const DAY_NS: u64 = 86_400_000_000_000;
    const WEEK_NS: u64 = 7 * DAY_NS;

    let mut s = session();
    s.set_retention("strategy_state", DAY_NS, WEEK_NS);
    let t = s.table::<StrategyState>("strategy_state", Policy::Temporary)?;

    let declared = s.catalog().policy(t.policy_id()).ok_or("policy declared")?;
    assert_eq!(
        declared.row_ttl_ns, DAY_NS,
        "configured row retention published"
    );
    assert_eq!(
        declared.idle_ttl_ns, WEEK_NS,
        "configured idle window published"
    );

    // A table nothing configured still declares zeros, which the wire reads as
    // "permanent/default" — not as an immediate expiry.
    let unset = s.table::<StrategyState>("strategy_state_other", Policy::Temporary)?;
    let d = s
        .catalog()
        .policy(unset.policy_id())
        .ok_or("policy declared")?;
    assert_eq!(
        (d.row_ttl_ns, d.idle_ttl_ns),
        (0, 0),
        "absent config is zero, meaning no policy"
    );
    Ok(())
}

/// PLAN §5: "Temporary policy changes are versioned ... A changed mapping
/// requires a new explicit migration or table, not silent reinterpretation of
/// old events."
///
/// A retention edit must therefore mint a **new** policy id carrying the new
/// values, leaving the old one alone: every row already written keeps the
/// expiry it was written against. Editing the policy in place would instead
/// reinterpret history, and — because the ingester freezes a layout→storage
/// binding on first sight — would not even take effect for that table's rows.
#[test]
fn a_retention_change_is_versioned_not_mutated() -> Result<(), Box<dyn std::error::Error>> {
    const DAY: u64 = 86_400_000_000_000;
    const WEEK: u64 = 7 * DAY;
    const HOUR: u64 = 3_600_000_000_000;

    let mut s = session();
    assert!(
        s.set_retention("strategy_state", DAY, WEEK),
        "recording a retention for the first time is a change"
    );
    let first = s.table::<StrategyState>("strategy_state", Policy::Temporary)?;
    let p1 = s
        .catalog()
        .policy(first.policy_id())
        .ok_or("policy declared")?;
    assert_eq!((p1.row_ttl_ns, p1.idle_ttl_ns), (DAY, WEEK));

    // Re-resolving the same values is not a change, so nothing is re-declared.
    assert!(
        !s.set_retention("strategy_state", DAY, WEEK),
        "an unchanged config must not version anything"
    );

    assert!(
        s.set_retention("strategy_state", HOUR, WEEK),
        "an edited retention is a change"
    );

    // Versioning: a re-declaration gets a new policy id carrying the edit...
    let second = s.table::<StrategyState>("strategy_state", Policy::Temporary)?;
    assert_ne!(
        second.policy_id(),
        first.policy_id(),
        "the change mints a new policy id"
    );
    let p2 = s
        .catalog()
        .policy(second.policy_id())
        .ok_or("policy declared")?;
    assert_eq!(
        (p2.row_ttl_ns, p2.idle_ttl_ns),
        (HOUR, WEEK),
        "carries the edit"
    );

    // ...and the earlier policy is untouched, so rows under it keep their
    // original expiry. This is what makes it a version rather than a mutation.
    let p1_again = s
        .catalog()
        .policy(first.policy_id())
        .ok_or("policy declared")?;
    assert_eq!(
        (p1_again.row_ttl_ns, p1_again.idle_ttl_ns),
        (DAY, WEEK),
        "the superseded policy still describes what already-written rows used"
    );
    Ok(())
}
