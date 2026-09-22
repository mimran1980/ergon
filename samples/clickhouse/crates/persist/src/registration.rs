//! Control-plane registration: sessions, prepared handles, dynamic layouts.
//!
//! `RecorderSession` is control-thread state. It owns registration requests
//! and immutable session metadata; changing static process metadata creates
//! a new registered metadata/layout version instead of editing previously
//! recorded meaning. Recording calls never register, look up strings, or
//! wait.

use crate::protocol::{
    Declaration, LayoutDeclaration, Policy, PolicyDeclaration, RowEncoding, SessionCatalog,
    SessionStartDeclaration, SymbolDeclaration, limits,
};
use crate::recorder::{
    EnableSlot, PreparedDynamic, PreparedSbe, PreparedTable, SharedSlot, Transport, Writer,
};
use crate::schema::{RowSchema, TypeCode, ValueSchema};
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Mutex;

/// Typed registration failures. Numeric codes also travel in counters.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum RegistrationError {
    /// Layout/column capacity exceeded.
    Capacity(&'static str),
    /// Conflicting definition for an existing identity.
    Conflict(&'static str),
    /// Table/policy name or value invalid (reserved prefix, malformed).
    Invalid(&'static str),
    /// Transport failed to deliver the declaration.
    Transport(String),
    /// Schema conversion mismatch (dynamic column type).
    TypeMismatch { column: String },
}

impl std::fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capacity(m) => write!(f, "capacity: {m}"),
            Self::Conflict(m) => write!(f, "conflict: {m}"),
            Self::Invalid(m) => write!(f, "invalid: {m}"),
            Self::Transport(m) => write!(f, "transport: {m}"),
            Self::TypeMismatch { column } => write!(f, "type mismatch: {column}"),
        }
    }
}

impl std::error::Error for RegistrationError {}

/// Delivery of declarations to the registration authority. The control
/// thread only; never the recording path.
pub trait RegistrationSink {
    /// Deliver one declaration. Idempotent retries must be safe.
    fn send(&mut self, decl: &Declaration) -> Result<(), RegistrationError>;
}

/// Registration sink that publishes declarations onto the recorded stream.
///
/// The ingester resolves layouts from these declarations; without them it sees
/// data records with no catalog and rejects every row. It shares the writer's
/// publication so a declaration and the rows that depend on it stay ordered —
/// two publications could not be ordered even if Aeron allowed them.
#[cfg(feature = "producer")]
#[derive(Clone)]
pub struct AeronSink {
    publication: crate::recorder::SharedPublication,
    scratch: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
}

#[cfg(feature = "producer")]
impl AeronSink {
    /// Sink publishing through `publication`.
    #[must_use]
    pub fn new(publication: crate::recorder::SharedPublication) -> Self {
        Self {
            publication,
            scratch: std::rc::Rc::new(std::cell::RefCell::new(Vec::with_capacity(4096))),
        }
    }
}

#[cfg(feature = "producer")]
impl RegistrationSink for AeronSink {
    fn send(&mut self, decl: &Declaration) -> Result<(), RegistrationError> {
        let mut buf = self.scratch.borrow_mut();
        // Grow only to the declared maximum, once: a declaration is bounded.
        if buf.len() < limits::MAX_RECORD_BYTES {
            buf.resize(limits::MAX_RECORD_BYTES, 0);
        }
        let len = decl
            .encode(&mut buf)
            .map_err(|_| RegistrationError::Transport("declaration encode".into()))?;
        if !self.publication.borrow_mut().offer(&buf[..len]) {
            return Err(RegistrationError::Transport(
                "declaration publication not connected".into(),
            ));
        }
        Ok(())
    }
}

/// In-memory registration sink for tests and the local laboratory.
#[derive(Clone, Default)]
pub struct InMemorySink {
    queue: std::rc::Rc<Mutex<Vec<Declaration>>>,
}

impl InMemorySink {
    /// New empty sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain delivered declarations in order.
    #[must_use]
    pub fn drain(&self) -> Vec<Declaration> {
        std::mem::take(&mut self.queue.lock().expect("poisoned"))
    }
}

impl RegistrationSink for InMemorySink {
    fn send(&mut self, decl: &Declaration) -> Result<(), RegistrationError> {
        self.queue.lock().expect("poisoned").push(decl.clone());
        Ok(())
    }
}

