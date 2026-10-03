//! What a lab application runs on, one module per concern:
//!
//! * [`app`]: bring a process up: logging, the [`Bus`](bus::Bus), the
//!   [`Persist`](persist::Persist) handle, the feed registry, the idle
//!   strategy, and SIGTERM.
//! * [`bus`]: the Aeron client, the application's identity on it, its drop
//!   counters, and shutdown.
//! * [`publication`] and [`subscription`]: feeds published and taken,
//!   including a persistent subscription that catches up from the archive.
//! * [`streams`]: the feed registry (`streams.yaml`).
//! * [`persist`]: rows recorded for the ingester, which writes `ClickHouse`;
//!   [`event`] for rows from `tracing` events.
//! * [`mod@metrics`], [`trace`], [`clock`], [`timer`] and [`idle`]: the hot-path tools.
//! * [`source`]: who recorded a row.
//! * [`rt`]: the runtime that owns the loop: an [`rt::Agent`] on one thread,
//!   live or on simulated time.

#[cfg(feature = "mimalloc")]
pub mod alloc_stats;
pub mod app;
pub mod bus;
pub mod clock;
pub mod event;
pub mod frames;
pub mod idle;
pub mod metrics;
mod os;
mod owned;
pub mod persist;
pub mod publication;
pub mod rt;
pub mod source;
mod spans;
pub mod streams;
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

/// Everything that can go wrong outside the recording hot path.
#[derive(Debug)]
pub enum Error {
    /// The SBE schema cannot be read.
    Schema(String),
    /// `tables.yaml` is missing or invalid.
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
            Self::Config(m) => write!(f, "tables.yaml: {m}"),
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

/// Where to publish and what to read.
#[derive(Clone, Debug)]
pub struct Settings {
    /// `tables.yaml`, re-read every second by
    /// [`Persist::poll`](persist::Persist::poll).
    pub config_path: PathBuf,
    /// This application's name: its entry under a table's `apps` switches
    /// that table for it alone. With [`Settings::host`] and [`Settings::pod`],
    /// it is written into every row the application records.
    pub app: String,
    /// The machine (in Kubernetes, the node).
    pub host: String,
    /// The pod, or the process's name outside Kubernetes.
    pub pod: String,
    /// This node's IP: the feeds this application publishes bind it, and the
    /// ones it subscribes to are taken from it. `HOST_IP` (from the downward
    /// API in Kubernetes).
    pub host_ip: String,
    /// The media driver's directory; `None` uses `AERON_DIR` or Aeron's default.
    pub aeron_dir: Option<String>,
    /// Run the Aeron client conductor in the application's loop
    /// ([`bus::Bus::do_work`]) rather than on its own thread, which would
    /// contend for a pinned core. `AERON_INVOKER`; the runtime's default.
    pub aeron_invoker: bool,
    /// Record on an exclusive publication (no CAS on the term tail) owned by
    /// the thread that connects: an application whose records come from one
    /// thread. Records from other threads (a library's `tracing` spans) are
    /// handed to it and published by its [`Persist::poll`](persist::Persist::poll).
    pub exclusive: bool,
    /// Defaults to [`persist::CHANNEL`].
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
            host: String::new(),
            pod: String::new(),
            host_ip: "127.0.0.1".into(),
            aeron_dir: None,
            aeron_invoker: false,
            exclusive: false,
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
            host: source::host_name(),
            pod: std::env::var("POD_NAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_default(),
            host_ip: std::env::var("HOST_IP").unwrap_or_else(|_| "127.0.0.1".into()),
            metrics_interval: duration("PERSIST_METRICS_INTERVAL", Duration::from_secs(5)),
            subscriber_timeout: duration("PERSIST_SUBSCRIBER_TIMEOUT", Duration::from_secs(10)),
            aeron_invoker: std::env::var("AERON_INVOKER").is_ok_and(|v| v == "1" || v == "true"),
            ..Self::new(
                std::env::var("PERSIST_CONFIG").unwrap_or_else(|_| "config/tables.yaml".into()),
            )
        }
    }
}
