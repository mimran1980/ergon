//! The recording writer: prepared handles, enable slots, bounded
//! publication.
//!
//! Deployment model: one single-threaded HFT process over Aeron IPC with a
//! co-located ArchivingMediaDriver recording the stream. Everything here is
//! single-threaded by design — enable slots are plain `Cell` words (no
//! atomics), counters are plain `u64` fields read directly off the writer,
//! and sequence numbers advance with a plain increment. The hot path
//! allocates nothing after warm-up, never waits, and never formats.

use crate::persist::{EncodeError, Persistable, RecordOutcome, RowWriter, reasons};
use crate::protocol::{RecordKind, encode_data_record, limits};
use crate::schema::RowSchema;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// One coherent word holding a handle's enabled state and policy reference:
/// bit 0 = enabled, bits 8..24 = policy id, bits 24..56 = policy revision.
///
/// The control path publishes schema/policy readiness before setting the
/// enable bit and clears the bit first when disabling; a recording call
/// loads this word once and acts on it.
#[derive(Debug, Default)]
pub struct EnableSlot {
    word: Cell<u64>,
}

const ENABLED_BIT: u64 = 1;
const POLICY_SHIFT: u32 = 8;
const REV_SHIFT: u32 = 24;

impl EnableSlot {
    /// New slot for `policy_id`, initially disabled.
    #[must_use]
    pub const fn new(policy_id: u16) -> Self {
        Self {
            word: Cell::new((policy_id as u64) << POLICY_SHIFT),
        }
    }

    /// Load the coherent word (enabled bit + policy id + policy revision).
    #[inline]
    #[must_use]
    pub fn load(&self) -> u64 {
        self.word.get()
    }

    /// Enabled bit from a loaded word.
    #[inline]
    #[must_use]
    pub const fn is_enabled_word(word: u64) -> bool {
        word & ENABLED_BIT != 0
    }

    /// Policy id from a loaded word.
    #[inline]
    #[must_use]
    pub const fn policy_id_word(word: u64) -> u16 {
        ((word >> POLICY_SHIFT) & 0xFFFF) as u16
    }

    /// Policy revision from a loaded word.
    #[inline]
    #[must_use]
    pub const fn policy_rev_word(word: u64) -> u32 {
        ((word >> REV_SHIFT) & 0xFFFF_FFFF) as u32
    }

    /// Enable after the policy is ready (control path).
    pub fn set_enabled(&self, policy_id: u16, policy_rev: u32) {
        self.word.set(
            ENABLED_BIT
                | (u64::from(policy_id) << POLICY_SHIFT)
                | (u64::from(policy_rev) << REV_SHIFT),
        );
    }

    /// Disable first; an in-flight call finishes under the previous policy.
    pub fn set_disabled(&self) {
        self.word.set(self.word.get() & !ENABLED_BIT);
    }
}

/// Shared enable slot (single-threaded: `Rc`, not `Arc`).
pub type SharedSlot = Rc<EnableSlot>;

/// One publication shared by the registration sink and the data transport.
///
/// They must be the same publication: a declaration and the rows that depend
/// on it have to reach the ingester in that order, and two exclusive
/// publications on one channel/stream cannot exist (and would not be ordered
/// even if they could).
#[cfg(feature = "producer")]
pub type SharedPublication = Rc<RefCell<AeronPublication>>;

/// Atomic enable slot for paths that cross thread boundaries (the
/// optional tracing adapter). Same word layout as [`EnableSlot`]; the
/// control thread mirrors config changes into it.
#[derive(Debug, Clone)]
pub struct SyncSlot(pub Arc<AtomicU64>);

impl SyncSlot {
    /// New slot for `policy_id`, initially disabled.
    #[must_use]
    pub fn new(policy_id: u16) -> Self {
        Self(Arc::new(AtomicU64::new((policy_id as u64) << POLICY_SHIFT)))
    }

