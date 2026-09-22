//! Recording wire protocol: declarations, the data envelope, and the
//! session catalog.
//!
//! The envelope and registration records use the checked-in SBE schema
//! (`schemas/recording.xml`, schema id 77). Session-local compact IDs are
//! assigned by the registration authority and are never reused within a
//! session; names appear only in registration records.

pub use crate::recording::{
    DataRecordDecoder, DataRecordEncoder, DeclarationKind, PayloadEncoding, RecordKind,
    RegisterLayoutDecoder, RegisterLayoutEncoder, RegisterPolicyDecoder, RegisterPolicyEncoder,
    RegisterSymbolDecoder, RegisterSymbolEncoder, SessionEndDecoder, SessionEndEncoder,
    SessionStartDecoder, SessionStartEncoder, TablePolicy,
};

use crate::persist::EncodeError;

fn sbe_err() -> EncodeError {
    EncodeError::SbeWire
}

/// Logical table lifetime policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Policy {
    /// Always-on market data; never disabled or expired by config.
    Permanent,
    /// Managed diagnostic table with retention controls.
    Temporary,
}

impl Policy {
    /// Wire enum value.
    #[must_use]
    pub const fn wire(self) -> TablePolicy {
        match self {
            Self::Permanent => TablePolicy::Permanent,
            Self::Temporary => TablePolicy::Temporary,
        }
    }

    /// From wire enum value.
    #[must_use]
    pub const fn from_wire(w: TablePolicy) -> Self {
        match w {
            TablePolicy::Permanent | TablePolicy::NullVal => Self::Permanent,
            TablePolicy::Temporary => Self::Temporary,
        }
    }
}

/// Payload encoding declared by a layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RowEncoding {
    /// Ordered typed row (fixed column order from the layout).
    OrderedRow,
    /// Dynamic row resolved by column handles at prepare time.
    DynamicRow,
    /// Original/generated SBE bytes with their original header.
    RawSbe,
    /// Opaque bytes (raw JSON capture, binary payloads).
    RawBytes,
}

impl RowEncoding {
    /// Wire enum value.
    #[must_use]
    pub const fn wire(self) -> PayloadEncoding {
        match self {
            Self::OrderedRow => PayloadEncoding::OrderedRow,
            Self::DynamicRow => PayloadEncoding::DynamicRow,
            Self::RawSbe => PayloadEncoding::RawSbe,
            Self::RawBytes => PayloadEncoding::RawBytes,
        }
    }

    /// From wire enum value; unknown values are source errors handled by callers.
    #[must_use]
    pub const fn from_wire(w: PayloadEncoding) -> Self {
        match w {
            PayloadEncoding::OrderedRow | PayloadEncoding::NullVal => Self::OrderedRow,
            PayloadEncoding::DynamicRow => Self::DynamicRow,
            PayloadEncoding::RawSbe => Self::RawSbe,
            PayloadEncoding::RawBytes => Self::RawBytes,
        }
    }
}

/// Bounded protocol defaults; validated once at registration.
pub mod limits {
    /// Maximum complete encoded record accepted by any publication.
    pub const MAX_RECORD_BYTES: usize = 1024 * 1024;
    /// Maximum columns per layout.
    pub const MAX_COLUMNS: usize = 256;
    /// Maximum logical tables per session dictionary.
    pub const MAX_TABLES: usize = 256;
    /// Maximum layouts per session dictionary.
    pub const MAX_LAYOUTS: usize = 4096;
    /// Maximum interned symbols per session dictionary.
    pub const MAX_SYMBOLS: usize = 65_536;
    /// Ingest batch row flush bound.
    pub const INGEST_BATCH_ROWS: usize = 8192;
    /// Ingest batch byte bound.
    pub const INGEST_BATCH_BYTES: usize = 8 * 1024 * 1024;
    /// Ingest batch time flush bound.
    pub const INGEST_BATCH_MS: u64 = 100;
    /// Per-event projected cap.
    pub const MAX_EVENT_BYTES: usize = 8 * 1024 * 1024;
    /// Maximum config file size.
    pub const MAX_CONFIG_BYTES: usize = 256 * 1024;
    /// Maximum config rules.
    pub const MAX_CONFIG_RULES: usize = 1024;
}

