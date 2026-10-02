//! Replay the Aeron Archive into ClickHouse.
//!
//! `persist-client` publishes. The archive records. [`Ingester`] replays from
//! its checkpoint, inserts, saves the checkpoint, and purges behind it. If
//! ClickHouse is slow, the archive holds the data.
//!
//! An SBE message is a table (`table.rs`). Any other name in `tables.yaml` is
//! a `tracing` event (`events.rs`). [`Writer`] inserts one RowBinary batch per
//! table per tick.
//!
//! A frame is routed by the schema id and template id in its header. That is
//! the same pair `schema::AnySchemaMessage` matches when an application has
//! the generated market and trading codecs. This ingester does not call that
//! enum. It reads every `.xml` under `PERSIST_SCHEMAS` and builds the table
//! from the schema text, so a schema that was not compiled into the `schema`
//! crate still persists.
//!
//! `tables.yaml` is re-read while running:
//!
//! ```yaml
//! tables:
//!   trade:         { kind: static }                  # created if missing, never altered
//!   book_snapshot: { kind: dynamic, enabled: false } # follows the schema
//! ```
//!
//! `static` is created once and never altered. A mismatched column is skipped
//! and the log prints the `ALTER`. `dynamic` adds a column when a new field
//! arrives. `enabled` is the application's switch. Listed tables are created
//! even when off, so a query against an empty one still works.
//!
//! A failed insert compares the table again, then retries the rows.

mod aeron_stats;
mod clickhouse;
mod events;
mod ingest;
mod metrics;
mod table;
mod traces;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use persist_client::metrics::{
    HISTOGRAM_TEMPLATE_ID, METRIC_DEF_TEMPLATE_ID, METRICS_TEMPLATE_ID, MetricDef,
};
use persist_client::source::{SOURCE_TEMPLATE_ID, Source as Origin};
use persist_client::trace::{TRACE_DEF_TEMPLATE_ID, TRACE_TEMPLATE_ID, TraceDef};
use persist_client::{TableConfig, event, parse_config};

use events::EventTable;

pub use clickhouse::ClickHouse;
use clickhouse::piece_token;
pub use ingest::Ingester;
pub use table::{Column, Shape, Table, tables_from_schema};

/// Everything that can go wrong.
#[derive(Debug)]
pub enum Error {
    /// The SBE schema cannot be turned into tables.
    Schema(String),
    /// A ClickHouse request failed.
    ClickHouse(String),
    /// `tables.yaml` is missing or invalid.
    Config(String),
    /// The media driver or the archive is unreachable or refused a request.
    Aeron(String),
    /// The checkpoint file cannot be read or written.
    Checkpoint(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(m) => write!(f, "schema: {m}"),
            Self::ClickHouse(m) => write!(f, "clickhouse: {m}"),
            Self::Config(m) => write!(f, "tables.yaml: {m}"),
            Self::Aeron(m) => write!(f, "aeron: {m}"),
            Self::Checkpoint(m) => write!(f, "checkpoint: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<persist_client::Error> for Error {
    fn from(e: persist_client::Error) -> Self {
        match e {
            persist_client::Error::Schema(m) => Self::Schema(m),
            persist_client::Error::Config(m) => Self::Config(m),
            persist_client::Error::Aeron(m) | persist_client::Error::Thread(m) => Self::Aeron(m),
            persist_client::Error::Encode(err) => Self::Schema(err.to_string()),
        }
    }
}

/// What to read, where to write, and how much to hold.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Target server and database.
    pub clickhouse: ClickHouse,
    /// `tables.yaml`, re-read while running.
    pub config_path: PathBuf,
    /// The media driver's directory; `None` uses `AERON_DIR` or Aeron's default.
    pub aeron_dir: Option<String>,
    /// The recorded channel; defaults to `persist_client::CHANNEL`.
    pub channel: String,
    /// The recorded stream; replays use the next stream id.
    pub stream_id: i32,
    /// Where the position of the last inserted record is kept.
    pub checkpoint_path: PathBuf,
    /// Replaying pauses while this much is waiting for ClickHouse; the rest
    /// stays in the archive.
    pub max_queued_bytes: usize,
    /// How often a table with unfixed problems is compared again, so running
    /// the logged `ALTER` takes effect without a restart.
    pub recheck: Duration,
    /// How often the media driver's counters, errors and losses are sampled
    /// into `aeron_counters`, `aeron_errors` and `aeron_loss`; zero never.
    pub aeron_stats_interval: Duration,
    /// The feed registry: every archived feed published on this node is
    /// recorded through a spy. `None` records the persist stream only.
    pub streams: Option<persist_client::streams::Streams>,
    /// This node's IP, which feeds published here bind.
    pub host_ip: String,
}

impl Settings {
    /// Defaults: the client's channel and stream, 64 MiB queued, 30 s recheck.
    #[must_use]
    pub fn new(
        clickhouse: ClickHouse,
        config_path: impl Into<PathBuf>,
        checkpoint_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            clickhouse,
            config_path: config_path.into(),
            aeron_dir: None,
            channel: persist_client::CHANNEL.to_string(),
            stream_id: persist_client::STREAM_ID,
            checkpoint_path: checkpoint_path.into(),
            max_queued_bytes: 64 << 20,
            recheck: Duration::from_secs(30),
            aeron_stats_interval: Duration::from_secs(5),
            streams: None,
            host_ip: "127.0.0.1".into(),
        }
    }

    /// [`Settings::new`] from `CLICKHOUSE_URL` (`http://localhost:8123`),
    /// `CLICKHOUSE_USER` (`lab`), `CLICKHOUSE_PASSWORD` (`lab`),
    /// `CLICKHOUSE_DATABASE` (`md`; a table's `database:` in `tables.yaml`
    /// overrides it), `PERSIST_CONFIG`
    /// (`config/tables.yaml`), `PERSIST_CHECKPOINT` (`persist.checkpoint`),
    /// `PERSIST_STREAMS` (`config/streams.yaml`, if it exists) and `HOST_IP`
    /// (`127.0.0.1`).
    pub fn from_env() -> Result<Self, Error> {
        let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        let streams_path = var("PERSIST_STREAMS", "config/streams.yaml");
        let streams = if Path::new(&streams_path).exists() {
            Some(persist_client::streams::Streams::load(&streams_path)?)
        } else {
            None
        };
        Ok(Self {
            streams,
            host_ip: var("HOST_IP", "127.0.0.1"),
            ..Self::from_env_without_feeds()
        })
    }

