//! Feeds this application takes: [`Subscription`] straight off the network,
//! or [`PersistentSubscription`] through the archive that records the feed,
//! catching up from it after a restart or a slow spell.

use std::ffi::CString;
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronAsyncAddSubscription, AeronSubscription, Handlers, IntoCString,
};

use crate::Error;
use crate::bus::Bus;
use crate::streams::Streams;

mod persistent;

/// The image session id of an archive replay: the low 32 bits of the
/// replay session id `start_replay` returns. The high 32 bits number the
/// replay, and pass `i32` once an archive has run a few billion replays
/// (Aeron's own clients take `(int) replaySessionId`).
#[must_use]
pub fn replay_image_session(replay_session: i64) -> i32 {
    u32::try_from(replay_session & 0xffff_ffff).map_or(0, u32::cast_signed)
}

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
    /// Live, or replayed from the archive while catching up.
    pub origin: Origin,
}

impl Delivery {
    /// The message was not replayed from the archive.
    #[must_use]
    pub fn is_live(self) -> bool {
        self.origin == Origin::Live
    }
}

/// A subscription to another service's feed, by its name.
///
/// Added without blocking, and added again every [`Subscription::RETRY`]
/// while the name does not resolve (its publisher is not deployed, or not
/// started yet).
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
}

enum State {
    Waiting(Instant),
    Adding(AeronAsyncAddSubscription),
    Ready(AeronSubscription),
}

impl Bus {
    /// Subscribe to `service`'s `kind` stream straight off the network,
    /// added as [`Subscription::poll`] is called. Best effort: a message published while this was not
    /// connected is gone; see [`Bus::subscribe`] for one that is not.
    ///
    /// # Errors
    ///
    /// `service` or `kind` is not in the registry.
    pub fn subscribe_live(
        &self,
        streams: &Streams,
        service: &str,
        kind: &str,
    ) -> Result<Subscription, Error> {
        Ok(self.subscription(
            &streams.subscription(service, kind, self.host_ip())?,
            streams.stream(service, kind)?,
        ))
    }

    /// Subscribe to `channel` (see
    /// [`crate::streams::Streams::subscription`]) and `stream_id` on this
    /// application's media driver, added as [`Subscription::poll`] is called.
    #[must_use]
    pub fn subscription(&self, channel: &str, stream_id: i32) -> Subscription {
        Subscription {
            aeron: self.aeron().clone(),
            channel: channel.into_c_string(),
            stream_id,
            state: State::Waiting(Instant::now()),
            session: None,
            superseded: [None; 4],
        }
    }
}

impl Subscription {
    /// How long to wait before adding a subscription that failed again: the
    /// name is resolved on the node's shared driver, so not in a hurry.
    pub const RETRY: Duration = Duration::from_secs(5);

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
        // A failed poll would fail again on every pass of a busy loop:
        // subscribe again instead.
        if let Err(e) = polled {
            self.retry(&e);
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
                    self.state = State::Ready(subscription);
                }
                Ok(None) => {}
                Err(e) => self.retry(&e),
            },
            _ => {}
        }
    }

    #[cold]
    fn retry(&mut self, e: &dyn std::fmt::Display) {
        log::warn!(
            "{:?} stream {}: {e}; retrying in {:?}",
            self.channel,
            self.stream_id,
            Self::RETRY
        );
        self.state = State::Waiting(Instant::now() + Self::RETRY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_replay_session_is_its_low_32_bits() {
        assert_eq!(replay_image_session(82_293_028_631), 688_650_007);
        assert_eq!(replay_image_session((19 << 32) | 0xFFFF_FFFF), -1);
        assert_eq!(replay_image_session(1_234), 1_234);
    }
}