/// Wire protocol constants shared by all envelopes.
pub mod wire {
    /// Recording SBE schema id.
    pub const SCHEMA_ID: u16 = crate::recording::DataRecordEncoder::SCHEMA_ID;
    /// Recording SBE schema version.
    pub const SCHEMA_VERSION: u16 = crate::recording::DataRecordEncoder::SCHEMA_VERSION;
    /// DataRecord template id.
    pub const DATA_RECORD_TEMPLATE: u16 = crate::recording::DataRecordEncoder::TEMPLATE_ID;
}

/// `SessionStart` declaration payload.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SessionStartDeclaration {
    /// Unique run identity for this process start.
    pub run_id: u64,
    /// Process start time, UTC ns.
    pub started_at_ns: u64,
    /// Static process name.
    pub process: String,
    /// Instance name within the process role.
    pub instance: String,
    /// Build revision string.
    pub build: String,
}

/// `RegisterSymbol` declaration payload.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SymbolDeclaration {
    /// Session-local symbol id.
    pub symbol_id: u32,
    /// Interned value.
    pub value: String,
}

/// `RegisterPolicy` declaration payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PolicyDeclaration {
    /// Session-local policy id.
    pub policy_id: u16,
    /// Table lifetime class.
    pub policy: Policy,
    /// Row retention for temporary tables; 0 = permanent/default.
    pub row_ttl_ns: u64,
    /// Idle-table cleanup for temporary tables; 0 = permanent/default.
    pub idle_ttl_ns: u64,
}

/// One column in a `RegisterLayout` declaration.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ColumnDeclaration {
    /// Column name (registration-only; never re-sent per record).
    pub name: String,
    /// Type code byte, see [`crate::schema::TypeCode`].
    pub type_code: u8,
    /// Flags byte, see [`crate::schema::flags`].
    pub flags: u8,
    /// Decimal precision.
    pub precision: u8,
    /// Decimal scale.
    pub scale: i8,
}

/// `RegisterLayout` declaration payload.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct LayoutDeclaration {
    /// Session-local layout id.
    pub layout_id: u16,
    /// Bound retention policy id.
    pub policy_id: u16,
    /// Payload encoding of data records using this layout.
    pub payload_encoding: RowEncoding,
    /// Fingerprint over the full descriptor and provenance.
    pub schema_fingerprint: u64,
    /// Deterministic projection revision for raw-SBE layouts.
    pub projection_revision: u32,
    /// SBE identity for raw-SBE layouts.
    pub sbe_schema_id: u16,
    /// SBE template id for raw-SBE layouts.
    pub sbe_template_id: u16,
    /// SBE acting version for raw-SBE layouts.
    pub sbe_version: u16,
    /// Catalog generation this declaration was committed under.
    pub catalog_generation: u32,
    /// Logical table name.
    pub table_name: String,
    /// Ordered columns (flat storage shape).
    pub columns: Vec<ColumnDeclaration>,
}

/// Any registration declaration.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Declaration {
    /// Session start.
    SessionStart(SessionStartDeclaration),
    /// Interned symbol.
    Symbol(SymbolDeclaration),
    /// Retention policy.
    Policy(PolicyDeclaration),
    /// Table layout.
    Layout(LayoutDeclaration),
    /// Session end marker.
    SessionEnd {
        /// End time, UTC ns.
        ended_at_ns: u64,
        /// Last sequence used by any writer.
        final_sequence: u64,
    },
}

impl Declaration {
    /// Declaration kind on the metadata stream.
    #[must_use]
    pub const fn kind(&self) -> DeclarationKind {
        match self {
            Self::SessionStart(_) => DeclarationKind::SessionStart,
            Self::Symbol(_) => DeclarationKind::RegisterSymbol,
            Self::Policy(_) => DeclarationKind::RegisterPolicy,
            Self::Layout(_) => DeclarationKind::RegisterLayout,
            Self::SessionEnd { .. } => DeclarationKind::SessionEnd,
        }
    }

