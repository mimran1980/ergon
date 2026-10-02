//! Feeds this application takes: [`Subscription`] straight off the network,
//! or [`PersistentSubscription`] through the archive that records the feed,
//! catching up from it after a restart or a slow spell.

use std::ffi::CString;
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronAsyncAddSubscription, AeronSubscription, Handlers, IntoCString,
};

use crate::bus::Bus;
use crate::throttle::Throttle;

mod persistent;

pub use persistent::PersistentSubscription;

/// Where a subscribed message came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Off the network, as it was published: current.
    Live,
    /// Replayed from the archive while catching up: possibly minutes old.
    /// Rebuild state from it, but decide nothing and measure no latency on
    /// it.
    Replay,
}

/// How a subscription delivered one message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delivery {
    /// The first message of a new subscription: the first overall, or of a
    /// publisher's new session after a restart or move, when whatever was
    /// built from the old session must be rebuilt.
    pub first: bool,
    pub origin: Origin,
}

impl Delivery {
    #[must_use]
    pub fn is_live(self) -> bool {
        self.origin == Origin::Live
    }
}

/// A subscription to another service's feed, by its name: added without
/// blocking, and added again every [`Subscription::RETRY`] while the name does
/// not resolve (its publisher is not deployed, or not started yet).
///
/// It tells its handler when a message is the first of a new publisher
/// session, which is a publisher restart or move: what the application
/// built from the old session (an order book, say) must be rebuilt. The
/// old session's last buffered messages can still arrive after the new
/// one's first; they are dropped.
pub struct Subscription {
    aeron: Aeron,
    channel: CString,
    stream_id: i32,
    state: State,
    /// The session messages are taken from, and the last few before it.
    session: Option<i32>,
    superseded: [Option<i32>; 4],
    /// A failure to subscribe: an undeployed publisher fails every retry.
    logged: Throttle,
    /// A failed poll, which a busy loop repeats on every pass.
    poll_failed: Throttle,
}

enum State {
    Waiting(Instant),
    Adding(AeronAsyncAddSubscription),
    Ready(AeronSubscription),
}

impl Bus {
    /// Subscribe to `channel` (see
    /// [`crate::streams::Streams::subscription`]) and `stream_id` on this
    /// application's media driver, in the background.
    #[must_use]
    pub fn subscription(&self, channel: &str, stream_id: i32) -> Subscription {
        Subscription {
            aeron: self.aeron().clone(),
            channel: channel.into_c_string(),
            stream_id,
            state: State::Waiting(Instant::now()),
            session: None,
            superseded: [None; 4],
            logged: Throttle::new(Subscription::REMIND),
            poll_failed: Throttle::new(Subscription::REMIND),
        }
    }
}

impl Subscription {
    /// How long to wait before adding a subscription that failed again: the
    /// name is resolved on the node's shared driver, so not in a hurry.
    pub const RETRY: Duration = Duration::from_secs(5);

    /// How often an unchanged failure is logged again.
    const REMIND: Duration = Duration::from_secs(60);

    /// Up to `limit` messages, each with how it was delivered: always live,
    /// as nothing is replayed here. Returns how many were taken: the work
    /// count for an idle strategy.
    #[inline]
    pub fn poll(&mut self, mut handler: impl FnMut(&[u8], Delivery), limit: usize) -> usize {
        let State::Ready(subscription) = &self.state else {
            self.connect();
            return 0;
        };
        let (session, superseded) = (&mut self.session, &mut self.superseded);
        let mut taken = 0;
        let polled = subscription.poll_fn(
            |message, header| {
                let Ok(values) = header.get_values() else {
                    return;
                };
                let id = values.frame().session_id();
                let live = |first| Delivery {
                    first,
                    origin: Origin::Live,
                };
                if *session == Some(id) {
                    taken += 1;
                    handler(message, live(false));
                } else if !superseded.contains(&Some(id)) {
                    if let Some(old) = session.replace(id) {
                        superseded.rotate_right(1);
                        superseded[0] = Some(old);
                    }
                    taken += 1;
                    handler(message, live(true));
                }
            },
            limit,
        );
        if let Err(e) = polled {
            self.poll_failed.log(
                log::Level::Warn,
                format_args!("{:?} stream {}: {e}", self.channel, self.stream_id),
            );
        }
        taken
    }

    /// The publisher is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        matches!(&self.state, State::Ready(s) if s.is_connected())
    }

    #[cold]
    fn connect(&mut self) {
        match &self.state {
            State::Waiting(at) if Instant::now() >= *at => {
                match self.aeron.async_add_subscription(
                    &self.channel,
                    self.stream_id,
                    Handlers::NONE,
                    Handlers::NONE,
                ) {
                    Ok(adding) => self.state = State::Adding(adding),
                    Err(e) => self.retry(&e),
                }
            }
            State::Adding(adding) => match adding.poll() {
                Ok(Some(subscription)) => {
                    log::info!("subscribed to {:?} stream {}", self.channel, self.stream_id);
                    self.logged.clear();
                    self.poll_failed.clear();
                    self.state = State::Ready(subscription);
                }
                Ok(None) => {}
                Err(e) => self.retry(&e),
            },
            _ => {}
        }
    }

    fn retry(&mut self, e: &dyn std::fmt::Display) {
        self.logged.log(
            log::Level::Info,
            format_args!(
                "{:?} stream {}: {e}; retrying every {:?}",
                self.channel,
                self.stream_id,
                Self::RETRY
            ),
        );
        self.state = State::Waiting(Instant::now() + Self::RETRY);
    }
}
