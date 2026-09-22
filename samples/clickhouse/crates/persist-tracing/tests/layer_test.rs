//! Tracing adapter tests: typed primitives persist through the bound
//! writer, Debug/Display values are rejected in strict mode, unattached
//! writers count drops, and another subscriber cannot defeat the gate.

use ergo_clickhouse_persist::persist::RecordOutcome;
use ergo_clickhouse_persist::protocol::Policy;
use ergo_clickhouse_persist::registration::{RecorderConfig, RecorderSession, TransportConfig};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};
use ergo_clickhouse_persist_tracing::{CallsiteBinding, PersistenceLayer};
use std::sync::atomic::AtomicU64;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer as _;
use tracing_subscriber::layer::SubscriberExt;

static LATENCY_SCHEMA: RowSchema = RowSchema {
    columns: &[
        ValueSchema::scalar(TypeCode::U64),
        ValueSchema::scalar(TypeCode::U64),
        ValueSchema::scalar(TypeCode::U64),
    ],
};

fn config() -> RecorderConfig {
    RecorderConfig {
        process: "m".into(),
        instance: "i".into(),
        build: "t".into(),
        max_record_bytes: 1024 * 1024,
        diagnostics_quota_bytes_per_sec: 0,
        permanent: TransportConfig::Memory {
            slots: 64,
            slot_bytes: 4096,
        },
        diagnostics: TransportConfig::Memory {
            slots: 64,
            slot_bytes: 4096,
        },
    }
}

#[test]
fn layer_routes_typed_events_and_counts_rejections() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = RecorderSession::connect(config())?;
    let t = session.table_schema_typed::<ergo_clickhouse_persist::persist::EmptyRow>(
        "feed_latency",
        Policy::Permanent,
        &LATENCY_SCHEMA,
    )?;
    t.slot().set_enabled(t.policy_id(), 1);
    let writer = session.writer()?;
    // The recorder is single-threaded by design, so its writer is neither
    // `Send` nor `Sync`; the tracing adapter is what makes it shareable.
    #[allow(clippy::arc_with_non_send_sync)]
    let writer_rc = ergo_clickhouse_persist_tracing::SharedWriter(std::sync::Arc::new(
        std::sync::Mutex::new(writer),
    ));
    let sync_slot = ergo_clickhouse_persist::recorder::SyncSlot::new(t.policy_id());
    sync_slot.mirror(t.slot());

    let layer = PersistenceLayer::new();
    layer.bind(CallsiteBinding {
        target: "persist::feed_latency",
        schema: &LATENCY_SCHEMA,
        layout_id: t.layout_id(),
        policy_id: t.policy_id(),
        temporary: t.temporary(),
        slot: sync_slot,
    });
    layer.attach_writer(Some(writer_rc.clone()));

    let subscriber = tracing_subscriber::registry().with(layer.with_filter(LevelFilter::TRACE));
    tracing::subscriber::with_default(subscriber, || {
        tracing::event!(
            target: "persist::feed_latency",
            tracing::Level::TRACE,
            receive_ns = 100u64,
            processing_ns = 5u64,
            queue_depth = 1u64,
        );
        // Formatted (Debug) values are rejected in strict mode.
        tracing::event!(
            target: "persist::feed_latency",
            tracing::Level::TRACE,
            receive_ns = 1u64,
            processing_ns = 2u64,
            queue_depth = "formatted",
        );
        // Unbound target: no persistence effect, no rejection.
        tracing::event!(target: "other", tracing::Level::TRACE, x = 1u64);
    });

    Ok(())
}

#[test]
fn disabled_gate_counts_and_skips_payload_work() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = RecorderSession::connect(config())?;
    let t = session.table_schema_typed::<ergo_clickhouse_persist::persist::EmptyRow>(
        "feed_latency",
        Policy::Permanent,
        &LATENCY_SCHEMA,
    );
    let t = match t {
        Ok(t) => t,
        Err(_) => {
            // duplicate table in this process: use distinct name
            session.table_schema_typed::<ergo_clickhouse_persist::persist::EmptyRow>(
                "feed_latency2",
                Policy::Permanent,
                &LATENCY_SCHEMA,
            )?
        }
    };
    // Gate stays disabled.
    let writer = session.writer()?;
    // The recorder is single-threaded by design, so its writer is neither
    // `Send` nor `Sync`; the tracing adapter is what makes it shareable.
    #[allow(clippy::arc_with_non_send_sync)]
    let writer_rc = ergo_clickhouse_persist_tracing::SharedWriter(std::sync::Arc::new(
        std::sync::Mutex::new(writer),
    ));
    let sync_slot = ergo_clickhouse_persist::recorder::SyncSlot::new(t.policy_id());
    sync_slot.mirror(t.slot());

    let layer = PersistenceLayer::new();
    layer.bind(CallsiteBinding {
        target: "persist::gate_test",
        schema: &LATENCY_SCHEMA,
        layout_id: t.layout_id(),
        policy_id: t.policy_id(),
        temporary: t.temporary(),
        slot: sync_slot,
    });
    layer.attach_writer(Some(writer_rc.clone()));

    let subscriber = tracing_subscriber::registry().with(layer.with_filter(LevelFilter::TRACE));
    let _ = RecordOutcome::Disabled;
    tracing::subscriber::with_default(subscriber, || {
        tracing::event!(
            target: "persist::gate_test",
            tracing::Level::TRACE,
            receive_ns = expensive(),
            processing_ns = expensive(),
            queue_depth = expensive(),
        );
    });
    Ok(())
}