/// Transport for data publications.
#[derive(Clone, Debug)]
pub enum TransportConfig {
    /// Pre-sized in-memory ring (tests/lab; zero alloc after construction).
    Memory {
        /// Number of retained record slots.
        slots: usize,
        /// Per-slot byte capacity.
        slot_bytes: usize,
    },
    /// An already-connected Aeron publication, shared with the registration
    /// sink so declarations and data share one ordered stream.
    #[cfg(feature = "producer")]
    AeronShared(crate::recorder::SharedPublication),
    /// Aeron exclusive publication (requires the `producer` feature).
    #[cfg(feature = "producer")]
    Aeron {
        /// Channel URI.
        channel: String,
        /// Stream id.
        stream_id: i32,
    },
}

/// Process/session configuration supplied at connect time.
#[derive(Clone, Debug)]
pub struct RecorderConfig {
    /// Static process name (registered once).
    pub process: String,
    /// Instance name within the process role.
    pub instance: String,
    /// Build revision string.
    pub build: String,
    /// Maximum complete encoded record; never grows buffers past it.
    pub max_record_bytes: usize,
    /// Diagnostics payload quota per second per writer, after the enable gate.
    pub diagnostics_quota_bytes_per_sec: u64,
    /// Permanent-stream transport.
    pub permanent: TransportConfig,
    /// Diagnostics-stream transport (capacity isolation).
    pub diagnostics: TransportConfig,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            process: "app".into(),
            instance: "default".into(),
            build: env!("CARGO_PKG_VERSION").into(),
            max_record_bytes: limits::MAX_RECORD_BYTES,
            diagnostics_quota_bytes_per_sec: 1024 * 1024,
            permanent: TransportConfig::Memory {
                slots: 4096,
                slot_bytes: limits::MAX_RECORD_BYTES,
            },
            diagnostics: TransportConfig::Memory {
                slots: 256,
                slot_bytes: 256 * 1024,
            },
        }
    }
}

/// Column value types accepted by dynamic layouts.
pub trait DynScalar: Copy {
    /// Storage type code.
    fn type_code() -> TypeCode;
    /// Little-endian fixed-width bytes.
    fn to_le_bytes(self) -> [u8; 16];
}

impl DynScalar for i64 {
    fn type_code() -> TypeCode {
        TypeCode::I64
    }
    fn to_le_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&i64::to_le_bytes(self));
        b
    }
}

impl DynScalar for u64 {
    fn type_code() -> TypeCode {
        TypeCode::U64
    }
    fn to_le_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&u64::to_le_bytes(self));
        b
    }
}

impl DynScalar for u32 {
    fn type_code() -> TypeCode {
        TypeCode::U32
    }
    fn to_le_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..4].copy_from_slice(&u32::to_le_bytes(self));
        b
    }
}

impl DynScalar for f64 {
    fn type_code() -> TypeCode {
        TypeCode::F64
    }
    fn to_le_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&f64::to_le_bytes(self));
        b
    }
}

/// Interned repeated value (opt-in for instrument identifiers).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Symbol(pub u32);

impl DynScalar for Symbol {
    fn type_code() -> TypeCode {
        TypeCode::U32
    }
    fn to_le_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..4].copy_from_slice(&u32::to_le_bytes(self.0));
        b
    }
}

/// Typed handle for one named dynamic column slot.
///
/// Carries the identity of the layout that minted it, so a handle from
/// another layout is rejected by [`PreparedDynamic::slot_of`] rather than
/// silently addressing a different column order.
#[derive(Clone, Copy, Debug)]
pub struct Column<T> {
    pub(crate) layout_token: u64,
    pub(crate) slot: usize,
    _t: PhantomData<fn(T)>,
}

/// Control-thread session: registration, catalogs, and writer construction.
pub struct RecorderSession {
    config: RecorderConfig,
    run_id: u64,
    started_at_ns: u64,
    catalog: SessionCatalog,
    sink: Box<dyn RegistrationSink>,
    next_writer_id: u16,
    /// Monotonic identity for dynamic-layout builders, so a column handle
    /// can be tied to the layout that minted it.
    next_layout_token: u64,
    sessions_enabled: bool,
    enable_slots: Vec<SharedSlot>,
    generation: Rc<Cell<u32>>,
    /// Effective retention per table, from the recorder's own config.
    ///
    /// PLAN §5: "Register a compact policy ID outside the trading loop". The
    /// policy a table declares must carry the retention the operator actually
    /// configured; without this the declaration went out with zeros and every
    /// temporary row expired at its capture time. Kept as a small Vec because
    /// it is written once at startup and read once per table declaration.
    retention: Vec<(String, u64, u64)>,
}

