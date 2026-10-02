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
//! * [`persist`]: rows recorded for the ingester, which writes ClickHouse;
//!   [`event`] for rows from `tracing` events.
//! * [`mod@metrics`], [`trace`], [`clock`] and [`idle`]: the hot-path tools.
//! * [`source`]: who recorded a row.

pub mod app;
pub mod bus;
pub mod clock;
pub mod event;
pub mod idle;
pub mod metrics;
pub mod persist;
pub mod publication;
pub mod source;
mod spans;
pub mod streams;
pub mod subscription;
pub mod trace;
mod value;

use std::path::PathBuf;
use std::time::Duration;

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
            ..Self::new(
                std::env::var("PERSIST_CONFIG").unwrap_or_else(|_| "config/tables.yaml".into()),
            )
        }
    }
}
