//! What a polling loop does when an iteration found no work: the trade
//! between latency and the CPU it burns.
//!
//! | `IDLE` | Wake-up latency | CPU |
//! |---|---|---|
//! | `spin` | the lowest: a pause instruction, then poll again | one core at 100% |
//! | `noop` | as low, without even the pause | one core at 100% |
//! | `yield` | a scheduler round trip | high when other threads are idle |
//! | `sleep` | up to about a millisecond | almost none |
//!
//! Production trading loops spin on isolated cores; the lab sleeps or
//! yields, so several nodes share one machine.

/// A loop's idle strategy; see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Idle {
    /// A pause instruction, then poll again.
    Spin,
    /// Poll again with no pause.
    Noop,
    /// Yield the core to the scheduler.
    Yield,
    /// Sleep for about a millisecond.
    Sleep,
}

impl Idle {
    /// `spin`, `noop`, `yield` or `sleep`.
    ///
    /// # Errors
    ///
    /// `name` is not one of those.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "spin" | "busy_spin" => Ok(Self::Spin),
            "noop" => Ok(Self::Noop),
            "yield" => Ok(Self::Yield),
            "sleep" => Ok(Self::Sleep),
            other => Err(format!(
                "idle strategy {other:?}: expected spin, noop, yield or sleep"
            )),
        }
    }

    /// From the environment variable `var`, else `default`.
    ///
    /// # Errors
    ///
    /// The variable is set to a name [`Idle::parse`] refuses.
    pub fn from_env(var: &str, default: Self) -> Result<Self, String> {
        std::env::var(var).map_or(Ok(default), |v| Self::parse(&v))
    }

    /// Called after each iteration: does nothing when it found work.
    #[inline]
    pub fn idle(self, work: usize) {
        if work > 0 {
            return;
        }
        match self {
            Self::Spin => std::hint::spin_loop(),
            Self::Noop => {}
            Self::Yield => std::thread::yield_now(),
            Self::Sleep => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_and_unknown_ones_are_refused() {
        assert_eq!(Idle::parse("spin"), Ok(Idle::Spin));
        assert_eq!(Idle::parse("busy_spin"), Ok(Idle::Spin));
        assert_eq!(Idle::parse("yield"), Ok(Idle::Yield));
        assert!(Idle::parse("sleepy").is_err());
    }
}