impl RecorderSession {
    /// Connect with the default in-memory registration sink.
    pub fn connect(config: RecorderConfig) -> Result<Self, RegistrationError> {
        Self::connect_with(config, Box::new(InMemorySink::new()))
    }

    /// Connect with an explicit registration sink.
    pub fn connect_with(
        config: RecorderConfig,
        sink: Box<dyn RegistrationSink>,
    ) -> Result<Self, RegistrationError> {
        if config.max_record_bytes > limits::MAX_RECORD_BYTES {
            return Err(RegistrationError::Invalid(
                "max_record_bytes over protocol limit",
            ));
        }
        let run_id = fresh_run_id();
        let started_at_ns = now_ns();
        let generation = Rc::new(Cell::new(1));
        let session = Self {
            run_id,
            started_at_ns,
            catalog: SessionCatalog::new(),
            sink,
            next_writer_id: 1,
            next_layout_token: 0,
            sessions_enabled: true,
            enable_slots: Vec::new(),
            generation,
            config,
            retention: Vec::new(),
        };
        Ok(session)
    }

    /// Run identity for this session.
    #[must_use]
    pub const fn run_id(&self) -> u64 {
        self.run_id
    }

    /// Register static process metadata (control plane only).
    ///
    /// Recognized keys are `process`, `instance`, and `build`; they override
    /// the configured values in the session start declaration, which the
    /// ingester expands. Unknown keys create a new metadata version in v2;
    /// v1 rejects them rather than recording undefined meaning.
    pub fn metadata(&mut self, key: &str, value: &str) -> Result<(), RegistrationError> {
        match key {
            "process" => self.config.process = value.to_string(),
            "instance" => self.config.instance = value.to_string(),
            "build" => self.config.build = value.to_string(),
            _ => return Err(RegistrationError::Invalid("unknown metadata key")),
        }
        Ok(())
    }

