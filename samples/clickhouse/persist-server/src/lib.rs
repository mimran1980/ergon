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

use std::collections::{BTreeMap, HashMap};
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
    /// `CLICKHOUSE_DATABASE` (`market`), `PERSIST_CONFIG`
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
                &var("CLICKHOUSE_DATABASE", "market"),
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
            Self::Histograms => {
                metrics::write_histogram(message, include, origin, dict.defs, out).map(|n| (n, 0))
            }
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
    }
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
    database_ready: bool,
    tables: Vec<TableState>,
    config_path: PathBuf,
    config_text: String,
    /// RowBinary for the insert in progress; reused.
    rows: Vec<u8>,
    /// Set for the uncommitted batch. Empty leaves ClickHouse's content checksum.
    dedup_token: String,
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
            database_ready: false,
            tables: load_schemas(schemas)?
                .into_iter()
                .map(|t| TableState::new(Source::Sbe(t)))
                .chain(fixed)
                .collect(),
            config_path: config_path.into(),
            config_text: String::new(),
            rows: Vec::new(),
            dedup_token: String::new(),
            recheck,
            queued_bytes: 0,
            skipped: 0,
            totals: BTreeMap::new(),
            last_summary: Instant::now(),
            recent_errors: BTreeMap::new(),
            shapes: HashMap::new(),
            origins: HashMap::new(),
            defs: HashMap::new(),
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
        self.queued_bytes += 13 + message.len();
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
        self.queued_bytes += 13 + message.len();
        true
    }

    /// Queue a `Metrics` or `Histogram` message, or hold it until the
    /// `MetricDef` of every series in it has arrived (each is sent every
    /// interval), as event rows wait for their shape.
    fn push_metrics(&mut self, message: &[u8], source: u64, counters: bool) -> bool {
        match metrics::unknown_series(message, &self.defs) {
            None => {
                self.shape_errors
                    .push("a malformed Metrics or Histogram message, skipped".into());
                true
            }
            Some(0) if counters => self.queue(Source::Metrics, message, source),
            Some(0) => self.queue(Source::Histograms, message, source),
            Some(_) => {
                self.queued_bytes += message.len();
                self.pending
                    .push((Instant::now(), source, self.feed, message.to_vec()));
                true
            }
        }
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

    pub(crate) fn clickhouse(&self) -> &ClickHouse {
        &self.ch
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

    /// Identify this tick's inserts. A retry of the same batch passes the
    /// same token, and ClickHouse drops it.
    pub(crate) fn set_dedup_token(&mut self, token: &str) {
        self.dedup_token.clear();
        self.dedup_token.push_str(token);
    }

    /// Reload `tables.yaml` if it changed, sync tables, insert what is queued.
    pub fn tick(&mut self) -> Report {
        let mut report = Report::default();
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
        for state in &mut self.tables {
            if matches!(
                state.source,
                Source::Metrics | Source::Histograms | Source::Traces
            ) {
                continue; // persistence's own: always there
            }
            let new = config.remove(state.source.name());
            if new.as_ref().map(|c| c.kind) != state.config.as_ref().map(|c| c.kind) {
                state.include = None; // kind changed: compare with ClickHouse again
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
        if !self.database_ready {
            match self.ch.create_database() {
                Ok(()) => self.database_ready = true,
                Err(e) => {
                    report.errors.push(e.to_string());
                    return;
                }
            }
        }
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
            match self.ch.sync(&shape, kind) {
                Ok(sync) => {
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
    }

    fn flush(&mut self, report: &mut Report) {
        for state in &mut self.tables {
            let Some(include) = &state.include else {
                continue; // re-synced first; the messages wait
            };
            if state.queued_count == 0 {
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
            for (source, feed, message) in records(&state.queued) {
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
            let columns: Vec<&str> = state.columns.iter().map(String::as_str).collect();
            let inserted = if rows == 0 {
                Ok(())
            } else {
                self.ch
                    .insert_token(state.source.name(), &columns, &self.rows, &self.dedup_token)
            };
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
                    self.database_ready = false;
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