    fn from_env_without_feeds() -> Self {
        let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        Self::new(
            ClickHouse::new(
                &var("CLICKHOUSE_URL", "http://localhost:8123"),
                &var("CLICKHOUSE_USER", "lab"),
                &var("CLICKHOUSE_PASSWORD", "lab"),
                &var("CLICKHOUSE_DATABASE", "md"),
            ),
            var("PERSIST_CONFIG", "config/tables.yaml"),
            var("PERSIST_CHECKPOINT", "persist.checkpoint"),
        )
    }
}

/// What one tick did. Everything here is also logged.
#[derive(Debug, Default)]
pub struct Report {
    /// Rows inserted per table.
    pub inserted: BTreeMap<String, usize>,
    /// DDL that was run.
    pub applied: Vec<String>,
    /// Schema/table mismatches that were not fixed (static tables, type changes).
    pub problems: Vec<String>,
    /// Failures (ClickHouse unreachable, bad config, undecodable message).
    pub errors: Vec<String>,
    /// Archive data deleted because every record in it is in ClickHouse.
    pub purged: Vec<String>,
}

/// Tables persistence owns: created by the ingester, not by `tables.yaml`,
/// and never the name of an SBE message or an event table.
pub const RESERVED_TABLES: [&str; 6] = [
    metrics::METRICS,
    metrics::HISTOGRAMS,
    persist_client::OTEL_TRACES,
    "aeron_counters",
    "aeron_errors",
    "aeron_loss",
];

/// Where a table's rows come from.
enum Source {
    Sbe(Table),
    Events(EventTable),
    /// `metrics`: counters and gauges.
    Metrics,
    /// `metrics_histogram`.
    Histograms,
    /// `otel_traces`.
    Traces,
}

/// What rows are decoded with: the dictionaries the messages refer to.
struct Dictionaries<'a> {
    shapes: &'a HashMap<u32, event::Shape>,
    defs: &'a HashMap<u64, MetricDef>,
    traces: &'a HashMap<u64, TraceDef>,
}

impl Source {
    fn name(&self) -> &str {
        match self {
            Self::Sbe(t) => &t.name,
            Self::Events(t) => &t.name,
            Self::Metrics => metrics::METRICS,
            Self::Histograms => metrics::HISTOGRAMS,
            Self::Traces => persist_client::OTEL_TRACES,
        }
    }

    /// `None` for an event table that has not had a row yet.
    fn shape(&self) -> Option<Shape> {
        match self {
            Self::Sbe(t) => Some(t.shape()),
            Self::Events(t) => t.shape(),
            Self::Metrics => Some(metrics::metrics_shape()),
            Self::Histograms => Some(metrics::histograms_shape()),
            Self::Traces => Some(traces::traces_shape()),
        }
    }

    /// Append the rows `message` makes, each ending with `origin` (its
    /// `host`, `pod` and `app` columns, already RowBinary; `who` as text).
    /// Returns how many rows, and how many event values did not fit their
    /// column's type.
    fn write_rows(
        &self,
        message: &[u8],
        include: &[bool],
        (origin, who): (&[u8], [&str; 3]),
        out: &mut Vec<u8>,
        dict: &Dictionaries<'_>,
    ) -> Result<(usize, usize), table::DecodeError> {
        let one = |misfits: usize, out: &mut Vec<u8>| {
            out.extend_from_slice(origin);
            (1, misfits)
        };
        match self {
            Self::Sbe(t) => t.write_row(message, include, out).map(|()| one(0, out)),
            Self::Events(t) => t
                .write_row(message, include, out, dict.shapes)
                .map(|n| one(n, out)),
            Self::Metrics => {
                metrics::write_metrics(message, include, origin, dict.defs, out).map(|n| (n, 0))
            }
            Self::Histograms => Err(table::DecodeError(
                "histogram rows are written from the 5 s fold, not from the queued message",
            )),
            Self::Traces => {
                traces::write_trace(message, include, origin, who, dict.traces, out).map(|n| (n, 0))
            }
        }
    }
}

/// `tables.yaml`'s entry for a table persistence owns: static, always on.
fn fixed_config() -> TableConfig {
    TableConfig {
        kind: persist_client::TableKind::Static,
        enabled: persist_client::Switch::On,
        apps: BTreeMap::new(),
        traces: BTreeMap::new(),
        database: None,
    }
}

/// The client for a table kept in `database` (its `tables.yaml` entry's
/// `database:`), or in the default database when that names none. Every
/// create, compare and insert of a table goes through here.
fn client_in<'a>(ch: &'a ClickHouse, database: Option<&str>) -> Cow<'a, ClickHouse> {
    match database {
        Some(db) if db != ch.database => Cow::Owned(ch.in_database(db)),
        _ => Cow::Borrowed(ch),
    }
}

/// `CREATE DATABASE` for `ch`'s database once, remembered in `created`.
fn ensure_database(created: &mut BTreeSet<String>, ch: &ClickHouse) -> Result<(), Error> {
    if !created.contains(&ch.database) {
        ch.create_database()?;
        created.insert(ch.database.clone());
    }
    Ok(())
}

/// The columns every table ends with: who recorded the row. The ingester
/// fills them from the `Source` message named by each frame's reserved value.
const ORIGIN_COLUMNS: [&str; 3] = ["host", "pod", "app"];

pub(crate) fn origin_columns() -> impl Iterator<Item = Column> {
    ORIGIN_COLUMNS.into_iter().map(|name| Column {
        name: name.into(),
        ch_type: "LowCardinality(String)".into(),
    })
}

struct TableState {
    source: Source,
    config: Option<TableConfig>,
    /// `Some` once the table exists and the writable columns are known.
    include: Option<Vec<bool>>,
    columns: Vec<String>,
    problems: Vec<String>,
    retry_at: Instant,
    /// SBE messages not yet inserted, each a `u32` LE length, the `u64` LE
    /// source id, then the message. Kept as SBE so they can be decoded again
    /// if the table changes.
    queued: Vec<u8>,
    queued_count: usize,
}

impl TableState {
    fn new(source: Source) -> Self {
        Self {
            source,
            config: None,
            include: None,
            columns: Vec::new(),
            problems: Vec::new(),
            retry_at: Instant::now(),
            queued: Vec::new(),
            queued_count: 0,
        }
    }

    /// `tables.yaml`'s `database:` for this table, if it names one.
    fn database(&self) -> Option<&str> {
        self.config.as_ref().and_then(|c| c.database.as_deref())
    }
}

