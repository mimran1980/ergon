//! Record SBE messages into ClickHouse.
//!
//! The SBE schema is the table definition: every message is a table and every
//! field a column (see `table.rs`). The application sizes a message with its
//! generated `compute_length_with_header`, and [`Persist::record`] hands the
//! encoder a slot of exactly that length in persist's buffer. A writer thread
//! batches the buffer into `INSERT … FORMAT RowBinary`. `on_trade` in
//! `recorder/src/main.rs` and `examples/record_latency.rs` show the calls.
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
//! A failed insert re-checks its table first, so a table altered or dropped
//! while recording is reported (static) or fixed (dynamic, or recreated) and
//! the records are retried. See [`Settings::max_buffered_bytes`] for how much
//! is held meanwhile; there is no disk spool.

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
    /// The writer thread could not be started.
    Thread(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(m) => write!(f, "schema: {m}"),
            Self::ClickHouse(m) => write!(f, "clickhouse: {m}"),
            Self::Config(m) => write!(f, "tables.yaml: {m}"),
            Self::Thread(m) => write!(f, "writer thread: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// Whether persistence may change a table's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TableKind {
    /// Created if missing, never altered.
    Static,
    /// Created and extended to follow the schema.
    Dynamic,
}

/// One entry of `tables.yaml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TableConfig {
    /// Static or dynamic.
    pub(crate) kind: TableKind,
    /// Record this table now (default `true`).
    #[serde(default = "yes")]
    pub(crate) enabled: bool,
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

/// Where to write, what to read, and how much to hold.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Target server and database.
    pub clickhouse: ClickHouse,
    /// `tables.yaml`, re-read while running.
    pub config_path: PathBuf,
    /// The record buffer's size. Two are allocated up front (the writer swaps
    /// them), and records waiting for a retry take at most as much again, so
    /// memory never exceeds three times this. When it is all in use, new
    /// records are dropped and counted; queued ones are kept.
    pub max_buffered_bytes: usize,
    /// How often the writer inserts. ClickHouse wants about one insert per
    /// table per second; more often only makes more parts to merge.
    pub flush_interval: Duration,
    /// How often a table with unfixed problems is compared again, so running
    /// the logged `ALTER` takes effect without a restart.
    pub recheck: Duration,
}

impl Settings {
    /// Defaults: 64 MiB buffer, 1 s flush, 30 s recheck.
    #[must_use]
    pub fn new(clickhouse: ClickHouse, config_path: impl Into<PathBuf>) -> Self {
        Self {
            clickhouse,
            config_path: config_path.into(),
            max_buffered_bytes: 64 << 20,
            flush_interval: Duration::from_secs(1),
            recheck: Duration::from_secs(30),
        }
    }

    /// [`Settings::new`] from `CLICKHOUSE_URL` (`http://localhost:8123`),
    /// `CLICKHOUSE_USER` (`lab`), `CLICKHOUSE_PASSWORD` (`lab`),
    /// `CLICKHOUSE_DATABASE` (`market`) and `PERSIST_CONFIG`
    /// (`config/tables.yaml`).
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
        )
    }
}

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