    /// Load the coherent word.
    #[inline]
    #[must_use]
    pub fn load(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }

    /// Mirror the state of a single-threaded slot (control thread).
    pub fn mirror(&self, slot: &EnableSlot) {
        self.0.store(slot.load(), Ordering::Release);
    }
}

/// Pre-sized in-memory publication ring: zero-alloc after construction,
/// bounded retention, drops on overflow (counted). Used for tests and the
/// local laboratory; the producer path uses Aeron IPC.
pub struct MemoryPublication {
    slots: Vec<Vec<u8>>,
    head: usize,
    queue: RefCell<std::collections::VecDeque<(usize, usize)>>,
    drops: Cell<u64>,
}

impl MemoryPublication {
    /// New ring with `slots` pre-sized buffers of `slot_bytes`.
    #[must_use]
    pub fn new(slots: usize, slot_bytes: usize) -> Self {
        Self {
            slots: vec![vec![0u8; slot_bytes]; slots.max(1)],
            head: 0,
            queue: RefCell::new(std::collections::VecDeque::with_capacity(slots.max(1))),
            drops: Cell::new(0),
        }
    }

    /// Offer bytes; copies into the next pre-sized slot (overwrites the
    /// oldest unconsumed record once full). Returns false when the record
    /// exceeds the slot capacity.
    pub fn offer(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() > self.slots[0].len() {
            self.drops.set(self.drops.get() + 1);
            return false;
        }
        let idx = self.head;
        self.head = (self.head + 1) % self.slots.len();
        self.slots[idx][..bytes.len()].copy_from_slice(bytes);
        let mut q = self.queue.borrow_mut();
        if q.len() >= self.slots.len() {
            q.pop_front();
            self.drops.set(self.drops.get() + 1);
        }
        q.push_back((idx, bytes.len()));
        true
    }

    /// Pop the oldest committed record, copying into `out`. Returns the
    /// copied length, or `None` when empty.
    pub fn pop(&mut self, out: &mut [u8]) -> Option<usize> {
        let (idx, len) = self.queue.borrow_mut().pop_front()?;
        if len > out.len() {
            return None;
        }
        out[..len].copy_from_slice(&self.slots[idx][..len]);
        Some(len)
    }

    /// Dropped record count.
    #[must_use]
    pub fn drops(&self) -> u64 {
        self.drops.get()
    }

    /// Retained-but-unconsumed record count.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.queue.borrow().len()
    }
}

/// Aeron exclusive publication over the pod-local Media Driver (IPC).
#[cfg(feature = "producer")]
pub struct AeronPublication {
    _aeron: Rc<rusteron_client::Aeron>,
    publication: rusteron_client::AeronExclusivePublication,
    max_message_length: usize,
}

#[cfg(feature = "producer")]
impl std::fmt::Debug for AeronPublication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AeronPublication")
            .field("max_message_length", &self.max_message_length)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "producer")]