/// Every message of every schema. A message is identified by its schema id
/// and template id, and a table by its name, so two schemas may not share
/// either: keep one version of each schema, the newest (it decodes records
/// made with the older ones).
fn load_schemas(schemas: &[&str]) -> Result<Vec<Table>, Error> {
    let mut tables: Vec<Table> = Vec::new();
    for xml in schemas {
        let loaded = tables_from_schema(xml)?;
        if loaded
            .first()
            .is_some_and(|t| t.schema_id == event::SCHEMA_ID)
        {
            return Err(Error::Schema(format!(
                "schema id {} is reserved for event rows (persist-client/schema/events.xml)",
                event::SCHEMA_ID
            )));
        }
        if let Some(t) = loaded
            .first()
            .filter(|t| tables.iter().any(|o| o.schema_id == t.schema_id))
        {
            return Err(Error::Schema(format!(
                "two schemas have id {}: keep only the newest version",
                t.schema_id
            )));
        }
        if let Some(t) = loaded
            .iter()
            .find(|t| RESERVED_TABLES.contains(&t.name.as_str()))
        {
            return Err(Error::Schema(format!(
                "message {} would be table {}, which persistence keeps for itself",
                t.name, t.name
            )));
        }
        if let Some(t) = loaded
            .iter()
            .find(|t| tables.iter().any(|o| o.name == t.name))
        {
            return Err(Error::Schema(format!(
                "two schemas have a message named for table {}",
                t.name
            )));
        }
        tables.extend(loaded);
    }
    Ok(tables)
}

/// The messages in a buffer of `u32` LE length-prefixed messages.
fn messages(mut rest: &[u8]) -> impl Iterator<Item = &[u8]> {
    std::iter::from_fn(move || {
        let (len, tail) = rest.split_first_chunk::<4>()?;
        let (message, tail) = tail.split_at_checked(u32::from_le_bytes(*len) as usize)?;
        rest = tail;
        Some(message)
    })
}

/// A queued record's length, source id and feed flag, before its message.
const RECORD_HEADER: usize = 4 + 8 + 1;

/// The most queued bytes one insert carries by default: a batch replayed after
/// a restart can hold a queue's worth, and inserted whole it outgrew the
/// client's timeout and the server's memory and was retried forever.
const MAX_INSERT_BYTES: usize = 1 << 20;

/// Indices of the queued records that start a new insert: each record that
/// would take its piece past `max` bytes of queue, so a piece holds at most
/// `max` unless one record alone is larger. Queue positions, not row bytes, so a
/// retry cuts in the same places whatever `tables.yaml` or the table's columns
/// say by then, and each piece's token keeps naming the same records.
fn piece_starts(queued: &[u8], max: usize) -> Vec<usize> {
    let mut starts = Vec::new();
    let (mut offset, mut piece) = (0, 0);
    for (index, (_, _, message)) in records(queued).enumerate() {
        let size = RECORD_HEADER + message.len();
        if offset > piece && offset + size - piece > max {
            starts.push(index);
            piece = offset;
        }
        offset += size;
    }
    starts
}

/// Insert piece `piece` of the batch named `token` from `rows` into
/// `(table, columns)`, then empty `rows` for the next. Nothing to send is fine.
fn insert_piece(
    client: &ClickHouse,
    (table, columns): (&str, &[&str]),
    rows: &mut Vec<u8>,
    token: &str,
    piece: usize,
) -> Result<(), Error> {
    let sent = if rows.is_empty() {
        Ok(())
    } else {
        client.insert_token(table, columns, rows, &piece_token(token, piece))
    };
    rows.clear();
    sent
}

/// `(source id, from a feed, message)` of each record in a table's queue.
fn records(mut rest: &[u8]) -> impl Iterator<Item = (u64, bool, &[u8])> {
    std::iter::from_fn(move || {
        let (len, tail) = rest.split_first_chunk::<4>()?;
        let (source, tail) = tail.split_first_chunk::<8>()?;
        let (feed, tail) = tail.split_first()?;
        let (message, tail) = tail.split_at_checked(u32::from_le_bytes(*len) as usize)?;
        rest = tail;
        Some((u64::from_le_bytes(*source), *feed != 0, message))
    })
}

/// Turns SBE messages into ClickHouse rows: keeps the tables in step with the
/// schema and `tables.yaml`, and inserts what [`Writer::push`] queued.
pub struct Writer {
    ch: ClickHouse,
    /// Databases known to exist. Cleared when an insert fails: one may
    /// have been dropped.
    databases: BTreeSet<String>,
    /// `tables.yaml`'s `database:` for persistence's own tables, by name.
    own_databases: BTreeMap<String, String>,
    /// Metric views made, by database then metric name: the table each reads.
    views: BTreeMap<String, BTreeMap<String, &'static str>>,
    /// A metric, or a metric table, is new since views were last made.
    views_due: bool,
    tables: Vec<TableState>,
    config_path: PathBuf,
    config_text: String,
    /// RowBinary for the insert in progress; reused.
    rows: Vec<u8>,
    /// Set for the uncommitted batch. Empty leaves ClickHouse's content checksum.
    dedup_token: String,
    /// The most queued bytes one insert carries.
    max_insert_bytes: usize,
    recheck: Duration,
    queued_bytes: usize,
    skipped: usize,
    totals: BTreeMap<String, u64>,
    last_summary: Instant,
    recent_errors: BTreeMap<String, Instant>,
    /// Event row layouts by id, from `Shape` messages (and the saved file).
    shapes: HashMap<u32, event::Shape>,
    /// Who recorded, by source id, from `Source` messages (and the saved file).
    origins: HashMap<u64, Origin>,
    /// Metric series by id, from `MetricDef` messages (and the saved file).
    defs: HashMap<u64, MetricDef>,
    /// Open 5 s histogram windows, and the rows waiting to be inserted.
    hist: metrics::Fold,
    /// Traces by id, from `TraceDef` messages (and the saved file).
    trace_defs: HashMap<u64, TraceDef>,
    /// Rows whose source id no `Source` message has named yet.
    unknown_origins: usize,
    /// Where new shapes and sources are saved, so rows after a restart
    /// still decode and are still attributed.
    shapes_path: Option<PathBuf>,
    unknown_shapes: usize,
    shape_errors: Vec<String>,
    /// Event rows whose shape has not arrived, when they did, and their
    /// source. They are queued, so nothing is checkpointed past them, and
    /// wait up to `shape_wait` for it: the application sends every shape
    /// every 5 s.
    pending: Vec<(Instant, u64, bool, Vec<u8>)>,
    /// The message being pushed is from a feed's recording.
    feed: bool,
    shape_wait: Duration,
}

