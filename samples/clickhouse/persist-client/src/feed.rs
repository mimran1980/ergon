//! A feed: a UDP publication other applications subscribe to (see
//! [`crate::streams`]), such as market data, recorded by the archive on
//! whichever node publishes it.
//!
//! [`Feed::record`] is [`Persist::record`] on the feed's publication: the
//! same exact-length claim stamped with the application's source id, the
//! same drop counting. Unlike it, a feed record does not ask `tables.yaml`:
//! subscribers need every message whether or not it is persisted, so the
//! ingester applies `enabled`, `apps` and `until` to feed tables when it
//! inserts them. Every message must fit one UDP frame ([`Feed::max_payload`]).

use std::ffi::CString;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronAsyncAddSubscription, AeronPublication, AeronSubscription, Handlers, IntoCString,
};

use crate::{DropKind, Error, Persist};

/// A publication of this application's on a feed's channel and stream.
pub struct Feed {
    persist: Persist,
    publication: AeronPublication,
    stream_id: i32,
    max_payload: usize,
    /// The heartbeat this feed last sent the `Source` message at.
    heartbeat: AtomicU64,
}

impl Persist {
    /// Publish on `channel` (see [`crate::streams::Streams::publication`])
    /// and `stream_id`.
    pub fn feed(&self, channel: &str, stream_id: i32) -> Result<Feed, Error> {
        let publication = self
            .inner
            .aeron
            .async_add_publication(&channel.into_c_string(), stream_id)
            .and_then(|p| p.poll_blocking(Duration::from_secs(10)))
            .map_err(|e| Error::Aeron(format!("{channel}: {e}")))?;
        let max_payload = publication
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        self.inner
            .shared
            .feeds
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(publication.clone());
        Ok(Feed {
            persist: self.clone(),
            publication,
            stream_id,
            max_payload,
            heartbeat: AtomicU64::new(u64::MAX),
        })
    }
}

impl Feed {
    /// Publish one message of exactly `len` bytes (header included) that
    /// `encode` writes into the claimed frame; see [`Persist::record`].
    #[inline]
    pub fn record<E>(
        &self,
        template_id: u16,
        len: usize,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        let p = &self.persist;
        let beat = p.inner.shared.heartbeat.load(Ordering::Relaxed);
        if beat != self.heartbeat.load(Ordering::Relaxed) {
            self.send_source(beat);
        }
        if len > self.max_payload {
            p.count(DropKind::TooLarge);
            return Ok(());
        }
        let mut claim = match p.try_claim_on(&self.publication, len) {
            Ok(claim) => claim,
            Err(kind) => {
                p.count(kind);
                return Ok(());
            }
        };
        let slot = claim.data();
        let written = encode(slot)?;
        let honest = written == len && slot.get(2..4) == Some(&template_id.to_le_bytes()[..]);
        debug_assert!(
            honest,
            "encode wrote {written} bytes of template {}, but claimed {len} bytes of {template_id}",
            u16::from_le_bytes([slot[2], slot[3]])
        );
        if !(honest && claim.commit().is_ok()) {
            p.drop_one();
        }
        Ok(())
    }

    /// The `Source` message on this feed, so its recording names who
    /// recorded it on its own; repeated every 5 s. Not counted as a drop
    /// when Aeron cannot take it: the next record tries again.
    #[cold]
    fn send_source(&self, beat: u64) {
        let p = &self.persist;
        let message = &p.inner.source_message;
        let sent = p
            .try_claim_on(&self.publication, message.len())
            .is_ok_and(|mut claim| {
                claim.data().copy_from_slice(message);
                claim.commit().is_ok()
            });
        if sent {
            self.heartbeat.store(beat, Ordering::Relaxed);
        }
    }

    /// The longest message one frame holds: size chunks from it.
    #[must_use]
    pub fn max_payload(&self) -> usize {
        self.max_payload
    }

    #[must_use]
    pub fn stream_id(&self) -> i32 {
        self.stream_id
    }

    /// A subscriber or the archive's spy is taking it.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.publication.is_connected()
    }
}

/// A subscription to another service's feed, by its name: added without
/// blocking, and added again every [`Subscriber::RETRY`] while the name does
/// not resolve (its publisher is not deployed, or not started yet).
///
/// It tells its handler when a message is the first of a new publisher
/// session, which is a publisher restart or move: what the application
/// built from the old session (an order book, say) must be rebuilt. The
/// old session's last buffered messages can still arrive after the new
/// one's first; they are dropped.
pub struct Subscriber {
    aeron: Aeron,
    channel: CString,
    stream_id: i32,
    state: State,
    /// The session messages are taken from, and the last few before it.
    session: Option<i32>,
    superseded: [Option<i32>; 4],
    /// A failure to subscribe has been logged: an undeployed publisher
    /// fails every retry.
    logged: bool,
}

enum State {
    Waiting(Instant),
    Adding(AeronAsyncAddSubscription),
    Ready(AeronSubscription),
}

impl Persist {
    /// Subscribe to `channel` (see
    /// [`crate::streams::Streams::subscription`]) and `stream_id` on this
    /// application's media driver, in the background.
    #[must_use]
    pub fn subscriber(&self, channel: &str, stream_id: i32) -> Subscriber {
        Subscriber {
            aeron: self.inner.aeron.clone(),
            channel: channel.into_c_string(),
            stream_id,
            state: State::Waiting(Instant::now()),
            session: None,
            superseded: [None; 4],
            logged: false,
        }
    }
}

impl Subscriber {
    /// How long to wait before adding a subscription that failed again: the
    /// name is resolved on the node's shared driver, so not in a hurry.
    pub const RETRY: Duration = Duration::from_secs(5);

    /// Up to `limit` messages, each with whether it starts a new session.
    /// Returns how many were taken: the work count for an idle strategy.
    #[inline]
    pub fn poll(&mut self, mut handler: impl FnMut(&[u8], bool), limit: usize) -> usize {
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
                if *session == Some(id) {
                    taken += 1;
                    handler(message, false);
                } else if !superseded.contains(&Some(id)) {
                    if let Some(old) = session.replace(id) {
                        superseded.rotate_right(1);
                        superseded[0] = Some(old);
                    }
                    taken += 1;
                    handler(message, true);
                }
            },
            limit,
        );
        if let Err(e) = polled {
            log::warn!("{:?} stream {}: {e}", self.channel, self.stream_id);
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

    fn retry(&mut self, e: &dyn std::fmt::Display) {
        if !self.logged {
            log::info!(
                "{:?} stream {}: {e}; retrying every {:?}",
                self.channel,
                self.stream_id,
                Self::RETRY
            );
            self.logged = true;
        }
        self.state = State::Waiting(Instant::now() + Self::RETRY);
    }
}