    /// Prepare a typed table bound to a storage schema, encoding, and
    /// explicit column names (low-level form used by codegen callers).
    pub fn table_schema(
        &mut self,
        table: &str,
        policy: Policy,
        schema: RowSchema,
        encoding: RowEncoding,
        fingerprint: u64,
        column_names: &[&str],
    ) -> Result<PreparedSchemaHandle, RegistrationError> {
        if table.starts_with("_record_") {
            return Err(RegistrationError::Invalid("reserved system prefix"));
        }
        if schema.len() > limits::MAX_COLUMNS {
            return Err(RegistrationError::Capacity("too many columns"));
        }
        let policy_id = self.register_policy_for(table, policy)?;
        let columns: Vec<crate::protocol::ColumnDeclaration> = schema
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| crate::protocol::ColumnDeclaration {
                name: column_names.get(i).copied().unwrap_or("col").to_string(),
                type_code: c.ty as u8,
                flags: c.wire_flags(),
                precision: c.precision,
                scale: c.scale,
            })
            .collect();
        let decl = LayoutDeclaration {
            layout_id: 0,
            policy_id,
            payload_encoding: encoding,
            schema_fingerprint: layout_fingerprint(table, fingerprint),
            projection_revision: 1,
            sbe_schema_id: 0,
            sbe_template_id: 0,
            sbe_version: 0,
            catalog_generation: 0,
            table_name: table.to_string(),
            columns,
        };
        let layout_id = self
            .catalog
            .register_layout(decl)
            .map_err(|_| RegistrationError::Capacity("layout dictionary full"))?;
        let decl = self
            .catalog
            .layout(layout_id)
            .cloned()
            .ok_or(RegistrationError::Conflict("layout vanished"))?;
        self.sink.send(&Declaration::Layout(decl))?;
        Ok(PreparedSchemaHandle {
            policy_id,
            fingerprint,
            layout_id,
        })
    }

    fn sync_generation(&mut self) {
        self.generation.set(self.catalog.generation());
    }

    /// Record a table's effective retention before it is declared.
    ///
    /// Returns `true` when this differs from what was recorded before, which is
    /// the signal the caller needs to *version* the change: PLAN §5 requires a
    /// retention edit to mint a new policy id and bind it to a new layout,
    /// because the ingester freezes a layout→storage binding on first sight and
    /// deliberately never recomputes it. Overwriting the policy in place would
    /// leave the existing binding — and therefore every row's expiry — at the
    /// old value.
    ///
    /// Called from the config path, which knows the operator's intent. A table
    /// with no entry declares zeros, which the wire documents as
    /// "permanent/default" — the correct reading of "nothing configured".
    pub fn set_retention(&mut self, table: &str, row_ttl_ns: u64, idle_ttl_ns: u64) -> bool {
        match self.retention.iter_mut().find(|(t, _, _)| t == table) {
            Some(entry) => {
                let changed = entry.1 != row_ttl_ns || entry.2 != idle_ttl_ns;
                entry.1 = row_ttl_ns;
                entry.2 = idle_ttl_ns;
                changed
            }
            None => {
                self.retention
                    .push((table.to_string(), row_ttl_ns, idle_ttl_ns));
                // A table we have never recorded a retention for is a change
                // only in the sense that nothing was declared yet; the caller
                // acts on this before the first declaration, when it is moot.
                true
            }
        }
    }

    /// A table's configured retention, or zeros when none was set.
    #[must_use]
    pub fn retention_for(&self, table: &str) -> (u64, u64) {
        self.retention
            .iter()
            .find(|(t, _, _)| t == table)
            .map_or((0, 0), |(_, r, i)| (*r, *i))
    }

    fn register_policy_for(
        &mut self,
        table: &str,
        policy: Policy,
    ) -> Result<u16, RegistrationError> {
        let (row_ttl_ns, idle_ttl_ns) = self.retention_for(table);
        let decl = PolicyDeclaration {
            policy_id: 0,
            policy,
            row_ttl_ns,
            idle_ttl_ns,
        };
        let policy_id = self
            .catalog
            .register_policy(decl)
            .map_err(|_| RegistrationError::Capacity("policy dictionary full"))?;
        let declaration = Declaration::Policy(PolicyDeclaration {
            policy_id,
            policy,
            // The *declared* policy is what the ingester persists and freezes
            // into its binding, so it has to carry the real retention — this
            // is the only place it can enter the system.
            row_ttl_ns,
            idle_ttl_ns,
        });
        self.sink.send(&declaration)?;
        self.sync_generation();
        Ok(policy_id)
    }

    /// Prepare a typed callsite table bound to an explicit static schema
    /// (used by `persist_table!` expansions).
    pub fn table_schema_typed<T: crate::persist::Persistable>(
        &mut self,
        table: &str,
        policy: Policy,
        schema: &'static RowSchema,
    ) -> Result<PreparedTable<T>, RegistrationError> {
        if table.starts_with("_record_") {
            return Err(RegistrationError::Invalid("reserved system prefix"));
        }
        if schema.len() > limits::MAX_COLUMNS {
            return Err(RegistrationError::Capacity("too many columns"));
        }
        let policy_id = self.register_policy_for(table, policy)?;
        let columns: Vec<crate::protocol::ColumnDeclaration> = schema
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| crate::protocol::ColumnDeclaration {
                name: format!("col{i}"),
                type_code: c.ty as u8,
                flags: c.wire_flags(),
                precision: c.precision,
                scale: c.scale,
            })
            .collect();
        let decl = LayoutDeclaration {
            layout_id: 0,
            policy_id,
            payload_encoding: RowEncoding::OrderedRow,
            schema_fingerprint: layout_fingerprint(table, schema.fingerprint()),
            projection_revision: 1,
            sbe_schema_id: 0,
            sbe_template_id: 0,
            sbe_version: 0,
            catalog_generation: 0,
            table_name: table.to_string(),
            columns,
        };
        let layout_id = self
            .catalog
            .register_layout(decl)
            .map_err(|_| RegistrationError::Capacity("layout dictionary full"))?;
        let decl = self
            .catalog
            .layout(layout_id)
            .cloned()
            .ok_or(RegistrationError::Conflict("layout vanished"))?;
        self.sink.send(&Declaration::Layout(decl))?;
        let slot = Rc::new(EnableSlot::new(policy_id));
        self.enable_slots.push(Rc::clone(&slot));
        self.sync_generation();
        Ok(PreparedTable {
            session_id: self.run_id,
            layout_id,
            policy_id,
            temporary: matches!(policy, Policy::Temporary),
            slot,
            _marker: PhantomData,
        })
    }

    /// Prepare a typed table for a [`crate::persist::Persistable`] row type.
    pub fn table<T: crate::persist::Persistable>(
        &mut self,
        table: &str,
        policy: Policy,
    ) -> Result<PreparedTable<T>, RegistrationError> {
        self.table_named::<T>(table, policy, &[])
    }

    /// Prepare a typed table with explicit storage column names.
    pub fn table_named<T: crate::persist::Persistable>(
        &mut self,
        table: &str,
        policy: Policy,
        column_names: &[&str],
    ) -> Result<PreparedTable<T>, RegistrationError> {
        if table.starts_with("_record_") {
            return Err(RegistrationError::Invalid("reserved system prefix"));
        }
        let schema = T::schema();
        if schema.len() > limits::MAX_COLUMNS {
            return Err(RegistrationError::Capacity("too many columns"));
        }
        let policy_id = self.register_policy_for(table, policy)?;
        let columns: Vec<crate::protocol::ColumnDeclaration> = schema
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| crate::protocol::ColumnDeclaration {
                name: column_names.get(i).copied().unwrap_or("col").to_string(),
                type_code: c.ty as u8,
                flags: c.wire_flags(),
                precision: c.precision,
                scale: c.scale,
            })
            .collect();
        let decl = LayoutDeclaration {
            layout_id: 0,
            policy_id,
            payload_encoding: RowEncoding::OrderedRow,
            schema_fingerprint: layout_fingerprint(table, schema.fingerprint()),
            projection_revision: 1,
            sbe_schema_id: 0,
            sbe_template_id: 0,
            sbe_version: 0,
            catalog_generation: 0,
            table_name: table.to_string(),
            columns,
        };
        let layout_id = self
            .catalog
            .register_layout(decl)
            .map_err(|_| RegistrationError::Capacity("layout dictionary full"))?;
        let decl = self
            .catalog
            .layout(layout_id)
            .cloned()
            .ok_or(RegistrationError::Conflict("layout vanished"))?;
        self.sink.send(&Declaration::Layout(decl))?;
        let slot = Rc::new(EnableSlot::new(policy_id));
        self.enable_slots.push(Rc::clone(&slot));
        self.sync_generation();
        Ok(PreparedTable {
            session_id: self.run_id,
            layout_id,
            policy_id,
            temporary: matches!(policy, Policy::Temporary),
            slot,
            _marker: PhantomData,
        })
    }

    /// Prepare a raw-SBE table with a projection descriptor and typed
    /// extra columns.
    pub fn sbe_table<E: crate::persist::Persistable>(
        &mut self,
        table: &str,
        policy: Policy,
        descriptor: &crate::schema::ProjectionDescriptor,
        extras_table: &str,
    ) -> Result<PreparedSbe, RegistrationError> {
        if table.starts_with("_record_") {
            return Err(RegistrationError::Invalid("reserved system prefix"));
        }
        let policy_id = self.register_policy_for(table, policy)?;
        let columns: Vec<crate::protocol::ColumnDeclaration> = descriptor
            .row_schema
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| crate::protocol::ColumnDeclaration {
                name: format!("col{i}"),
                type_code: c.ty as u8,
                flags: c.wire_flags(),
                precision: c.precision,
                scale: c.scale,
            })
            .collect();
        let decl = LayoutDeclaration {
            layout_id: 0,
            policy_id,
            payload_encoding: RowEncoding::RawSbe,
            schema_fingerprint: layout_fingerprint(table, descriptor.schema_fingerprint),
            projection_revision: descriptor.projection_revision,
            sbe_schema_id: descriptor.sbe_schema_id,
            sbe_template_id: descriptor.sbe_template_id,
            sbe_version: descriptor.sbe_version,
            catalog_generation: 0,
            table_name: table.to_string(),
            columns,
        };
        let layout_id = self
            .catalog
            .register_layout(decl)
            .map_err(|_| RegistrationError::Capacity("layout dictionary full"))?;
        let decl = self
            .catalog
            .layout(layout_id)
            .cloned()
            .ok_or(RegistrationError::Conflict("layout vanished"))?;
        self.sink.send(&Declaration::Layout(decl))?;

        // Extras layout for the typed metadata row.
        let extras = self.table_named::<E>(extras_table, policy, &[])?;
        let slot = Rc::new(EnableSlot::new(policy_id));
        self.enable_slots.push(Rc::clone(&slot));
        self.sync_generation();
        Ok(PreparedSbe {
            session_id: self.run_id,
            layout_id,
            policy_id,
            temporary: matches!(policy, Policy::Temporary),
            extras_layout_id: extras.layout_id,
            extras_slot: extras.slot,
            slot,
        })
    }

    /// Prepare a dynamic (quant-friendly) table whose columns resolve at
    /// prepare time on the control thread.
    pub fn dynamic_table(
        &mut self,
        table: &str,
        policy: Policy,
    ) -> Result<DynamicLayoutBuilder<'_>, RegistrationError> {
        if table.starts_with("_record_") {
            return Err(RegistrationError::Invalid("reserved system prefix"));
        }
        let policy_id = self.register_policy_for(table, policy)?;
        let layout_token = self.next_layout_token;
        self.next_layout_token += 1;
        Ok(DynamicLayoutBuilder {
            session: self,
            table: table.to_string(),
            policy,
            policy_id,
            layout_token,
            names: Vec::new(),
            columns: Vec::new(),
        })
    }

    /// Intern a symbol value; returns its compact id.
    ///
    /// The declaration is sent once per new value (idempotent retransmission
    /// after a crash is permitted; regular rebroadcast is not).
    pub fn intern_symbol(&mut self, value: &str) -> Result<Symbol, RegistrationError> {
        let known_before = self.catalog.symbols().iter().any(|s| s.value == value);
        let id = self
            .catalog
            .intern_symbol(value)
            .map_err(|_| RegistrationError::Capacity("symbol dictionary full"))?;
        if !known_before {
            self.sink.send(&Declaration::Symbol(SymbolDeclaration {
                symbol_id: id,
                value: value.to_string(),
            }))?;
        }
        Ok(Symbol(id))
    }

    /// Construct a thread-confined writer with pre-sized buffers.
    pub fn writer(&mut self) -> Result<Writer, RegistrationError> {
        let writer_id = self.next_writer_id;
        self.next_writer_id = self
            .next_writer_id
            .checked_add(1)
            .ok_or(RegistrationError::Capacity("writer ids exhausted"))?;
        let permanent = make_transport(&self.config.permanent)?;
        let diagnostics = make_transport(&self.config.diagnostics)?;
        let scratch = vec![0u8; self.config.max_record_bytes];
        Ok(Writer::new(
            self.run_id,
            writer_id,
            permanent,
            diagnostics,
            scratch,
            self.config.diagnostics_quota_bytes_per_sec,
            Rc::clone(&self.generation),
        ))
    }

    /// Publish the session start declaration (idempotent).
    pub fn declare_session_start(&mut self) -> Result<(), RegistrationError> {
        if !self.sessions_enabled {
            return Ok(());
        }
        let decl = Declaration::SessionStart(SessionStartDeclaration {
            run_id: self.run_id,
            started_at_ns: self.started_at_ns,
            process: self.config.process.clone(),
            instance: self.config.instance.clone(),
            build: self.config.build.clone(),
        });
        self.sink.send(&decl)?;
        self.sessions_enabled = false;
        Ok(())
    }

    /// Publish the session end declaration.
    pub fn declare_session_end(&mut self, final_sequence: u64) -> Result<(), RegistrationError> {
        let decl = Declaration::SessionEnd {
            ended_at_ns: now_ns(),
            final_sequence,
        };
        self.sink.send(&decl)
    }

    /// Session catalog snapshot (control plane).
    #[must_use]
    pub fn catalog(&self) -> &SessionCatalog {
        &self.catalog
    }
}

