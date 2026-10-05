//! What a busy-loop application starts with: logging, the bus, the persist
//! handle, its idle strategy, and SIGTERM.
//!
//! Each is the loop's to own: [`App`]'s fields go into the runtime's
//! [`Config`](crate::rt::Config), which takes its region from the bus, drives
//! the bus's conductor, reads the SIGTERM pipe, and at shutdown closes what
//! the application publishes.

use crate::Settings;
use crate::bus::Bus;
use crate::idle::Idle;
use crate::persist::Persist;
use crate::rt::{Stop, sigterm};

/// Why [`App::start_with`] could not bring a process up.
#[derive(Debug)]
pub enum Error {
    /// The bus or persist connection failed.
    Runtime(crate::Error),
    /// `SIGTERM` could not be registered.
    Signal(std::io::Error),
    /// `IDLE` or `TIMER_SLACK` does not parse ([`Idle::parse`]).
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

/// A starting application's handles.
pub struct App {
    /// The Aeron client and this application's identity on it.
    pub bus: Bus,
    /// Rows, metrics, and traces recorded for the ingester.
    pub persist: Persist,
    /// What the loop does when an iteration found no work.
    pub idle: Idle,
    /// SIGTERM's pipe: the loop that reads it stops.
    pub stop: Stop,
}

impl App {
    /// Logging, the bus and the persist handle for `schema`, the idle
    /// strategy (`IDLE`, else `idle`), and `SIGTERM`.
    ///
    /// Persist records on an exclusive publication of its own, and is
    /// neither `Send` nor `Sync`: the loop runs on the calling thread.
    ///
    /// # Errors
    ///
    /// The bus or persist connection, `IDLE`, or registering `SIGTERM`
    /// failed.
    pub fn start_with(schema: &str, idle: Idle) -> Result<Self, Error> {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
        let settings = Settings::from_env();
        let bus = Bus::connect(&settings)?;
        let persist = Persist::connect(schema, &bus, settings)?;
        Ok(Self {
            bus,
            persist,
            // The caller's default (spin for `Invoker::from_env`) unless
            // `IDLE` names another.
            idle: Idle::from_env("IDLE", idle).map_err(Error::Idle)?,
            stop: sigterm().map_err(Error::Signal)?,
        })
    }
}
