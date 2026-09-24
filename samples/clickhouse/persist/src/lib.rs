//! Record SBE messages into ClickHouse.
//!
//! The SBE schema is the table definition: every message is a table and every
//! field a column (see `table.rs`). The application records a message by
//! encoding it with its generated SBE encoder straight into persist's buffer;
//! a writer thread batches the buffer into `INSERT … FORMAT RowBinary`
//! (`recorder/src/main.rs` is the full example):
//!
//! ```text
//! let (persist, writer) = Persist::start(SCHEMA, Settings::from_env())?;
//! persist.record(TradeEncoder::TEMPLATE_ID, |buf| {
//!     Ok(TradeEncoder::wrap_and_apply_header(buf, 0)
//!         .fixed(&fields)
//!         .symbol(b"BTCUSDT")?
//!         .encoded_length_with_header())
//! })?;
//! writer.stop(); // flushes what is queued
//! ```
//!
//! Recording never allocates, never copies, and never waits on ClickHouse: a
//! disabled table costs one atomic load, an enabled one an uncontended lock
//! plus the encode itself.
//!
//! `config/tables.yaml` decides what is recorded, and is re-read while running:
//!
//! ```yaml
//! tables:
//!   trade:         { kind: static }                  # created if missing, never altered
//!   book_snapshot: { kind: dynamic, enabled: false } # follows the schema; toggle live
//! ```
//!
//! * **static** tables are created once and then never altered. If the schema
//!   and the table disagree, the mismatching columns are not written and an
//!   ERROR names the `ALTER` that would fix it; everything else keeps flowing.
//! * **dynamic** tables follow the schema: a new SBE field becomes
//!   `ALTER TABLE … ADD COLUMN` the next time the recorder starts.
//! * `enabled` switches recording on or off within a second, no restart.
//!
//! Records wait in memory until they are inserted. If ClickHouse is down they
//! are retained up to `max_buffered_bytes`, then dropped and counted — there
//! is no disk spool.

mod clickhouse;
mod table;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::Deserialize;

pub use clickhouse::ClickHouse;
pub use table::{Table, tables_from_schema};

/// Everything that can go wrong outside the recording hot path.
#[derive(Debug)]
pub enum Error {
    /// The SBE schema cannot be turned into tables.
    Schema(String),
    /// A ClickHouse request failed.
    ClickHouse(String),
    /// `tables.yaml` is missing or invalid.
    Config(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(m) => write!(f, "schema: {m}"),
            Self::ClickHouse(m) => write!(f, "clickhouse: {m}"),
            Self::Config(m) => write!(f, "tables.yaml: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// Whether persistence may change a table's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableKind {
    /// Created if missing, never altered.
    Static,
    /// Created and extended to follow the schema.
    Dynamic,
}

/// One entry of `tables.yaml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableConfig {
    /// Static or dynamic.
    pub kind: TableKind,
    /// Record this table now (default `true`).
    #[serde(default = "yes")]
    pub enabled: bool,
}

const fn yes() -> bool {
    true
}

/// Parse `tables.yaml`.
fn parse_config(text: &str) -> Result<BTreeMap<String, TableConfig>, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        tables: BTreeMap<String, TableConfig>,
    }
    serde_yaml::from_str::<File>(text)
        .map(|f| f.tables)
        .map_err(|e| Error::Config(e.to_string()))
}

/// Connection and file locations, usually from the environment.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Target server and database.
    pub clickhouse: ClickHouse,
    /// `tables.yaml`, re-read while running.
    pub config_path: PathBuf,
    /// Size of each of the two record buffers, allocated once up front.
    /// Records that do not fit are dropped and counted.
    pub max_buffered_bytes: usize,
    /// How often the writer inserts. ClickHouse wants about one insert per
    /// table per second; more often only makes more parts to merge.
    pub flush_interval: Duration,
    /// How often a table with unfixed problems is compared again, so running
    /// the logged `ALTER` takes effect without a restart.
    pub recheck: Duration,
}