    /// Maximum encoded size of this declaration (used to bound buffers).
    #[must_use]
    pub fn max_encoded_len(&self) -> usize {
        const SLACK: usize = 64;
        match self {
            Self::SessionStart(d) => {
                crate::recording::SessionStartEncoder::ENCODED_LENGTH
                    + d.process.len()
                    + d.instance.len()
                    + d.build.len()
                    + SLACK
            }
            Self::Symbol(d) => {
                crate::recording::RegisterSymbolEncoder::ENCODED_LENGTH + d.value.len() + SLACK
            }
            Self::Policy(_) => crate::recording::RegisterPolicyEncoder::ENCODED_LENGTH + SLACK,
            Self::Layout(d) => {
                let mut names = d.table_name.len();
                for c in &d.columns {
                    names += c.name.len();
                }
                crate::recording::RegisterLayoutEncoder::compute_length_with_header(d.columns.len())
                    + names
                    + SLACK
            }
            Self::SessionEnd { .. } => crate::recording::SessionEndEncoder::ENCODED_LENGTH + SLACK,
        }
    }

    /// Encode this declaration into `buf` using the recording SBE codecs.
    ///
    /// Returns the total encoded length including the message header.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        use crate::recording as rec;
        match self {
            Self::SessionStart(d) => {
                let (p, i, b) = (
                    d.process.as_bytes(),
                    d.instance.as_bytes(),
                    d.build.as_bytes(),
                );
                let total = rec::SessionStartEncoder::ENCODED_LENGTH + p.len() + i.len() + b.len();
                if buf.len() < total {
                    return Err(EncodeError::BufferTooSmall {
                        needed: total,
                        actual: buf.len(),
                    });
                }
                let enc = rec::SessionStartEncoder::wrap_and_apply_header(buf, 0).fixed(
                    &rec::SessionStartFixedFields {
                        run_id: d.run_id,
                        started_at_ns: d.started_at_ns,
                        process: rec::VarAsciiEncoding((p.len() as u32).to_le_bytes()),
                        instance: rec::VarAsciiEncoding((i.len() as u32).to_le_bytes()),
                        build: rec::VarAsciiEncoding((b.len() as u32).to_le_bytes()),
                    },
                );
                let tail = enc.into_remaining_mut();
                let mut cur = 0;
                tail[cur..cur + p.len()].copy_from_slice(p);
                cur += p.len();
                tail[cur..cur + i.len()].copy_from_slice(i);
                cur += i.len();
                tail[cur..cur + b.len()].copy_from_slice(b);
                Ok(total)
            }
            Self::Symbol(d) => {
                let v = d.value.as_bytes();
                let total = rec::RegisterSymbolEncoder::ENCODED_LENGTH + v.len();
                if buf.len() < total {
                    return Err(EncodeError::BufferTooSmall {
                        needed: total,
                        actual: buf.len(),
                    });
                }
                let enc = rec::RegisterSymbolEncoder::wrap_and_apply_header(buf, 0).fixed(
                    &rec::RegisterSymbolFixedFields {
                        symbol_id: d.symbol_id,
                        value: rec::VarAsciiEncoding((v.len() as u32).to_le_bytes()),
                    },
                );
                let tail = enc.into_remaining_mut();
                tail[..v.len()].copy_from_slice(v);
                Ok(total)
            }
            Self::Policy(d) => {
                if buf.len() < rec::RegisterPolicyEncoder::ENCODED_LENGTH {
                    return Err(EncodeError::BufferTooSmall {
                        needed: rec::RegisterPolicyEncoder::ENCODED_LENGTH,
                        actual: buf.len(),
                    });
                }
                let complete = rec::RegisterPolicyEncoder::wrap_and_apply_header(buf, 0).fixed(
                    &rec::RegisterPolicyFixedFields {
                        policy_id: d.policy_id,
                        temporary: d.policy.wire(),
                        row_ttl_ns: d.row_ttl_ns,
                        idle_ttl_ns: d.idle_ttl_ns,
                    },
                );
                Ok(complete.encoded_length_with_header())
            }
            Self::Layout(d) => {
                let tn = d.table_name.as_bytes();
                let mut names_len = tn.len();
                for c in &d.columns {
                    names_len += c.name.len();
                }
                let count = d.columns.len();
                let body =
                    rec::RegisterLayoutEncoder::compute_length_with_header(count).saturating_sub(8);
                let total = body + 8 + names_len;
                if buf.len() < total {
                    return Err(EncodeError::BufferTooSmall {
                        needed: total,
                        actual: buf.len(),
                    });
                }
                if count > u16::MAX as usize {
                    return Err(EncodeError::Overflow);
                }
                let complete = rec::RegisterLayoutEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&rec::RegisterLayoutFixedFields {
                        layout_id: d.layout_id,
                        policy_id: d.policy_id,
                        payload_encoding: d.payload_encoding.wire(),
                        schema_fingerprint: d.schema_fingerprint,
                        projection_revision: d.projection_revision,
                        sbe_schema_id: d.sbe_schema_id,
                        sbe_template_id: d.sbe_template_id,
                        sbe_version: d.sbe_version,
                        catalog_generation: d.catalog_generation,
                        table_name: rec::VarAsciiEncoding((tn.len() as u32).to_le_bytes()),
                    })
                    .columns(count as u16, |g| {
                        for c in &d.columns {
                            let name = c.name.as_bytes();
                            g.add(|e| {
                                e.name(rec::VarAsciiEncoding((name.len() as u32).to_le_bytes()))
                                    .type_code(c.type_code)
                                    .flags(c.flags)
                                    .precision(c.precision)
                                    .scale(c.scale);
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })
                    .map_err(|_| sbe_err())?;
                let after_group = complete.encoded_length_with_header();
                let tail = complete.into_remaining_mut();
                let mut cur = 0;
                for c in &d.columns {
                    let name = c.name.as_bytes();
                    tail[cur..cur + name.len()].copy_from_slice(name);
                    cur += name.len();
                }
                tail[cur..cur + tn.len()].copy_from_slice(tn);
                Ok(after_group + names_len)
            }
            Self::SessionEnd {
                ended_at_ns,
                final_sequence,
            } => {
                if buf.len() < rec::SessionEndEncoder::ENCODED_LENGTH {
                    return Err(EncodeError::BufferTooSmall {
                        needed: rec::SessionEndEncoder::ENCODED_LENGTH,
                        actual: buf.len(),
                    });
                }
                let complete = rec::SessionEndEncoder::wrap_and_apply_header(buf, 0).fixed(
                    &rec::SessionEndFixedFields {
                        ended_at_ns: *ended_at_ns,
                        final_sequence: *final_sequence,
                    },
                );
                Ok(complete.encoded_length_with_header())
            }
        }
    }

    /// Decode a declaration of `kind` from `buf` (message start at 0).
    pub fn decode(kind: DeclarationKind, buf: &[u8]) -> Result<Self, EncodeError> {
        use crate::recording as rec;
        fn tail_str<'a>(
            buf: &'a [u8],
            block_end: usize,
            offsets: &mut usize,
            len: u32,
        ) -> Result<&'a [u8], EncodeError> {
            let start = block_end + *offsets;
            let end = start
                .checked_add(len as usize)
                .ok_or(EncodeError::Overflow)?;
            if end > buf.len() {
                return Err(EncodeError::BufferTooSmall {
                    needed: end,
                    actual: buf.len(),
                });
            }
            *offsets += len as usize;
            Ok(&buf[start..end])
        }
        fn to_string(v: &[u8]) -> Result<String, EncodeError> {
            let s = std::str::from_utf8(v).map_err(|_| EncodeError::InvalidUtf8)?;
            Ok(s.to_string())
        }
        fn verify_decl(kind: DeclarationKind, buf: &[u8]) -> Result<(), EncodeError> {
            let r = match kind {
                DeclarationKind::SessionStart => rec::SessionStartDecoder::verify(buf).map(|_| ()),
                DeclarationKind::RegisterSymbol => {
                    rec::RegisterSymbolDecoder::verify(buf).map(|_| ())
                }
                DeclarationKind::RegisterPolicy => {
                    rec::RegisterPolicyDecoder::verify(buf).map(|_| ())
                }
                DeclarationKind::RegisterLayout => {
                    rec::RegisterLayoutDecoder::verify(buf).map(|_| ())
                }
                DeclarationKind::SessionEnd => rec::SessionEndDecoder::verify(buf).map(|_| ()),
                DeclarationKind::NullVal => return Err(EncodeError::Custom(902)),
            };
            r.map_err(|_| EncodeError::SbeWire)
        }
        verify_decl(kind, buf)?;
        match kind {
            DeclarationKind::SessionStart => {
                let d = rec::SessionStartDecoder::decode(buf, 0)?;
                let block_end = 8 + rec::SessionStartDecoder::BLOCK_LENGTH;
                let mut off = 0usize;
                let process = to_string(tail_str(buf, block_end, &mut off, d.process().length())?)?;
                let instance =
                    to_string(tail_str(buf, block_end, &mut off, d.instance().length())?)?;
                let build = to_string(tail_str(buf, block_end, &mut off, d.build().length())?)?;
                Ok(Self::SessionStart(SessionStartDeclaration {
                    run_id: d.run_id(),
                    started_at_ns: d.started_at_ns(),
                    process,
                    instance,
                    build,
                }))
            }
            DeclarationKind::RegisterSymbol => {
                let d = rec::RegisterSymbolDecoder::decode(buf, 0)?;
                let block_end = 8 + rec::RegisterSymbolDecoder::BLOCK_LENGTH;
                let mut off = 0usize;
                let value = to_string(tail_str(buf, block_end, &mut off, d.value().length())?)?;
                Ok(Self::Symbol(SymbolDeclaration {
                    symbol_id: d.symbol_id(),
                    value,
                }))
            }
            DeclarationKind::RegisterPolicy => {
                let d = rec::RegisterPolicyDecoder::decode(buf, 0)?;
                Ok(Self::Policy(PolicyDeclaration {
                    policy_id: d.policy_id(),
                    policy: Policy::from_wire(d.temporary()),
                    row_ttl_ns: d.row_ttl_ns(),
                    idle_ttl_ns: d.idle_ttl_ns(),
                }))
            }
            DeclarationKind::RegisterLayout => {
                let d = rec::RegisterLayoutDecoder::decode(buf, 0)?;
                // Wire order: block (incl. table-name length prefix), group
                // dimension, entries, then var-data tail: column names in
                // group order, then the table-name data.
                let block_end = 8 + rec::RegisterLayoutDecoder::BLOCK_LENGTH;
                let count = d.columns_count()?;
                let tail_start = block_end + 4 + count * 8;
                let mut off = 0usize;
                let mut columns = Vec::new();
                let mut col_names = Vec::new();
                for e in d.columns()? {
                    let len = e.name_value().length();
                    let name = tail_str(buf, tail_start, &mut off, len)?.to_vec();
                    col_names.push(name);
                    columns.push(ColumnDeclaration {
                        name: String::new(),
                        type_code: e.type_code(),
                        flags: e.flags(),
                        precision: e.precision(),
                        scale: e.scale(),
                    });
                }
                let table_name = to_string(tail_str(
                    buf,
                    tail_start,
                    &mut off,
                    d.table_name().length(),
                )?)?;
                for (col, name) in columns.iter_mut().zip(col_names) {
                    col.name = to_string(&name)?;
                }
                Ok(Self::Layout(LayoutDeclaration {
                    layout_id: d.layout_id(),
                    policy_id: d.policy_id(),
                    payload_encoding: RowEncoding::from_wire(d.payload_encoding()),
                    schema_fingerprint: d.schema_fingerprint(),
                    projection_revision: d.projection_revision(),
                    sbe_schema_id: d.sbe_schema_id(),
                    sbe_template_id: d.sbe_template_id(),
                    sbe_version: d.sbe_version(),
                    catalog_generation: d.catalog_generation(),
                    table_name,
                    columns,
                }))
            }
            DeclarationKind::SessionEnd => {
                let d = rec::SessionEndDecoder::decode(buf, 0)?;
                Ok(Self::SessionEnd {
                    ended_at_ns: d.ended_at_ns(),
                    final_sequence: d.final_sequence(),
                })
            }
            DeclarationKind::NullVal => Err(EncodeError::Custom(902)),
        }
    }
}

