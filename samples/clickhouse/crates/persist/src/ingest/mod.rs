//! Ingest framework: replay sources, durable catalog, ClickHouse schema
//! evolution and batched binary inserts, checkpoints, and retention.
//!
//! The central ingester reads as much as the replay budget allows from the
//! Archive, projects rows, and pushes them in batches — one RowBinary
//! insert per flush, bounded by rows (8192), bytes (8 MiB), or time
//! (100 ms). A source checkpoint commits only after ClickHouse
//! acknowledges all destination rows of the contiguous complete prefix.

#[cfg(feature = "archive")]
pub mod archive;
pub mod catalog;
pub mod checkpoint;
pub mod clickhouse;
pub mod lifecycle;

use crate::protocol::{DataEnvelope, Declaration};

/// Replay buffering budget per source; reading stops when exhausted so a
/// slow ClickHouse cannot create an unbounded queue.
#[derive(Clone, Copy, Debug)]
pub struct ReplayBudget {
    /// Bytes allowed in flight for this source.
    pub bytes: usize,
}

impl Default for ReplayBudget {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
        }
    }
}

/// One validated item from a replay source, in stream order.
#[derive(Clone, Debug)]
pub enum SourceItem {
    /// Registration declaration (catalog replay).
    Declaration(Declaration),
    /// Data record envelope.
    Record(DataEnvelope),
}

/// Position of an item in the source stream (Aeron recording position).
pub type SourcePosition = i64;

