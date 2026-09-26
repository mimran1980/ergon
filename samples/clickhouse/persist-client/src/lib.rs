//! Record data for ClickHouse: the application side.
//!
//! Two ways in, onto one Aeron stream:
//!
//! * [`Persist::record`] encodes an SBE message straight into the Aeron
//!   publication. Its table is the SBE message.
//! * [`Persist::layer`] records `tracing` events that name a table, e.g.
//!   `tracing::info!(table = "signal", instrument = %id, edge = 0.25)`. The
//!   table's columns are the events' fields (see [`event`]).
//!
//! A separate ingester (`persist-server`) reads the Aeron Archive recording of
//! that stream, inserts every message into the ClickHouse table its SBE
//! message defines, and purges the recording behind it. The application never
//! talks to ClickHouse and never waits for it.
//!
//! Hold a [`Persist`], or [`install`](Persist::install) one for the process
//! and record from anywhere with the free functions [`record`], [`enabled`],
//! [`record_row`] and [`event_enabled`], without passing a handle around.
//! Before one is installed, or when none ever is (a test, a tool), they do
//! nothing: `encode` is never called. Each costs one load more than holding
//! the handle.
//!
//! A disabled SBE table costs one relaxed atomic load; an enabled one a
//! `try_claim`, the encode and a commit. No lock, no allocation, no copy. A
//! disabled event table costs a read lock and a lookup, with no formatting.
//!
//! `config/tables.yaml` is re-read every second, so recording switches on and
//! off while the application runs. `enabled` is read here, `kind` by the
//! ingester:
//!
//! ```yaml
//! tables:
//!   trade:         { kind: static }
//!   book_snapshot: { kind: dynamic, enabled: false }
//! ```

pub mod event;
mod value;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use rusteron_client::{Aeron, AeronContext, AeronOfferError, AeronPublication, IntoCString};
use serde::Deserialize;
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::registry::LookupSpan;

/// The stream applications publish on and the ingester's archive records.
pub const STREAM_ID: i32 = 1001;

/// IPC through the one media driver the application, the archive and the
/// ingester share. The 64 KiB MTU fits a message of up to 65 472 bytes in
/// one `try_claim`.
pub const CHANNEL: &str = "aeron:ipc?term-length=16m|mtu=65504";

/// Everything that can go wrong outside the recording hot path.
#[derive(Debug)]
pub enum Error {
    /// The SBE schema cannot be read.
    Schema(String),
    /// `tables.yaml` is missing or invalid.
    Config(String),
    /// The media driver is unreachable or refused a request.
    Aeron(String),
    /// A background thread could not be started.
    Thread(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(m) => write!(f, "schema: {m}"),
            Self::Config(m) => write!(f, "tables.yaml: {m}"),
            Self::Aeron(m) => write!(f, "aeron: {m}"),
            Self::Thread(m) => write!(f, "thread: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// Whether the ingester may change a table's columns.
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
pub fn parse_config(text: &str) -> Result<BTreeMap<String, TableConfig>, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        tables: BTreeMap<String, TableConfig>,
    }
    serde_yaml::from_str::<File>(text)
        .map(|f| f.tables)
        .map_err(|e| Error::Config(e.to_string()))
}

/// Apply an override file to a parsed `tables.yaml`: it switches tables on
/// or off for one application, and names no table `tables.yaml` lacks.
///
/// ```yaml
/// tables:
///   book_deltas: { enabled: false }
/// ```
pub fn apply_overrides(
    config: &mut BTreeMap<String, TableConfig>,
    text: &str,
) -> Result<(), Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Override {
        enabled: bool,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        tables: BTreeMap<String, Override>,
    }
    let file: File = serde_yaml::from_str(text).map_err(|e| Error::Config(e.to_string()))?;
    for (name, o) in file.tables {
        config
            .get_mut(&name)
            .ok_or_else(|| Error::Config(format!("{name} is not in tables.yaml")))?
            .enabled = o.enabled;
    }
    Ok(())
}

/// `(table name, template id)` of every message in an SBE schema.
fn schema_tables(xml: &str) -> Result<Vec<(String, u16)>, Error> {
    let ir = ergo_sbe::parse(xml).map_err(|e| Error::Schema(e.to_string()))?;
    ir.tokens
        .iter()
        .filter(|t| t.signal == ergo_sbe::Signal::BeginMessage)
        .map(|t| match t.id {
            Some(id) => Ok((snake_case(&t.name), id)),
            None => Err(Error::Schema(format!("message {} has no id", t.name))),
        })
        .collect()
}

