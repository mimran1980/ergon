//! What a polling loop does when an iteration found no work: the trade
//! between latency and the CPU it burns.
//!
//! | `IDLE` | Wake-up latency | CPU |
//! |---|---|---|
//! | `spin` | the lowest: a pause instruction, then poll again | one core at 100% |
//! | `noop` | as low, without even the pause | one core at 100% |
//! | `yield` | a scheduler round trip | high when other threads are idle |
//! | `sleep[:<period>]` | the period (default `1ms`), plus the timer slack | low |
//! | `backoff[:<spins>,<yields>,<min>,<max>]` | none while busy; at most `max` once idle | low once idle |
//!
//! `backoff` is Agrona's `BackoffIdleStrategy`. After each iteration that
//! found no work it spins `spins` times, then yields `yields` times, then
//! parks: for `min`, doubling each idle iteration up to `max`. Any work starts
//! it over, so a busy loop never parks. `backoff` alone is Agrona's defaults:
//! `backoff:10,5,1us,1ms`. Periods take `ns`, `us`, `ms` or `s`.
//!
//! A sleep or a park is a timed `nanosleep`, and Linux wakes it up to the
//! thread's timer slack late: 50 µs by default. On an Azure `D4s_v6` (Linux
//! 6.12) a requested 1 µs sleep took 61 µs at p50 with the default slack, and
//! 7 µs with the slack at 1 ns, which then spent most of a core in sleeps.
//! `TIMER_SLACK` sets the slack of the thread that runs the loop
//! ([`App::start_with`](crate::app::App::start_with)); Linux only.
//!
//! Production trading loops spin on isolated cores; the lab backs off, so
//! several nodes share one machine.

use std::time::Duration;

/// A loop's idle strategy; see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Idle {
    /// A pause instruction, then poll again.
    Spin,
    /// Poll again with no pause.
    Noop,
    /// Yield the core to the scheduler.
    Yield,
    /// Sleep for this long.
    Sleep(Duration),
    /// Spin, then yield, then park for ever longer periods; see [`Backoff`].
    Backoff(Backoff),
}

/// Agrona's `BackoffIdleStrategy`: after an iteration that found no work,
/// spin, then yield, then park for a period that doubles each idle
/// iteration, up to a cap. Any work starts it over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
    spins: u32,
    yields: u32,
    min_park_ns: u64,
    max_park_ns: u64,
    /// Iterations without work since the last one with some.
    idle: u32,
}

/// One idle iteration of a [`Backoff`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Spin,
    Yield,
    Park(Duration),
}

impl Backoff {
    /// Agrona's defaults: 10 spins, 5 yields, parks from 1 µs to 1 ms.
    pub const DEFAULT: Self = Self {
        spins: 10,
        yields: 5,
        min_park_ns: 1_000,
        max_park_ns: 1_000_000,
        idle: 0,
    };

    /// `spins` spins and `yields` yields, then parks from `min_park` to
    /// `max_park`.
    ///
    /// # Errors
    ///
    /// `min_park` is zero, or `max_park` is shorter than `min_park`.
    pub fn new(
        spins: u32,
        yields: u32,
        min_park: Duration,
        max_park: Duration,
    ) -> Result<Self, String> {
        if min_park.is_zero() {
            return Err("backoff: the shortest park must be longer than zero".into());
        }
        if max_park < min_park {
            return Err(format!(
                "backoff: the longest park {max_park:?} is shorter than the shortest {min_park:?}"
            ));
        }
        let ns = |d: Duration| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX);
        Ok(Self {
            spins,
            yields,
            min_park_ns: ns(min_park),
            max_park_ns: ns(max_park),
            idle: 0,
        })
    }

    /// The next idle iteration's step.
    fn step(&mut self) -> Step {
        let n = self.idle;
        self.idle = n.saturating_add(1);
        if n < self.spins {
            return Step::Spin;
        }
        let n = n - self.spins;
        if n < self.yields {
            return Step::Yield;
        }
        let doublings = n - self.yields;
        let park = self
            .min_park_ns
            .checked_shl(doublings)
            .filter(|&ns| ns >> doublings == self.min_park_ns)
            .map_or(self.max_park_ns, |ns| ns.min(self.max_park_ns));
        Step::Park(Duration::from_nanos(park))
    }
}

impl Idle {
    /// `spin`, `noop`, `yield`, `sleep[:<period>]` or
    /// `backoff[:<spins>,<yields>,<min park>,<max park>]`; see the module docs.
    ///
    /// # Errors
    ///
    /// `spec` is none of those, or its arguments do not parse.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let (name, args) = spec
            .split_once(':')
            .map_or((spec, None), |(n, a)| (n, Some(a)));
        match (name, args) {
            ("spin" | "busy_spin", None) => Ok(Self::Spin),
            ("noop", None) => Ok(Self::Noop),
            ("yield", None) => Ok(Self::Yield),
            ("sleep", None) => Ok(Self::Sleep(Duration::from_millis(1))),
            ("sleep", Some(period)) => match duration(period)? {
                d if d.is_zero() => Err("sleep: the period must be longer than zero".into()),
                d => Ok(Self::Sleep(d)),
            },
            ("backoff", None) => Ok(Self::Backoff(Backoff::DEFAULT)),
            ("backoff", Some(args)) => {
                let parts: Vec<&str> = args.split(',').map(str::trim).collect();
                let [spins, yields, min, max] = parts[..] else {
                    return Err(format!(
                        "backoff:{args}: expected <spins>,<yields>,<min park>,<max park>"
                    ));
                };
                let count = |s: &str| {
                    s.parse::<u32>()
                        .map_err(|e| format!("backoff: {s:?} is not a count: {e}"))
                };
                Backoff::new(
                    count(spins)?,
                    count(yields)?,
                    duration(min)?,
                    duration(max)?,
                )
                .map(Self::Backoff)
            }
            _ => Err(format!(
                "idle strategy {spec:?}: expected spin, noop, yield, sleep[:<period>] \
                 or backoff[:<spins>,<yields>,<min park>,<max park>]"
            )),
        }
    }

    /// From the environment variable `var`, else `default`.
    ///
    /// # Errors
    ///
    /// The variable is set to a value [`Idle::parse`] refuses.
    pub fn from_env(var: &str, default: Self) -> Result<Self, String> {
        std::env::var(var).map_or(Ok(default), |v| Self::parse(&v))
    }

    /// Called after each iteration with the work it found: nothing but a
    /// compare when there was some.
    #[inline]
    pub fn idle(&mut self, work: usize) {
        if work > 0 {
            if let Self::Backoff(b) = self
                && b.idle != 0
            {
                b.idle = 0;
            }
            return;
        }
        match self {
            Self::Spin => std::hint::spin_loop(),
            Self::Noop => {}
            Self::Yield => std::thread::yield_now(),
            Self::Sleep(period) => std::thread::sleep(*period),
            Self::Backoff(b) => match b.step() {
                Step::Spin => std::hint::spin_loop(),
                Step::Yield => std::thread::yield_now(),
                Step::Park(period) => std::thread::sleep(period),
            },
        }
    }
}