/// Encode a `DataRecord` envelope into `buf`; returns total length.
#[allow(clippy::too_many_arguments)]
pub fn encode_data_record(
    buf: &mut [u8],
    kind: RecordKind,
    writer_id: u16,
    layout_id: u16,
    policy_id: u16,
    sequence: u64,
    captured_at_ns: u64,
    catalog_generation: u32,
    payload: &[u8],
) -> Result<usize, EncodeError> {
    let needed = crate::recording::DataRecordEncoder::compute_length_with_header(payload.len());
    if buf.len() < needed {
        return Err(EncodeError::BufferTooSmall {
            needed,
            actual: buf.len(),
        });
    }
    let complete = crate::recording::DataRecordEncoder::wrap_and_apply_header(buf, 0)
        .fixed(&crate::recording::DataRecordFixedFields {
            record_kind: kind,
            writer_id,
            layout_id,
            policy_id,
            sequence,
            captured_at_ns,
            catalog_generation,
        })
        .payload(payload)?;
    Ok(complete.encoded_length_with_header())
}

/// A decoded data-record envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataEnvelope {
    /// Envelope metadata (writer, sequence, capture time, generation).
    pub metadata: crate::persist::RecordMetadata,
    /// Payload kind.
    pub kind: RecordKind,
    /// Layout the payload belongs to.
    pub layout_id: u16,
    /// Policy bound to the layout.
    pub policy_id: u16,
    /// Payload bytes.
    pub payload: Vec<u8>,
}