impl Settings {
    /// `CLICKHOUSE_URL` (`http://localhost:8123`), `CLICKHOUSE_USER` (`lab`),
    /// `CLICKHOUSE_PASSWORD` (`lab`), `CLICKHOUSE_DATABASE` (`market`) and
    /// `PERSIST_CONFIG` (`config/tables.yaml`).
    #[must_use]
    pub fn from_env() -> Self {
        let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        Self {
            clickhouse: ClickHouse::new(
                &var("CLICKHOUSE_URL", "http://localhost:8123"),
                &var("CLICKHOUSE_USER", "lab"),
                &var("CLICKHOUSE_PASSWORD", "lab"),
                &var("CLICKHOUSE_DATABASE", "market"),
            ),
            config_path: var("PERSIST_CONFIG", "config/tables.yaml").into(),
            max_buffered_bytes: 256 << 20,
            flush_interval: Duration::from_secs(1),
            recheck: Duration::from_secs(30),
        }
    }
}

/// Largest message [`Persist::record`] guarantees room for. Recording is
/// refused (and counted as dropped) once less than this is free, so an
/// encoder is never handed a buffer too short for its fixed block.
pub const MAX_MESSAGE: usize = 64 * 1024;

#[derive(Debug)]
struct Shared {
    /// Indexed by SBE template id.
    enabled: Vec<AtomicBool>,
    /// Records waiting for the writer, which swaps it for an empty one.
    pending: Mutex<Frames>,
    dropped: AtomicU64,
}

/// Encoded messages, each a `u32` LE length then the message. Allocated once
/// at full size, so recording never allocates.
#[derive(Debug)]
struct Frames {
    data: Box<[u8]>,
    len: usize,
}

impl Frames {
    fn new(capacity: usize) -> Self {
        Self {
            data: vec![0; capacity].into_boxed_slice(),
            len: 0,
        }
    }
}

/// The recording handle. Cheap to clone; share it with every callback.
#[derive(Clone, Debug)]
pub struct Persist {
    shared: Arc<Shared>,
}

impl Persist {
    /// Load the schema and `tables.yaml`, and prepare the writer without
    /// starting it. Tests drive [`Writer::tick`] themselves; applications
    /// use [`Persist::start`].
    pub fn new(schema_xml: &str, settings: Settings) -> Result<(Self, Writer), Error> {
        let tables = tables_from_schema(schema_xml)?;
        let slots = tables
            .iter()
            .map(|t| usize::from(t.template_id) + 1)
            .max()
            .unwrap_or(0);
        let shared = Arc::new(Shared {
            enabled: (0..slots).map(|_| AtomicBool::new(false)).collect(),
            pending: Mutex::new(Frames::new(settings.max_buffered_bytes)),
            dropped: AtomicU64::new(0),
        });
        let mut writer = Writer {
            shared: Arc::clone(&shared),
            ch: settings.clickhouse,
            database_ready: false,
            tables: tables.into_iter().map(TableState::new).collect(),
            config_path: settings.config_path,
            config_text: String::new(),
            spare: Frames::new(settings.max_buffered_bytes),
            max_retained: settings.max_buffered_bytes,
            recheck: settings.recheck,
            totals: BTreeMap::new(),
            last_summary: Instant::now(),
            recent_errors: BTreeMap::new(),
        };
        let text = std::fs::read_to_string(&writer.config_path)
            .map_err(|e| Error::Config(format!("{}: {e}", writer.config_path.display())))?;
        let mut report = Report::default();
        writer.apply_config(text, &mut report)?;
        writer.log(&report);
        Ok((Self { shared }, writer))
    }

    /// [`Persist::new`] plus a writer thread inserting every
    /// [`Settings::flush_interval`].
    pub fn start(schema_xml: &str, settings: Settings) -> Result<(Self, Handle), Error> {
        let interval = settings.flush_interval;
        let (persist, mut writer) = Self::new(schema_xml, settings)?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("persist".into())
            .spawn(move || {
                while !flag.load(Ordering::Relaxed) {
                    let started = Instant::now();
                    writer.tick();
                    std::thread::sleep(interval.saturating_sub(started.elapsed()));
                }
                writer.tick();
            })
            .map_err(|e| Error::Config(format!("cannot start writer thread: {e}")))?;
        Ok((
            persist,
            Handle {
                stop,
                thread: Some(thread),
            },
        ))
    }

    /// Is `template_id` recorded right now? One relaxed atomic load.
    #[inline]
    #[must_use]
    pub fn enabled(&self, template_id: u16) -> bool {
        self.shared
            .enabled
            .get(usize::from(template_id))
            .is_some_and(|e| e.load(Ordering::Relaxed))
    }