/// `bidPrice` -> `bid_price`, `BookSnapshot` -> `book_snapshot`.
#[must_use]
pub fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') && !out.ends_with('.') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Where to publish and what to read.
#[derive(Clone, Debug)]
pub struct Settings {
    /// `tables.yaml`, re-read every second.
    pub config_path: PathBuf,
    /// This application's own `enabled` values (see [`apply_overrides`]),
    /// re-read every second. A missing file overrides nothing.
    pub overrides_path: Option<PathBuf>,
    /// The media driver's directory; `None` uses `AERON_DIR` or Aeron's default.
    pub aeron_dir: Option<String>,
    /// Defaults to [`CHANNEL`].
    pub channel: String,
    /// Defaults to [`STREAM_ID`].
    pub stream_id: i32,
}

impl Settings {
    /// The default channel and stream, and the default media driver.
    #[must_use]
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
            overrides_path: None,
            aeron_dir: None,
            channel: CHANNEL.to_string(),
            stream_id: STREAM_ID,
        }
    }

    /// [`Settings::new`] with `PERSIST_CONFIG` (`config/tables.yaml`) and,
    /// if set, `PERSIST_OVERRIDES`.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            overrides_path: std::env::var_os("PERSIST_OVERRIDES").map(PathBuf::from),
            ..Self::new(
                std::env::var("PERSIST_CONFIG").unwrap_or_else(|_| "config/tables.yaml".into()),
            )
        }
    }
}

/// The handle the free functions use; see [`Persist::install`].
static INSTALLED: OnceLock<Persist> = OnceLock::new();

/// The handle [`Persist::install`] installed, if any.
#[inline]
#[must_use]
pub fn installed() -> Option<&'static Persist> {
    INSTALLED.get()
}

/// [`Persist::record`] with the installed handle. With none installed it
/// does nothing and never calls `encode`.
#[inline]
pub fn record<E>(
    template_id: u16,
    len: usize,
    encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
) -> Result<(), E> {
    match INSTALLED.get() {
        Some(persist) => persist.record(template_id, len, encode),
        None => Ok(()),
    }
}

/// [`Persist::enabled`] with the installed handle; `false` with none.
#[inline]
#[must_use]
pub fn enabled(template_id: u16) -> bool {
    INSTALLED.get().is_some_and(|p| p.enabled(template_id))
}

/// [`Persist::event_enabled`] with the installed handle; `false` with none.
#[inline]
#[must_use]
pub fn event_enabled(table: &str) -> bool {
    INSTALLED.get().is_some_and(|p| p.event_enabled(table))
}

/// [`Persist::record_row`] with the installed handle. With none installed it
/// does nothing.
pub fn record_row<'a>(table: &str, fields: impl IntoIterator<Item = (&'a str, event::Value<'a>)>) {
    if let Some(persist) = INSTALLED.get() {
        persist.record_row(table, fields);
    }
}

/// [`Persist::record_value`] with the installed handle. With none installed
/// it does nothing.
#[inline]
pub fn record_value<T: ?Sized + serde::Serialize>(table: &str, value: &T) {
    if let Some(persist) = INSTALLED.get() {
        persist.record_value(table, value);
    }
}

/// The recording handle. Cheap to clone; share it with every callback, or
/// [`install`](Persist::install) it once and use the free functions.
#[derive(Clone)]
pub struct Persist {
    inner: Arc<Inner>,
}

struct Inner {
    /// Unique per `Persist` ever made: keys the per-thread call-site cache,
    /// which an address could not (a new `Persist` may reuse an old one's).
    id: u64,
    publication: AeronPublication,
    _aeron: Aeron,
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    watcher: Option<JoinHandle<()>>,
}

/// State the config watcher writes and the hot path reads.
struct Shared {
    /// Indexed by SBE template id.
    enabled: Vec<AtomicBool>,
    /// Event tables' switches by name, including tables `tables.yaml` does
    /// not list (off). Call sites cache theirs, so the lock is taken when a
    /// call site first names a table, not per event.
    events: RwLock<HashMap<String, Arc<AtomicBool>>>,
    /// Every event shape made so far, by id: sent again every 5 s.
    shapes: Mutex<HashMap<u32, Arc<event::Shape>>>,
    dropped: AtomicU64,
}