impl AeronPublication {
    /// Whether the publication has a subscriber.
    ///
    /// Until it does, every `offer` is rejected: an IPC publication with no
    /// subscriber has no image, so a producer that starts before the Archive's
    /// recording subscription must wait rather than offer.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.publication.is_connected()
    }

    /// Connect an exclusive publication to `channel`/`stream_id`.
    pub fn connect(
        channel: &str,
        stream_id: i32,
    ) -> Result<Self, crate::registration::RegistrationError> {
        let dir = std::env::var("AERON_DIR").unwrap_or_default();
        let ctx = rusteron_client::AeronContext::new().map_err(|e| {
            crate::registration::RegistrationError::Transport(format!("context: {e}"))
        })?;
        if !dir.is_empty() {
            ctx.set_dir(&rusteron_client::cformat!("{dir}"))
                .map_err(|e| {
                    crate::registration::RegistrationError::Transport(format!("dir: {e}"))
                })?;
        }
        let aeron = rusteron_client::Aeron::new(&ctx).map_err(|e| {
            crate::registration::RegistrationError::Transport(format!("aeron: {e}"))
        })?;
        aeron.start().map_err(|e| {
            crate::registration::RegistrationError::Transport(format!("start: {e}"))
        })?;
        let c_channel = rusteron_client::cformat!("{channel}");
        let publication = aeron
            .add_exclusive_publication(&c_channel, stream_id, std::time::Duration::from_secs(5))
            .map_err(|e| {
                crate::registration::RegistrationError::Transport(format!("publication: {e}"))
            })?;
        let mut max_message_length = 0usize;
        if let Ok(constants) = publication.get_constants() {
            max_message_length = constants.max_message_length;
        }
        Ok(Self {
            _aeron: Rc::new(aeron),
            publication,
            max_message_length,
        })
    }

    /// One bounded offer attempt; never waits. Copies `bytes` (borrowed)
    /// into a claim when the message fits one fragment.
    pub fn offer(&mut self, bytes: &[u8]) -> bool {
        if self.max_message_length > 0 && bytes.len() > self.max_message_length {
            return false;
        }
        match self.publication.try_claim_owned(bytes.len()) {
            Ok(mut claim) => {
                claim.data()[..bytes.len()].copy_from_slice(bytes);
                claim.commit().is_ok()
            }
            Err(_) => {
                // Fall back to a single offer attempt (transport fragmentation).
                self.publication
                    .offer_raw(bytes, rusteron_client::Handlers::NONE)
                    > 0
            }
        }
    }
}

/// Data publication transport.
pub enum Transport {
    /// In-memory ring.
    Memory(MemoryPublication),
    /// Aeron exclusive publication.
    #[cfg(feature = "producer")]
    Aeron(AeronPublication),
    /// Aeron publication shared with the registration sink.
    #[cfg(feature = "producer")]
    AeronShared(SharedPublication),
}

impl Transport {
    fn offer(&mut self, bytes: &[u8]) -> bool {
        match self {
            Self::Memory(p) => p.offer(bytes),
            #[cfg(feature = "producer")]
            Self::Aeron(p) => p.offer(bytes),
            #[cfg(feature = "producer")]
            Self::AeronShared(p) => p.borrow_mut().offer(bytes),
        }
    }
}

/// Diagnostics quota bucket: bytes per second, lazily refilled on the
/// enabled diagnostics path only. Permanent recording never consults it.
struct Quota {
    per_second: u64,
    available: f64,
    last: Option<Instant>,
}

impl Quota {
    fn new(per_second: u64) -> Self {
        Self {
            per_second,
            available: per_second as f64,
            last: None,
        }
    }

    fn try_take(&mut self, n: u64) -> bool {
        if self.per_second == 0 {
            return true;
        }
        let now = Instant::now();
        match self.last {
            None => self.last = Some(now),
            Some(last) => {
                let elapsed = now.duration_since(last).as_secs_f64();
                if elapsed > 0.0 {
                    self.available = (self.available + elapsed * self.per_second as f64)
                        .min(self.per_second as f64 * 2.0);
                    self.last = Some(now);
                }
            }
        }
        if self.available >= n as f64 {
            self.available -= n as f64;
            true
        } else {
            false
        }
    }
}

/// Writer counters: plain fields; the owning thread reads them directly
/// (status export samples the writer between record calls).
#[derive(Debug, Default, Clone, Copy)]
pub struct WriterCounters {
    /// Published record count.
    pub published: u64,
    /// Dropped record count.
    pub dropped: u64,
    /// Invalid record count.
    pub invalid: u64,
    /// Bytes accepted into the transport.
    pub bytes_published: u64,
    /// Transports-full events (backpressure).
    pub transport_full: u64,
    /// Diagnostics quota drops.
    pub quota_drops: u64,
}

