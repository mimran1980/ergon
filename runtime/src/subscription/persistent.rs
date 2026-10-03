//! A persistent subscription over the archive recording of one stream.
//!
//! Replays until it catches the live stream. If it falls behind, it rejoins
//! from the recording. The publisher does not wait.
//!
//! A restart or a move is a new recording. Aeron 1.52.2 replays the old one
//! to its end and then fails. [`PersistentSubscription`] finds the new recording by the
//! service's name and replays from the first message. The handler is told,
//! so the caller can drop state built from the old session.
//!
//! ponytail: a new session that starts at the old stop position would be
//! treated as the same recording. Check the session id if that matters.
//!
//! The archive lookup is a state machine that [`PersistentSubscription::poll`]
//! advances one step at a time: connect to the service's archive, list its
//! recordings, subscribe. Neither the connect nor the listing waits: the
//! archive may be in another region. Resolving the service's name does
//! (`getaddrinfo`, about a millisecond in the cluster), once per lookup.
//! The archive is addressed by IP. The driver caches a channel hostname per
//! endpoint, so a name would keep reaching the old node after a move.
//!
//! ponytail: Aeron's own `PersistentSubscriptionBuilder::build` waits for the
//! driver to register four counters (one round trip each). Handing it
//! counters added asynchronously would remove that, once their ownership
//! across the C context is clear.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronArchive, AeronArchiveAsyncConnect, AeronArchiveContext,
    AeronArchivePersistentSubscription, AeronArchiveProxy, AeronArchiveRecordingDescriptor,
    AeronArchiveRecordingDescriptorPoller, Handler, IntoCString, PersistentSubscriptionBuilder,
};

use super::{Delivery, Origin};
use crate::Error;
use crate::bus::Bus;
use crate::streams::Streams;

/// A replay's stream id is the live one plus this: unique on the node, as
/// live stream ids are unique in the registry.
const REPLAY_STREAM_OFFSET: i32 = 1_000_000;

/// A persistent subscription to one stream of a registry service.
pub struct PersistentSubscription {
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
}

enum State {
    Waiting(Instant),
    Connecting {
        connect: AeronArchiveAsyncConnect,
        /// The archive's control channel, by IP.
        channel: String,
        _ctx: AeronArchiveContext,
        until: Instant,
    },
    Listing(Box<Listing>),
    Running {
        subscription: AeronArchivePersistentSubscription,
        recording: i64,
        // Kept for as long as the subscription that was built from it.
        _archive: AeronArchiveContext,
    },
}

impl Bus {
    /// Subscribe to `service`'s `kind` stream through the archive that
    /// records it, from this node. The first subscription joins the live
    /// stream; each after a restart of the publisher replays the new session
    /// from its start. [`Bus::subscribe_live`] is the one with no archive.
    ///
    /// # Errors
    ///
    /// `service` or `kind` is not in the registry.
    pub fn subscribe(
        &self,
        streams: &Streams,
        service: &str,
        kind: &str,
    ) -> Result<PersistentSubscription, Error> {
        let host_ip = self.host_ip();
        let stream_id = streams.stream(service, kind)?;
        Ok(PersistentSubscription {
            aeron: self.aeron().clone(),
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
        })
    }
}

impl PersistentSubscription {
    /// How long to wait before asking the archive again.
    pub const RETRY: Duration = Duration::from_secs(1);