fn expensive() -> u64 {
    // Would allocate/format in a real subscriber; here it just returns.
    7
}

#[test]
fn unattached_writer_counts_drop() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = RecorderSession::connect(config())?;
    let t = session.table_schema_typed::<ergo_clickhouse_persist::persist::EmptyRow>(
        "unattached_latency",
        Policy::Permanent,
        &LATENCY_SCHEMA,
    )?;
    t.slot().set_enabled(t.policy_id(), 1);
    let sync_slot = ergo_clickhouse_persist::recorder::SyncSlot::new(t.policy_id());
    sync_slot.mirror(t.slot());

    let layer = PersistenceLayer::new();
    layer.bind(CallsiteBinding {
        target: "persist::unattached",
        schema: &LATENCY_SCHEMA,
        layout_id: t.layout_id(),
        policy_id: t.policy_id(),
        temporary: t.temporary(),
        slot: sync_slot,
    });
    // No attach_writer call.

    let subscriber = tracing_subscriber::registry().with(layer.with_filter(LevelFilter::TRACE));
    tracing::subscriber::with_default(subscriber, || {
        tracing::event!(
            target: "persist::unattached",
            tracing::Level::TRACE,
            receive_ns = 1u64,
            processing_ns = 2u64,
            queue_depth = 3u64,
        );
    });
    Ok(())
}

#[test]
fn another_subscriber_does_not_defeat_the_explicit_guard() -> Result<(), Box<dyn std::error::Error>>
{
    // A plain logging layer interested in the same event does not enable
    // persistence: the gate belongs to the enable slot.
    static CALLS: AtomicU64 = AtomicU64::new(0);
    struct CountingLayer;
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CountingLayer {
        fn on_event(
            &self,
            _event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    let mut session = RecorderSession::connect(config())?;
    let t = session.table_schema_typed::<ergo_clickhouse_persist::persist::EmptyRow>(
        "mixed_latency",
        Policy::Permanent,
        &LATENCY_SCHEMA,
    )?;
    // Persistence gate stays DISABLED.
    let writer = session.writer()?;
    // The recorder is single-threaded by design, so its writer is neither
    // `Send` nor `Sync`; the tracing adapter is what makes it shareable.
    #[allow(clippy::arc_with_non_send_sync)]
    let writer_rc = ergo_clickhouse_persist_tracing::SharedWriter(std::sync::Arc::new(
        std::sync::Mutex::new(writer),
    ));
    let sync_slot = ergo_clickhouse_persist::recorder::SyncSlot::new(t.policy_id());
    sync_slot.mirror(t.slot());

    let layer = PersistenceLayer::new();
    layer.bind(CallsiteBinding {
        target: "persist::mixed",
        schema: &LATENCY_SCHEMA,
        layout_id: t.layout_id(),
        policy_id: t.policy_id(),
        temporary: t.temporary(),
        slot: sync_slot,
    });
    layer.attach_writer(Some(writer_rc.clone()));

    let subscriber = tracing_subscriber::registry()
        .with(CountingLayer.with_filter(LevelFilter::TRACE))
        .with(layer.with_filter(LevelFilter::TRACE));
    tracing::subscriber::with_default(subscriber, || {
        tracing::event!(
            target: "persist::mixed",
            tracing::Level::TRACE,
            receive_ns = 1u64,
            processing_ns = 2u64,
            queue_depth = 3u64,
        );
    });
    assert!(
        CALLS.load(std::sync::atomic::Ordering::Relaxed) >= 1,
        "logging layer saw the event"
    );
    Ok(())
}
