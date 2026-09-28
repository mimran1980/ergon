//! A reliable feed that loses nothing: Aeron's persistent subscription over
//! the recording that the archive on a service's node makes of one of its
//! streams (see [`crate::streams`]).
//!
//! It replays the recording until it catches the live stream, then takes the
//! live stream; a subscriber that falls behind drops back to the recording
//! and rejoins, losing nothing, and the publisher never waits for it.
//!
//! A persistent subscription follows one recording. When the publisher
//! restarts or moves, its new session is a new recording, on the archive of
//! the node it now runs on. Aeron's persistent subscription replays the old
//! recording to its end and fails when the live stream it finds next is at
//! another position (1.52.2): a new session is. [`Persistent`] then finds
//! the new recording, through the service's name, and replays it from its
//! first message, so nothing the new publisher sent is missed.
//!
//! ponytail: a new session exactly at the old one's stop position would be
//! joined as its continuation; check each live message's session against
//! the recording's if that ever matters. Its handler is told when a message is the first of a
//! subscription, because whatever was built from the old session (an order
//! book, say) must be rebuilt.
//!
//! Finding a recording asks the archive, which takes round trips: it runs on
//! a thread of its own, and [`Persistent::poll`] never waits for it. It
//! resolves the service's name itself, each time, and talks to the archive
//! by IP: the media driver resolves a channel's hostname once per shared
//! endpoint, and while the old node's archive still answered, a channel
//! naming the service kept reaching it for a minute after a move. A
//! persistent subscription so belongs to the archive it was made on.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchivePersistentSubscription,
    IntoCString, PersistentSubscriptionBuilder,
};

use crate::feed::{Delivery, Origin};
use crate::streams::Streams;
use crate::{Error, Persist};

/// A replay's stream id is the live one plus this: unique on the node, as
/// live stream ids are unique in the registry.
pub const REPLAY_STREAM_OFFSET: i32 = 1_000_000;

/// A persistent subscription to one stream of a registry service.
pub struct Persistent {
    aeron: Aeron,
    /// `md-binance stream 2011`, for the log.
    name: String,
    /// The service's name, which resolves to the node it runs on, and the
    /// archive's control port there.
    host: String,
    archive_port: u16,
    /// Where the archive answers and replays to: this node.
    local: String,
    live: String,
    stream_id: i32,
    /// The service's control port: its recordings' channels carry it.
    port: u16,
    state: State,
    /// Replay the next recording from its start: it is a new session.
    from_start: bool,
    /// The next message is the first of a new subscription.
    fresh: bool,
    /// The last failure logged: each different one is logged once.
    logged: Option<String>,
}

/// A recording to follow, and the archive holding it, by IP.
struct Found {
    recording: i64,
    archive: String,
}

enum State {
    Waiting(Instant),
    Finding(Receiver<Result<Found, String>>),
    Running {
        subscription: AeronArchivePersistentSubscription,
        recording: i64,
        // Kept for as long as the subscription that was built from it.
        _archive: AeronArchiveContext,
    },
}

impl Persist {
    /// Subscribe to `service`'s `kind` stream through the archive that
    /// records it, from this node (`host_ip`). The first subscription joins
    /// the live stream; each after a restart of the publisher replays the
    /// new session from its start.
    pub fn persistent(
        &self,
        streams: &Streams,
        service: &str,
        kind: &str,
        host_ip: &str,
    ) -> Result<Persistent, Error> {
        let stream_id = streams.stream(service, kind)?;
        Ok(Persistent {
            aeron: self.inner.aeron.clone(),
            name: format!("{service} stream {stream_id}"),
            host: streams.host(service),
            archive_port: streams.archive_port,
            local: format!("aeron:udp?endpoint={host_ip}:0"),
            live: streams.subscription(service, kind, host_ip)?,
            stream_id,
            port: streams.port(service)?,
            state: State::Waiting(Instant::now()),
            from_start: false,
            fresh: false,
            logged: None,
        })
    }
}

impl Persistent {
    /// How long to wait before asking the archive again.
    pub const RETRY: Duration = Duration::from_secs(1);

    /// Up to `limit` messages, each with how it was delivered: whether it is
    /// the first of a new subscription (a new publisher session, when not
    /// the first), and whether it is [`Origin::Live`] or an
    /// [`Origin::Replay`] from the recording while catching up, possibly
    /// minutes old. Returns how many were taken: the work count for an idle
    /// strategy.
    #[inline]
    pub fn poll(&mut self, mut handler: impl FnMut(&[u8], Delivery), limit: usize) -> usize {
        let State::Running { subscription, .. } = &self.state else {
            self.advance();
            return 0;
        };
        // Replay until the persistent subscription has switched to the live
        // stream: messages it merges while switching count as replayed.
        let origin = if subscription.is_live() {
            Origin::Live
        } else {
            Origin::Replay
        };
        let fresh = &mut self.fresh;
        let taken = subscription
            .poll_fn(
                |m, _| {
                    handler(
                        m,
                        Delivery {
                            first: std::mem::take(fresh),
                            origin,
                        },
                    );
                },
                limit,
            )
            .unwrap_or(0);
        if taken <= 0 && subscription.has_failed() {
            let why = subscription.get_failure_reason().map_or_else(
                || "it stopped behind the live stream: a new session".to_owned(),
                |(_, reason)| reason,
            );
            self.failed(&why);
        }
        taken.max(0) as usize
    }

    /// Start by replaying the current recording from its start rather
    /// than joining the live stream: for a consumer that must see what its
    /// publisher sent while it was down, such as an exchange's orders.
    #[must_use]
    pub fn from_start(mut self) -> Self {
        self.from_start = true;
        self
    }