impl Persist {
    /// Read the schema and `tables.yaml`, connect to the media driver, and
    /// start re-reading `tables.yaml` every second.
    pub fn connect(schema_xml: &str, settings: Settings) -> Result<Self, Error> {
        let schema = schema_tables(schema_xml)?;
        let slots = schema.iter().map(|(_, id)| usize::from(*id) + 1).max();
        let shared = Arc::new(Shared {
            enabled: (0..slots.unwrap_or(0))
                .map(|_| AtomicBool::new(false))
                .collect(),
            events: RwLock::new(HashMap::new()),
            shapes: Mutex::new(HashMap::new()),
            dropped: AtomicU64::new(0),
        });
        let (aeron, publication) =
            publish(&settings).map_err(|e| Error::Aeron(format!("{}: {e}", settings.channel)))?;
        let mut watcher = Watcher {
            publication: publication.clone(),
            ticks: 0,
            path: settings.config_path.clone(),
            overrides_path: settings.overrides_path.clone(),
            text: (String::new(), None),
            error: None,
            dropped: 0,
            schema,
            shared: Arc::clone(&shared),
        };
        watcher.reload()?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("persist-config".into())
            .spawn(move || {
                while !flag.load(Ordering::Relaxed) {
                    std::thread::park_timeout(Duration::from_secs(1));
                    watcher.tick();
                }
            })
            .map_err(|e| Error::Thread(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Inner {
                id: {
                    static NEXT: AtomicU64 = AtomicU64::new(0);
                    NEXT.fetch_add(1, Ordering::Relaxed)
                },
                publication,
                _aeron: aeron,
                shared,
                stop,
                watcher: Some(thread),
            }),
        })
    }

    /// Make this the process's handle, for [`record`], [`enabled`],
    /// [`record_row`] and [`event_enabled`] to use from anywhere. It stays
    /// installed until the process exits. Only the first call installs; a
    /// later one returns `false` and changes nothing.
    pub fn install(&self) -> bool {
        INSTALLED.set(self.clone()).is_ok()
    }

    /// Is `template_id` recorded right now? One relaxed atomic load. Check it
    /// before preparing a message that is costly to size or encode.
    #[inline]
    #[must_use]
    pub fn enabled(&self, template_id: u16) -> bool {
        self.inner
            .shared
            .enabled
            .get(usize::from(template_id))
            .is_some_and(|e| e.load(Ordering::Relaxed))
    }

    /// Record one message of `template_id` that is exactly `len` bytes,
    /// header included (the generated `compute_length_with_header`). If its
    /// table is enabled, `encode` writes the message into a slot of exactly
    /// `len` bytes of the Aeron term buffer and returns the length it wrote;
    /// otherwise `encode` is never called.
    ///
    /// The record is dropped and counted in [`Persist::dropped`] when Aeron
    /// cannot take it (no archive recording yet, back pressure, larger than
    /// the MTU), or when `encode` wrote another length or template than
    /// claimed (debug builds panic on that). An error from `encode` is
    /// returned as is, and nothing is published.
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
        let mut claim = loop {
            match self.inner.publication.try_claim_owned(len) {
                Ok(claim) => break claim,
                // Term rotation: Aeron asks for an immediate retry.
                Err(AeronOfferError::AdminAction) => {}
                Err(_) => {
                    self.drop_one();
                    return Ok(());
                }
            }
        };
        let slot = claim.data();
        // An `encode` error drops the claim, which aborts it.
        let written = encode(slot)?;
        let honest = written == len && slot.get(2..4) == Some(&template_id.to_le_bytes()[..]);
        debug_assert!(
            honest,
            "encode wrote {written} bytes of template {}, but claimed {len} bytes of {template_id}",
            u16::from_le_bytes([slot[2], slot[3]])
        );
        if !(honest && claim.commit().is_ok()) {
            self.drop_one();
        }
        Ok(())
    }

    /// A `tracing` layer that records events naming an enabled table:
    /// `tracing::info!(table = "signal", instrument = %id, edge = 0.25)`.
    /// Add it to the application's subscriber. Its filter is its own: it
    /// takes only events with a `table` field, and leaves every other
    /// callsite to the other layers (or disabled, when it is alone).
    #[must_use]
    pub fn layer<S>(&self) -> impl Layer<S> + Send + Sync + 'static
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        event::PersistLayer {
            persist: self.clone(),
        }
        .with_filter(filter_fn(|meta| {
            meta.fields().field(event::TABLE).is_some()
        }))
    }

    /// Is the event table `table` recorded right now? Check it before
    /// building a row that is costly to make.
    #[must_use]
    pub fn event_enabled(&self, table: &str) -> bool {
        let events = self
            .inner
            .shared
            .events
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        events
            .get(table)
            .is_some_and(|on| on.load(Ordering::Relaxed))
    }

    /// The switch of event table `table`, made (off) if `tables.yaml` does
    /// not list it; the config watcher flips it.
    pub(crate) fn event_switch(&self, table: &str) -> Arc<AtomicBool> {
        let events = &self.inner.shared.events;
        if let Some(on) = events
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(table)
        {
            return Arc::clone(on);
        }
        let mut events = events.write().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(events.entry(table.to_owned()).or_default())
    }

    /// Publish a message that is already built; `false` when Aeron could
    /// not take it. Not counted as dropped: that is the caller's decision.
    pub(crate) fn publish(&self, bytes: &[u8]) -> bool {
        publish_bytes(&self.inner.publication, bytes)
    }

    /// Claim exactly `len` bytes, let `write` fill them, and commit; or drop
    /// and count the record.
    pub(crate) fn claim(&self, len: usize, write: impl FnOnce(&mut [u8])) {
        let claimed = loop {
            match self.inner.publication.try_claim_owned(len) {
                Err(AeronOfferError::AdminAction) => {}
                other => break other,
            }
        };
        match claimed {
            Ok(mut claim) => {
                write(claim.data());
                if claim.commit().is_err() {
                    self.drop_one();
                }
            }
            Err(_) => self.drop_one(),
        }
    }

    /// Is the archive recording this stream yet? Until it is, every record
    /// is dropped and counted.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.inner.publication.is_connected()
    }

    #[cold]
    fn drop_one(&self) {
        self.inner.shared.dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Records dropped so far (see [`Persist::record`]). Also logged, once a
    /// second while it grows.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.inner.shared.dropped.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for Persist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Persist")
            .field("dropped", &self.dropped())
            .finish_non_exhaustive()
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.watcher.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

fn publish_bytes(publication: &AeronPublication, bytes: &[u8]) -> bool {
    let claimed = loop {
        match publication.try_claim_owned(bytes.len()) {
            Err(AeronOfferError::AdminAction) => {}
            other => break other,
        }
    };
    claimed.is_ok_and(|mut claim| {
        claim.data().copy_from_slice(bytes);
        claim.commit().is_ok()
    })
}

fn publish(settings: &Settings) -> Result<(Aeron, AeronPublication), rusteron_client::AeronCError> {
    let ctx = AeronContext::new()?;
    if let Some(dir) = &settings.aeron_dir {
        ctx.set_dir(&dir.as_str().into_c_string())?;
    }
    let aeron = Aeron::new(&ctx)?;
    aeron.start()?;
    let publication = aeron
        .async_add_publication(
            &settings.channel.as_str().into_c_string(),
            settings.stream_id,
        )?
        .poll_blocking(Duration::from_secs(10))?;
    Ok((aeron, publication))
}

/// Applies `tables.yaml` to [`Shared::enabled`]; runs on its own thread.
struct Watcher {
    /// For sending the event shapes again.
    publication: AeronPublication,
    ticks: u64,
    path: PathBuf,
    overrides_path: Option<PathBuf>,
    /// The last applied `tables.yaml` and override file.
    text: (String, Option<String>),
    /// The last error logged, so a broken file is reported once.
    error: Option<String>,
    dropped: u64,
    schema: Vec<(String, u16)>,
    shared: Arc<Shared>,
}

impl Watcher {
    fn tick(&mut self) {
        // Every 5 s, every event shape again: an ingester that starts after
        // the first one, with none saved, learns them from the stream.
        self.ticks += 1;
        if self.ticks.is_multiple_of(5) {
            let shapes: Vec<_> = self
                .shared
                .shapes
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .values()
                .cloned()
                .collect();
            for shape in shapes {
                publish_bytes(&self.publication, shape.message());
            }
        }
        match self.reload() {
            Ok(()) => self.error = None,
            Err(e) => {
                let e = format!("{e}; keeping the previous configuration");
                if self.error.as_ref() != Some(&e) {
                    log::error!("{e}");
                    self.error = Some(e);
                }
            }
        }
        let dropped = self.shared.dropped.load(Ordering::Relaxed);
        if dropped > self.dropped {
            log::warn!(
                "{} records dropped in the last second ({dropped} in all): Aeron could not take them",
                dropped - self.dropped
            );
            self.dropped = dropped;
        }
    }

    /// Apply `tables.yaml` and the override file if either changed since
    /// the last call.
    fn reload(&mut self) -> Result<(), Error> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| Error::Config(format!("{}: {e}", self.path.display())))?;
        let overrides = match &self.overrides_path {
            Some(path) => match std::fs::read_to_string(path) {
                Ok(text) => Some(text),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(Error::Config(format!("{}: {e}", path.display()))),
            },
            None => None,
        };
        let text = (text, overrides);
        if text == self.text {
            return Ok(());
        }
        let mut config = parse_config(&text.0)?;
        if let (Some(overrides), Some(path)) = (&text.1, &self.overrides_path) {
            apply_overrides(&mut config, overrides)
                .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
        }
        let first = self.text.0.is_empty();
        let toggled = |name: &str, was: bool, on: bool| {
            if was != on || (first && on) {
                log::info!("recording {name}: {}", if on { "on" } else { "off" });
            }
        };
        for (name, id) in &self.schema {
            let on = config.get(name).is_some_and(|c| c.enabled);
            toggled(
                name,
                self.shared.enabled[usize::from(*id)].swap(on, Ordering::Relaxed),
                on,
            );
        }
        // Every other table is recorded from `tracing` events.
        let mut switches = self
            .shared
            .events
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        for (name, switch) in switches.iter() {
            if !config.contains_key(name) && switch.swap(false, Ordering::Relaxed) {
                toggled(name, true, false);
            }
        }
        for (name, c) in config
            .into_iter()
            .filter(|(name, _)| !self.schema.iter().any(|(n, _)| n == name))
        {
            let was = switches
                .entry(name.clone())
                .or_default()
                .swap(c.enabled, Ordering::Relaxed);
            toggled(&name, was, c.enabled);
        }
        drop(switches);
        self.text = text;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Nothing in this test binary installs a handle.
    #[test]
    fn without_an_installed_handle_recording_does_nothing() -> TestResult {
        assert!(installed().is_none());
        record(1, 64, |_| Err("encoded with nothing installed"))?;
        assert!(!enabled(1) && !event_enabled("spread"));
        record_row("spread", [("bps", event::Value::F64(1.0))]);
        Ok(())
    }

    #[test]
    fn overrides_switch_tables_for_one_application() -> TestResult {
        let mut config = parse_config(
            "tables:\n  trade: { kind: static }\n  book: { kind: dynamic, enabled: false }\n",
        )?;
        apply_overrides(
            &mut config,
            "tables:\n  trade: { enabled: false }\n  book: { enabled: true }\n",
        )?;
        assert!(!config["trade"].enabled && config["book"].enabled);
        assert_eq!(
            config["trade"].kind,
            TableKind::Static,
            "kind is not overridden"
        );
        assert!(
            apply_overrides(&mut config, "tables:\n  nope: { enabled: true }\n").is_err(),
            "a table tables.yaml lacks is an error, not a new table"
        );
        assert!(
            apply_overrides(
                &mut config,
                "tables:\n  trade: { kind: dynamic, enabled: true }\n"
            )
            .is_err(),
            "kind belongs to tables.yaml"
        );
        Ok(())
    }

    #[test]
    fn snake_case_names() -> TestResult {
        assert_eq!(snake_case("BookSnapshot"), "book_snapshot");
        assert_eq!(snake_case("bidPrice"), "bid_price");
        assert_eq!(snake_case("tsEvent"), "ts_event");
        assert_eq!(snake_case("price"), "price");
        Ok(())
    }
}