/// Intermediate handle carrying policy/fingerprint/layout between
/// registration steps (used by codegen callers).
pub struct PreparedSchemaHandle {
    /// Resolved policy id.
    pub policy_id: u16,
    /// Schema fingerprint.
    pub fingerprint: u64,
    /// Resolved layout id.
    pub layout_id: u16,
}

/// Builder for a dynamic table layout; columns resolve by name on the
/// control thread.
pub struct DynamicLayoutBuilder<'s> {
    session: &'s mut RecorderSession,
    table: String,
    policy: Policy,
    policy_id: u16,
    layout_token: u64,
    names: Vec<String>,
    columns: Vec<ValueSchema>,
}

impl<'s> DynamicLayoutBuilder<'s> {
    /// Resolve one typed column handle by name.
    pub fn column<T: DynScalar>(&mut self, name: &str) -> Result<Column<T>, RegistrationError> {
        if self.names.iter().any(|n| n == name) {
            return Err(RegistrationError::Conflict("duplicate column"));
        }
        if self.names.len() >= limits::MAX_COLUMNS {
            return Err(RegistrationError::Capacity("too many columns"));
        }
        let slot = self.names.len();
        self.names.push(name.to_string());
        self.columns.push(ValueSchema::scalar(T::type_code()));
        Ok(Column {
            layout_token: self.layout_token,
            slot,
            _t: PhantomData,
        })
    }

