//! Optional `tracing_subscriber::Layer` adapter for the persistence
//! recorder.
//!
//! Registered scalar/byte `tracing` events are routed into prepared
//! tables through typed visitor methods — numbers, booleans, strings, and
//! byte slices are written directly into reusable storage. Display/Debug
//! field values are rejected (counted) in strict mode, never parsed back
//! into columns.
//!
//! This is a convenience adapter with its own measured overhead; it is
//! not the implementation of the strict recording calls. A filter for
//! this layer does not disable another layer's interest, and another
//! logging subscriber does not defeat the explicit macro guard — use
//! `persist_event!`/`persist_dto!` when disabled-table laziness must hold
//! regardless of logging configuration.

use ergo_clickhouse_persist::persist::{RecordOutcome, RowWriter};
use ergo_clickhouse_persist::recorder::{SyncSlot, Writer};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode};
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};

/// Shared writer handle for the adapter surface.
///
/// # Safety contract
///
/// `Writer` is a single-threaded, thread-confined type (plain `Cell`/`Rc`
/// state by design). The adapter must be attached and its events recorded
/// from the writer's own thread — the single-threaded HFT deployment this
/// sample targets. The `Send`/`Sync` impls exist only to satisfy the
/// dispatcher's nominal bounds; no cross-thread access occurs when that
/// contract holds.
#[derive(Clone)]
pub struct SharedWriter(pub Arc<Mutex<Writer>>);
// SAFETY: see the type-level contract above.
unsafe impl Send for SharedWriter {}
// SAFETY: see the type-level contract above.
unsafe impl Sync for SharedWriter {}
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// Binding of a tracing callsite target to a prepared layout.
///
/// The tracing field visit order must match the schema's column order;
/// the strict adapter writes positionally and counts mismatches.
#[derive(Clone)]
pub struct CallsiteBinding {
    /// Event target (e.g. `persist::feed_latency`).
    pub target: &'static str,
    /// Storage schema (column order defines the storage order).
    pub schema: &'static RowSchema,
    /// Session-local layout id.
    pub layout_id: u16,
    /// Bound policy id.
    pub policy_id: u16,
    /// Whether the table routes to the diagnostics publication.
    pub temporary: bool,
    /// Atomic enable slot (config-controlled; mirrored by the control thread).
    pub slot: SyncSlot,
}

/// Strict-mode persistence layer.
pub struct PersistenceLayer {
    bindings: Mutex<Vec<CallsiteBinding>>,
    /// Writer shared with the application. The adapter is a convenience
    /// path, not the HFT recording call, so a short mutex handoff is
    /// acceptable here and its overhead is measured separately.
    writer: Mutex<Option<SharedWriter>>,
    /// Counted rejections (unexpected field types, unattached writer).
    rejected: std::sync::atomic::AtomicU64,
    /// Published rows through this layer.
    published: std::sync::atomic::AtomicU64,
    /// Disabled passes through the gate (no payload work).
    gated: std::sync::atomic::AtomicU64,
}

impl PersistenceLayer {
    /// New empty layer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: Mutex::new(Vec::new()),
            writer: Mutex::new(None),
            rejected: std::sync::atomic::AtomicU64::new(0),
            published: std::sync::atomic::AtomicU64::new(0),
            gated: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Bind a callsite target to a prepared layout (control thread).
    pub fn bind(&self, binding: CallsiteBinding) {
        self.bindings.lock().expect("poisoned").push(binding);
    }

    /// Attach the writer (control thread). Passing `None` detaches.
    pub fn attach_writer(&self, writer: Option<SharedWriter>) {
        *self.writer.lock().expect("poisoned") = writer;
    }