/// A positioned batch from a source.
#[derive(Clone, Debug)]
pub struct SourceBatch {
    /// Items in order.
    pub items: Vec<SourceItem>,
    /// Position the batch started at (== the previous batch's `end_position`).
    pub start_position: SourcePosition,
    /// Position of the last item (checkpoint candidate).
    pub end_position: SourcePosition,
    /// Approximate byte weight of the batch.
    pub bytes: usize,
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCatalog { generation, needed } => {
                write!(f, "missing catalog: have {generation}, need {needed}")
            }
            Self::InvalidDeclaration(m) => write!(f, "invalid declaration: {m}"),
            Self::Malformed(m) => write!(f, "malformed frame: {m}"),
            Self::Disconnected(m) => write!(f, "disconnected: {m}"),
            Self::CheckpointOutsideHistory {
                checkpoint,
                archive_start,
            } => {
                write!(
                    f,
                    "checkpoint {checkpoint} outside archive history ({archive_start}..)"
                )
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// Replay source errors pause the affected source; independent sources
/// continue.
#[derive(Clone, Debug, PartialEq)]
pub enum SourceError {
    /// A data record requires a newer catalog generation than loaded.
    MissingCatalog { generation: u32, needed: u32 },
    /// A declaration failed validation.
    InvalidDeclaration(String),
    /// Envelope structurally invalid (malformed lengths, unknown kind).
    Malformed(&'static str),
    /// Transport-level failure (reconnect required).
    Disconnected(String),
    /// Checkpoint state lies outside the retained archive history.
    CheckpointOutsideHistory { checkpoint: i64, archive_start: i64 },
}

/// Replay source: yields validated items in order with positions.
pub trait IngestSource {
    /// Next positioned batch within the budget, or `None` when drained.
    fn next_batch(&mut self, budget: &ReplayBudget) -> Result<Option<SourceBatch>, SourceError>;
}

/// Simple pull source over an in-memory publication ring (tests/lab).
pub struct MemorySource {
    pub(crate) publication: crate::recorder::MemoryPublication,
    buf: Vec<u8>,
    position: SourcePosition,
}

impl MemorySource {
    /// Wrap a recorded ring.
    #[must_use]
    pub fn new(publication: crate::recorder::MemoryPublication) -> Self {
        Self {
            publication,
            buf: vec![0u8; crate::protocol::limits::MAX_RECORD_BYTES],
            position: 0,
        }
    }
}

impl IngestSource for MemorySource {
    fn next_batch(&mut self, _budget: &ReplayBudget) -> Result<Option<SourceBatch>, SourceError> {
        let mut items = Vec::new();
        let mut bytes = 0usize;
        let start = self.position;
        let mut end = self.position;
        while bytes < _budget.bytes / 2 {
            let n = match self.publication.pop(&mut self.buf) {
                Some(n) => n,
                None => break,
            };
            self.position += n as SourcePosition;
            end = self.position;
            // Session declarations and data travel as encoded recording
            // messages; classify by header template id.
            let item = classify_frame(&self.buf[..n])?;
            bytes += n;
            items.push(item);
        }
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(SourceBatch {
                items,
                start_position: start,
                end_position: end,
                bytes,
            }))
        }
    }
}

/// Classify one framing element by its recording schema template id.
pub fn classify_frame(buf: &[u8]) -> Result<SourceItem, SourceError> {
    use crate::recording::DeclarationKind;
    if buf.len() < 8 {
        return Err(SourceError::Malformed("short frame"));
    }
    // `MAX_EVENT_BYTES` was a declared bound with no enforcement anywhere, so
    // an oversized frame would be decoded rather than rejected.
    if buf.len() > crate::protocol::limits::MAX_EVENT_BYTES {
        return Err(SourceError::Malformed("frame exceeds MAX_EVENT_BYTES"));
    }
    let template_id = u16::from_le_bytes([buf[2], buf[3]]);
    if template_id == crate::protocol::wire::DATA_RECORD_TEMPLATE {
        let env = crate::protocol::decode_data_record(buf)
            .map_err(|_| SourceError::Malformed("data record decode"))?;
        Ok(SourceItem::Record(env))
    } else {
        let kind = match template_id {
            1 => DeclarationKind::SessionStart,
            2 => DeclarationKind::RegisterSymbol,
            3 => DeclarationKind::RegisterPolicy,
            4 => DeclarationKind::RegisterLayout,
            6 => DeclarationKind::SessionEnd,
            _ => return Err(SourceError::Malformed("unknown template id")),
        };
        let decl = crate::protocol::Declaration::decode(kind, buf)
            .map_err(|_| SourceError::InvalidDeclaration("decode failed".into()))?;
        Ok(SourceItem::Declaration(decl))
    }
}

/// Batched insert buffer holding row-major RowBinary bytes — exactly the
/// wire form ClickHouse expects, so a flush is one contiguous write. Rows
/// accumulate as the replay loop drains the Archive; flush on the first
/// of rows/bytes/time bounds.
pub struct BatchBuffer {
    body: Vec<u8>,
    rows: usize,
    max_rows: usize,
    max_bytes: usize,
    opened: Option<std::time::Instant>,
}

impl BatchBuffer {
    /// New buffer with the protocol default bounds.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limits(
            crate::protocol::limits::INGEST_BATCH_ROWS,
            crate::protocol::limits::INGEST_BATCH_BYTES,
        )
    }

    /// New buffer with explicit bounds.
    #[must_use]
    pub fn with_limits(max_rows: usize, max_bytes: usize) -> Self {
        Self {
            body: Vec::with_capacity(64 * 1024),
            rows: 0,
            max_rows,
            max_bytes,
            opened: None,
        }
    }

    /// Rows pending.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Alias for [`Self::rows`] (flush-call naming).
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.rows
    }

    /// Whether the batch should flush now.
    #[must_use]
    pub fn should_flush(&self) -> bool {
        if self.rows == 0 {
            return false;
        }
        if self.rows >= self.max_rows || self.body.len() >= self.max_bytes {
            return true;
        }
        match self.opened {
            Some(t) => t.elapsed().as_millis() as u64 >= crate::protocol::limits::INGEST_BATCH_MS,
            None => false,
        }
    }

    /// Append one row; `values` yields each column's encoded bytes in
    /// schema order (row-major, matching the RowBinary wire form).
    pub fn push_row<I: Iterator<Item = Vec<u8>>>(&mut self, values: I) {
        if self.opened.is_none() {
            self.opened = Some(std::time::Instant::now());
        }
        for v in values {
            self.body.extend_from_slice(&v);
        }
        self.rows += 1;
    }

    /// The contiguous row-major body for a single INSERT.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Reset after an acknowledged flush.
    pub fn clear(&mut self) {
        self.body.clear();
        self.rows = 0;
        self.opened = None;
    }
}

impl Default for BatchBuffer {
    fn default() -> Self {
        Self::new()
    }
}