impl Writer {
    /// Load the schemas and `tables.yaml`. Nothing is sent to ClickHouse
    /// until the first [`Writer::tick`].
    pub fn new(
        schemas: &[&str],
        clickhouse: ClickHouse,
        config_path: impl Into<PathBuf>,
        recheck: Duration,
    ) -> Result<Self, Error> {
        let fixed = [Source::Metrics, Source::Histograms, Source::Traces].map(|source| {
            let mut state = TableState::new(source);
            state.config = Some(fixed_config());
            state
        });
        let mut writer = Self {
            ch: clickhouse,
            databases: BTreeSet::new(),
            own_databases: BTreeMap::new(),
            views: BTreeMap::new(),
            views_due: true,
            tables: load_schemas(schemas)?
                .into_iter()
                .map(|t| TableState::new(Source::Sbe(t)))
                .chain(fixed)
                .collect(),
            config_path: config_path.into(),
            config_text: String::new(),
            rows: Vec::new(),
            dedup_token: String::new(),
            max_insert_bytes: MAX_INSERT_BYTES,
            recheck,
            queued_bytes: 0,
            skipped: 0,
            totals: BTreeMap::new(),
            last_summary: Instant::now(),
            recent_errors: BTreeMap::new(),
            shapes: HashMap::new(),
            origins: HashMap::new(),
            defs: HashMap::new(),
            hist: metrics::Fold::default(),
            trace_defs: HashMap::new(),
            unknown_origins: 0,
            shapes_path: None,
            unknown_shapes: 0,
            shape_errors: Vec::new(),
            pending: Vec::new(),
            feed: false,
            shape_wait: Duration::from_secs(30),
        };
        let text = std::fs::read_to_string(&writer.config_path)
            .map_err(|e| Error::Config(format!("{}: {e}", writer.config_path.display())))?;
        writer.apply_config(text)?;
        Ok(writer)
    }

    /// Queue one SBE message or event row, header included, for its table.
    /// `source` is the Aeron frame's reserved value: the id of the `Source`
    /// that recorded it, or 0 for none. `false` when its table is not in
    /// `tables.yaml`: the message is skipped, and counted in the next
    /// tick's errors.
    ///
    /// A generated cross-schema enum can supply this slice with
    /// `writer.push(message.as_bytes(), source)`. This writer uses the loaded
    /// XML, so it also accepts schema ids outside that enum's configured set.
    pub fn push(&mut self, message: &[u8], source: u64) -> bool {
        self.push_message(message, source)
    }

    /// [`Writer::push`] of a message from a recording; `feed` when it is a
    /// feed's, whose tables `tables.yaml` switches here, when inserted
    /// (subscribers needed every message, so it was published regardless).
    pub fn push_from(&mut self, message: &[u8], source: u64, feed: bool) -> bool {
        self.feed = feed;
        let queued = self.push_message(message, source);
        self.feed = false;
        queued
    }