/// Decode a `DataRecord` envelope from `buf`.
pub fn decode_data_record(buf: &[u8]) -> Result<DataEnvelope, EncodeError> {
    // Untrusted archive input: prove structure before constructing a flyweight.
    crate::recording::DataRecordDecoder::verify(buf)?;
    let d = crate::recording::DataRecordDecoder::decode(buf, 0)?;
    let metadata = crate::persist::RecordMetadata {
        run_id: 0, // run id travels in the session binding, not per record
        writer_id: d.writer_id(),
        sequence: d.sequence(),
        captured_at_ns: d.captured_at_ns(),
        catalog_generation: d.catalog_generation(),
    };
    let payload = d.payload_slice()?.to_vec();
    Ok(DataEnvelope {
        metadata,
        kind: d.record_kind(),
        layout_id: d.layout_id(),
        policy_id: d.policy_id(),
        payload,
    })
}

/// Registration capacity/validation failure codes used by the catalog.
mod catalog_errors {
    /// Dictionary capacity exhausted.
    pub const DICTIONARY_FULL: u16 = 910;
    /// Too many columns.
    pub const TOO_MANY_COLUMNS: u16 = 911;
    /// Layout id space exhausted.
    pub const LAYOUT_ID_EXHAUSTED: u16 = 912;
    /// Policy capacity exhausted.
    pub const POLICY_FULL: u16 = 913;
    /// Policy id space exhausted.
    pub const POLICY_ID_EXHAUSTED: u16 = 914;
    /// Symbol capacity exhausted.
    pub const SYMBOL_FULL: u16 = 915;
    /// Symbol id space exhausted.
    pub const SYMBOL_ID_EXHAUSTED: u16 = 916;
    /// Conflicting redefinition of an existing identity.
    pub const CONFLICTING_DEFINITION: u16 = 917;
}

