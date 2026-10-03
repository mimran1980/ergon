//! What a busy-loop application starts with: logging, the bus, the persist
//! handle, the feed registry, its idle strategy, and SIGTERM.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::Settings;
use crate::bus::Bus;
use crate::idle::Idle;
use crate::metrics::Metrics;
use crate::persist::Persist;
use crate::streams::Streams;

/// Why [`App::start`] could not bring a process up.
#[derive(Debug)]
pub enum Error {
    /// The node check, the bus or persist connection, or the stream
    /// registry failed.
    Runtime(crate::Error),
    /// `SIGTERM` could not be registered.
    Signal(std::io::Error),
    /// `IDLE` is not `spin`, `noop`, `yield`, or `sleep`.
    Idle(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(err) => write!(f, "{err}"),
            Self::Signal(err) => write!(f, "SIGTERM: {err}"),
            Self::Idle(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(err) => Some(err),
            Self::Signal(err) => Some(err),
            Self::Idle(_) => None,
        }
    }
}

impl From<crate::Error> for Error {
    fn from(err: crate::Error) -> Self {
        Self::Runtime(err)
    }
}

/// A running application's handles.
pub struct App {
    /// The Aeron client and this application's identity on it.
    pub bus: Bus,
    /// Rows, metrics, and traces recorded for the ingester.
    pub persist: Persist,
    /// The feed registry this process loaded.
    pub streams: Streams,
    /// Where `streams` came from, to follow its changes.
    pub streams_path: String,
    /// `REGION`: which `md-*` feeds, and whose engine and exchange.
    pub region: String,
    /// What the loop does when an iteration found no work.
    pub idle: Idle,
    /// Set on SIGTERM: close the feeds and exit.
    pub stop: Arc<AtomicBool>,
}

impl App {
    /// Logging, the bus and the persist handle for `schema`, the feed
    /// registry, the idle strategy, and `SIGTERM`.
    ///
    /// # Errors
    ///
    /// The node check, the bus or persist connection, the stream file,
    /// `IDLE`, or registering `SIGTERM` failed.
    pub fn start(schema: &str) -> Result<Self, Error> {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
        crate::streams::check_node_network()?;
        let settings = Settings::from_env();
        let bus = Bus::connect(&settings)?;
        let persist = Persist::connect(schema, &bus, settings)?;
        let stop = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop))
            .map_err(Error::Signal)?;
        let streams_path =
            std::env::var("PERSIST_STREAMS").unwrap_or_else(|_| "config/streams.yaml".into());
        Ok(Self {
            bus,
            persist,
            streams: Streams::load(&streams_path)?,
            streams_path,
            region: std::env::var("REGION").unwrap_or_else(|_| "an1".into()),
            // Lab default: yield. For the best latency, spin (or noop) on an
            // isolated core.
            idle: Idle::from_env("IDLE", Idle::Yield).map_err(Error::Idle)?,
            stop,
        })
    }

    /// This application's metrics; see [`Persist::metrics`].
    #[must_use]
    pub fn metrics(&self) -> Metrics {
        self.persist.metrics()
    }

    /// SIGTERM arrived: the feeds are closed, so subscribers turn to this
    /// service's next pod at once.
    #[must_use]
    pub fn stopping(&self) -> bool {
        if self.stop.load(Ordering::Relaxed) {
            log::info!("SIGTERM: closing the feeds");
            self.bus.shutdown();
            return true;
        }
        false
    }
}