/// The messages in a buffer of `u32` LE length-prefixed messages.
fn messages(mut rest: &[u8]) -> impl Iterator<Item = &[u8]> {
    std::iter::from_fn(move || {
        let (len, tail) = rest.split_first_chunk::<4>()?;
        let (message, tail) = tail.split_at_checked(u32::from_le_bytes(*len) as usize)?;
        rest = tail;
        Some(message)
    })
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
            rows: Vec::new(),
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
            .map_err(|e| Error::Thread(e.to_string()))?;
        Ok((
            persist,
            Handle {
                stop,
                thread: Some(thread),
            },
        ))
    }

    /// Is `template_id` recorded right now? One relaxed atomic load. Check it
    /// before preparing a message that is costly to size or encode.
    #[inline]
    #[must_use]
    pub fn enabled(&self, template_id: u16) -> bool {
        self.shared
            .enabled
            .get(usize::from(template_id))
            .is_some_and(|e| e.load(Ordering::Relaxed))
    }

    /// Record one message of `template_id` that is exactly `len` bytes,
    /// header included (the generated `compute_length_with_header`). If its
    /// table is enabled, `encode` writes the message into a slot of exactly
    /// `len` bytes and returns the length it wrote; otherwise `encode` is
    /// never called.
    ///
    /// The record is dropped and counted in [`Persist::dropped`] when the
    /// buffer has no room for it, or when `encode` wrote another length or
    /// template than claimed (debug builds panic on that). An error from
    /// `encode` is returned as is, and nothing is queued.
    #[inline]
    pub fn record<E>(
        &self,
        template_id: u16,
        len: usize,
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
        let at = pending.len;
        let start = at + 4;
        let end = start + len;
        let fits = len >= table::HEADER_LEN && end <= pending.data.len();
        let (Ok(prefix), true) = (u32::try_from(len), fits) else {
            self.drop_one();
            return Ok(());
        };
        let slot = &mut pending.data[start..end];
        let written = encode(slot)?;
        let honest = written == len && slot[2..4] == template_id.to_le_bytes();
        debug_assert!(
            honest,
            "encode wrote {written} bytes of template {}, but claimed {len} bytes of {template_id}",
            u16::from_le_bytes([slot[2], slot[3]])
        );
        if honest {
            pending.data[at..start].copy_from_slice(&prefix.to_le_bytes());
            pending.len = end;
        } else {
            self.drop_one();
        }
        Ok(())
    }

    fn drop_one(&self) {
        self.shared.dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Records dropped so far: no room in the buffer, a mis-sized encode, or
    /// a table removed from `tables.yaml` while its records were queued.
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
        if self.thread.take().is_some_and(|t| t.join().is_err()) {
            log::error!("the persist writer thread panicked; records since then were not written");
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
    /// SBE messages not yet inserted, length-prefixed. Kept as SBE rather
    /// than rows so they can be decoded again if the table changes.
    queued: Vec<u8>,
    queued_count: usize,
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
            queued: Vec::new(),
            queued_count: 0,
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
    /// RowBinary for the insert in progress; reused.
    rows: Vec<u8>,
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
    /// `false` while an enabled table's columns are unknown, so records stay
    /// in the record buffer instead of being taken for a table that cannot
    /// accept them.
    fn sync_tables(&mut self, report: &mut Report) -> bool {
        if !self.database_ready {
            match self.ch.create_database() {
                Ok(()) => self.database_ready = true,
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
            if due && now >= state.retry_at {
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
            // ponytail: one table that never syncs holds back every table until
            // the buffer fills; give each table its own queue if that matters.
            ready &= state.include.is_some() || !config.enabled;
        }
        ready
    }

    /// Move the record buffer's messages to their tables' queues, unless
    /// those queues are full: then they wait, and `record` drops new ones
    /// once the buffer fills.
    fn drain(&mut self, report: &mut Report) {
        let queued: usize = self.tables.iter().map(|s| s.queued.len()).sum();
        {
            let mut pending = self
                .shared
                .pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if queued + pending.len > self.max_retained {
                report.errors.push(format!(
                    "{queued} bytes of records are waiting for ClickHouse; holding new records"
                ));
                return;
            }
            std::mem::swap(&mut *pending, &mut self.spare);
        }
        for message in messages(&self.spare.data[..self.spare.len]) {
            let template = u16::from_le_bytes([message[2], message[3]]);
            match self
                .tables
                .iter_mut()
                .find(|s| s.table.template_id == template)
            {
                Some(state) if state.config.is_some() => {
                    let len = message.len() as u32;
                    state.queued.extend_from_slice(&len.to_le_bytes());
                    state.queued.extend_from_slice(message);
                    state.queued_count += 1;
                }
                // Removed from tables.yaml after it was recorded.
                _ => {
                    self.shared.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        self.spare.len = 0;
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
            let mut rows = 0;
            for message in messages(&state.queued) {
                match state.table.write_row(message, include, &mut self.rows) {
                    Ok(()) => rows += 1,
                    Err(e) => report.errors.push(format!(
                        "{}: undecodable message skipped: {}",
                        state.table.name, e.0
                    )),
                }
            }
            let columns: Vec<&str> = state.columns.iter().map(String::as_str).collect();
            let inserted = match rows {
                0 => Ok(()),
                _ => self.ch.insert(&state.table.name, &columns, &self.rows),
            };
            match inserted {
                Ok(()) => {
                    if rows > 0 {
                        report.inserted.insert(state.table.name.clone(), rows);
                        *self.totals.entry(state.table.name.clone()).or_default() += rows as u64;
                    }
                    state.queued.clear();
                    state.queued_count = 0;
                }
                Err(e) => {
                    report.errors.push(format!(
                        "{}: insert failed, keeping {} records to retry: {e}",
                        state.table.name, state.queued_count
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