/// Control-plane session catalog: assigns compact IDs and tracks the
/// generation counter that data envelopes carry.
#[derive(Debug)]
pub struct SessionCatalog {
    next_layout_id: u16,
    next_policy_id: u16,
    next_symbol_id: u32,
    catalog_generation: u32,
    layouts: Vec<LayoutDeclaration>,
    policies: Vec<PolicyDeclaration>,
    symbols: Vec<SymbolDeclaration>,
}

impl Default for SessionCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionCatalog {
    /// New empty catalog starting IDs at 1.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_layout_id: 1,
            next_policy_id: 1,
            next_symbol_id: 1,
            catalog_generation: 1,
            layouts: Vec::new(),
            policies: Vec::new(),
            symbols: Vec::new(),
        }
    }

    /// Current catalog generation carried by data envelopes.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.catalog_generation
    }

    /// Resolve a layout descriptor by id.
    #[must_use]
    pub fn layout(&self, id: u16) -> Option<&LayoutDeclaration> {
        self.layouts.iter().find(|l| l.layout_id == id)
    }

    /// Resolve a policy by id.
    #[must_use]
    pub fn policy(&self, id: u16) -> Option<&PolicyDeclaration> {
        self.policies.iter().find(|p| p.policy_id == id)
    }

    /// Resolve a symbol by id.
    #[must_use]
    pub fn symbol(&self, id: u32) -> Option<&SymbolDeclaration> {
        self.symbols.iter().find(|s| s.symbol_id == id)
    }

    /// All layouts (registration order).
    #[must_use]
    pub fn layouts(&self) -> &[LayoutDeclaration] {
        &self.layouts
    }

    /// All policies (registration order).
    #[must_use]
    pub fn policies(&self) -> &[PolicyDeclaration] {
        &self.policies
    }

    /// All symbols (registration order).
    #[must_use]
    pub fn symbols(&self) -> &[SymbolDeclaration] {
        &self.symbols
    }

    /// Register a new layout, allocating the next session-local id.
    ///
    /// Duplicate identical definitions (same fingerprint, name, and columns)
    /// return the existing id so retries cannot double-allocate; conflicting
    /// definitions are rejected.
    pub fn register_layout(&mut self, mut decl: LayoutDeclaration) -> Result<u16, EncodeError> {
        if self.layouts.len() >= limits::MAX_LAYOUTS {
            return Err(EncodeError::Custom(catalog_errors::DICTIONARY_FULL));
        }
        if decl.columns.len() > limits::MAX_COLUMNS {
            return Err(EncodeError::Custom(catalog_errors::TOO_MANY_COLUMNS));
        }
        if let Some(existing) = self
            .layouts
            .iter()
            .find(|l| l.schema_fingerprint == decl.schema_fingerprint)
        {
            if existing.table_name == decl.table_name && existing.columns == decl.columns {
                return Ok(existing.layout_id);
            }
            return Err(EncodeError::Custom(catalog_errors::CONFLICTING_DEFINITION));
        }
        decl.layout_id = self.next_layout_id;
        decl.catalog_generation = self.catalog_generation;
        let id = decl.layout_id;
        self.layouts.push(decl);
        self.next_layout_id = self
            .next_layout_id
            .checked_add(1)
            .ok_or(EncodeError::Custom(catalog_errors::LAYOUT_ID_EXHAUSTED))?;
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
        Ok(id)
    }

    /// Register a retention policy, allocating the next id.
    ///
    /// Policies are versioned; each distinct retention set gets a new id so
    /// overlapping config revisions resolve independently.
    pub fn register_policy(&mut self, mut decl: PolicyDeclaration) -> Result<u16, EncodeError> {
        if self.policies.len() >= limits::MAX_TABLES {
            return Err(EncodeError::Custom(catalog_errors::POLICY_FULL));
        }
        decl.policy_id = self.next_policy_id;
        let id = decl.policy_id;
        self.policies.push(decl);
        self.next_policy_id = self
            .next_policy_id
            .checked_add(1)
            .ok_or(EncodeError::Custom(catalog_errors::POLICY_ID_EXHAUSTED))?;
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
        Ok(id)
    }

    /// Intern a symbol, allocating the next id (idempotent by value).
    pub fn intern_symbol(&mut self, value: &str) -> Result<u32, EncodeError> {
        if let Some(existing) = self.symbols.iter().find(|s| s.value == value) {
            return Ok(existing.symbol_id);
        }
        if self.symbols.len() >= limits::MAX_SYMBOLS {
            return Err(EncodeError::Custom(catalog_errors::SYMBOL_FULL));
        }
        let id = self.next_symbol_id;
        self.symbols.push(SymbolDeclaration {
            symbol_id: id,
            value: value.to_string(),
        });
        self.next_symbol_id = self
            .next_symbol_id
            .checked_add(1)
            .ok_or(EncodeError::Custom(catalog_errors::SYMBOL_ID_EXHAUSTED))?;
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
        Ok(id)
    }
}
