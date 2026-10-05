//! What a lab application runs on, one module per concern:
//!
//! * [`app`]: bring a process up: logging, the [`Bus`](bus::Bus), the
//!   [`Persist`](persist::Persist) handle, the region, the idle strategy,
//!   and SIGTERM.
//! * [`bus`]: the Aeron client, driven from the loop, the application's
//!   identity on it, and its feeds' drop counters.
//! * [`publication`] and [`subscription`]: feeds published and taken,
//!   including a persistent subscription that catches up from the archive.
//! * [`directory`]: how an application's feed names map to Aeron; the
//!   application supplies the mapping.
//! * [`persist`]: rows recorded for the ingester, which writes `ClickHouse`,
//!   from the loop; [`event`] for rows of tables with no SBE message.
//! * [`bridge`]: `tracing` events and spans into the same stream, from any
//!   thread, on a publication of their own.
//! * [`mod@metrics`], [`trace`], [`clock`], [`timer`] and [`idle`]: the hot-path tools.
//! * [`source`]: who recorded a row.
//! * [`rt`]: the runtime that owns the loop: an [`rt::Agent`] on one thread,
//!   live or on simulated time.

#[cfg(feature = "mimalloc")]
pub mod alloc_stats;
pub mod app;
pub mod bridge;
pub mod bus;
#[cfg(feature = "clickhouse")]
pub mod clickhouse;
#[cfg(feature = "clickhouse")]
pub mod clickhouse_source;
pub mod clock;
pub mod directory;
pub mod event;
pub mod frames;
pub mod idle;
pub mod journal;
pub mod metrics;
#[cfg(test)]
#[macro_use]
mod not_send;
mod os;
pub mod persist;
pub mod publication;
pub mod rt;
pub mod source;
pub mod subscription;
pub mod timer;
pub mod trace;
mod value;

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::path::PathBuf;
use std::time::Duration;

/// A `HashMap` with a fixed hasher: the same keys iterate in the same order on
/// every run, which `RandomState` does not. For agent state a backtest must
/// reproduce.
pub type DetMap<K, V> = HashMap<K, V, BuildHasherDefault<DefaultHasher>>;

// The client's handles stay on the loop's thread: this fails to compile when
// one of them becomes `Send`.
#[cfg(test)]
assert_not_send!(
    persist::Persist,
    trace::Tracer,
    metrics::Metrics,
    metrics::Counter,
    metrics::Gauge,
    metrics::Histogram,
    bus::Bus,
    rt::Ctx,
    rt::Invoker,
    rt::Runtime,
    publication::Publication,
);

/// Everything that can go wrong outside the recording hot path.
#[derive(Debug)]
pub enum Error {
    /// The SBE schema cannot be read.
    Schema(String),
    /// A configuration is missing or invalid: `tables.yaml`, a timer, a
    /// simulation's inputs, or the application's directory.
    Config(String),
    /// The media driver is unreachable or refused a request.
    Aeron(String),
    /// A source, metric, or trace dictionary message could not be encoded.
    Encode(event::codec::sbe_rt::EncodeError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(m) => write!(f, "schema: {m}"),
            Self::Config(m) => write!(f, "config: {m}"),
            Self::Aeron(m) => write!(f, "aeron: {m}"),
            Self::Encode(err) => write!(f, "encode: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encode(err) => Some(err),
            Self::Schema(_) | Self::Config(_) | Self::Aeron(_) => None,
        }
    }
}

impl From<event::EncodeError> for Error {
    fn from(err: event::EncodeError) -> Self {
        Self::Encode(err)
    }
}

/// The region of an application whose deployment names none.
pub const UNKNOWN_REGION: &str = "unknown";

