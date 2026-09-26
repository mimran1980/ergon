//! Ingest recorded SBE messages into ClickHouse.
//!
//! Applications record with `persist-client`, which publishes each message on
//! an Aeron stream. The Aeron Archive records that stream to disk, and the
//! [`Ingester`] replays the recording from its checkpoint: it inserts the
//! messages, saves the new checkpoint, and purges the recording up to it. The
//! archive is the buffer, so neither a slow ClickHouse nor a restarted
//! ingester loses a record.
//!
//! The SBE schema is the table definition: every message is a table and every
//! field a column (see `table.rs`). Any other table in `tables.yaml` is fed by
//! `tracing` events, and its columns are their fields (see `events.rs`).
//! [`Writer`] batches each table into one `INSERT … FORMAT RowBinary` a
//! second.
//!
//! `config/tables.yaml` names the tables and is re-read while running:
//!
//! ```yaml
//! tables:
//!   trade:         { kind: static }                  # created if missing, never altered
//!   book_snapshot: { kind: dynamic, enabled: false } # follows the schema
//! ```
//!
//! * **static** tables are created once and then never altered. If the schema
//!   and the table disagree, the mismatching columns are not written and an
//!   ERROR names the `ALTER` that would fix it; everything else keeps flowing.
//! * **dynamic** tables follow the schema: a new SBE field becomes
//!   `ALTER TABLE … ADD COLUMN` the next time the ingester starts, and a new
//!   event field as soon as it arrives.
//! * `enabled` is read by the application (`persist-client`): it decides what
//!   is recorded. Every listed table is created here, so queries against a
//!   disabled one still work.
//!
//! A failed insert re-checks its table first, so a table altered or dropped
//! while recording is reported (static) or fixed (dynamic, or recreated) and
//! the records are retried.