    /// Rejected (unsupported/misbound/unattached) event count.
    #[must_use]
    pub fn rejected(&self) -> u64 {
        self.rejected.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Published rows through this layer.
    #[must_use]
    pub fn published(&self) -> u64 {
        self.published.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gated (disabled) passes.
    #[must_use]
    pub fn gated(&self) -> u64 {
        self.gated.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn binding_for(&self, target: &str) -> Option<CallsiteBinding> {
        self.bindings
            .lock()
            .expect("poisoned")
            .iter()
            .find(|b| b.target == target)
            .cloned()
    }
}

impl Default for PersistenceLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed visitor writing field values straight into a row writer.
struct RowVisitor<'r, 'a> {
    row: &'r mut RowWriter<'a>,
    schema: &'a RowSchema,
    next: usize,
    rejected: std::sync::atomic::AtomicU64,
}

impl Visit for RowVisitor<'_, '_> {
    fn record_debug(&mut self, field: &Field, _value: &dyn std::fmt::Debug) {
        // Strict mode: formatted values are rejected, not parsed back.
        tracing::trace!(field = %field.name(), "persist-tracing: rejected formatted field");
        self.rejected
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_i64(&mut self, _field: &Field, value: i64) {
        match self.schema.columns.get(self.next).map(|c| c.ty) {
            Some(TypeCode::U64) => {
                let _ = self.row.write_u64(value as u64);
            }
            Some(TypeCode::I64) => {
                let _ = self.row.write_i64(value);
            }
            Some(TypeCode::I32) => {
                let _ = self.row.write_i32(i32::try_from(value).unwrap_or(0));
            }
            Some(TypeCode::U32) => {
                let _ = self.row.write_u32(u32::try_from(value).unwrap_or(0));
            }
            _ => {
                self.rejected
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
        }
        self.next += 1;
    }

    fn record_u64(&mut self, _field: &Field, value: u64) {
        match self.schema.columns.get(self.next).map(|c| c.ty) {
            Some(TypeCode::U64) => {
                let _ = self.row.write_u64(value);
            }
            Some(TypeCode::I64) => {
                let _ = self.row.write_i64(i64::try_from(value).unwrap_or(0));
            }
            Some(TypeCode::U32) => {
                let _ = self.row.write_u32(u32::try_from(value).unwrap_or(0));
            }
            _ => {
                self.rejected
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
        }
        self.next += 1;
    }

    fn record_bool(&mut self, _field: &Field, value: bool) {
        let _ = self.row.write_bool(value);
        self.next += 1;
    }

    fn record_str(&mut self, _field: &Field, value: &str) {
        let _ = self.row.write_str(value);
        self.next += 1;
    }

    fn record_bytes(&mut self, _field: &Field, value: &[u8]) {
        let _ = self.row.write_bytes(value);
        self.next += 1;
    }
}

impl<S: Subscriber> Layer<S> for PersistenceLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let Some(binding) = self.binding_for(event.metadata().target()) else {
            return; // unbound target: not a persistence event
        };
        if !ergo_clickhouse_persist::recorder::EnableSlot::is_enabled_word(binding.slot.load()) {
            self.gated
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }
        // Writer attached?
        let Some(writer_shared) = self.writer.lock().expect("poisoned").clone() else {
            // Unattached: counted drop, never a fresh allocation.
            self.rejected
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        };
        let mut writer = writer_shared.0.lock().expect("poisoned");
        let schema = binding.schema;
        let outcome = writer.record_raw_row(
            &binding.slot,
            binding.layout_id,
            binding.policy_id,
            binding.temporary,
            schema,
            now_ns(),
            |row: &mut RowWriter<'_>| {
                let mut visitor = RowVisitor {
                    row,
                    schema,
                    next: 0,
                    rejected: std::sync::atomic::AtomicU64::new(0),
                };
                event.record(&mut visitor);
                let rejected = visitor.rejected.load(std::sync::atomic::Ordering::Relaxed);
                if rejected > 0 {
                    return Err(ergo_clickhouse_persist::persist::EncodeError::Custom(60));
                }
                Ok(())
            },
        );
        match outcome {
            RecordOutcome::Published => {
                self.published
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            RecordOutcome::Disabled => {
                self.gated
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {
                self.rejected
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

fn now_ns() -> u64 {
    ergo_clickhouse_persist::registration::now_ns()
}
