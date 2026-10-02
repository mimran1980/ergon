//! Logging for a failure that repeats: a retry that fails the same way every
//! second, a poll that errors on every pass of a spinning loop.
//!
//! A [`Throttle`] logs a message the first time, again as soon as it changes,
//! and as a reminder no more than once per interval while it stays the same.
//! [`Throttle::clear`] on success, so the next failure is logged at once.
//!
//! ponytail: the message is formatted on every call, to tell whether it
//! changed. That is fine on a failure path, not for per-message logging.

use std::fmt::Display;
use std::time::{Duration, Instant};

/// Logs one failure, and repeats of it at most once per interval.
#[derive(Debug)]
pub struct Throttle {
    every: Duration,
    last: Option<(Instant, String)>,
}

impl Throttle {
    /// A throttle that repeats an unchanged message every `every` at most.
    #[must_use]
    pub const fn new(every: Duration) -> Self {
        Self { every, last: None }
    }

    /// Log `message` at `level` unless it is the one last logged, less than
    /// one interval ago.
    pub fn log(&mut self, level: log::Level, message: impl Display) {
        let message = message.to_string();
        if self.due(Instant::now(), &message) {
            log::log!(level, "{message}");
        }
    }

    /// Forget the last message: the next one is logged at once.
    pub fn clear(&mut self) {
        self.last = None;
    }

    fn due(&mut self, now: Instant, message: &str) -> bool {
        match &self.last {
            Some((at, last)) if last == message && now.duration_since(*at) < self.every => false,
            _ => {
                self.last = Some((now, message.to_owned()));
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repeat_waits_for_the_interval_and_a_change_does_not() {
        let start = Instant::now();
        let at = |s| start + Duration::from_secs(s);
        let mut throttle = Throttle::new(Duration::from_secs(10));

        assert!(throttle.due(at(0), "refused"), "the first is logged");
        assert!(!throttle.due(at(1), "refused"), "a repeat is not");
        assert!(!throttle.due(at(9), "refused"));
        assert!(
            throttle.due(at(10), "refused"),
            "a reminder after the interval"
        );
        assert!(
            !throttle.due(at(11), "refused"),
            "counted from the reminder"
        );

        assert!(
            throttle.due(at(12), "timed out"),
            "a change is logged at once"
        );
        assert!(
            throttle.due(at(13), "refused"),
            "even back to an earlier one"
        );

        throttle.clear();
        assert!(throttle.due(at(14), "refused"), "after clear, at once");
    }
}