    fn push_message(&mut self, message: &[u8], source: u64) -> bool {
        let id = |at: usize| {
            message
                .get(at..at + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
        };
        let state = if id(4) == Some(event::SCHEMA_ID) {
            if id(2) == Some(event::SHAPE_TEMPLATE_ID) {
                self.add_shape(message);
                return true;
            }
            if id(2) == Some(SOURCE_TEMPLATE_ID) {
                self.add_origin(message);
                return true;
            }
            if id(2) == Some(METRIC_DEF_TEMPLATE_ID) {
                self.add_def(message);
                return true;
            }
            if id(2) == Some(TRACE_DEF_TEMPLATE_ID) {
                self.add_trace_def(message);
                return true;
            }
            if id(2) == Some(METRICS_TEMPLATE_ID) || id(2) == Some(HISTOGRAM_TEMPLATE_ID) {
                return self.push_metrics(message, source, id(2) == Some(METRICS_TEMPLATE_ID));
            }
            if id(2) == Some(TRACE_TEMPLATE_ID) {
                return self.push_trace(message, source);
            }
            let shape = message
                .get(8..12)
                .and_then(|b| self.shapes.get(&u32::from_le_bytes(b.try_into().ok()?)));
            let Some(shape) = shape.filter(|_| id(2) == Some(event::ROW_TEMPLATE_ID)) else {
                if id(2) != Some(event::ROW_TEMPLATE_ID) {
                    self.skipped += 1;
                    return false;
                }
                self.queued_bytes += message.len();
                self.pending
                    .push((Instant::now(), source, self.feed, message.to_vec()));
                return true;
            };
            self.tables.iter_mut().find_map(|s| match &mut s.source {
                Source::Events(t) if t.name == shape.table && s.config.is_some() => {
                    if t.learn(shape) {
                        s.include = None; // new columns: compare with ClickHouse again
                        s.retry_at = Instant::now();
                    }
                    Some(s)
                }
                _ => None,
            })
        } else {
            let (template, schema) = (id(2), id(4));
            self.tables.iter_mut().find(|s| {
                matches!(&s.source, Source::Sbe(t)
                    if Some(t.template_id) == template && Some(t.schema_id) == schema)
                    && s.config.is_some()
            })
        };
        let (Some(state), Ok(len)) = (state, u32::try_from(message.len())) else {
            self.skipped += 1;
            return false;
        };
        state.queued.extend_from_slice(&len.to_le_bytes());
        state.queued.extend_from_slice(&source.to_le_bytes());
        state.queued.push(u8::from(self.feed));
        state.queued.extend_from_slice(message);
        state.queued_count += 1;
        self.queued_bytes += RECORD_HEADER + message.len();
        true
    }

    /// Queue a `Trace` message, or hold it until its `TraceDef` arrives.
    fn push_trace(&mut self, message: &[u8], source: u64) -> bool {
        match traces::unknown_def(message, &self.trace_defs) {
            None => {
                self.shape_errors
                    .push("a malformed Trace message, skipped".into());
                true
            }
            Some(false) => self.queue(Source::Traces, message, source),
            Some(true) => {
                self.queued_bytes += message.len();
                self.pending
                    .push((Instant::now(), source, self.feed, message.to_vec()));
                true
            }
        }
    }

    /// Queue `message` for the fixed table `kind` names.
    fn queue(&mut self, kind: Source, message: &[u8], source: u64) -> bool {
        let state = self
            .tables
            .iter_mut()
            .find(|s| std::mem::discriminant(&s.source) == std::mem::discriminant(&kind));
        let (Some(state), Ok(len)) = (state, u32::try_from(message.len())) else {
            self.skipped += 1;
            return false;
        };
        state.queued.extend_from_slice(&len.to_le_bytes());
        state.queued.extend_from_slice(&source.to_le_bytes());
        state.queued.push(u8::from(self.feed));
        state.queued.extend_from_slice(message);
        state.queued_count += 1;
        self.queued_bytes += RECORD_HEADER + message.len();
        true
    }

    /// Queue a `Metrics` message, or fold a `Histogram` message into its 5 s
    /// window. Hold either until the `MetricDef` of every series in it has
    /// arrived (each is sent every interval), as event rows wait for their shape.
    fn push_metrics(&mut self, message: &[u8], source: u64, counters: bool) -> bool {
        match metrics::unknown_series(message, &self.defs) {
            None => {
                self.shape_errors
                    .push("a malformed Metrics or Histogram message, skipped".into());
                true
            }
            Some(0) if counters => self.queue(Source::Metrics, message, source),
            Some(0) => self.fold_histogram(message, source),
            Some(_) => {
                self.queued_bytes += message.len();
                self.pending
                    .push((Instant::now(), source, self.feed, message.to_vec()));
                true
            }
        }
    }

    /// Fold one known `Histogram` message. The open window is not charged:
    /// counting it would hold the checkpoint until the window closed.
    fn fold_histogram(&mut self, message: &[u8], source: u64) -> bool {
        let before = self.hist.ready.len();
        if let Err(err) = self.hist.push_message(message, source, self.feed) {
            self.shape_errors
                .push(format!("a malformed Histogram message, skipped: {}", err.0));
            return true;
        }
        self.charge_ready(before);
        true
    }

    fn charge_ready(&mut self, before: usize) {
        self.queued_bytes += (self.hist.ready.len() - before) * metrics::HIST_ROW_BYTES;
    }

    /// Close histogram windows the wall clock has passed. `caught_up` is
    /// false while a replay is still short of its batch or the queue is at
    /// its cap, so a partial read does not publish a window early.
    pub(crate) fn flush_elapsed_histograms(&mut self, now: u64, caught_up: bool) {
        let before = self.hist.ready.len();
        self.hist.flush_elapsed(now, caught_up);
        self.charge_ready(before);
    }

    /// Keep every event shape and source in `path` (each a `u32` LE
    /// length, then its `Shape` or `Source` message), and load those already
    /// there. A row whose `Shape` or `Source` message was purged before a
    /// restart still decodes, and is still attributed.
    pub fn keep_shapes(&mut self, path: impl Into<PathBuf>) -> Result<(), Error> {
        let path = path.into();
        let fail =
            |e: &dyn std::fmt::Display| Error::Checkpoint(format!("{}: {e}", path.display()));
        match std::fs::read(&path) {
            Ok(bytes) => {
                for message in messages(&bytes) {
                    if message.get(2..4) == Some(&SOURCE_TEMPLATE_ID.to_le_bytes()[..]) {
                        let origin =
                            Origin::decode(message).ok_or_else(|| fail(&"a malformed source"))?;
                        self.origins.insert(origin.id, origin);
                        continue;
                    }
                    if message.get(2..4) == Some(&TRACE_DEF_TEMPLATE_ID.to_le_bytes()[..]) {
                        let def = TraceDef::decode(message)
                            .ok_or_else(|| fail(&"a malformed trace def"))?;
                        self.trace_defs.insert(def.def, def);
                        continue;
                    }
                    if message.get(2..4) == Some(&METRIC_DEF_TEMPLATE_ID.to_le_bytes()[..]) {
                        let def = MetricDef::decode(message)
                            .ok_or_else(|| fail(&"a malformed metric def"))?;
                        self.defs.insert(def.series, def);
                        continue;
                    }
                    let shape =
                        event::Shape::decode(message).ok_or_else(|| fail(&"a malformed shape"))?;
                    self.shapes.insert(shape.id, shape);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(fail(&e)),
        }
        self.shapes_path = Some(path);
        Ok(())
    }

    /// Learn a shape from its `Shape` message, and save it if it is new.
    fn add_shape(&mut self, message: &[u8]) {
        let Some(shape) = event::Shape::decode(message) else {
            self.shape_errors
                .push("a malformed Shape message, skipped".into());
            return;
        };
        match self.shapes.get(&shape.id) {
            Some(known) if known.table == shape.table && known.fields == shape.fields => {}
            Some(known) => self.shape_errors.push(format!(
                "event shape id {} is both {} and {}: keeping the first",
                shape.id, known.table, shape.table
            )),
            None => {
                self.save(message);
                let id = shape.id;
                self.shapes.insert(id, shape);
                // The rows that were waiting for it.
                let waiting: Vec<_> = self
                    .pending
                    .extract_if(.., |(_, _, _, row)| {
                        row.get(8..12) == Some(&id.to_le_bytes()[..])
                    })
                    .collect();
                for (_, source, feed, row) in waiting {
                    self.queued_bytes -= row.len();
                    self.push_from(&row, source, feed);
                }
            }
        }
    }

    /// Learn who a source id is from its `Source` message, and save it if new.
    fn add_origin(&mut self, message: &[u8]) {
        let Some(origin) = Origin::decode(message) else {
            self.shape_errors
                .push("a malformed Source message, skipped".into());
            return;
        };
        if self.origins.get(&origin.id) != Some(&origin) {
            self.save(message);
            self.origins.insert(origin.id, origin);
        }
    }

    /// Learn a metric series from its `MetricDef` message, save it if new,
    /// and queue the snapshots that were waiting for it.
    fn add_def(&mut self, message: &[u8]) {
        let Some(def) = MetricDef::decode(message) else {
            self.shape_errors
                .push("a malformed MetricDef message, skipped".into());
            return;
        };
        if self.defs.get(&def.series) == Some(&def) {
            return;
        }
        self.save(message);
        self.defs.insert(def.series, def);
        self.views_due = true;
        self.repush([METRICS_TEMPLATE_ID, HISTOGRAM_TEMPLATE_ID]);
    }

    /// Learn a trace from its `TraceDef` message, save it if new, and queue
    /// the traces that were waiting for it.
    fn add_trace_def(&mut self, message: &[u8]) {
        let Some(def) = TraceDef::decode(message) else {
            self.shape_errors
                .push("a malformed TraceDef message, skipped".into());
            return;
        };
        if self.trace_defs.get(&def.def) == Some(&def) {
            return;
        }
        self.save(message);
        self.trace_defs.insert(def.def, def);
        self.repush([TRACE_TEMPLATE_ID, TRACE_TEMPLATE_ID]);
    }

    /// Push again the pending messages of these templates: a dictionary
    /// message they may have been waiting for has arrived.
    fn repush(&mut self, templates: [u16; 2]) {
        let waiting: Vec<_> = self
            .pending
            .extract_if(.., |(_, _, _, m)| {
                m.get(2..4)
                    .is_some_and(|t| templates.iter().any(|x| t == x.to_le_bytes()))
            })
            .collect();
        for (_, source, feed, m) in waiting {
            self.queued_bytes -= m.len();
            self.push_from(&m, source, feed);
        }
    }

    /// Append a dictionary message (`Shape`, `Source`, `MetricDef`,
    /// `TraceDef`) to the saved file.
    fn save(&mut self, message: &[u8]) {
        let Some(path) = &self.shapes_path else {
            return;
        };
        let mut entry = (message.len() as u32).to_le_bytes().to_vec();
        entry.extend_from_slice(message);
        let saved = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, &entry));
        if let Err(e) = saved {
            self.shape_errors.push(format!("{}: {e}", path.display()));
        }
    }

    /// How long an event row waits for its shape before it is reported and
    /// dropped (default 30 s).
    pub fn wait_for_shapes(&mut self, wait: Duration) {
        self.shape_wait = wait;
    }

    /// The client for one of persistence's own tables that is not a source
    /// here (`aeron_*`), in the database `tables.yaml` gives it.
    pub(crate) fn clickhouse_for(&self, table: &str) -> ClickHouse {
        client_in(&self.ch, self.own_databases.get(table).map(String::as_str)).into_owned()
    }

    /// Each recording application's name by its Aeron client id, from its
    /// `Source` message: the driver names C clients' counters by id only.
    pub(crate) fn client_names(&self) -> HashMap<i64, String> {
        self.origins
            .values()
            .filter(|o| o.client != 0)
            .map(|o| {
                let name = if o.app.is_empty() { &o.pod } else { &o.app };
                (o.client, name.clone())
            })
            .collect()
    }

    /// Bytes of messages waiting to be inserted.
    #[must_use]
    pub fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }

    /// Insert at most `bytes` of a table's queue at a time (1 MiB by
    /// default); a larger queue goes in several inserts.
    pub fn set_max_insert_bytes(&mut self, bytes: usize) {
        self.max_insert_bytes = bytes;
    }

    /// Identify this tick's inserts. A retry of the same batch passes the
    /// same token, and ClickHouse drops it. Only a named batch is split
    /// at [`Writer::set_max_insert_bytes`].
    pub fn set_dedup_token(&mut self, token: &str) {
        self.dedup_token.clear();
        self.dedup_token.push_str(token);
    }

    /// Close elapsed histogram windows, reload `tables.yaml` if it changed,
    /// sync tables, and insert what is queued.
    ///
    /// Call after delivering all available live messages. Archive ingestion
    /// checks replay positions before closing windows and uses its own tick.
    pub fn tick(&mut self) -> Report {
        let mut report = Report::default();
        let now = persist_client::clock::Clock::new().wall().epoch_ns();
        self.flush_elapsed_histograms(u64::try_from(now).unwrap_or(0), true);
        self.run(&mut report);
        self.log(&report);
        report
    }

    fn run(&mut self, report: &mut Report) {
        match std::fs::read_to_string(&self.config_path) {
            Ok(text) if text != self.config_text => {
                if let Err(e) = self.apply_config(text) {
                    report
                        .errors
                        .push(format!("{e}; keeping the previous configuration"));
                }
            }
            Ok(_) => {}
            Err(e) => report.errors.push(format!(
                "{}: {e}; keeping the previous configuration",
                self.config_path.display()
            )),
        }
        if self.skipped > 0 {
            report.errors.push(format!(
                "{} records skipped: their table is not in tables.yaml",
                std::mem::take(&mut self.skipped)
            ));
        }
        let now = Instant::now();
        let wait = self.shape_wait;
        let expired: Vec<_> = self
            .pending
            .extract_if(.., |(at, ..)| now.duration_since(*at) >= wait)
            .collect();
        for (.., row) in expired {
            self.queued_bytes -= row.len();
            self.unknown_shapes += 1;
        }
        if self.unknown_shapes > 0 {
            report.errors.push(format!(
                "{} rows skipped: their Shape or MetricDef message did not arrive within {wait:?} (none saved)",
                std::mem::take(&mut self.unknown_shapes)
            ));
        }
        report.errors.append(&mut self.shape_errors);
        self.sync_tables(report);
        self.flush(report);
        if self.unknown_origins > 0 {
            report.errors.push(format!(
                "{} rows written with no host, pod or app: their source's Source message has not arrived (none saved)",
                std::mem::take(&mut self.unknown_origins)
            ));
        }
    }

    fn apply_config(&mut self, text: String) -> Result<(), Error> {
        let mut config = parse_config(&text)?;
        self.own_databases = RESERVED_TABLES
            .iter()
            .filter_map(|name| Some(((*name).to_owned(), config.get(*name)?.database.clone()?)))
            .collect();
        for state in &mut self.tables {
            let new = if matches!(
                state.source,
                Source::Metrics | Source::Histograms | Source::Traces
            ) {
                // Persistence's own: always there; only its database is set here.
                Some(TableConfig {
                    database: self.own_databases.get(state.source.name()).cloned(),
                    ..fixed_config()
                })
            } else {
                config.remove(state.source.name())
            };
            let placement =
                |c: &Option<TableConfig>| c.as_ref().map(|c| (c.kind, c.database.clone()));
            if placement(&new) != placement(&state.config) {
                // Kind or database changed: compare with ClickHouse again.
                state.include = None;
                state.retry_at = Instant::now();
            }
            state.config = new;
        }
        // Tables that are not SBE messages are fed by `tracing` events;
        // persistence's own tables are listed only to switch them.
        for (name, table_config) in config
            .into_iter()
            .filter(|(name, _)| !RESERVED_TABLES.contains(&name.as_str()))
        {
            let mut state = TableState::new(Source::Events(EventTable::new(name)));
            state.config = Some(table_config);
            self.tables.push(state);
        }
        self.config_text = text;
        Ok(())
    }