/// Payload location for the envelope encode: either the row scratch
/// (typed/dynamic rows) or borrowed caller bytes (raw SBE).
enum PayloadRef<'p> {
    /// Encoded row in the writer's row scratch, `n` bytes.
    Row(usize),
    /// Borrowed caller bytes.
    Bytes(&'p [u8]),
}

/// Prepared typed-table handle bound to a session.
///
/// Session-scoped, immutable, cheap to clone (`Rc` on the slot).
pub struct PreparedTable<T> {
    pub(crate) session_id: u64,
    pub(crate) layout_id: u16,
    pub(crate) policy_id: u16,
    pub(crate) temporary: bool,
    pub(crate) slot: SharedSlot,
    pub(crate) _marker: PhantomData<fn(T)>,
}

impl<T> Clone for PreparedTable<T> {
    fn clone(&self) -> Self {
        Self {
            session_id: self.session_id,
            layout_id: self.layout_id,
            policy_id: self.policy_id,
            temporary: self.temporary,
            slot: Rc::clone(&self.slot),
            _marker: PhantomData,
        }
    }
}

impl<T> PreparedTable<T> {
    /// Session-local layout id.
    #[must_use]
    pub const fn layout_id(&self) -> u16 {
        self.layout_id
    }
    /// Bound policy id.
    #[must_use]
    pub const fn policy_id(&self) -> u16 {
        self.policy_id
    }
    /// Enable slot for config-reload wiring.
    #[must_use]
    pub const fn slot(&self) -> &SharedSlot {
        &self.slot
    }
    /// Whether this handle routes to the diagnostics publication.
    #[must_use]
    pub const fn temporary(&self) -> bool {
        self.temporary
    }
}

impl PreparedDynamic {
    /// Session-local layout id.
    #[must_use]
    pub const fn layout_id(&self) -> u16 {
        self.layout_id
    }
    /// Bound policy id.
    #[must_use]
    pub const fn policy_id(&self) -> u16 {
        self.policy_id
    }
    /// Enable slot for config-reload wiring.
    #[must_use]
    pub const fn slot(&self) -> &SharedSlot {
        &self.slot
    }
}

/// Prepared raw-SBE handle with a typed extras layout.
#[derive(Clone)]
pub struct PreparedSbe {
    pub(crate) session_id: u64,
    pub(crate) layout_id: u16,
    pub(crate) policy_id: u16,
    pub(crate) temporary: bool,
    pub(crate) slot: SharedSlot,
    pub(crate) extras_layout_id: u16,
    pub(crate) extras_slot: SharedSlot,
}

impl PreparedSbe {
    /// Session-local layout id of the raw-SBE layout.
    #[must_use]
    pub const fn layout_id(&self) -> u16 {
        self.layout_id
    }
    /// Enable slot for the raw-SBE table.
    #[must_use]
    pub const fn slot(&self) -> &SharedSlot {
        &self.slot
    }
    /// Enable slot for the typed extras layout.
    #[must_use]
    pub const fn extras_slot(&self) -> &SharedSlot {
        &self.extras_slot
    }
    /// Bound policy id.
    #[must_use]
    pub const fn policy_id(&self) -> u16 {
        self.policy_id
    }
}

/// Prepared dynamic-table handle with resolved column slots.
pub struct PreparedDynamic {
    pub(crate) session_id: u64,
    pub(crate) layout_id: u16,
    pub(crate) policy_id: u16,
    pub(crate) temporary: bool,
    pub(crate) schema: Rc<RowSchema>,
    pub(crate) slot: SharedSlot,
    /// Identity of the builder this layout came from; column handles carry
    /// the same token so a handle minted on another layout is rejected.
    pub(crate) layout_token: u64,
}

impl PreparedDynamic {
    /// Storage schema (column order defines wire order).
    #[must_use]
    pub fn schema(&self) -> &RowSchema {
        &self.schema
    }