    /// Record one message of `template_id`. If its table is enabled, `encode`
    /// writes the message (header included) straight into persist's buffer
    /// and returns its length; otherwise `encode` is never called.
    ///
    /// A full buffer drops the record and counts it in [`Persist::dropped`].
    /// An error from `encode` is returned as is, and nothing is queued.
    #[inline]
    pub fn record<E>(
        &self,
        template_id: u16,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        if !self.enabled(template_id) {
            return Ok(());
        }
        // ponytail: one mutex, held for the encode. Uncontended it is one
        // atomic swap each way, and the writer only holds it to swap buffers.
        // Many hot threads recording at once would want a buffer each.
        let mut pending = self
            .shared
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let start = pending.len + 4;
        let free = pending.data.len().saturating_sub(start);
        if free < MAX_MESSAGE {
            drop(pending);
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        let len = encode(&mut pending.data[start..])?;
        debug_assert!(len <= free, "encode returned a length past its buffer");
        let len = len.min(free);
        let at = pending.len;
        pending.data[at..start].copy_from_slice(&(len as u32).to_le_bytes());
        pending.len = start + len;
        debug_assert_eq!(
            pending.data[start + 2..start + 4],
            template_id.to_le_bytes(),
            "recorded a message under another template id"
        );
        Ok(())
    }

    /// Records dropped so far because the buffer was full.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }
}

/// Owns the writer thread; stopping (or dropping) it flushes what is queued.
pub struct Handle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Handle {
    /// Stop the writer after a final flush.
    pub fn stop(mut self) {
        self.join();
    }

    fn join(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.join();
    }
}

/// What one [`Writer::tick`] did. Everything here is also logged.
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
    /// Tables whose `enabled` flag changed, with the new value.
    pub toggled: Vec<(String, bool)>,
}

struct TableState {
    table: Table,
    config: Option<TableConfig>,
    /// `Some` once the table exists and the writable columns are known.
    include: Option<Vec<bool>>,
    columns: Vec<String>,
    problems: Vec<String>,
    retry_at: Instant,
    rows: Vec<u8>,
    row_count: usize,
}

impl TableState {
    fn new(table: Table) -> Self {
        Self {
            table,
            config: None,
            include: None,
            columns: Vec::new(),
            problems: Vec::new(),
            retry_at: Instant::now(),
            rows: Vec::new(),
            row_count: 0,
        }
    }

    fn enabled(&self) -> bool {
        self.config.is_some_and(|c| c.enabled)
    }
}

/// Moves queued records into ClickHouse. Runs on the writer thread.
pub struct Writer {
    shared: Arc<Shared>,
    ch: ClickHouse,
    database_ready: bool,
    tables: Vec<TableState>,
    config_path: PathBuf,
    config_text: String,
    spare: Frames,
    max_retained: usize,
    recheck: Duration,
    totals: BTreeMap<String, u64>,
    last_summary: Instant,
    recent_errors: BTreeMap<String, Instant>,
}