    /// Create/compare every table named in `tables.yaml`, enabled or not, so
    /// a disabled table exists (empty) and queries against it still work.
    fn sync_tables(&mut self, report: &mut Report) {
        let now = Instant::now();
        for state in &mut self.tables {
            let Some(kind) = state.config.as_ref().map(|c| c.kind) else {
                continue;
            };
            // Tables with outstanding problems are re-checked, so running the
            // suggested ALTER is picked up without a restart.
            let due = state.include.is_none() || !state.problems.is_empty();
            let Some(mut shape) = state
                .source
                .shape()
                .filter(|_| due && now >= state.retry_at)
            else {
                continue;
            };
            shape.columns.extend(origin_columns());
            let ch = client_in(&self.ch, state.database());
            if let Err(e) = ensure_database(&mut self.databases, &ch) {
                // ClickHouse is unreachable: one error, not one per table.
                report.errors.push(e.to_string());
                return;
            }
            match ch.sync(&shape, kind) {
                Ok(sync) => {
                    self.views_due |= matches!(state.source, Source::Metrics | Source::Histograms);
                    report.applied.extend(sync.applied);
                    if sync.problems.is_empty() && !state.problems.is_empty() {
                        log::info!("{}: fixed, writing every column", state.source.name());
                    }
                    if sync.problems != state.problems {
                        report.problems.extend(
                            sync.problems
                                .iter()
                                .map(|p| format!("{}: {p}", state.source.name())),
                        );
                    }
                    state.columns = shape
                        .columns
                        .into_iter()
                        .zip(&sync.include)
                        .filter(|(_, i)| **i)
                        .map(|(c, _)| c.name)
                        .collect();
                    state.include = Some(sync.include);
                    state.problems = sync.problems;
                    state.retry_at = now + self.recheck;
                }
                Err(e) => {
                    report.errors.push(format!("{}: {e}", state.source.name()));
                    state.retry_at = now + Duration::from_secs(5);
                }
            }
        }
        self.sync_views(report);
    }

    /// A view per metric name beside `metrics` and `metrics_histogram`,
    /// when `tables.yaml` gives those a database of their own:
    /// `SELECT * FROM metrics.tick_to_trade_ns` reads that metric's rows.
    /// A metric named like a table gets none; when a counter and a histogram
    /// share a name, the first one seen keeps it. Both are reported.
    fn sync_views(&mut self, report: &mut Report) {
        if !std::mem::take(&mut self.views_due) {
            return;
        }
        for state in &self.tables {
            let table = match state.source {
                Source::Metrics => metrics::METRICS,
                Source::Histograms => metrics::HISTOGRAMS,
                _ => continue,
            };
            // Only in a database of their own, once the table is there.
            let (Some(database), Some(_)) = (state.database(), &state.include) else {
                continue;
            };
            let ch = client_in(&self.ch, Some(database));
            let made = self.views.entry(database.to_owned()).or_default();
            for def in self
                .defs
                .values()
                .filter(|d| metrics::table_of(d.kind) == table)
            {
                let name = def.name.as_str();
                if RESERVED_TABLES.contains(&name)
                    || self.tables.iter().any(|s| s.source.name() == name)
                {
                    report.problems.push(format!(
                        "{database}.{name}: metric {name} has no view; a table has that name"
                    ));
                    continue;
                }
                match made.get(name) {
                    Some(t) if *t == table => {}
                    Some(t) => report.problems.push(format!(
                        "{database}.{name}: the view shows {t}; {name}'s {table} rows have no view"
                    )),
                    None => match ch.create_metric_view(name, table) {
                        Ok(ddl) => {
                            report.applied.push(ddl);
                            made.insert(name.to_owned(), table);
                        }
                        Err(e) => {
                            report.errors.push(format!("{database}.{name}: {e}"));
                            self.views_due = true;
                        }
                    },
                }
            }
        }
    }

    /// Insert histogram rows the fold has closed. A failed insert puts the
    /// attempted rows back and leaves them charged, so the checkpoint waits.
    fn flush_histograms(&mut self, report: &mut Report) {
        if self.hist.ready.is_empty() {
            return;
        }
        let Some(index) = self
            .tables
            .iter()
            .position(|state| matches!(state.source, Source::Histograms))
        else {
            return;
        };
        let Some(include_all) = self.tables[index].include.clone() else {
            return;
        };
        let charged = self.hist.ready.len();
        let ready = std::mem::take(&mut self.hist.ready);
        let (include, origin_flags) =
            include_all.split_at(include_all.len() - ORIGIN_COLUMNS.len());
        let config = self.tables[index].config.clone();
        let now = jiff::Timestamp::now();
        self.rows.clear();
        let mut attempted = Vec::new();
        let mut dropped = 0usize;
        for row in ready {
            let known = self
                .origins
                .get(&row.source)
                .map(|origin| (origin.host.clone(), origin.pod.clone(), origin.app.clone()));
            if known.is_none() && row.source != 0 {
                self.unknown_origins += 1;
            }
            let (host, pod, app) = known.unwrap_or_default();
            if row.feed && !config.as_ref().is_some_and(|c| c.is_on(&app, now)) {
                dropped += 1;
                continue;
            }
            let Some(def) = self.defs.get(&row.series).cloned() else {
                report.errors.push(format!(
                    "{}: series {} has no MetricDef, dropped",
                    metrics::HISTOGRAMS,
                    row.series
                ));
                dropped += 1;
                continue;
            };
            let mut origin = Vec::new();
            for (name, keep) in [&host, &pod, &app].into_iter().zip(origin_flags) {
                if *keep {
                    table::write_string(name.as_bytes(), &mut origin);
                }
            }
            metrics::write_ready(&row, include, &origin, &def, &mut self.rows);
            attempted.push(row);
        }
        let written = attempted.len();
        debug_assert_eq!(charged, written + dropped);
        let columns = self.tables[index].columns.clone();
        let column_refs: Vec<&str> = columns.iter().map(String::as_str).collect();
        let inserted = if written == 0 {
            Ok(())
        } else {
            client_in(&self.ch, self.tables[index].database()).insert_token(
                metrics::HISTOGRAMS,
                &column_refs,
                &self.rows,
                &self.dedup_token,
            )
        };
        match inserted {
            Ok(()) => {
                if written > 0 {
                    report
                        .inserted
                        .insert(metrics::HISTOGRAMS.to_string(), written);
                    *self
                        .totals
                        .entry(metrics::HISTOGRAMS.to_string())
                        .or_default() += written as u64;
                }
                self.queued_bytes -= (written + dropped) * metrics::HIST_ROW_BYTES;
            }
            Err(e) => {
                report.errors.push(format!(
                    "{}: insert failed, keeping {} records to retry: {e}",
                    metrics::HISTOGRAMS,
                    attempted.len()
                ));
                self.queued_bytes -= dropped * metrics::HIST_ROW_BYTES;
                self.hist.ready = attempted;
                let state = &mut self.tables[index];
                state.include = None;
                state.retry_at = Instant::now();
                self.databases.clear();
                self.views.clear();
            }
        }
    }