    /// Resolve a typed column handle to its slot.
    ///
    /// A handle registered on a different dynamic layout addresses a
    /// different column order, so it is rejected rather than silently
    /// writing into the wrong slot.
    pub fn slot_of<T>(
        &self,
        column: &super::registration::Column<T>,
    ) -> Result<usize, crate::registration::RegistrationError> {
        if column.layout_token != self.layout_token {
            return Err(crate::registration::RegistrationError::Conflict(
                "column handle belongs to another layout",
            ));
        }
        Ok(column.slot)
    }
}

/// Thread-confined recording writer with pre-sized buffers.
///
/// Sequence numbers advance with a plain increment — the process restarts
/// long before a `u64` could exhaust. A single writer may use separate
/// permanent/temporary publications for capacity isolation; sequence
/// numbers are unique across both.
pub struct Writer {
    session_id: u64,
    writer_id: u16,
    next_sequence: u64,
    permanent: Transport,
    diagnostics: Transport,
    row_scratch: Vec<u8>,
    envelope_scratch: Vec<u8>,
    quota: Quota,
    counters: WriterCounters,
    generation: Rc<Cell<u32>>,
}

impl Writer {
    pub(crate) fn new(
        session_id: u64,
        writer_id: u16,
        permanent: Transport,
        diagnostics: Transport,
        scratch: Vec<u8>,
        quota_per_second: u64,
        generation: Rc<Cell<u32>>,
    ) -> Self {
        let envelope_scratch = scratch.clone();
        Self {
            session_id,
            writer_id,
            next_sequence: 1,
            permanent,
            diagnostics,
            row_scratch: scratch,
            envelope_scratch,
            quota: Quota::new(quota_per_second),
            counters: WriterCounters::default(),
            generation,
        }
    }

    /// Counters (plain read; the owner samples between record calls).
    #[must_use]
    pub const fn counters(&self) -> &WriterCounters {
        &self.counters
    }

    /// Current sequence (next assigned value).
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Drain retained in-memory publication frames (lab/fixture ingest).
    /// Empty when the permanent transport is Aeron.
    pub fn drain_permanent(&mut self) -> Vec<Vec<u8>> {
        match &mut self.permanent {
            Transport::Memory(ring) => {
                let mut out = Vec::new();
                let mut buf = vec![0u8; limits::MAX_RECORD_BYTES];
                while let Some(n) = ring.pop(&mut buf) {
                    out.push(buf[..n].to_vec());
                }
                out
            }
            #[cfg(feature = "producer")]
            Transport::Aeron(_) | Transport::AeronShared(_) => Vec::new(),
        }
    }