    /// Taking the live stream, not replaying.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(&self.state, State::Running { subscription, .. } if subscription.is_live())
    }

    #[cold]
    fn failed(&mut self, why: &str) {
        if let State::Running { recording, .. } = &self.state {
            log::info!(
                "{}: recording {recording} ended ({why}); finding the next",
                self.name
            );
        }
        self.from_start = true;
        self.logged = None;
        self.state = State::Finding(self.find());
    }

    #[cold]
    fn advance(&mut self) {
        match &self.state {
            State::Waiting(at) if Instant::now() >= *at => self.state = State::Finding(self.find()),
            State::Finding(found) => match found.try_recv() {
                Ok(Ok(found)) => match self.subscribe(&found) {
                    Ok(state) => {
                        log::info!(
                            "{}: following recording {} on {}, {}",
                            self.name,
                            found.recording,
                            found.archive,
                            if self.from_start {
                                "from its start"
                            } else {
                                "from live"
                            }
                        );
                        self.fresh = true;
                        self.logged = None;
                        self.state = state;
                    }
                    Err(e) => self.retry(&e),
                },
                Ok(Err(e)) => self.retry(&e),
                Err(TryRecvError::Disconnected) => self.retry(&"the search thread ended"),
                Err(TryRecvError::Empty) => {}
            },
            _ => {}
        }
    }

    fn retry(&mut self, e: &dyn std::fmt::Display) {
        // Each different failure once: an undeployed publisher fails every
        // retry the same way.
        let e = e.to_string();
        if self.logged.as_ref() != Some(&e) {
            log::info!("{}: {e}; retrying every {:?}", self.name, Self::RETRY);
            self.logged = Some(e);
        }
        self.state = State::Waiting(Instant::now() + Self::RETRY);
    }

    /// Ask the service's archive, on another thread, for the recording of
    /// its current session: the newest still recording. The service's name
    /// is resolved here, now, and the archive then reached by that IP.
    fn find(&self) -> Receiver<Result<Found, String>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let (aeron, local) = (self.aeron.clone(), self.local.clone());
        let (host, archive_port) = (self.host.clone(), self.archive_port);
        let (stream_id, port) = (self.stream_id, format!(":{}", self.port));
        let spawned = std::thread::Builder::new()
            .name("find-recording".into())
            .spawn(move || {
                let found = resolve(&host, archive_port).and_then(|channel| {
                    let ctx = archive_context(&aeron, &channel, &local)?;
                    let archive = AeronArchiveAsyncConnect::new_with_aeron(&ctx, &aeron)
                        .map_err(|e| e.to_string())?
                        .poll_blocking(Duration::from_secs(5))
                        .map_err(|e| format!("connecting to its archive: {e}"))?;
                    let mut newest = None;
                    archive
                        .list_recordings_for_uri_fn(&mut 0, 0, i32::MAX, c"aeron", stream_id, |d| {
                            // Still recording (no stop position), and
                            // this service's: its control port.
                            if d.stop_position() < 0 && d.original_channel().contains(&port) {
                                newest = newest.max(Some(d.recording_id()));
                            }
                        })
                        .map_err(|e| format!("listing its recordings: {e}"))?;
                    newest
                        .map(|recording| Found {
                            recording,
                            archive: channel,
                        })
                        .ok_or_else(|| "no recording of its current session yet".to_owned())
                });
                let _ = tx.send(found);
            });
        if let Err(e) = spawned {
            let (tx, rx) = std::sync::mpsc::channel();
            let _ = tx.send(Err(e.to_string()));
            return rx;
        }
        rx
    }

    fn subscribe(&self, found: &Found) -> Result<State, String> {
        let (recording, archive) = (
            found.recording,
            archive_context(&self.aeron, &found.archive, &self.local)?,
        );
        let builder = PersistentSubscriptionBuilder::new()
            .and_then(|b| b.aeron(&self.aeron))
            .and_then(|b| b.archive_context(&archive))
            .and_then(|b| b.recording_id(recording))
            .and_then(|b| b.live_channel(&self.live))
            .and_then(|b| b.live_stream_id(self.stream_id))
            .and_then(|b| b.replay_channel(&self.local))
            .and_then(|b| b.replay_stream_id(self.stream_id + REPLAY_STREAM_OFFSET));
        let builder = if self.from_start {
            builder.and_then(PersistentSubscriptionBuilder::start_from_beginning)
        } else {
            builder.and_then(PersistentSubscriptionBuilder::start_from_live)
        };
        let subscription = builder
            .and_then(PersistentSubscriptionBuilder::build)
            .map_err(|e| format!("subscribing to recording {recording}: {e}"))?;
        Ok(State::Running {
            subscription,
            recording,
            _archive: archive,
        })
    }
}

/// The control channel of the archive at `host:port`, by the IP `host`
/// resolves to now.
fn resolve(host: &str, port: u16) -> Result<String, String> {
    use std::net::ToSocketAddrs;
    let ip = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("resolving {host}: {e}"))?
        .find(std::net::SocketAddr::is_ipv4)
        .ok_or_else(|| format!("{host} has no IPv4 address"))?;
    Ok(format!("aeron:udp?endpoint={ip}"))
}

/// A client context for the archive at `archive`, answering to `local`.
fn archive_context(
    aeron: &Aeron,
    archive: &str,
    local: &str,
) -> Result<AeronArchiveContext, String> {
    let ctx = AeronArchiveContext::new().map_err(|e| e.to_string())?;
    ctx.set_aeron(aeron).map_err(|e| e.to_string())?;
    ctx.set_control_request_channel(&archive.into_c_string())
        .map_err(|e| e.to_string())?;
    ctx.set_control_response_channel(&local.into_c_string())
        .map_err(|e| e.to_string())?;
    Ok(ctx)
}