    fn flush(&mut self, report: &mut Report) {
        self.flush_histograms(report);
        for state in &mut self.tables {
            let Some(include) = &state.include else {
                continue; // re-synced first; the messages wait
            };
            // Histogram rows come from the 5 s fold, not from a queued message.
            if state.queued_count == 0 || matches!(state.source, Source::Histograms) {
                continue;
            }
            self.rows.clear();
            let (mut rows, mut misfits) = (0, 0);
            let (include, origin) = include.split_at(include.len() - ORIGIN_COLUMNS.len());
            let dict = Dictionaries {
                shapes: &self.shapes,
                defs: &self.defs,
                traces: &self.trace_defs,
            };
            let mut names = Vec::new();
            let now = jiff::Timestamp::now();
            let columns: Vec<&str> = state.columns.iter().map(String::as_str).collect();
            let client = client_in(&self.ch, state.database());
            // A named batch goes in pieces, each encoded and sent before the
            // next: neither one insert nor this buffer grows with the queue.
            let starts = if self.dedup_token.is_empty() {
                Vec::new()
            } else {
                piece_starts(&state.queued, self.max_insert_bytes)
            };
            let mut starts = starts.into_iter().peekable();
            let (mut piece, mut inserted) = (0, Ok(()));
            for (index, (source, feed, message)) in records(&state.queued).enumerate() {
                if starts.next_if_eq(&index).is_some() {
                    inserted = insert_piece(
                        &client,
                        (state.source.name(), &columns),
                        &mut self.rows,
                        &self.dedup_token,
                        piece,
                    );
                    piece += 1;
                    if inserted.is_err() {
                        break;
                    }
                }
                let known = self.origins.get(&source);
                if known.is_none() && source != 0 {
                    self.unknown_origins += 1;
                }
                let known = known.map_or(["", "", ""], |o| {
                    [o.host.as_str(), o.pod.as_str(), o.app.as_str()]
                });
                // A feed was published whatever `tables.yaml` says; whether
                // it is kept is decided now, for the app that recorded it.
                if feed
                    && !state
                        .config
                        .as_ref()
                        .is_some_and(|c| c.is_on(known[2], now))
                {
                    continue;
                }
                names.clear();
                for (name, _) in known.iter().zip(origin).filter(|(_, i)| **i) {
                    table::write_string(name.as_bytes(), &mut names);
                }
                match state.source.write_rows(
                    message,
                    include,
                    (&names, known),
                    &mut self.rows,
                    &dict,
                ) {
                    Ok((n, bad)) => {
                        rows += n;
                        misfits += bad;
                    }
                    Err(e) => report.errors.push(format!(
                        "{}: undecodable message skipped: {}",
                        state.source.name(),
                        e.0
                    )),
                }
            }
            if misfits > 0 {
                report.errors.push(format!(
                    "{}: {misfits} value(s) did not match their column's type (set by the first shape seen); wrote NULL",
                    state.source.name()
                ));
            }
            if inserted.is_ok() {
                inserted = insert_piece(
                    &client,
                    (state.source.name(), &columns),
                    &mut self.rows,
                    &self.dedup_token,
                    piece,
                );
            }
            match inserted {
                Ok(()) => {
                    if rows > 0 {
                        report
                            .inserted
                            .insert(state.source.name().to_string(), rows);
                        *self
                            .totals
                            .entry(state.source.name().to_string())
                            .or_default() += rows as u64;
                    }
                    self.queued_bytes -= state.queued.len();
                    state.queued.clear();
                    state.queued_count = 0;
                }
                Err(e) => {
                    report.errors.push(format!(
                        "{}: insert failed, keeping {} records to retry: {e}",
                        state.source.name(),
                        state.queued_count
                    ));
                    // The table (or database) may have been altered or dropped
                    // since it was synced: compare it again before retrying.
                    state.include = None;
                    state.retry_at = Instant::now();
                    self.databases.clear();
                    self.views.clear();
                }
            }
        }
    }

    fn log(&mut self, report: &Report) {
        for ddl in &report.applied {
            log::info!("applied: {ddl}");
        }
        for purged in &report.purged {
            log::info!("{purged}");
        }
        for p in &report.problems {
            log::error!("{p}");
        }
        // The same failure repeats every tick while ClickHouse is down;
        // say it once per 30 s.
        let now = Instant::now();
        self.recent_errors
            .retain(|_, at| now.duration_since(*at) < Duration::from_secs(30));
        for e in &report.errors {
            if !self.recent_errors.contains_key(e) {
                self.recent_errors.insert(e.clone(), now);
                log::error!("{e}");
            }
        }
        if self.last_summary.elapsed() >= Duration::from_secs(60) {
            self.last_summary = Instant::now();
            log::info!("rows written so far: {:?}", self.totals);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(lens: &[usize]) -> Vec<u8> {
        let mut queued = Vec::new();
        for (i, len) in lens.iter().enumerate() {
            queued.extend_from_slice(&u32::try_from(*len).unwrap_or(0).to_le_bytes());
            queued.extend_from_slice(&(i as u64).to_le_bytes());
            queued.push(u8::from(i % 2 == 0));
            queued.extend(std::iter::repeat_n(0, *len));
        }
        queued
    }

    #[test]
    fn pieces_start_at_each_limit_of_queue_bytes() {
        // Records of 13 + 87 = 100 bytes, at most 250 per piece.
        let queued = queue(&[87; 7]);
        assert_eq!(piece_starts(&queued, 250), [2, 4, 6]);
        assert_eq!(piece_starts(&queued, 100), [1, 2, 3, 4, 5, 6]);
        assert!(piece_starts(&queued, 1 << 20).is_empty());
        assert!(piece_starts(&[], 1).is_empty());
    }

    #[test]
    fn a_record_larger_than_the_limit_is_a_piece_of_its_own() {
        let queued = queue(&[10, 500, 10, 10]);
        assert_eq!(piece_starts(&queued, 100), [1, 2]);
    }
}