    /// Register the layout and return the prepared dynamic table.
    pub fn prepare(self) -> Result<PreparedDynamic, RegistrationError> {
        let schema = RowSchema::from_vec(self.columns);
        let fingerprint = layout_fingerprint(&self.table, schema.fingerprint());
        let decl = LayoutDeclaration {
            layout_id: 0,
            policy_id: self.policy_id,
            payload_encoding: RowEncoding::DynamicRow,
            schema_fingerprint: fingerprint,
            projection_revision: 1,
            sbe_schema_id: 0,
            sbe_template_id: 0,
            sbe_version: 0,
            catalog_generation: 0,
            table_name: self.table.clone(),
            columns: self
                .names
                .iter()
                .zip(schema.columns)
                .map(|(n, c)| crate::protocol::ColumnDeclaration {
                    name: n.clone(),
                    type_code: c.ty as u8,
                    flags: c.wire_flags(),
                    precision: c.precision,
                    scale: c.scale,
                })
                .collect(),
        };
        let layout_id = self
            .session
            .catalog
            .register_layout(decl)
            .map_err(|_| RegistrationError::Capacity("layout dictionary full"))?;
        let decl = self
            .session
            .catalog
            .layout(layout_id)
            .cloned()
            .ok_or(RegistrationError::Conflict("layout vanished"))?;
        self.session.sink.send(&Declaration::Layout(decl))?;
        let slot = Rc::new(EnableSlot::new(self.policy_id));
        self.session.enable_slots.push(Rc::clone(&slot));
        self.session.sync_generation();
        Ok(PreparedDynamic {
            session_id: self.session.run_id,
            layout_id,
            policy_id: self.policy_id,
            temporary: matches!(self.policy, Policy::Temporary),
            schema: Rc::new(schema),
            slot,
            layout_token: self.layout_token,
        })
    }
}