    /// Writer identity within the run.
    #[must_use]
    pub const fn writer_id(&self) -> u16 {
        self.writer_id
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_and_publish(
        &mut self,
        temporary: bool,
        kind: RecordKind,
        layout_id: u16,
        policy_id: u16,
        captured_at_ns: u64,
        payload: PayloadRef<'_>,
    ) -> RecordOutcome {
        // Field-disjoint borrows: row_scratch (read), envelope_scratch
        // (write), counters/quota (write) — no allocation, no wait.
        let payload_slice: &[u8] = match payload {
            PayloadRef::Row(n) => &self.row_scratch[..n],
            PayloadRef::Bytes(b) => b,
        };
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let len = match encode_data_record(
            &mut self.envelope_scratch,
            kind,
            self.writer_id,
            layout_id,
            policy_id,
            sequence,
            captured_at_ns,
            self.generation.get(),
            payload_slice,
        ) {
            Ok(l) => l,
            Err(EncodeError::BufferTooSmall { .. }) => {
                self.counters.invalid += 1;
                return RecordOutcome::Invalid(reasons::OVERSIZE);
            }
            Err(_) => {
                self.counters.invalid += 1;
                return RecordOutcome::Invalid(reasons::CALLBACK_ERROR);
            }
        };
        // Diagnostics quota is enforced only after the enable guard and
        // only on the temporary publication; quota exhaustion is a counted
        // drop that cannot slow or suspend permanent recording.
        if temporary && !self.quota.try_take(len as u64) {
            self.counters.dropped += 1;
            self.counters.quota_drops += 1;
            return RecordOutcome::Dropped(reasons::QUOTA_EXHAUSTED);
        }
        let ok = {
            let transport = match temporary {
                true => &mut self.diagnostics,
                false => &mut self.permanent,
            };
            transport.offer(&self.envelope_scratch[..len])
        };
        if ok {
            self.counters.published += 1;
            self.counters.bytes_published += len as u64;
            RecordOutcome::Published
        } else {
            self.counters.dropped += 1;
            self.counters.transport_full += 1;
            RecordOutcome::Dropped(reasons::TRANSPORT_FULL)
        }
    }

    /// Record a typed row value into a table.
    pub fn record<T: Persistable>(
        &mut self,
        table: &PreparedTable<T>,
        value: &T,
        captured_at_ns: u64,
    ) -> RecordOutcome {
        self.record_with(table, captured_at_ns, |row| value.encode(row))
    }

    /// Record a row via a closure that writes into the supplied
    /// [`RowWriter`]. The closure runs only when the table is enabled; a
    /// callback error aborts the row before publication.
    pub fn record_with<T, F>(
        &mut self,
        table: &PreparedTable<T>,
        captured_at_ns: u64,
        f: F,
    ) -> RecordOutcome
    where
        T: Persistable,
        F: FnOnce(&mut RowWriter<'_>) -> Result<(), EncodeError>,
    {
        // A foreign-session handle is a programming error: reject before
        // the gate and before publication.
        if table.session_id != self.session_id {
            self.counters.invalid += 1;
            return RecordOutcome::Invalid(reasons::SESSION_MISMATCH);
        }
        if !EnableSlot::is_enabled_word(table.slot.load()) {
            // Disabled calls consume no sequence number, no clock read, and
            // no counter increment on this path.
            return RecordOutcome::Disabled;
        }
        let payload_len = {
            let mut row = match RowWriter::new(&mut self.row_scratch, T::schema()) {
                Ok(r) => r,
                Err(_) => {
                    self.counters.invalid += 1;
                    return RecordOutcome::Invalid(reasons::CALLBACK_ERROR);
                }
            };
            if f(&mut row).is_err() {
                self.counters.invalid += 1;
                return RecordOutcome::Invalid(reasons::CALLBACK_ERROR);
            }
            row.position()
        };
        self.encode_and_publish(
            table.temporary,
            RecordKind::TypedRow,
            table.layout_id,
            table.policy_id,
            captured_at_ns,
            PayloadRef::Row(payload_len),
        )
    }

    /// Record into a dynamic table via column handles.
    pub fn record_dynamic<F>(
        &mut self,
        table: &PreparedDynamic,
        captured_at_ns: u64,
        f: F,
    ) -> RecordOutcome
    where
        F: FnOnce(&mut RowWriter<'_>) -> Result<(), EncodeError>,
    {
        // Order matches `record_with`/`record_sbe`: a foreign-session handle
        // is an error regardless of whether the table happens to be enabled.
        if table.session_id != self.session_id {
            self.counters.invalid += 1;
            return RecordOutcome::Invalid(reasons::SESSION_MISMATCH);
        }
        if !EnableSlot::is_enabled_word(table.slot.load()) {
            return RecordOutcome::Disabled;
        }
        let payload_len = {
            let mut row = match RowWriter::new(&mut self.row_scratch, &table.schema) {
                Ok(r) => r,
                Err(_) => {
                    self.counters.invalid += 1;
                    return RecordOutcome::Invalid(reasons::CALLBACK_ERROR);
                }
            };
            if f(&mut row).is_err() {
                self.counters.invalid += 1;
                return RecordOutcome::Invalid(reasons::CALLBACK_ERROR);
            }
            row.position()
        };
        self.encode_and_publish(
            table.temporary,
            RecordKind::TypedRow,
            table.layout_id,
            table.policy_id,
            captured_at_ns,
            PayloadRef::Row(payload_len),
        )
    }

    /// Type-erased row recording used by the tracing adapter: the enable
    /// gate, session-free layout ids, schema, and closure are supplied
    /// explicitly. The enable word is read first.
    #[allow(clippy::too_many_arguments)]
    pub fn record_raw_row<F>(
        &mut self,
        slot: &SyncSlot,
        layout_id: u16,
        policy_id: u16,
        temporary: bool,
        schema: &RowSchema,
        captured_at_ns: u64,
        f: F,
    ) -> RecordOutcome
    where
        F: FnOnce(&mut RowWriter<'_>) -> Result<(), EncodeError>,
    {
        if !EnableSlot::is_enabled_word(slot.load()) {
            return RecordOutcome::Disabled;
        }
        let payload_len = {
            let mut row = match RowWriter::new(&mut self.row_scratch, schema) {
                Ok(r) => r,
                Err(_) => {
                    self.counters.invalid += 1;
                    return RecordOutcome::Invalid(reasons::CALLBACK_ERROR);
                }
            };
            if f(&mut row).is_err() {
                self.counters.invalid += 1;
                return RecordOutcome::Invalid(reasons::CALLBACK_ERROR);
            }
            row.position()
        };
        self.encode_and_publish(
            temporary,
            RecordKind::TypedRow,
            layout_id,
            policy_id,
            captured_at_ns,
            PayloadRef::Row(payload_len),
        )
    }

    /// Record raw SBE bytes (original header retained) plus a typed extras
    /// row into the separately registered extras layout. Two envelopes are
    /// published under distinct sequence numbers; each attempt is counted
    /// independently.
    pub fn record_sbe<E: Persistable>(
        &mut self,
        table: &PreparedSbe,
        sbe_bytes: &[u8],
        extras: &E,
        captured_at_ns: u64,
    ) -> RecordOutcome {
        if table.session_id != self.session_id {
            self.counters.invalid += 1;
            return RecordOutcome::Invalid(reasons::SESSION_MISMATCH);
        }
        if !EnableSlot::is_enabled_word(table.slot.load()) {
            return RecordOutcome::Disabled;
        }
        if sbe_bytes.len() > limits::MAX_RECORD_BYTES {
            self.counters.invalid += 1;
            return RecordOutcome::Invalid(reasons::OVERSIZE);
        }
        // The envelope encode copies the borrowed payload into the
        // pre-sized envelope scratch; the caller's buffer is free the
        // moment this call returns.
        let raw_outcome = self.encode_and_publish(
            table.temporary,
            RecordKind::RawSbe,
            table.layout_id,
            table.policy_id,
            captured_at_ns,
            PayloadRef::Bytes(sbe_bytes),
        );
        // Extras row publishes only when its own gate is enabled; it never
        // blocks or undoes the raw record.
        if EnableSlot::is_enabled_word(table.extras_slot.load()) && table.extras_layout_id != 0 {
            let payload_len = {
                let mut row = match RowWriter::new(&mut self.row_scratch, E::schema()) {
                    Ok(r) => r,
                    Err(_) => {
                        self.counters.invalid += 1;
                        return raw_outcome;
                    }
                };
                if extras.encode(&mut row).is_err() {
                    self.counters.invalid += 1;
                    return raw_outcome;
                }
                row.position()
            };
            let _ = self.encode_and_publish(
                table.temporary,
                RecordKind::TypedRow,
                table.extras_layout_id,
                table.policy_id,
                captured_at_ns,
                PayloadRef::Row(payload_len),
            );
        }
        raw_outcome
    }
}