/// A period: a whole number followed by `ns`, `us` (or `µs`), `ms` or `s`.
///
/// # Errors
///
/// The text is not a number with one of those units.
pub fn duration(text: &str) -> Result<Duration, String> {
    let text = text.trim();
    let split = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let n: u64 = number
        .parse()
        .map_err(|_| format!("{text:?}: expected a whole number then ns, us, ms or s"))?;
    match unit {
        "ns" => Ok(Duration::from_nanos(n)),
        "us" | "µs" => Ok(Duration::from_micros(n)),
        "ms" => Ok(Duration::from_millis(n)),
        "s" => Ok(Duration::from_secs(n)),
        _ => Err(format!(
            "{text:?}: expected a whole number then ns, us, ms or s"
        )),
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
        assert!(Idle::parse("spin:1us").is_err(), "spin takes no argument");
    }

    #[test]
    fn a_sleep_takes_its_period_and_defaults_to_a_millisecond() -> Result<(), String> {
        assert_eq!(Idle::parse("sleep")?, Idle::Sleep(Duration::from_millis(1)));
        assert_eq!(
            Idle::parse("sleep:50us")?,
            Idle::Sleep(Duration::from_micros(50))
        );
        assert_eq!(
            Idle::parse("sleep:250µs")?,
            Idle::Sleep(Duration::from_micros(250))
        );
        assert!(Idle::parse("sleep:0us").is_err());
        assert!(Idle::parse("sleep:5").is_err(), "a unit is required");
        assert!(Idle::parse("sleep:1.5ms").is_err(), "whole numbers only");
        Ok(())
    }

    #[test]
    fn backoff_alone_is_agronas_defaults_and_takes_four_arguments() -> Result<(), String> {
        assert_eq!(Idle::parse("backoff")?, Idle::Backoff(Backoff::DEFAULT));
        assert_eq!(
            Idle::parse("backoff:10,5,1us,1ms")?,
            Idle::Backoff(Backoff::DEFAULT)
        );
        assert!(Idle::parse("backoff:10,5,1us").is_err(), "three arguments");
        assert!(Idle::parse("backoff:10,5,0ns,1ms").is_err(), "a zero park");
        assert!(
            Idle::parse("backoff:10,5,1ms,1us").is_err(),
            "max below min"
        );
        assert!(
            Idle::parse("backoff:-1,5,1us,1ms").is_err(),
            "a negative count"
        );
        Ok(())
    }

    #[test]
    fn backoff_spins_then_yields_then_parks_doubling_to_its_cap() -> Result<(), String> {
        let us = Duration::from_micros;
        let mut b = Backoff::new(2, 1, us(1), us(8))?;
        let steps: Vec<Step> = (0..8).map(|_| b.step()).collect();
        assert_eq!(
            steps,
            [
                Step::Spin,
                Step::Spin,
                Step::Yield,
                Step::Park(us(1)),
                Step::Park(us(2)),
                Step::Park(us(4)),
                Step::Park(us(8)),
                Step::Park(us(8)),
            ]
        );
        // Work starts it over.
        let mut idle = Idle::Backoff(b);
        idle.idle(1);
        let Idle::Backoff(mut b) = idle else {
            return Err("still a backoff".into());
        };
        assert_eq!(b.step(), Step::Spin);
        Ok(())
    }

    #[test]
    fn a_park_that_would_overflow_stays_at_its_cap() -> Result<(), String> {
        let mut b = Backoff::new(0, 0, Duration::from_secs(1), Duration::from_mins(1))?;
        let parks: Vec<Step> = (0..80).map(|_| b.step()).collect();
        assert_eq!(parks[79], Step::Park(Duration::from_mins(1)));
        assert!(
            parks
                .iter()
                .all(|s| matches!(s, Step::Park(d) if *d <= Duration::from_mins(1)))
        );
        Ok(())
    }

    #[test]
    fn periods_take_their_units() -> Result<(), String> {
        assert_eq!(duration("1ns")?, Duration::from_nanos(1));
        assert_eq!(duration("7us")?, Duration::from_micros(7));
        assert_eq!(duration("7µs")?, Duration::from_micros(7));
        assert_eq!(duration(" 3ms ")?, Duration::from_millis(3));
        assert_eq!(duration("2s")?, Duration::from_secs(2));
        assert!(duration("ms").is_err());
        assert!(duration("5m").is_err());
        Ok(())
    }
}