/// Where to publish and what to read.
#[derive(Clone, Debug)]
pub struct Settings {
    /// `tables.yaml`, checked every second by
    /// [`Persist::poll`](persist::Persist::poll), and read again when it
    /// changed.
    pub config_path: PathBuf,
    /// This application's name: its entry under a table's `apps` switches
    /// that table for it alone. With [`Settings::host`] and [`Settings::pod`],
    /// it is written into every row the application records.
    pub app: String,
    /// Backtest run id, empty for live.
    pub run: String,
    /// Simulated connection epoch, when running a backtest.
    pub sim_start: Option<clock::Nanos>,
    /// The machine (in Kubernetes, the node).
    pub host: String,
    /// The pod, or the process's name outside Kubernetes.
    pub pod: String,
    /// This node's IP: the feeds this application publishes bind it, and the
    /// ones it subscribes to are taken from it. `HOST_IP` (from the downward
    /// API in Kubernetes).
    pub host_ip: String,
    /// Where this application runs, as the deployment names it: `REGION`,
    /// [`UNKNOWN_REGION`] when unset. Agents read it as [`rt::Ctx::region`].
    pub region: String,
    /// The media driver's directory; `None` uses `AERON_DIR` or Aeron's default.
    pub aeron_dir: Option<String>,
    /// Defaults to [`persist::CHANNEL`], on which persist records on an
    /// exclusive publication of its own.
    pub channel: String,
    /// Defaults to [`persist::STREAM_ID`].
    pub stream_id: i32,
    /// How often [`Persist::poll`](persist::Persist::poll) publishes (default 5 s), at
    /// multiples of it in UNIX time.
    pub metrics_interval: Duration,
    /// How long [`Persist::connect`](persist::Persist::connect) waits for a subscriber to record the
    /// stream. [`Duration::ZERO`] returns as soon as the publication exists;
    /// records until a subscriber arrives are dropped.
    pub subscriber_timeout: Duration,
}

impl Settings {
    /// The default channel and stream, and the default media driver.
    #[must_use]
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
            app: String::new(),
            run: String::new(),
            sim_start: None,
            host: String::new(),
            pod: String::new(),
            host_ip: "127.0.0.1".into(),
            region: UNKNOWN_REGION.into(),
            aeron_dir: None,
            channel: persist::CHANNEL.to_string(),
            stream_id: persist::STREAM_ID,
            subscriber_timeout: Duration::from_secs(10),
            metrics_interval: Duration::from_secs(5),
        }
    }

    /// [`Settings::new`] with `PERSIST_CONFIG` (`config/tables.yaml`),
    /// `PERSIST_APP` (the app's name, default none), the host from
    /// [`source::host_name`], the pod from `POD_NAME`, else `HOSTNAME`
    /// (which Kubernetes sets to the pod's name), and the durations
    /// `PERSIST_METRICS_INTERVAL` and `PERSIST_SUBSCRIBER_TIMEOUT` (`5s`).
    #[must_use]
    pub fn from_env() -> Self {
        let duration = |var, default| {
            std::env::var(var)
                .ok()
                .and_then(|v| v.parse::<jiff::SignedDuration>().ok())
                .and_then(|d| Duration::try_from(d).ok())
                .unwrap_or(default)
        };
        Self {
            app: std::env::var("PERSIST_APP").unwrap_or_default(),
            run: std::env::var("PERSIST_RUN").unwrap_or_default(),
            host: source::host_name(),
            pod: std::env::var("POD_NAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_default(),
            host_ip: std::env::var("HOST_IP").unwrap_or_else(|_| "127.0.0.1".into()),
            region: std::env::var("REGION")
                .ok()
                .filter(|r| !r.is_empty())
                .unwrap_or_else(|| UNKNOWN_REGION.into()),
            metrics_interval: duration("PERSIST_METRICS_INTERVAL", Duration::from_secs(5)),
            subscriber_timeout: duration("PERSIST_SUBSCRIBER_TIMEOUT", Duration::from_secs(10)),
            ..Self::new(
                std::env::var("PERSIST_CONFIG").unwrap_or_else(|_| "config/tables.yaml".into()),
            )
        }
    }
}