    /// How long each step of the lookup may take.
    const STEP: Duration = Duration::from_secs(5);

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
        usize::try_from(taken.max(0)).unwrap_or(0)
    }

    /// Start by replaying the current recording from its start rather
    /// than joining the live stream: for a consumer that must see what its
    /// publisher sent while it was down, such as an exchange's orders.
    #[must_use]
    pub const fn from_start(mut self) -> Self {
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
        self.state = State::Waiting(Instant::now());
    }

    /// One step of the lookup: each state either moves on, gives up into
    /// [`State::Waiting`], or stays as it is until the next call.
    #[cold]
    fn advance(&mut self) {
        let now = Instant::now();
        let next = match &mut self.state {
            State::Waiting(at) if now >= *at => {
                resolve(&self.host, self.archive_port).and_then(|channel| self.connect(&channel))
            }
            State::Connecting {
                connect,
                channel,
                until,
                ..
            } => match connect.poll() {
                Ok(Some(archive)) => Listing::start(archive, channel, self.stream_id, self.port),
                Ok(None) if now >= *until => Err("connecting to its archive: timed out".into()),
                Ok(None) => return,
                Err(e) => Err(format!("connecting to its archive: {e}")),
            },
            State::Listing(listing) => match listing.poll(now) {
                Ok(Some(recording)) => {
                    let channel = std::mem::take(&mut listing.channel);
                    self.subscribe(recording, &channel)
                }
                Ok(None) => return,
                Err(e) => Err(e),
            },
            State::Waiting(_) | State::Running { .. } => return,
        };
        match next {
            Ok(state) => self.state = state,
            Err(e) => self.retry(&e),
        }
    }

    fn retry(&mut self, e: &str) {
        log::warn!("{}: {e}; retrying in {:?}", self.name, Self::RETRY);
        self.state = State::Waiting(Instant::now() + Self::RETRY);
    }

    /// Start connecting to the archive at `channel`.
    fn connect(&self, channel: &str) -> Result<State, String> {
        let ctx = archive_context(&self.aeron, channel, &self.local)?;
        let connect = AeronArchiveAsyncConnect::new_with_aeron(&ctx, &self.aeron)
            .map_err(|e| format!("connecting to its archive: {e}"))?;
        Ok(State::Connecting {
            connect,
            channel: channel.to_owned(),
            _ctx: ctx,
            until: Instant::now() + Self::STEP,
        })
    }

    fn subscribe(&mut self, recording: i64, channel: &str) -> Result<State, String> {
        let archive = archive_context(&self.aeron, channel, &self.local)?;
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
        log::info!(
            "{}: following recording {recording} on {channel}, {}",
            self.name,
            if self.from_start {
                "from its start"
            } else {
                "from live"
            }
        );
        self.fresh = true;
        Ok(State::Running {
            subscription,
            recording,
            _archive: archive,
        })
    }
}

/// Asking a connected archive for the service's recordings, through its
/// own proxy and descriptor poller: Aeron's `list_recordings_for_uri` in
/// two halves, sending now and polling for the answer. The newest still
/// recording (no stop position) on the service's control port is its
/// current session's.
struct Listing {
    // Field order is drop order: the archive is closed before the consumer
    // it may call.
    _archive: AeronArchive,
    poller: AeronArchiveRecordingDescriptorPoller,
    _consumer: Handler<Box<dyn FnMut(AeronArchiveRecordingDescriptor)>>,
    channel: String,
    newest: Arc<AtomicI64>,
    until: Instant,
}

impl Listing {
    fn start(
        archive: AeronArchive,
        channel: &str,
        stream_id: i32,
        port: u16,
    ) -> Result<State, String> {
        let newest = Arc::new(AtomicI64::new(-1));
        let (found, port) = (Arc::clone(&newest), format!(":{port}"));
        let consumer: Handler<Box<dyn FnMut(AeronArchiveRecordingDescriptor)>> =
            Handler::new(Box::new(move |d: AeronArchiveRecordingDescriptor| {
                if d.stop_position() < 0 && d.original_channel().contains(&port) {
                    found.fetch_max(d.recording_id(), Ordering::Relaxed);
                }
            }));
        let inner = archive.get_inner_ref();
        let proxy = AeronArchiveProxy::from(inner.archive_proxy);
        let poller = AeronArchiveRecordingDescriptorPoller::from(inner.recording_descriptor_poller);
        let correlation = archive.next_correlation_id();
        poller.reset(correlation, i32::MAX, Some(&consumer));
        if !proxy.list_recordings_for_uri(correlation, 0, i32::MAX, c"aeron", stream_id) {
            return Err("listing its recordings: the request was not sent".into());
        }
        Ok(State::Listing(Box::new(Self {
            _archive: archive,
            poller,
            _consumer: consumer,
            channel: channel.to_owned(),
            newest,
            until: Instant::now() + PersistentSubscription::STEP,
        })))
    }

    /// The recording once the list is complete.
    fn poll(&self, now: Instant) -> Result<Option<i64>, String> {
        self.poller
            .poll()
            .map_err(|e| format!("listing its recordings: {e}"))?;
        if !self.poller.is_dispatch_complete() {
            return if now >= self.until {
                Err("listing its recordings: timed out".into())
            } else {
                Ok(None)
            };
        }
        match self.newest.load(Ordering::Relaxed) {
            -1 => Err("no recording of its current session yet".into()),
            recording => Ok(Some(recording)),
        }
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