mod clickhouse;
mod events;
mod ingest;
mod table;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
        }
    }

    /// [`Settings::new`] from `CLICKHOUSE_URL` (`http://localhost:8123`),
    /// `CLICKHOUSE_USER` (`lab`), `CLICKHOUSE_PASSWORD` (`lab`),
    /// `CLICKHOUSE_DATABASE` (`market`), `PERSIST_CONFIG`
    /// (`config/tables.yaml`) and `PERSIST_CHECKPOINT` (`persist.checkpoint`).
    #[must_use]
    pub fn from_env() -> Self {
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

/// Where a table's rows come from.
enum Source {
    Sbe(Table),
    Events(EventTable),
}

impl Source {
    fn name(&self) -> &str {
        match self {
            Self::Sbe(t) => &t.name,
            Self::Events(t) => &t.name,
        }
    }

    /// `None` for an event table that has not had a row yet.
    fn shape(&self) -> Option<Shape> {
        match self {
            Self::Sbe(t) => Some(t.shape()),
            Self::Events(t) => t.shape(),
        }
    }

    /// Returns how many event values did not fit their column's type.
    fn write_row(
        &self,
        message: &[u8],
        include: &[bool],
        out: &mut Vec<u8>,
        shapes: &HashMap<u32, event::Shape>,
    ) -> Result<usize, table::DecodeError> {
        match self {
            Self::Sbe(t) => t.write_row(message, include, out).map(|()| 0),
            Self::Events(t) => t.write_row(message, include, out, shapes),
        }
    }
}

struct TableState {
    source: Source,
    config: Option<TableConfig>,
    /// `Some` once the table exists and the writable columns are known.
    include: Option<Vec<bool>>,
    columns: Vec<String>,
    problems: Vec<String>,
    retry_at: Instant,
    /// SBE messages not yet inserted, each a `u32` LE length then the
    /// message. Kept as SBE so they can be decoded again if the table changes.
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
    recheck: Duration,
    queued_bytes: usize,
    skipped: usize,
    totals: BTreeMap<String, u64>,
    last_summary: Instant,
    recent_errors: BTreeMap<String, Instant>,
    /// Event row layouts by id, from `Shape` messages (and the saved file).
    shapes: HashMap<u32, event::Shape>,
    /// Where new shapes are saved, so rows after a restart still decode.
    shapes_path: Option<PathBuf>,
    unknown_shapes: usize,
    shape_errors: Vec<String>,
    /// Event rows whose shape has not arrived, and when they did. They are
    /// queued, so nothing is checkpointed past them, and wait up to
    /// `shape_wait` for it: the application sends every shape every 5 s.
    pending: Vec<(Instant, Vec<u8>)>,
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
        let mut writer = Self {
            ch: clickhouse,
            database_ready: false,
            tables: load_schemas(schemas)?
                .into_iter()
                .map(|t| TableState::new(Source::Sbe(t)))
                .collect(),
            config_path: config_path.into(),
            config_text: String::new(),
            rows: Vec::new(),
            recheck,
            queued_bytes: 0,
            skipped: 0,
            totals: BTreeMap::new(),
            last_summary: Instant::now(),
            recent_errors: BTreeMap::new(),
            shapes: HashMap::new(),
            shapes_path: None,
            unknown_shapes: 0,
            shape_errors: Vec::new(),
            pending: Vec::new(),
            shape_wait: Duration::from_secs(30),
        };
        let text = std::fs::read_to_string(&writer.config_path)
            .map_err(|e| Error::Config(format!("{}: {e}", writer.config_path.display())))?;
        writer.apply_config(text)?;
        Ok(writer)
    }

    /// Queue one SBE message or event row, header included, for its table.
    /// `false` when its table is not in `tables.yaml`: the message is
    /// skipped, and counted in the next tick's errors.
    pub fn push(&mut self, message: &[u8]) -> bool {
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
            let shape = message
                .get(8..12)
                .and_then(|b| self.shapes.get(&u32::from_le_bytes(b.try_into().ok()?)));
            let Some(shape) = shape.filter(|_| id(2) == Some(event::ROW_TEMPLATE_ID)) else {
                if id(2) != Some(event::ROW_TEMPLATE_ID) {
                    self.skipped += 1;
                    return false;
                }
                self.queued_bytes += message.len();
                self.pending.push((Instant::now(), message.to_vec()));
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
        state.queued.extend_from_slice(message);
        state.queued_count += 1;
        self.queued_bytes += 4 + message.len();
        true
    }

    /// Keep every event shape in `path` (each a `u32` LE length, then its
    /// `Shape` message), and load those already there. A row whose `Shape`
    /// message was purged before a restart still decodes.
    pub fn keep_shapes(&mut self, path: impl Into<PathBuf>) -> Result<(), Error> {
        let path = path.into();
        let fail =
            |e: &dyn std::fmt::Display| Error::Checkpoint(format!("{}: {e}", path.display()));
        match std::fs::read(&path) {
            Ok(bytes) => {
                for message in messages(&bytes) {
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
                if let Some(path) = &self.shapes_path {
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
                let id = shape.id;
                self.shapes.insert(id, shape);
                // The rows that were waiting for it.
                let waiting: Vec<_> = self
                    .pending
                    .extract_if(.., |(_, row)| row.get(8..12) == Some(&id.to_le_bytes()[..]))
                    .collect();
                for (_, row) in waiting {
                    self.queued_bytes -= row.len();
                    self.push(&row);
                }
            }
        }
    }

    /// How long an event row waits for its shape before it is reported and
    /// dropped (default 30 s).
    pub fn wait_for_shapes(&mut self, wait: Duration) {
        self.shape_wait = wait;
    }

    /// Bytes of messages waiting to be inserted.
    #[must_use]
    pub fn queued_bytes(&self) -> usize {
        self.queued_bytes
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
            .extract_if(.., |(at, _)| now.duration_since(*at) >= wait)
            .collect();
        for (_, row) in expired {
            self.queued_bytes -= row.len();
            self.unknown_shapes += 1;
        }
        if self.unknown_shapes > 0 {
            report.errors.push(format!(
                "{} event rows skipped: their shape did not arrive within {wait:?} (no Shape message, and none saved)",
                std::mem::take(&mut self.unknown_shapes)
            ));
        }
        report.errors.append(&mut self.shape_errors);
        self.sync_tables(report);
        self.flush(report);
    }

    fn apply_config(&mut self, text: String) -> Result<(), Error> {
        let mut config = parse_config(&text)?;
        for state in &mut self.tables {
            let new = config.remove(state.source.name());
            if new.map(|c| c.kind) != state.config.map(|c| c.kind) {
                state.include = None; // kind changed: compare with ClickHouse again
                state.retry_at = Instant::now();
            }
            state.config = new;
        }
        // Tables that are not SBE messages are fed by `tracing` events.
        for (name, table_config) in config {
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
            let Some(config) = state.config else {
                continue;
            };
            // Tables with outstanding problems are re-checked, so running the
            // suggested ALTER is picked up without a restart.
            let due = state.include.is_none() || !state.problems.is_empty();
            let Some(shape) = state
                .source
                .shape()
                .filter(|_| due && now >= state.retry_at)
            else {
                continue;
            };
            match self.ch.sync(&shape, config.kind) {
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
            for message in messages(&state.queued) {
                match state
                    .source
                    .write_row(message, include, &mut self.rows, &self.shapes)
                {
                    Ok(n) => {
                        rows += 1;
                        misfits += n;
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
            let inserted = match rows {
                0 => Ok(()),
                _ => self.ch.insert(state.source.name(), &columns, &self.rows),
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
