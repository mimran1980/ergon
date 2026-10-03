//! Select the driver once, outside the live duty cycle.

use super::sim::{Sim, SimConfig};
use super::{Agent, Config, Ctx, Invoker};
use crate::Error;

/// The source of inputs and time for the same application.
pub enum Mode {
    /// Live Aeron feeds, hardware time, and exclusive publications.
    Live(Config),
    /// Recorded inputs and simulated time; optional wall-clock pacing or journal.
    Replay {
        /// Window, sources, pace and optional journal.
        config: SimConfig,
        /// File sources, encoded with [`crate::frames::FrameLog`].
        logs: Vec<Vec<u8>>,
    },
    /// Historical inputs, simulated time and optional venue loopback.
    Backtest {
        /// Sources, run persistence and loopback latencies.
        config: SimConfig,
        /// File sources, when running without infrastructure.
        logs: Vec<Vec<u8>>,
    },
}

impl From<Config> for Mode {
    fn from(config: Config) -> Self {
        Self::Live(config)
    }
}

enum Driver {
    Live(Box<Invoker>),
    Sim(Box<Sim>),
}

/// Owns a live, replay or backtest loop over the same [`Agent`].
/// Mode selection happens once in [`Runtime::run`]; the live loop uses
/// [`Invoker`] directly and contains no simulation dispatch branches.
pub struct Runtime {
    driver: Driver,
}

impl Runtime {
    /// Build a driver. A plain [`Config`] selects live mode for compatibility.
    ///
    /// # Errors
    /// Invalid configuration or an unavailable input source.
    pub fn new(mode: impl Into<Mode>) -> Result<Self, Error> {
        let driver = match mode.into() {
            Mode::Live(config) => Driver::Live(Box::new(Invoker::new(config)?)),
            Mode::Replay { config, logs } | Mode::Backtest { config, logs } => {
                Driver::Sim(Box::new(Sim::new(config, logs)?))
            }
        };
        Ok(Self { driver })
    }

    /// Live duty-cycle invoker for an embedding framework such as a feed actor.
    ///
    /// # Errors
    /// Invalid configuration or an unavailable Aeron client.
    pub fn invoker(config: Config) -> Result<Invoker, Error> {
        Invoker::new(config)
    }

    /// Build the live driver from the application environment.
    ///
    /// # Errors
    /// The application's bus, schemas, settings or signal handler failed.
    pub fn from_env(schema: &str) -> Result<Self, crate::app::Error> {
        Ok(Self {
            driver: Driver::Live(Box::new(Invoker::from_env(schema)?)),
        })
    }

    /// Context used to construct an agent before entering its loop.
    pub fn ctx(&mut self) -> &mut Ctx {
        match &mut self.driver {
            Driver::Live(live) => live.ctx(),
            Driver::Sim(sim) => sim.ctx(),
        }
    }

    /// Read-only access to the current context.
    #[must_use]
    pub fn ctx_ref(&self) -> &Ctx {
        match &self.driver {
            Driver::Live(live) => live.ctx_ref(),
            Driver::Sim(sim) => sim.ctx_ref(),
        }
    }

    /// Shutdown was requested by the agent or a live signal handler.
    #[must_use]
    pub fn is_stopping(&self) -> bool {
        match &self.driver {
            Driver::Live(live) => live.is_stopping(),
            Driver::Sim(sim) => sim.ctx_ref().stopping,
        }
    }

    /// Run the selected driver and return the agent's final state.
    ///
    /// # Errors
    /// Agent startup or input processing failed.
    pub fn run<A: Agent>(self, mut agent: A) -> Result<A, Error> {
        match self.driver {
            Driver::Live(live) => live.run(agent),
            Driver::Sim(mut sim) => {
                sim.run(&mut agent)?;
                Ok(agent)
            }
        }
    }

    /// Start an embedded live agent. For new embedding code use [`Invoker`].
    ///
    /// # Errors
    /// Startup failed, or this runtime uses a simulation driver.
    pub fn start<A: Agent>(&mut self, agent: &mut A) -> Result<(), Error> {
        match &mut self.driver {
            Driver::Live(live) => live.start(agent),
            Driver::Sim(_) => Err(Error::Config("simulation must use Runtime::run".into())),
        }
    }

    /// One live duty cycle; simulation is driven by [`Runtime::run`].
    #[inline]
    pub fn cycle<A: Agent>(&mut self, agent: &mut A) -> usize {
        match &mut self.driver {
            Driver::Live(live) => live.cycle(agent),
            Driver::Sim(_) => 0,
        }
    }

    /// Stop an embedded live agent and close its feeds.
    ///
    /// # Errors
    /// Its final journal checkpoint could not be recorded.
    pub fn finish<A: Agent>(&mut self, agent: &mut A) -> Result<(), Error> {
        match &mut self.driver {
            Driver::Live(live) => live.finish(agent),
            Driver::Sim(_) => Ok(()),
        }
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("now", &self.ctx_ref().now())
            .finish_non_exhaustive()
    }
}