fn make_transport(cfg: &TransportConfig) -> Result<Transport, RegistrationError> {
    Ok(match cfg {
        TransportConfig::Memory { slots, slot_bytes } => {
            Transport::Memory(crate::recorder::MemoryPublication::new(*slots, *slot_bytes))
        }
        #[cfg(feature = "producer")]
        TransportConfig::AeronShared(shared) => Transport::AeronShared(Rc::clone(shared)),
        #[cfg(feature = "producer")]
        TransportConfig::Aeron { channel, stream_id } => Transport::Aeron(
            crate::recorder::AeronPublication::connect(channel, *stream_id)?,
        ),
    })
}

/// Layout fingerprint: schema content plus table provenance, so two
/// tables with identical column shapes never collide in the dictionary.
#[must_use]
pub fn layout_fingerprint(table: &str, schema_fp: u64) -> u64 {
    let mut h = schema_fp;
    for b in table.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Millisecond-resolution monotonic-ish UTC nanoseconds (lab clock; the
/// producer should inject its own high-resolution source when available).
#[must_use]
pub fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Fresh run id from the clock plus address entropy (lab-grade uniqueness).
#[must_use]
pub fn fresh_run_id() -> u64 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let t = now_ns();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    t ^ (u64::from(n) << 32) ^ (&t as *const u64 as u64)
}

/// Convenience alias for config reload wiring.
pub type SharedEnableSlot = SharedSlot;

#[allow(non_snake_case)]
#[cfg(test)]
mod EncodeCheck {
    #[test]
    fn encode_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<crate::persist::EncodeError>();
    }
}