impl Writer {
    /// Reload `tables.yaml` if it changed, sync tables, insert queued records.
    pub fn tick(&mut self) -> Report {
        let mut report = Report::default();
        match std::fs::read_to_string(&self.config_path) {
            Ok(text) if text != self.config_text => {
                if let Err(e) = self.apply_config(text, &mut report) {
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
        let ready = self.sync_tables(&mut report);
        if ready {
            self.drain(&mut report);
        }
        self.flush(&mut report);
        self.log(&report);
        report
    }

    fn apply_config(&mut self, text: String, report: &mut Report) -> Result<(), Error> {
        let mut config = parse_config(&text)?;
        for state in &mut self.tables {
            let new = config.remove(&state.table.name);
            if new.map(|c| c.kind) != state.config.map(|c| c.kind) {
                state.include = None; // kind changed: compare with ClickHouse again
                state.retry_at = Instant::now();
            }
            let enabled = new.is_some_and(|c| c.enabled);
            if enabled != state.enabled() || (state.config.is_none() && new.is_some()) {
                report.toggled.push((state.table.name.clone(), enabled));
            }
            state.config = new;
            if let Some(slot) = self
                .shared
                .enabled
                .get(usize::from(state.table.template_id))
            {
                slot.store(enabled, Ordering::Relaxed);
            }
        }
        for unknown in config.keys() {
            report.errors.push(format!(
                "tables.yaml names `{unknown}`, which is not a message in the schema"
            ));
        }
        self.config_text = text;
        Ok(())
    }

    /// Create/compare every table named in `tables.yaml`, enabled or not, so
    /// a disabled table exists (empty) and queries against it still work.
    /// `false` while ClickHouse cannot be reached, so queued records stay
    /// queued instead of being decoded.
    fn sync_tables(&mut self, report: &mut Report) -> bool {
        if !self.database_ready {
            let sql = format!("CREATE DATABASE IF NOT EXISTS `{}`", self.ch.database);
            match self.ch.query(&sql) {
                Ok(_) => self.database_ready = true,
                Err(e) => {
                    report.errors.push(e.to_string());
                    return false;
                }
            }
        }
        let now = Instant::now();
        let mut ready = true;
        for state in &mut self.tables {
            let Some(config) = state.config else {
                continue;
            };
            // Tables with outstanding problems are re-checked, so running the
            // suggested ALTER is picked up without a restart.
            let due = state.include.is_none() || !state.problems.is_empty();
            if due && now >= state.retry_at && state.rows.is_empty() {
                match self.ch.sync(&state.table, config.kind) {
                    Ok(sync) => {
                        report.applied.extend(sync.applied);
                        if sync.problems.is_empty() && !state.problems.is_empty() {
                            log::info!("{}: fixed, writing every column", state.table.name);
                        }
                        if sync.problems != state.problems {
                            report.problems.extend(
                                sync.problems
                                    .iter()
                                    .map(|p| format!("{}: {p}", state.table.name)),
                            );
                        }
                        state.columns = state
                            .table
                            .columns()
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
                        report.errors.push(format!("{}: {e}", state.table.name));
                        state.retry_at = now + Duration::from_secs(5);
                    }
                }
            }
            // An enabled table whose columns are not known yet (never synced,
            // or in its retry back-off) cannot take rows: keep everything queued.
            // ponytail: one table that never syncs holds back every table until
            // the buffer fills; give each table its own queue if that matters.
            ready &= state.include.is_some() || !config.enabled;
        }
        ready
    }

    fn drain(&mut self, report: &mut Report) {
        {
            let mut pending = self
                .shared
                .pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            std::mem::swap(&mut *pending, &mut self.spare);
        }
        let frames = &self.spare.data[..self.spare.len];
        let mut at = 0;
        while let Some(len) = frames
            .get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
        {
            let msg = &frames[at + 4..at + 4 + len as usize];
            at += 4 + len as usize;
            let template = u16::from_le_bytes([msg[2], msg[3]]);
            let Some(state) = self
                .tables
                .iter_mut()
                .find(|s| s.table.template_id == template)
            else {
                continue;
            };
            let Some(include) = &state.include else {
                // Its table could not be synced (only possible for a table
                // that was just disabled); nowhere to write it.
                self.shared.dropped.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            match state.table.write_row(msg, include, &mut state.rows) {
                Ok(()) => state.row_count += 1,
                Err(e) => report.errors.push(format!(
                    "{}: undecodable message skipped: {}",
                    state.table.name, e.0
                )),
            }
        }
        self.spare.len = 0;
    }

    fn flush(&mut self, report: &mut Report) {
        let mut retained = 0;
        for state in &mut self.tables {
            if state.row_count == 0 {
                continue;
            }
            let columns: Vec<&str> = state.columns.iter().map(String::as_str).collect();
            match self.ch.insert(&state.table.name, &columns, &state.rows) {
                Ok(()) => {
                    report
                        .inserted
                        .insert(state.table.name.clone(), state.row_count);
                    *self.totals.entry(state.table.name.clone()).or_default() +=
                        state.row_count as u64;
                    state.rows.clear();
                    state.row_count = 0;
                }
                Err(e) => {
                    report.errors.push(format!(
                        "{}: insert failed, keeping the rows to retry: {e}",
                        state.table.name
                    ));
                    retained += state.rows.len();
                    if retained > self.max_retained {
                        report.errors.push(format!(
                            "{}: dropping {} rows, retry buffer full",
                            state.table.name, state.row_count
                        ));
                        self.shared
                            .dropped
                            .fetch_add(state.row_count as u64, Ordering::Relaxed);
                        state.rows.clear();
                        state.row_count = 0;
                    }
                }
            }
        }
    }

    fn log(&mut self, report: &Report) {
        for (table, on) in &report.toggled {
            log::info!("recording {table}: {}", if *on { "on" } else { "off" });
        }
        for ddl in &report.applied {
            log::info!("applied: {ddl}");
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
            log::info!(
                "rows written so far: {:?}; dropped: {}",
                self.totals,
                self.shared.dropped.load(Ordering::Relaxed)
            );
        }
    }
}
