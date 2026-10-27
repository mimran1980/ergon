//! The [`AeronCluster`] client — owns the Aeron transport and drives the
//! full SBE session handshake. Java-parity entry point for the **Ergo Aeron
//! Cluster** client (experimental prototype on `rusteron-client`):
//!
//! → [`cluster-test-harness/tests/connect_to_cluster.rs`](https://github.com/mimran1980/ergon/blob/main/cluster-test-harness/tests/connect_to_cluster.rs)
//!
//! Hot path: [`AeronCluster::try_claim`] writes SessionMessageHeader via ergo-sbe
//! into the claim; fill app payload then [`ClusterClaim::commit`].

use std::ffi::{CStr, CString};
use std::io::Write;
use std::time::{Duration, Instant};

use rusteron_client::{
    Aeron, AeronAsyncAddExclusivePublication, AeronAsyncAddSubscription, AeronClaim, AeronContext,
    AeronExclusivePublication, AeronFragmentClosureAssembler, AeronImage, AeronSubscription, Handlers, IdleStrategy,
    cformat,
};

use crate::protocol::{self, HandshakeEvent};
use crate::uri;

use crate::codecs::session::{
    AdminRequestEncoder, AdminRequestFixedFields, AdminRequestType, SessionCloseRequestEncoder,
    SessionKeepAliveEncoder, SessionMessageHeaderEncoder,
};
/// Re-offer interval for SessionConnectRequest — roughly `message_timeout / 4`,
/// clamped to [50, 1000] ms (mirrors Java `AeronCluster.AsyncConnect`).
fn connect_reoffer_interval_ms(message_timeout_ms: u64) -> u64 {
    (message_timeout_ms / 4).clamp(50, 1_000)
}

/// Create a deadline from now + `timeout_ms` milliseconds, or return
/// [`ClusterError::InvalidTimeout`] if the timeout is zero or would overflow
/// [`Instant`].
fn checked_deadline(phase: &'static str, timeout_ms: u64) -> Result<Instant, ClusterError> {
    if timeout_ms == 0 {
        return Err(ClusterError::InvalidTimeout {
            phase,
            reason: "timeout must be positive",
        });
    }
    Instant::now()
        .checked_add(Duration::from_millis(timeout_ms))
        .ok_or(ClusterError::InvalidTimeout {
            phase,
            reason: "deadline exceeds Instant::MAX",
        })
}

use crate::egress::EgressListener;
use crate::error::ClusterError;
use crate::error::PublicationFailure;
use crate::state::SessionState;

/// Header-inclusive SessionMessageHeader (prefer generated `ENCODED_LENGTH`).
const MSG_HDR_TOTAL: usize = SessionMessageHeaderEncoder::ENCODED_LENGTH;

/// The ingress publication's `(max_message_length, max_payload_length)`.
/// Fails closed: a limit of 0 would read as a real limit to callers sizing
/// messages from [`AeronCluster::ingress_max_message_length`].
fn publication_limits(publication: &AeronExclusivePublication) -> Result<(usize, usize), ClusterError> {
    let constants = publication
        .get_constants()
        .map_err(|e| ClusterError::aeron("ingress_constants", e))?;
    Ok((constants.max_message_length, constants.max_payload_length))
}

/// The egress image a session is bound to: the one that delivered its
/// SessionEvent(OK) or latest NewLeaderEvent (Java `AeronCluster.egressImage`).
/// While a failover is in progress the old leader's image lingers beside the
/// new one, so only this image's closure means the session lost its leader.
struct EgressImage {
    image: AeronImage,
    correlation_id: i64,
    session_id: i32,
}

impl EgressImage {
    /// Retain the image for the session id of the fragment's header, or `None`
    /// when that image is already gone.
    fn bind(egress: &AeronSubscription, session_id: Option<i32>) -> Option<Self> {
        let session_id = session_id?;
        let image = egress.image_by_session_id(session_id)?;
        let correlation_id = image.get_constants().ok()?.correlation_id();
        Some(Self {
            image,
            correlation_id,
            session_id,
        })
    }
}

/// A NewLeaderEvent for this session, applied once the poll returns. The last
/// one in a poll wins, as Java applies each in order.
struct NewLeader {
    leadership_term_id: i64,
    leader_member_id: i32,
    ingress_endpoints: String,
    /// Session id of the egress image that delivered the event.
    image_session_id: Option<i32>,
}

/// The new leader's ingress publication while Aeron adds it and it connects.
/// The session adopts the new term and leader only once it has connected.
struct PendingIngress {
    /// `None` once the add has resolved or failed. A failed add leaves the
    /// session waiting out `message_timeout`, as Java's
    /// `AWAIT_NEW_LEADER_CONNECTION` does, unless another event comes.
    add: Option<AeronAsyncAddExclusivePublication>,
    added: Option<AeronExclusivePublication>,
    leadership_term_id: i64,
    leader_member_id: i32,
}

/// Context for the dispatch function — set up before each poll cycle.
struct PollCtx<'a, L: EgressListener> {
    listener: &'a mut L,
    new_leader: &'a mut Option<NewLeader>,
    decode_err: &'a mut Option<ClusterError>,
    expected_session_id: i64,
    session_closed: &'a mut bool,
}

fn dispatch_regular<L: EgressListener>(ctx: &mut PollCtx<L>, data: &[u8], hdr: rusteron_client::AeronHeader) {
    let reserved_value = hdr.reserved_value().unwrap_or(0);
    // Decode once — the Fragment is used for state tracking AND listener dispatch.
    let frag = match crate::fragment::Fragment::decode(data) {
        Ok(Some(f)) => f,
        Ok(None) => {
            if let Err(error) = crate::egress::dispatch_extension(ctx.listener, data)
                && ctx.decode_err.is_none()
            {
                *ctx.decode_err = Some(error);
            }
            return;
        }
        Err(e) => {
            std::hint::cold_path();
            if ctx.decode_err.is_none() {
                *ctx.decode_err = Some(e);
            }
            return;
        }
    };
    // State tracking — no second decode
    match &frag {
        crate::fragment::Fragment::NewLeader {
            leadership_term_id,
            leader_member_id,
            ingress_endpoints,
            cluster_session_id,
        } if *cluster_session_id == ctx.expected_session_id => {
            std::hint::cold_path();
            *ctx.new_leader = Some(NewLeader {
                leadership_term_id: *leadership_term_id,
                leader_member_id: *leader_member_id,
                ingress_endpoints: ingress_endpoints.to_string(),
                image_session_id: hdr.session_id(),
            });
        }
        crate::fragment::Fragment::SessionEvent {
            code: crate::codecs::session::EventCode::CLOSED,
            cluster_session_id,
            ..
        } if *cluster_session_id == ctx.expected_session_id => {
            std::hint::cold_path();
            *ctx.session_closed = true;
        }
        _ => {}
    }
    // Pass the already-decoded fragment — no second decode
    if let Err(e) = crate::egress::dispatch(ctx.listener, ctx.expected_session_id, frag, reserved_value)
        && ctx.decode_err.is_none()
    {
        *ctx.decode_err = Some(e);
    }
}

/// Java `AeronCluster.SEND_ATTEMPTS`.
const SEND_ATTEMPTS: usize = 3;

/// `offer_parts` accepts at most this many slices, header included.
const MAX_OFFER_PARTS: usize = 8;

/// One member's ingress publication while connect adds it and waits for it
/// to connect. Steady state keeps only the leader.
struct MemberIngress {
    endpoint: String,
    add: Option<AeronAsyncAddExclusivePublication>,
    publication: Option<AeronExclusivePublication>,
}

/// The connect request in flight. The same bytes are re-offered, so the
/// correlation id stays the one the cluster will echo. A challenge response
/// and a redirect each mint a new id.
struct ConnectAttempt {
    response_channel: Option<Vec<u8>>,
    correlation_id: Option<i64>,
    connect_bytes: Option<Vec<u8>>,
    challenge_bytes: Option<Vec<u8>>,
    challenged: bool,
    last_offer: Instant,
    reoffer: Duration,
    credentials: Vec<u8>,
    client_info: Vec<u8>,
    response_stream_id: i32,
}

impl ConnectAttempt {
    fn new(builder: &crate::SessionBuilder) -> Self {
        let reoffer = Duration::from_millis(connect_reoffer_interval_ms(builder.message_timeout_ms));
        Self {
            response_channel: None,
            correlation_id: None,
            connect_bytes: None,
            challenge_bytes: None,
            challenged: false,
            last_offer: Instant::now().checked_sub(reoffer).unwrap_or_else(Instant::now),
            reoffer,
            credentials: encoded_credentials(builder),
            client_info: protocol::client_info_bytes(builder_client_name(builder)),
            response_stream_id: builder.egress_stream_id,
        }
    }

    /// Encode the connect request once the response channel and a correlation
    /// id exist. A challenge in flight is left alone.
    fn prepare(&mut self, aeron: &Aeron) -> Result<(), ClusterError> {
        if self.challenged || self.connect_bytes.is_some() {
            return Ok(());
        }
        let Some(channel) = self.response_channel.as_deref() else {
            return Ok(());
        };
        if self.correlation_id.is_none() {
            self.correlation_id = Some(next_correlation_id(aeron)?);
        }
        let correlation_id = self
            .correlation_id
            .ok_or_else(|| ClusterError::connect("missing correlation id"))?;
        self.connect_bytes = Some(protocol::encode_connect_request(
            correlation_id,
            self.response_stream_id,
            channel,
            &self.credentials,
            &self.client_info,
        )?);
        Ok(())
    }

    fn offer_connect(&mut self, ingress: &AeronExclusivePublication) -> Result<bool, ClusterError> {
        if self.challenged {
            return Ok(false);
        }
        let Some(bytes) = self.connect_bytes.as_deref() else {
            return Ok(false);
        };
        if self.last_offer.elapsed() < self.reoffer {
            return Ok(false);
        }
        self.last_offer = Instant::now();
        offer_handshake(ingress, bytes, "connect")
    }

    fn offer_challenge(&mut self, ingress: &AeronExclusivePublication) -> Result<bool, ClusterError> {
        let Some(bytes) = self.challenge_bytes.as_deref() else {
            return Ok(true);
        };
        if offer_handshake(ingress, bytes, "challenge_response")? {
            self.challenge_bytes = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn on_redirect(&mut self) {
        self.correlation_id = None;
        self.connect_bytes = None;
        self.challenged = false;
        self.challenge_bytes = None;
        self.last_offer = Instant::now().checked_sub(self.reoffer).unwrap_or_else(Instant::now);
    }

    fn on_challenge(&mut self, aeron: &Aeron, cluster_session_id: i64, credentials: &[u8]) -> Result<(), ClusterError> {
        let correlation_id = next_correlation_id(aeron)?;
        self.correlation_id = Some(correlation_id);
        self.challenge_bytes = Some(protocol::encode_challenge_response(
            correlation_id,
            cluster_session_id,
            credentials,
        )?);
        self.challenged = true;
        self.connect_bytes = None;
        Ok(())
    }
}

fn builder_client_name(builder: &crate::SessionBuilder) -> &str {
    builder
        .client_name
        .as_deref()
        .and_then(|name| name.to_str().ok())
        .unwrap_or(protocol::DEFAULT_CLIENT_NAME)
}

fn encoded_credentials(builder: &crate::SessionBuilder) -> Vec<u8> {
    builder
        .credentials
        .as_ref()
        .and_then(|supplier| supplier.encoded_credentials().map(|cow| cow.into_owned()))
        .unwrap_or_default()
}

fn challenge_response_bytes(builder: &crate::SessionBuilder, encoded_challenge: &[u8]) -> Vec<u8> {
    builder
        .credentials
        .as_ref()
        .and_then(|supplier| supplier.on_challenge(encoded_challenge).map(|cow| cow.into_owned()))
        .unwrap_or_default()
}

fn next_correlation_id(aeron: &Aeron) -> Result<i64, ClusterError> {
    let id = aeron.next_correlation_id();
    if id < 0 {
        Err(ClusterError::connect("Aeron::next_correlation_id failed"))
    } else {
        Ok(id)
    }
}

/// `Ok(n)` is the resolved URI. `Err` means the wildcard port is not bound yet.
fn resolve_response_channel(egress: &AeronSubscription) -> Option<Vec<u8>> {
    let mut buf = [0u8; 4096];
    match egress.try_resolve_channel_endpoint_port(&mut buf) {
        Ok(n) if n > 0 => {
            let written = (n as usize).min(buf.len());
            let end = buf[..written].iter().position(|byte| *byte == 0).unwrap_or(written);
            (end > 0).then(|| buf[..end].to_vec())
        }
        _ => None,
    }
}

fn offer_handshake(
    ingress: &AeronExclusivePublication,
    bytes: &[u8],
    context: &'static str,
) -> Result<bool, ClusterError> {
    let result = ingress.offer_raw(bytes, Handlers::NONE);
    if result > 0 {
        return Ok(true);
    }
    let error = ClusterError::from_offer_raw(context, result);
    if error.is_retryable() { Ok(false) } else { Err(error) }
}

fn idle_once(idle: &Option<std::sync::Arc<std::sync::Mutex<dyn IdleStrategy + Send + Sync>>>) {
    if let Some(idle) = idle {
        idle.lock().expect("idle mutex poisoned").idle(0);
    } else {
        for _ in 0..10 {
            std::hint::spin_loop();
        }
        std::thread::sleep(Duration::from_micros(1));
    }
}

fn install_error_handler(ctx: &AeronContext, handler: Option<crate::config::ErrorHandler>) -> Result<(), ClusterError> {
    let callback = move |code: std::os::raw::c_int, message: &str| {
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(handler) = &handler {
                handler(code, message);
            } else {
                let _ = writeln!(std::io::stderr(), "ERROR: ({code}): {message}");
            }
        }))
        .is_err();
        if panicked {
            let _ = writeln!(std::io::stderr(), "ERROR: ({code}): error handler panicked");
        }
    };
    ctx.set_error_handler(Some(callback))
        .map_err(|error| ClusterError::aeron("set_error_handler", error))?;
    Ok(())
}

/// Clone an injected client, or create one in `aeron_dir`. An injected client
/// is not started and keeps whatever name and error handler it already has.
fn create_aeron(builder: &crate::SessionBuilder, aeron_dir: &str) -> Result<Aeron, ClusterError> {
    if let Some(aeron) = &builder.shared_aeron {
        return Ok(aeron.clone());
    }
    let dir = cformat!("{aeron_dir}");
    let ctx = AeronContext::new().map_err(|error| ClusterError::aeron("AeronContext", error))?;
    ctx.set_dir(&dir)
        .map_err(|error| ClusterError::aeron("set_dir", error))?;
    match &builder.client_name {
        Some(name) => ctx.set_client_name(name),
        None => ctx.set_client_name(c"cluster-client"),
    }
    .map_err(|error| ClusterError::aeron("set_client_name", error))?;
    install_error_handler(&ctx, builder.error_handler.clone())?;
    let aeron = Aeron::new(&ctx).map_err(|error| ClusterError::aeron("Aeron::new", error))?;
    aeron
        .start()
        .map_err(|error| ClusterError::aeron("Aeron::start", error))?;
    Ok(aeron)
}

fn open_member(aeron: &Aeron, builder: &crate::SessionBuilder, endpoint: &str) -> Result<MemberIngress, ClusterError> {
    let channel = uri::ingress_channel_for_endpoint(builder.ingress_channel_c_str(), endpoint)?;
    let add = aeron
        .async_add_exclusive_publication(&channel, builder.ingress_stream_id)
        .map_err(|error| ClusterError::reconnect(format!("ingress to {endpoint}: {error}")))?;
    Ok(MemberIngress {
        endpoint: endpoint.to_owned(),
        add: Some(add),
        publication: None,
    })
}

fn poll_member_adds(members: &mut [MemberIngress]) -> Result<(), ClusterError> {
    for member in members {
        if member.publication.is_some() {
            continue;
        }
        let Some(add) = member.add.as_ref() else {
            continue;
        };
        if let Some(publication) = add.poll().map_err(|error| ClusterError::aeron("pub", error))? {
            member.publication = Some(publication);
            member.add = None;
        }
    }
    Ok(())
}

/// The connected publication whose endpoint matches `preferred`, when set.
/// The publication is taken out of `members` so it is not closed with the rest.
fn take_connected(
    members: &mut [MemberIngress],
    preferred: Option<&str>,
) -> Option<(String, AeronExclusivePublication)> {
    let index = members.iter().position(|member| {
        let connected = member
            .publication
            .as_ref()
            .is_some_and(AeronExclusivePublication::is_connected);
        connected && preferred.is_none_or(|endpoint| member.endpoint == endpoint)
    })?;
    let member = &mut members[index];
    let publication = member.publication.take()?;
    Some((member.endpoint.clone(), publication))
}

fn poll_subscription(
    egress: &AeronSubscription,
) -> Result<Option<(crate::poller::EgressEvent, Option<i32>)>, ClusterError> {
    let mut event = None;
    let mut error = None;
    egress
        .poll_fn(
            |data, header| {
                if event.is_none() && error.is_none() {
                    match crate::poller::parse_event(data) {
                        Ok(parsed) => event = Some((parsed, header.session_id())),
                        Err(parse_error) => error = Some(parse_error),
                    }
                }
            },
            1,
        )
        .map_err(|aeron_error| ClusterError::aeron("connect_poll", aeron_error))?;
    if let Some(parse_error) = error {
        return Err(parse_error);
    }
    Ok(event)
}

fn prepare_attempt(attempt: &mut ConnectAttempt, aeron: Option<&Aeron>) -> Result<(), ClusterError> {
    if let Some(aeron) = aeron {
        attempt.prepare(aeron)?;
    }
    Ok(())
}

fn offer_connect_on(
    attempt: &mut ConnectAttempt,
    ingress: Option<&AeronExclusivePublication>,
) -> Result<bool, ClusterError> {
    match ingress {
        Some(ingress) => attempt.offer_connect(ingress),
        None => Ok(false),
    }
}

fn offer_challenge_on(
    attempt: &mut ConnectAttempt,
    ingress: Option<&AeronExclusivePublication>,
) -> Result<bool, ClusterError> {
    match ingress {
        Some(ingress) => attempt.offer_challenge(ingress),
        None => Ok(false),
    }
}

fn begin_challenge(
    attempt: &mut ConnectAttempt,
    aeron: Option<&Aeron>,
    cluster_session_id: i64,
    credentials: &[u8],
) -> Result<(), ClusterError> {
    let aeron = aeron.ok_or_else(|| ClusterError::connect("no aeron client for challenge"))?;
    attempt.on_challenge(aeron, cluster_session_id, credentials)
}

/// A connected Aeron cluster client. Owns the Aeron client, ingress
/// exclusive publication, and egress subscription.
pub struct AeronCluster {
    /// Declared first so the retained image is released before `egress`.
    egress_image: Option<EgressImage>,
    pending_ingress: Option<PendingIngress>,
    _aeron: Aeron,
    ingress: AeronExclusivePublication,
    egress: AeronSubscription,
    cluster_session_id: i64,
    leadership_term_id: i64,
    leader_member_id: i32,
    state: SessionState,
    /// Wall-clock when the session entered `AwaitingNewLeader` (None while
    /// Connected). Drives the `new_leader_timeout` enforced by
    /// [`Self::poll`].
    awaiting_leader_since: Option<Instant>,
    /// Configured ingress stream id, retained so a `NewLeaderEvent`
    /// reconnect uses the same stream the session was established on.
    ingress_stream_id: i32,
    /// Configured ingress channel, the template a new leader's publication
    /// keeps every setting of but the endpoint.
    ingress_channel: Option<CString>,
    /// `newLeaderTimeout` from the builder (ms). Enforced by
    /// [`Self::poll`].
    new_leader_timeout_ms: u64,
    /// `messageTimeout` from the builder (ms): how long a new leader's
    /// ingress publication may take to connect.
    message_timeout_ms: u64,
    /// When the last keep-alive was sent (drives auto-scheduling).
    last_keep_alive: Instant,
    /// Interval between keep-alive sends (derived from message_timeout_ms).
    keep_alive_interval_ms: u64,
    /// Cached from the ingress publication's constants — the maximum total
    /// frame length (header + payload) that Aeron will accept in one offer.
    max_message_length: usize,
    /// Cached from the ingress publication's constants — the maximum payload
    /// that fits in a single claim (excludes the 32-byte session header).
    max_payload_length: usize,
    /// rusteron fragment assembler for regular egress.
    regular_assembler: AeronFragmentClosureAssembler,
}

impl Drop for AeronCluster {
    fn drop(&mut self) {
        if self.state != SessionState::Closed && self.state != SessionState::PendingClose {
            let _ = self.close();
        }
    }
}

impl AeronCluster {
    /// Bytes of SessionMessageHeader every application message carries ahead
    /// of its payload (Java `AeronCluster.SESSION_HEADER_LENGTH`).
    pub const SESSION_HEADER_LENGTH: usize = MSG_HDR_TOTAL;

    /// Connect to a cluster whose media driver lives at `aeron_dir`.
    ///
    /// Performs the SBE handshake: `SessionConnectRequest`, then egress until
    /// `SessionEvent(OK)`. When ingress endpoints are set, a publication to
    /// every member is added together and the first connected member is
    /// offered the request. A single ingress channel keeps the blocking add.
    pub fn connect(builder: &crate::SessionBuilder, aeron_dir: &str) -> Result<Self, ClusterError> {
        builder.validate()?;
        if builder.ingress_endpoints.is_some() {
            Self::connect_fanout(builder, aeron_dir)
        } else {
            Self::connect_single(builder, aeron_dir)
        }
    }

    fn connect_single(builder: &crate::SessionBuilder, aeron_dir: &str) -> Result<Self, ClusterError> {
        let aeron = create_aeron(builder, aeron_dir)?;
        let egress_channel = uri::egress_channel_disabling_rejoin(builder.egress_for_aeron()?)?;
        let ingress_channel = builder.resolve_initial_ingress_for_aeron()?;
        let egress = aeron
            .add_subscription(
                &egress_channel,
                builder.egress_stream_id,
                Handlers::NONE,
                Handlers::NONE,
                Duration::from_secs(5),
            )
            .map_err(|error| ClusterError::aeron("add_subscription", error))?;
        let ingress = aeron
            .add_exclusive_publication(&ingress_channel, builder.ingress_stream_id, Duration::from_secs(5))
            .map_err(|error| ClusterError::aeron("add_exclusive_publication", error))?;
        let (max_message_length, max_payload_length) = publication_limits(&ingress)?;
        let mut client = Self {
            egress_image: None,
            pending_ingress: None,
            _aeron: aeron,
            ingress,
            egress,
            cluster_session_id: -1,
            leadership_term_id: -1,
            leader_member_id: -1,
            state: SessionState::Closed,
            awaiting_leader_since: None,
            ingress_stream_id: builder.ingress_stream_id,
            ingress_channel: builder.ingress_channel_c_str().map(CStr::to_owned),
            new_leader_timeout_ms: protocol::conclude_new_leader_timeout_ms(builder.new_leader_timeout_ms, None),
            message_timeout_ms: builder.message_timeout_ms,
            last_keep_alive: Instant::now(),
            keep_alive_interval_ms: connect_reoffer_interval_ms(builder.message_timeout_ms),
            max_message_length,
            max_payload_length,
            regular_assembler: AeronFragmentClosureAssembler::new()
                .map_err(|error| ClusterError::aeron("AeronFragmentClosureAssembler", error))?,
        };
        client.handshake(builder)?;
        Ok(client)
    }

    /// Publications to every ingress endpoint, one deadline for the adds and
    /// the handshake. Only the leader's publication is kept.
    fn connect_fanout(builder: &crate::SessionBuilder, aeron_dir: &str) -> Result<Self, ClusterError> {
        let aeron = create_aeron(builder, aeron_dir)?;
        let egress_channel = uri::egress_channel_disabling_rejoin(builder.egress_for_aeron()?)?;
        let pending_egress = aeron
            .async_add_subscription(
                &egress_channel,
                builder.egress_stream_id,
                Handlers::NONE,
                Handlers::NONE,
            )
            .map_err(|error| ClusterError::aeron("sub", error))?;
        let map = builder.ingress_endpoints.as_deref().unwrap_or("");
        let mut members = Vec::new();
        for endpoint in crate::endpoints::parse_ingress_endpoints(map)? {
            members.push(open_member(&aeron, builder, &endpoint.endpoint)?);
        }
        let regular_assembler = AeronFragmentClosureAssembler::new()
            .map_err(|error| ClusterError::aeron("AeronFragmentClosureAssembler", error))?;
        let mut egress = None;
        let mut ingress = None;
        let mut active_endpoint = None;
        let mut preferred = None;
        let mut attempt = ConnectAttempt::new(builder);
        let deadline = checked_deadline("connect", builder.message_timeout_ms)?;
        let idle = builder.idle.clone();
        let mut established = None;
        while Instant::now() < deadline && established.is_none() {
            if egress.is_none()
                && let Some(subscription) = pending_egress
                    .poll()
                    .map_err(|error| ClusterError::aeron("sub", error))?
            {
                egress = Some(subscription);
            }
            poll_member_adds(&mut members)?;
            if ingress.is_none()
                && let Some((endpoint, publication)) = take_connected(&mut members, preferred.as_deref())
            {
                active_endpoint = Some(endpoint);
                ingress = Some(publication);
            }
            if attempt.response_channel.is_none()
                && let Some(subscription) = &egress
            {
                attempt.response_channel = resolve_response_channel(subscription);
            }
            attempt.prepare(&aeron)?;
            if let Some(publication) = ingress.as_ref() {
                let _ = attempt.offer_connect(publication)?;
                if attempt.challenge_bytes.is_some() {
                    let _ = attempt.offer_challenge(publication)?;
                }
            }
            if let Some(subscription) = &egress
                && let Some((event, image_session_id)) = poll_subscription(subscription)?
                && let Some(correlation_id) = attempt.correlation_id
            {
                match protocol::classify_handshake(event, correlation_id, image_session_id)? {
                    HandshakeEvent::Established {
                        cluster_session_id,
                        leadership_term_id,
                        leader_member_id,
                        leader_heartbeat_timeout_ns,
                        image_session_id,
                    } => {
                        established = Some((
                            cluster_session_id,
                            leadership_term_id,
                            leader_member_id,
                            leader_heartbeat_timeout_ns,
                            image_session_id,
                        ));
                    }
                    HandshakeEvent::AuthRejected => return Err(ClusterError::AuthRejected),
                    HandshakeEvent::Redirect { endpoint } => {
                        preferred = Some(endpoint.clone());
                        if active_endpoint.as_deref() != Some(endpoint.as_str()) {
                            ingress = None;
                            active_endpoint = None;
                        }
                        members.retain(|member| member.endpoint == endpoint);
                        let held = ingress.is_some() || members.iter().any(|member| member.endpoint == endpoint);
                        if !held {
                            members.push(open_member(&aeron, builder, &endpoint)?);
                        }
                        attempt.on_redirect();
                    }
                    HandshakeEvent::Challenge {
                        cluster_session_id,
                        encoded_challenge,
                    } => {
                        let response = challenge_response_bytes(builder, &encoded_challenge);
                        attempt.on_challenge(&aeron, cluster_session_id, &response)?;
                    }
                    HandshakeEvent::Ignore => {}
                }
            }
            if established.is_none() {
                idle_once(&idle);
            }
        }
        let Some((
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            leader_heartbeat_timeout_ns,
            image_session_id,
        )) = established
        else {
            return Err(ClusterError::ConnectFailed {
                reason: "timeout waiting for SessionEvent(OK)".into(),
            });
        };
        drop(members);
        let ingress =
            ingress.ok_or_else(|| ClusterError::connect("fan-out finished without an ingress publication"))?;
        let egress = egress.ok_or_else(|| ClusterError::connect("fan-out finished without an egress subscription"))?;
        let (max_message_length, max_payload_length) = publication_limits(&ingress)?;
        Ok(Self {
            egress_image: EgressImage::bind(&egress, image_session_id),
            pending_ingress: None,
            _aeron: aeron,
            ingress,
            egress,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            state: SessionState::Connected,
            awaiting_leader_since: None,
            ingress_stream_id: builder.ingress_stream_id,
            ingress_channel: builder.ingress_channel_c_str().map(CStr::to_owned),
            new_leader_timeout_ms: protocol::conclude_new_leader_timeout_ms(
                builder.new_leader_timeout_ms,
                leader_heartbeat_timeout_ns,
            ),
            message_timeout_ms: builder.message_timeout_ms,
            last_keep_alive: Instant::now(),
            keep_alive_interval_ms: connect_reoffer_interval_ms(builder.message_timeout_ms),
            max_message_length,
            max_payload_length,
            regular_assembler,
        })
    }

    /// Resolve the egress port, offer one correlated connect request, and
    /// accept only the `SessionEvent` or `Challenge` that echoes that id.
    ///
    /// A redirect replaces the ingress publication and mints a new id. A
    /// challenge response also mints a new id; the original connect is not
    /// offered again. The same bytes are re-offered on
    /// [`connect_reoffer_interval_ms`] until the deadline.
    fn handshake(&mut self, builder: &crate::SessionBuilder) -> Result<(), ClusterError> {
        let mut attempt = ConnectAttempt::new(builder);
        let deadline = checked_deadline("connect", builder.message_timeout_ms)?;
        let idle = builder.idle.clone();
        while Instant::now() < deadline {
            if attempt.response_channel.is_none() {
                attempt.response_channel = resolve_response_channel(&self.egress);
            }
            attempt.prepare(&self._aeron)?;
            let _ = attempt.offer_connect(&self.ingress)?;
            if attempt.challenge_bytes.is_some() {
                let _ = attempt.offer_challenge(&self.ingress)?;
            }
            if let Some((event, image_session_id)) = poll_subscription(&self.egress)?
                && let Some(correlation_id) = attempt.correlation_id
            {
                match protocol::classify_handshake(event, correlation_id, image_session_id)? {
                    HandshakeEvent::Established {
                        cluster_session_id,
                        leadership_term_id,
                        leader_member_id,
                        leader_heartbeat_timeout_ns,
                        image_session_id,
                    } => {
                        self.cluster_session_id = cluster_session_id;
                        self.leadership_term_id = leadership_term_id;
                        self.leader_member_id = leader_member_id;
                        self.new_leader_timeout_ms = protocol::conclude_new_leader_timeout_ms(
                            builder.new_leader_timeout_ms,
                            leader_heartbeat_timeout_ns,
                        );
                        self.egress_image = EgressImage::bind(&self.egress, image_session_id);
                        self.state = SessionState::Connected;
                        self.last_keep_alive = Instant::now();
                        return Ok(());
                    }
                    HandshakeEvent::AuthRejected => return Err(ClusterError::AuthRejected),
                    HandshakeEvent::Redirect { endpoint } => {
                        self.reconnect_ingress(builder, &endpoint)?;
                        attempt.on_redirect();
                    }
                    HandshakeEvent::Challenge {
                        cluster_session_id,
                        encoded_challenge,
                    } => {
                        let response = challenge_response_bytes(builder, &encoded_challenge);
                        attempt.on_challenge(&self._aeron, cluster_session_id, &response)?;
                    }
                    HandshakeEvent::Ignore => {}
                }
            }
            idle_once(&idle);
        }
        Err(ClusterError::ConnectFailed {
            reason: "timeout waiting for SessionEvent(OK)".into(),
        })
    }

    /// Recreate the ingress publication pointed at the leader's endpoint
    /// (blocking connect's REDIRECT), keeping the configured channel settings.
    fn reconnect_ingress(&mut self, builder: &crate::SessionBuilder, endpoint: &str) -> Result<(), ClusterError> {
        let channel = uri::ingress_channel_for_endpoint(builder.ingress_channel_c_str(), endpoint)?;
        let ingress = self
            ._aeron
            .add_exclusive_publication(&channel, builder.ingress_stream_id, Duration::from_secs(5))
            .map_err(|e| ClusterError::reconnect(format!("ingress to {endpoint}: {e}")))?;
        let (max_message_length, max_payload_length) = publication_limits(&ingress)?;
        self.ingress = ingress;
        self.max_message_length = max_message_length;
        self.max_payload_length = max_payload_length;
        Ok(())
    }

    /// Publish an application message. Prepends the SessionMessageHeader
    /// (leadershipTermId + clusterSessionId + timestamp).
    ///
    /// Messages that fit in one term-buffer fragment use the zero-alloc
    /// [`Self::try_claim`] path; larger messages fall back to an assembled
    /// header+payload buffer via `offer_raw` (Aeron handles transport-level
    /// fragmentation automatically). Returns the stream **position** assigned
    /// by Aeron, not the payload length. Mirrors Java
    /// `AeronCluster.offer(buffer)`.
    pub fn offer(&mut self, payload: &[u8]) -> Result<i64, ClusterError> {
        if self.state != SessionState::Connected {
            std::hint::cold_path();
            return Err(ClusterError::NotConnected);
        }
        let total = MSG_HDR_TOTAL
            .checked_add(payload.len())
            .ok_or(ClusterError::PayloadTooLarge {
                operation: "offer",
                requested: payload.len(),
                maximum: self.max_message_length,
            })?;
        // Reject frames that exceed the publication's max message length
        // before making any Aeron call.
        if total > self.max_message_length {
            std::hint::cold_path();
            return Err(ClusterError::PayloadTooLarge {
                operation: "offer",
                requested: total,
                maximum: self.max_message_length,
            });
        }
        // Fast path: zero-alloc claim when the header+payload fits in one fragment.
        if total <= self.max_payload_length {
            return match self.try_claim(payload.len()) {
                Ok(mut claim) => {
                    claim.payload_mut().copy_from_slice(payload);
                    claim.commit().map_err(|e| self.track_publication_error(e))
                }
                Err(e) => Err(e),
            };
        }
        // Fallback: gather header + payload in two slices via offer_parts.
        // Aeron fragments automatically; no heap allocation, no payload copy.
        let mut header = [0u8; MSG_HDR_TOTAL];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut header, 0)
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id)
            .timestamp(0);
        let r = self
            .ingress
            .offer_parts(&[&header, payload])
            .map_err(|e| ClusterError::from_offer_error("offer", e));
        self.track_ingress_publication_result_offer(r)
    }

    /// Publish `parts` as one message, session header first.
    ///
    /// The header is built on the stack and offered with the parts. No heap
    /// allocation. An empty `parts` sends the header alone. More than seven
    /// parts is [`PublicationFailure::TooManyParts`], because the header
    /// uses one of the eight gather slots.
    ///
    /// # Errors
    ///
    /// [`ClusterError::NotConnected`] when the session is not connected,
    /// [`ClusterError::PayloadTooLarge`] when the gathered length exceeds the
    /// publication, and [`ClusterError::Publication`] when the offer does not
    /// land.
    pub fn offer_vectors(&mut self, parts: &[&[u8]]) -> Result<i64, ClusterError> {
        if self.state != SessionState::Connected {
            std::hint::cold_path();
            return Err(ClusterError::NotConnected);
        }
        if parts.len() >= MAX_OFFER_PARTS {
            std::hint::cold_path();
            return Err(ClusterError::Publication {
                failure: PublicationFailure::TooManyParts,
                context: "offer_vectors",
            });
        }
        let mut header = [0u8; MSG_HDR_TOTAL];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut header, 0)
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id)
            .timestamp(0);
        let mut gathered: [&[u8]; MAX_OFFER_PARTS] = [&[]; MAX_OFFER_PARTS];
        gathered[0] = &header;
        let mut total = MSG_HDR_TOTAL;
        for (index, part) in parts.iter().enumerate() {
            total = total.checked_add(part.len()).ok_or(ClusterError::PayloadTooLarge {
                operation: "offer_vectors",
                requested: part.len(),
                maximum: self.max_message_length,
            })?;
            gathered[index + 1] = part;
        }
        if total > self.max_message_length {
            std::hint::cold_path();
            return Err(ClusterError::PayloadTooLarge {
                operation: "offer_vectors",
                requested: total,
                maximum: self.max_message_length,
            });
        }
        let result = self
            .ingress
            .offer_parts(&gathered[..=parts.len()])
            .map_err(|error| ClusterError::from_offer_error("offer_vectors", error));
        self.track_ingress_publication_result_offer(result)
    }

    /// Send a SessionKeepAlive to hold the session open.
    ///
    /// Encoded into a stack array sized to the generated
    /// [`SessionKeepAliveEncoder::ENCODED_LENGTH`] — no heap allocation.
    /// Retries a back-pressured offer up to three times, as Java's
    /// `AeronCluster.SEND_ATTEMPTS` does. [`Self::poll`]
    /// does not call this; a caller that only polls loses the session when
    /// the cluster's message timeout elapses.
    pub fn send_keep_alive(&mut self) -> Result<(), ClusterError> {
        let mut buf = [0u8; SessionKeepAliveEncoder::ENCODED_LENGTH];
        SessionKeepAliveEncoder::wrap_and_apply_header(&mut buf, 0)
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id);
        let mut last_error = None;
        for attempt in 0..SEND_ATTEMPTS {
            let result = self.ingress.offer_raw(&buf, Handlers::NONE);
            if result > 0 {
                self.last_keep_alive = Instant::now();
                return Ok(());
            }
            let failure = PublicationFailure::from_raw(result);
            self.note_ingress_failure(failure);
            let error = ClusterError::Publication {
                failure,
                context: "keep_alive",
            };
            if !error.is_retryable() || self.state != SessionState::Connected {
                return Err(error);
            }
            last_error = Some(error);
            if attempt + 1 < SEND_ATTEMPTS {
                std::hint::spin_loop();
            }
        }
        Err(last_error.unwrap_or(ClusterError::NotConnected))
    }

    /// Send a keep-alive when the interval has elapsed.
    ///
    /// [`Self::poll`] does not call this. Call it from the same loop
    /// that polls egress.
    #[inline]
    pub fn keep_alive_if_due(&mut self) -> Result<(), ClusterError> {
        let now = Instant::now();
        if self.state == SessionState::Connected
            && now.saturating_duration_since(self.last_keep_alive) >= Duration::from_millis(self.keep_alive_interval_ms)
        {
            self.send_keep_alive()?;
            self.last_keep_alive = now;
        }
        Ok(())
    }

    /// Apply [`protocol::ingress_failure_state`] for one publication failure.
    #[cold]
    fn note_ingress_failure(&mut self, failure: PublicationFailure) {
        let next = protocol::ingress_failure_state(self.state, failure);
        if next == self.state {
            return;
        }
        match next {
            SessionState::PendingClose => {
                let _ = self.close();
            }
            SessionState::AwaitingNewLeader => {
                self.state = SessionState::AwaitingNewLeader;
                self.awaiting_leader_since = Some(Instant::now());
            }
            _ => {}
        }
    }

    /// Apply a typed `ClusterError::Publication` from `offer` / `try_claim`
    /// to the session state (Java `trackIngressPublicationResult`).
    #[cold]
    fn track_publication_error(&mut self, err: ClusterError) -> ClusterError {
        if let ClusterError::Publication { failure, .. } = &err {
            self.note_ingress_failure(*failure);
        }
        err
    }

    /// Track the result of an `offer_parts` call.
    fn track_ingress_publication_result_offer(
        &mut self,
        result: Result<i64, ClusterError>,
    ) -> Result<i64, ClusterError> {
        match result {
            Ok(pos) => Ok(pos),
            Err(error) => Err(self.track_publication_error(error)),
        }
    }

    /// Send an AdminRequest (e.g. snapshot) on the ingress publication.
    ///
    /// Responses arrive as admin events on the egress listener
    /// ([`crate::EgressListener::on_admin_response`]).
    fn send_admin_request(
        &mut self,
        correlation_id: i64,
        request_type: AdminRequestType,
        payload: &[u8],
    ) -> Result<i64, ClusterError> {
        if self.state != SessionState::Connected {
            std::hint::cold_path();
            return Err(ClusterError::NotConnected);
        }
        let len = AdminRequestEncoder::compute_encoded_length_with_message_header(payload.len());
        let mut claim = self
            .ingress
            .try_claim_owned(len)
            .map_err(|error| self.track_publication_error(ClusterError::from_offer_error("admin_request", error)))?;
        if claim.data().len() < len {
            std::hint::cold_path();
            return Err(ClusterError::BufferTooSmall {
                needed: len,
                actual: claim.data().len(),
            });
        }
        let _written = AdminRequestEncoder::wrap_and_apply_header(claim.data(), 0)
            .fixed(&AdminRequestFixedFields {
                leadership_term_id: self.leadership_term_id,
                cluster_session_id: self.cluster_session_id,
                correlation_id,
                request_type,
            })
            .payload(payload)?
            .as_bytes_with_header()
            .len();
        claim
            .commit()
            .map_err(|error| ClusterError::aeron("admin_request", error))
    }

    /// Send a SessionCloseRequest and mark the session PendingClose.
    pub fn close(&mut self) -> Result<(), ClusterError> {
        if self.state == SessionState::Closed {
            return Err(ClusterError::SessionClosed);
        }
        let mut buf = [0u8; SessionCloseRequestEncoder::HEADER_LENGTH + SessionCloseRequestEncoder::BLOCK_LENGTH];
        SessionCloseRequestEncoder::wrap_and_apply_header(&mut buf, 0)
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id);
        // Local close always proceeds: the SessionCloseRequest is an advisory
        // notification (mirrors Java AeronCluster.close()). A failed offer must
        // not block local cleanup or leave the session half-closed, so set the
        // state first and send best-effort.
        self.state = SessionState::PendingClose;
        self.pending_ingress = None;
        let _ = self.ingress.offer_raw(&buf, Handlers::NONE);
        Ok(())
    }

    /// The one poll: egress fragments to `listener`, then session state.
    ///
    /// Reassembles fragments into whole messages, decodes each once, and
    /// calls `listener` for this session's messages only (another session's
    /// are dropped). Then drives the session state (Java `pollStateChanges`):
    /// a `NewLeaderEvent` starts adding the new leader's ingress publication
    /// without waiting, and a later poll adopts the new term and leader once
    /// it has connected; it also detects leader loss, enforces
    /// `new_leader_timeout` (after a `NewLeaderEvent`, `message_timeout`),
    /// and finishes `PendingClose` → `Closed`. Never blocks.
    ///
    /// Call it every loop iteration, with [`Self::keep_alive_if_due`]. A
    /// `fragment_limit` of 0 drives the state alone. Returns the fragments
    /// polled.
    ///
    /// # Errors
    ///
    /// A malformed frame ([`ClusterError::ProtocolError`]), a listener panic
    /// ([`ClusterError::ListenerPanicked`]), an Aeron poll failure, or a
    /// session-state failure (leader lost past its timeout, a failed
    /// reconnect).
    pub fn poll<L: EgressListener>(&mut self, listener: &mut L, fragment_limit: usize) -> Result<i32, ClusterError> {
        let mut new_leader: Option<NewLeader> = None;
        let mut decode_err: Option<ClusterError> = None;
        let mut session_closed = false;
        let mut ctx = PollCtx {
            listener,
            new_leader: &mut new_leader,
            decode_err: &mut decode_err,
            expected_session_id: self.cluster_session_id,
            session_closed: &mut session_closed,
        };
        let n = self
            .regular_assembler
            .poll(&self.egress, &mut ctx, dispatch_regular::<L>, fragment_limit)
            .map_err(|e| ClusterError::aeron("poll", e))?;
        self.apply_polled(session_closed, new_leader, decode_err)?;
        Ok(n)
    }

    /// Apply what one egress poll saw. A CLOSED for this session ends it first,
    /// whatever the state (Java sets `PENDING_CLOSE` unconditionally), so
    /// neither an error below nor a NewLeaderEvent in the same poll can lose
    /// or undo it.
    fn apply_polled(
        &mut self,
        session_closed: bool,
        new_leader: Option<NewLeader>,
        decode_err: Option<ClusterError>,
    ) -> Result<(), ClusterError> {
        if session_closed && matches!(self.state, SessionState::Connected | SessionState::AwaitingNewLeader) {
            self.state = SessionState::PendingClose;
            self.pending_ingress = None;
        }
        if let Some(event) = new_leader {
            self.on_new_leader_event(event)?;
        }
        if let Some(e) = decode_err {
            std::hint::cold_path();
            return Err(e);
        }
        self.poll_state()
    }

    /// Handle a `NewLeaderEvent` without blocking (Java `onNewLeader`, whose
    /// publication add waits; this one is polled). Every fallible step runs
    /// before any session field changes. The session stays
    /// `AwaitingNewLeader`, with `message_timeout` from now, until
    /// [`Self::poll_pending_ingress`] swaps the connected publication in. A
    /// later event replaces a pending add. The fragment assemblers are kept:
    /// they buffer per session id, so the old leader's leftovers are harmless.
    #[cold]
    fn on_new_leader_event(&mut self, event: NewLeader) -> Result<(), ClusterError> {
        if !matches!(self.state, SessionState::Connected | SessionState::AwaitingNewLeader) {
            return Ok(());
        }
        let NewLeader {
            leadership_term_id,
            leader_member_id,
            ingress_endpoints,
            image_session_id,
        } = event;
        let endpoint =
            crate::poller::parse_leader_endpoint(&ingress_endpoints, leader_member_id)?.ok_or_else(|| {
                ClusterError::ReconnectFailed {
                    reason: format!(
                        "NewLeaderEvent listed no endpoint for leader member {leader_member_id}: {ingress_endpoints}"
                    ),
                }
            })?;
        let channel = uri::ingress_channel_for_endpoint(self.ingress_channel.as_deref(), &endpoint)?;
        let publication = self
            ._aeron
            .async_add_exclusive_publication(&channel, self.ingress_stream_id)
            .map_err(|e| ClusterError::reconnect(format!("ingress to {endpoint}: {e}")))?;
        self.pending_ingress = Some(PendingIngress {
            add: Some(publication),
            added: None,
            leadership_term_id,
            leader_member_id,
        });
        self.egress_image = EgressImage::bind(&self.egress, image_session_id);
        self.state = SessionState::AwaitingNewLeader;
        self.awaiting_leader_since = Some(Instant::now());
        Ok(())
    }

    /// Swap in the new leader's ingress publication once Aeron has added it
    /// and it has connected (Java `AWAIT_NEW_LEADER_CONNECTION`), adopting its
    /// term and leader and returning the session to `Connected`. One
    /// non-blocking poll; a failed add is reported once and leaves the session
    /// to time out unless another `NewLeaderEvent` comes.
    fn poll_pending_ingress(&mut self) -> Result<(), ClusterError> {
        let Some(pending) = self.pending_ingress.as_mut() else {
            return Ok(());
        };
        if let Some(add) = &pending.add {
            let polled = add.poll();
            match polled {
                Ok(None) => return Ok(()),
                Ok(Some(publication)) => pending.added = Some(publication),
                Err(e) => {
                    pending.add = None;
                    return Err(ClusterError::reconnect(format!("new leader ingress publication: {e}")));
                }
            }
            pending.add = None;
        }
        let Some(ingress) = pending.added.take_if(|publication| publication.is_connected()) else {
            return Ok(());
        };
        let (max_message_length, max_payload_length) = publication_limits(&ingress)?;
        let (leadership_term_id, leader_member_id) = (pending.leadership_term_id, pending.leader_member_id);
        self.pending_ingress = None;
        self.ingress = ingress;
        self.max_message_length = max_message_length;
        self.max_payload_length = max_payload_length;
        self.leadership_term_id = leadership_term_id;
        self.leader_member_id = leader_member_id;
        if self.state == SessionState::AwaitingNewLeader {
            self.state = SessionState::Connected;
            self.awaiting_leader_since = None;
        }
        Ok(())
    }

    /// The cluster-assigned session id (set after connect completes).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn cluster_session_id(&self) -> i64 {
        self.cluster_session_id
    }
    /// The current leadership term (set after connect completes).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn leadership_term_id(&self) -> i64 {
        self.leadership_term_id
    }
    /// The current leader's member id (set after connect completes).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn leader_member_id(&self) -> i32 {
        self.leader_member_id
    }

    /// True when the ingress publication has a connected subscriber (the
    /// leader). Java `publication.isConnected()` analogue for
    /// backpressure-aware callers.
    #[must_use = "the connectivity decision is the point of calling this"]
    #[inline]
    pub fn is_ingress_connected(&self) -> bool {
        self.ingress.is_connected()
    }

    /// True when the ingress publication is closed (fatal for this handle).
    #[must_use = "the closed-state decision is the point of calling this"]
    #[inline]
    pub fn is_ingress_closed(&self) -> bool {
        self.ingress.is_closed()
    }

    /// Current ingress publication position.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn ingress_position(&self) -> i64 {
        self.ingress.position()
    }

    /// Largest message the ingress publication accepts, session header
    /// included (Java `ingressPublication().maxMessageLength()`, a function
    /// of the channel's term length). [`Self::offer`] takes payloads up to
    /// this minus [`Self::SESSION_HEADER_LENGTH`]; check the largest message
    /// against it at start-up rather than meet
    /// [`ClusterError::PayloadTooLarge`] later. A REDIRECT or new leader
    /// re-adds the configured ingress channel, changing only a unicast
    /// endpoint, so the limit holds across failover.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn ingress_max_message_length(&self) -> usize {
        self.max_message_length
    }

    /// Largest single-frame message the ingress publication accepts, session
    /// header included: the channel's MTU less the frame header (Java
    /// `ingressPublication().maxPayloadLength()`). [`Self::try_claim`] takes
    /// payloads up to the smaller of this and
    /// [`Self::ingress_max_message_length`], minus
    /// [`Self::SESSION_HEADER_LENGTH`]; [`Self::offer`] fragments anything
    /// larger.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn ingress_max_payload_length(&self) -> usize {
        self.max_payload_length
    }

    /// True while the egress image this session is bound to is open (Java
    /// `!egressImage.isClosed()`). Going false while `Connected` is the
    /// leader-loss signal consumed by [`Self::poll`]. Other
    /// images on the egress subscription do not count: the image a failed
    /// leader leaves behind can close while its replacement is open.
    #[must_use = "the connectivity decision is the point of calling this"]
    #[inline]
    pub fn is_egress_connected(&self) -> bool {
        self.egress_image.as_ref().is_some_and(|bound| !bound.image.is_closed())
    }

    /// Correlation id of the egress image this session is bound to: the image
    /// that delivered its SessionEvent(OK) or, after a failover, its latest
    /// NewLeaderEvent. Unique per image, so a change means the session moved
    /// to another image even when the session id is reused. `None` when that
    /// image had already closed when the event was handled.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn egress_image_correlation_id(&self) -> Option<i64> {
        self.egress_image.as_ref().map(|bound| bound.correlation_id)
    }

    /// Session id of the egress image this session is bound to (see
    /// [`Self::egress_image_correlation_id`]): the cluster member's egress
    /// publication.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn egress_image_session_id(&self) -> Option<i32> {
        self.egress_image.as_ref().map(|bound| bound.session_id)
    }

    /// Request the cluster take a snapshot (Java
    /// `sendAdminRequestToTakeASnapshot`). The response arrives on the egress
    /// listener as [`crate::EgressListener::on_admin_response`].
    pub fn send_admin_request_to_take_snapshot(&mut self, correlation_id: i64) -> Result<i64, ClusterError> {
        self.send_admin_request(correlation_id, AdminRequestType::SNAPSHOT, &[])
    }

    /// Session-state transitions without polling fragments (Java
    /// `pollStateChanges`); the second half of [`Self::poll`].
    fn poll_state(&mut self) -> Result<(), ClusterError> {
        self.poll_pending_ingress()?;
        let connected = self.is_egress_connected();
        // The new leader's egress image closed before its ingress connected:
        // wait for another NewLeaderEvent from now (Java `onDisconnected`).
        if self.pending_ingress.is_some() && !connected {
            self.pending_ingress = None;
            self.awaiting_leader_since = Some(Instant::now());
        }
        // Once a NewLeaderEvent has come, the wait is for its publication,
        // bounded by message_timeout as Java bounds AWAIT_NEW_LEADER_CONNECTION.
        let awaiting = if self.pending_ingress.is_some() {
            Awaiting {
                what: "connection to the new leader within message_timeout",
                timeout_ms: self.message_timeout_ms,
            }
        } else {
            Awaiting {
                what: "NewLeaderEvent within new_leader_timeout",
                timeout_ms: self.new_leader_timeout_ms,
            }
        };
        let result = apply_state_transition(&mut self.state, &mut self.awaiting_leader_since, awaiting, connected);
        if self.state == SessionState::Closed {
            self.pending_ingress = None;
        }
        result
    }

    /// Begin a poll-driven async connect. Returns an
    /// [`AsyncClusterConnect`] whose `poll()` advances the handshake.
    pub fn connect_async(builder: crate::SessionBuilder, aeron_dir: impl Into<String>) -> AsyncClusterConnect {
        AsyncClusterConnect::new(builder, aeron_dir.into())
    }
    /// Current session state: `Connected`, `NotConnected`, `Closed`.
    #[must_use = "the state decision is the point of calling this"]
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Zero-copy publish: claim a region of the ingress term buffer,
    /// write the SessionMessageHeader into the first 32 bytes via ergon,
    /// and expose the remaining `payload_len` bytes for the caller to fill
    /// directly. Mirrors Java `AeronCluster.tryClaim(length, claim)`.
    ///
    /// # Errors
    ///
    /// - [`ClusterError::NotConnected`] if the session is not Connected
    /// - [`ClusterError::PayloadTooLarge`] if header + payload exceeds
    ///   [`Self::ingress_max_payload_length`] or
    ///   [`Self::ingress_max_message_length`]
    /// - [`ClusterError::Publication`] on claim failure / backpressure
    /// - [`ClusterError::BufferTooSmall`] if the claim buffer is shorter than
    ///   the 32-byte session header
    ///
    /// # Hot path
    ///
    /// This is the hot publish path. On success there is no temp buffer copy of
    /// the application payload — fill [`ClusterClaim::payload_mut`] then
    /// [`ClusterClaim::commit`]. Abort with [`ClusterClaim::abort`] (or drop).
    ///
    /// ```rust,no_run
    /// use ergo_aeron_cluster::{AeronCluster, ClusterError};
    ///
    /// fn publish(client: &mut AeronCluster, app_bytes: &[u8]) -> Result<(), ClusterError> {
    ///     let mut claim = client.try_claim(app_bytes.len())?;
    ///     claim.payload_mut().copy_from_slice(app_bytes);
    ///     claim.commit()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn try_claim(&mut self, payload_len: usize) -> Result<ClusterClaim, ClusterError> {
        if self.state != SessionState::Connected {
            std::hint::cold_path();
            return Err(ClusterError::NotConnected);
        }
        // A claim is one frame, so Aeron also caps it at the max payload length.
        let maximum = self.max_message_length.min(self.max_payload_length);
        let total = MSG_HDR_TOTAL
            .checked_add(payload_len)
            .ok_or(ClusterError::PayloadTooLarge {
                operation: "try_claim",
                requested: payload_len,
                maximum,
            })?;
        if total > maximum {
            std::hint::cold_path();
            return Err(ClusterError::PayloadTooLarge {
                operation: "try_claim",
                requested: total,
                maximum,
            });
        }
        let mut claim = self
            .ingress
            .try_claim_owned(total)
            .map_err(|e| self.track_publication_error(ClusterError::from_offer_error("try_claim", e)))?;

        // Write the SessionMessageHeader (schema 111) into the claim's
        // first 32 bytes via the ergon encoder.
        if claim.data().len() < MSG_HDR_TOTAL {
            std::hint::cold_path();
            return Err(ClusterError::BufferTooSmall {
                needed: MSG_HDR_TOTAL,
                actual: claim.data().len(),
            });
        }
        SessionMessageHeaderEncoder::wrap_into_claim(&mut claim.data()[..MSG_HDR_TOTAL])?
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id)
            .timestamp(0);

        Ok(ClusterClaim { claim, payload_len })
    }
}

/// What an `AwaitingNewLeader` session waits for, and for how long.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Awaiting {
    what: &'static str,
    timeout_ms: u64,
}

/// Pure state-machine transition logic, extracted for unit testing.
/// Called by [`AeronCluster::poll`].
pub(crate) fn apply_state_transition(
    state: &mut SessionState,
    awaiting_leader_since: &mut Option<Instant>,
    awaiting: Awaiting,
    egress_connected: bool,
) -> Result<(), ClusterError> {
    match *state {
        SessionState::Connected if !egress_connected => {
            *state = SessionState::AwaitingNewLeader;
            *awaiting_leader_since = Some(Instant::now());
        }
        SessionState::AwaitingNewLeader => {
            if let Some(since) = *awaiting_leader_since
                && since.elapsed() >= Duration::from_millis(awaiting.timeout_ms)
            {
                std::hint::cold_path();
                *state = SessionState::Closed;
                return Err(ClusterError::Disconnected {
                    reason: format!("no {} ({}ms)", awaiting.what, awaiting.timeout_ms),
                });
            }
        }
        SessionState::PendingClose => {
            *state = SessionState::Closed;
        }
        _ => {}
    }
    Ok(())
}

/// A zero-copy claim on the ingress publication. The caller writes the
/// application payload into `payload_mut()` (the bytes after the 32-byte
/// SessionMessageHeader) then calls `commit()`.
#[must_use = "call commit() to publish or abort() to discard the claim"]
pub struct ClusterClaim {
    claim: AeronClaim,
    payload_len: usize,
}

impl ClusterClaim {
    /// The writable payload region (after the SessionMessageHeader).
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let start = MSG_HDR_TOTAL;
        &mut self.claim.data()[start..start + self.payload_len]
    }

    /// Stream position Aeron assigned to this claim.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn position(&self) -> i64 {
        self.claim.position()
    }

    /// Commit the claimed bytes, publishing them to the cluster.
    /// Note: commit errors cannot be classified as retryable — commit
    /// returns a generic Aeron error (`AeronCError`), not a publication
    /// sentinel like `offer`/`try_claim`. The caller should treat a failed
    /// commit as terminal for this claim.
    #[inline]
    pub fn commit(self) -> Result<i64, ClusterError> {
        self.claim.commit().map_err(|e| ClusterError::aeron("claim_commit", e))
    }

    /// Abort the claim, discarding it as padding.
    #[inline]
    pub fn abort(self) -> Result<(), ClusterError> {
        self.claim.abort().map_err(|e| ClusterError::aeron("claim_abort", e))
    }
}

/// Poll-driven **Aeron** async connect — mirrors Java `AeronCluster.AsyncConnect`.
///
/// Not Tokio/`async`/`await`. Drive [`Self::poll`] from your event loop until
/// [`Self::is_complete`], then [`Self::finish`]. Handles challenge-response and
/// redirect between polls.
///
/// No poll waits on the media driver for a subscription or publication: they
/// are added with Aeron's async add and picked up by later polls, including
/// the publication a REDIRECT asks for. The first poll creates the Aeron
/// client itself, which maps the driver's CnC file and waits for the driver
/// only when it is not running yet.
#[must_use = "poll until is_complete(), then finish() to obtain the cluster client"]
pub struct AsyncClusterConnect {
    /// Declared first so the retained image is released before `egress`.
    egress_image: Option<EgressImage>,
    pending_egress: Option<AeronAsyncAddSubscription>,
    pending_ingress: Option<AeronAsyncAddExclusivePublication>,
    aeron: Option<Aeron>,
    ingress: Option<AeronExclusivePublication>,
    egress: Option<AeronSubscription>,
    /// Every member while `ingressEndpoints` is set. Empty for one channel.
    members: Vec<MemberIngress>,
    builder: crate::SessionBuilder,
    aeron_dir: String,
    attempt: ConnectAttempt,
    /// Leader endpoint from a REDIRECT. Restricts which member is adopted.
    preferred_endpoint: Option<String>,
    /// Endpoint of the publication currently in `ingress`.
    active_endpoint: Option<String>,
    /// User timeout, or 20s, until an OK replaces it with twice the heartbeat.
    concluded_new_leader_timeout_ms: u64,
    step: ConnectStep,
    started: Instant,
    /// Deadline computed on first poll (lazy to avoid panicking in `new`).
    deadline: Option<Instant>,
    cluster_session_id: i64,
    leadership_term_id: i64,
    leader_member_id: i32,
}

/// State machine step for [`AsyncClusterConnect`]. Each call to
/// [`AsyncClusterConnect::poll`] may advance one step; a step that waits on
/// the media driver spans several polls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectStep {
    /// Building the Aeron transport (client, publication, subscription).
    CreateTransport,
    /// Offering the session-connect request, once a REDIRECT's publication
    /// exists.
    SendConnect,
    /// Waiting for the NewLeaderEvent / OK response.
    PollResponse,
    /// Connect complete — [`AsyncClusterConnect::finish`] may be called.
    Done,
}

impl AsyncClusterConnect {
    pub(crate) fn new(builder: crate::SessionBuilder, aeron_dir: String) -> Self {
        let attempt = ConnectAttempt::new(&builder);
        let concluded_new_leader_timeout_ms =
            protocol::conclude_new_leader_timeout_ms(builder.new_leader_timeout_ms, None);
        Self {
            egress_image: None,
            pending_egress: None,
            pending_ingress: None,
            aeron: None,
            ingress: None,
            egress: None,
            members: Vec::new(),
            builder,
            aeron_dir,
            attempt,
            preferred_endpoint: None,
            active_endpoint: None,
            concluded_new_leader_timeout_ms,
            step: ConnectStep::CreateTransport,
            started: Instant::now(),
            deadline: None, // set on first poll via checked_deadline
            cluster_session_id: -1,
            leadership_term_id: -1,
            leader_member_id: -1,
        }
    }

    /// Current connect step.
    ///
    /// ```rust
    /// # use ergo_aeron_cluster::{SessionBuilder, ConnectStep, ClusterError};
    /// # fn poll_loop(builder: SessionBuilder) -> Result<(), ClusterError> {
    /// let mut conn = builder.connect_async("/dev/shm/aeron-driver");
    /// while !conn.is_complete() {
    ///     conn.poll()?;
    ///     match conn.step() {
    ///         ConnectStep::CreateTransport => { /* Aeron init */ }
    ///         ConnectStep::SendConnect     => { /* offered connect */ }
    ///         ConnectStep::PollResponse    => { /* waiting for leader */ }
    ///         ConnectStep::Done            => { /* ready */ }
    ///         _ => { /* forward-compat: future steps */ }
    ///     }
    /// }
    /// let _client = conn.finish()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Always include a wildcard (`_`) arm — the enum is
    /// [`#[non_exhaustive]`](ConnectStep) so a release can add steps without
    /// breaking your `match`.
    #[must_use = "the connect-step decision is the point of calling this"]
    pub fn step(&self) -> ConnectStep {
        self.step
    }

    /// True once the connect has completed and `finish()` can be called.
    #[must_use = "the completion decision is the point of calling this"]
    pub fn is_complete(&self) -> bool {
        self.step == ConnectStep::Done
    }

    /// Advance the connect by one unit of work. Returns `Ok(true)` if
    /// more polling is needed, `Ok(false)` once complete. Idempotent after
    /// completion — calling `poll()` on an already-completed connect
    /// returns `Ok(false)` regardless of the deadline.
    pub fn poll(&mut self) -> Result<bool, ClusterError> {
        // Lazy deadline — computed on first poll to avoid panicking in `new()`.
        if self.deadline.is_none() {
            self.deadline = Some(checked_deadline("async_connect", self.builder.message_timeout_ms)?);
        }
        // Skip deadline after completion — an extra defensive poll after a
        // successful-but-slow connect must not throw away the recovered cluster.
        if !self.is_complete() && Instant::now() > self.deadline.expect("deadline set above") {
            return Err(ClusterError::Timeout {
                phase: "async_connect",
                after_ms: self.started.elapsed().as_millis() as u64,
            });
        }
        match self.step {
            ConnectStep::CreateTransport => {
                if self.aeron.is_none() {
                    self.builder.validate()?;
                    let aeron = create_aeron(&self.builder, &self.aeron_dir)?;
                    let egress_channel = uri::egress_channel_disabling_rejoin(self.builder.egress_for_aeron()?)?;
                    self.pending_egress = Some(
                        aeron
                            .async_add_subscription(
                                &egress_channel,
                                self.builder.egress_stream_id,
                                Handlers::NONE,
                                Handlers::NONE,
                            )
                            .map_err(|error| ClusterError::aeron("sub", error))?,
                    );
                    if self.builder.ingress_endpoints.is_some() {
                        let map = self.builder.ingress_endpoints.clone().unwrap_or_default();
                        for endpoint in crate::endpoints::parse_ingress_endpoints(&map)? {
                            self.members
                                .push(open_member(&aeron, &self.builder, &endpoint.endpoint)?);
                        }
                    } else {
                        let ingress = self.builder.resolve_initial_ingress_for_aeron()?;
                        self.pending_ingress = Some(
                            aeron
                                .async_add_exclusive_publication(&ingress, self.builder.ingress_stream_id)
                                .map_err(|error| ClusterError::aeron("pub", error))?,
                        );
                    }
                    self.aeron = Some(aeron);
                }
                self.poll_pending_transport()?;
                poll_member_adds(&mut self.members)?;
                self.adopt_connected_member();
                if self.attempt.response_channel.is_none()
                    && let Some(egress) = &self.egress
                {
                    self.attempt.response_channel = resolve_response_channel(egress);
                }
                if self.egress.is_some() && self.attempt.response_channel.is_some() && self.ingress.is_some() {
                    self.step = ConnectStep::SendConnect;
                }
                Ok(true)
            }
            ConnectStep::SendConnect => {
                // Retry the offer across polls until the publication
                // connects and the connect request lands. A frozen driver
                // leaves the step here.
                self.poll_pending_transport()?;
                poll_member_adds(&mut self.members)?;
                self.adopt_connected_member();
                prepare_attempt(&mut self.attempt, self.aeron.as_ref())?;
                if offer_connect_on(&mut self.attempt, self.ingress.as_ref())? {
                    self.step = ConnectStep::PollResponse;
                }
                Ok(true)
            }
            ConnectStep::PollResponse => {
                if self.attempt.challenge_bytes.is_some() {
                    let _ = offer_challenge_on(&mut self.attempt, self.ingress.as_ref())?;
                }
                if let Some((event, image_session_id)) = self.poll_one_event()? {
                    if let Some(correlation_id) = self.attempt.correlation_id {
                        match protocol::classify_handshake(event, correlation_id, image_session_id)? {
                            HandshakeEvent::Established {
                                cluster_session_id,
                                leadership_term_id,
                                leader_member_id,
                                leader_heartbeat_timeout_ns,
                                image_session_id,
                            } => {
                                self.cluster_session_id = cluster_session_id;
                                self.leadership_term_id = leadership_term_id;
                                self.leader_member_id = leader_member_id;
                                self.concluded_new_leader_timeout_ms = protocol::conclude_new_leader_timeout_ms(
                                    self.builder.new_leader_timeout_ms,
                                    leader_heartbeat_timeout_ns,
                                );
                                self.egress_image = self
                                    .egress
                                    .as_ref()
                                    .and_then(|egress| EgressImage::bind(egress, image_session_id));
                                self.step = ConnectStep::Done;
                                return Ok(false);
                            }
                            HandshakeEvent::AuthRejected => return Err(ClusterError::AuthRejected),
                            HandshakeEvent::Redirect { endpoint } => self.follow_redirect(&endpoint)?,
                            HandshakeEvent::Challenge {
                                cluster_session_id,
                                encoded_challenge,
                            } => {
                                let response = challenge_response_bytes(&self.builder, &encoded_challenge);
                                begin_challenge(&mut self.attempt, self.aeron.as_ref(), cluster_session_id, &response)?;
                                let _ = offer_challenge_on(&mut self.attempt, self.ingress.as_ref())?;
                            }
                            HandshakeEvent::Ignore => {}
                        }
                    }
                } else if !self.attempt.challenged {
                    let _ = offer_connect_on(&mut self.attempt, self.ingress.as_ref())?;
                }
                Ok(true)
            }
            ConnectStep::Done => Ok(false),
        }
    }

    /// Consume the in-progress connect and yield the connected client.
    pub fn finish(self) -> Result<AeronCluster, ClusterError> {
        if self.step != ConnectStep::Done {
            return Err(ClusterError::ConnectFailed {
                reason: "connect not complete".into(),
            });
        }
        let keep_alive_interval_ms = connect_reoffer_interval_ms(self.builder.message_timeout_ms);
        let concluded_new_leader_timeout_ms = self.concluded_new_leader_timeout_ms;
        let Self {
            egress_image,
            aeron,
            ingress,
            egress,
            builder,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            ..
        } = self;
        let ingress =
            ingress.ok_or_else(|| ClusterError::connect("async connect finished without ingress publication"))?;
        let (max_message_length, max_payload_length) = publication_limits(&ingress)?;
        Ok(AeronCluster {
            egress_image,
            pending_ingress: None,
            _aeron: aeron.ok_or_else(|| ClusterError::connect("async connect finished without Aeron client"))?,
            ingress,
            egress: egress
                .ok_or_else(|| ClusterError::connect("async connect finished without egress subscription"))?,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            state: SessionState::Connected,
            awaiting_leader_since: None,
            ingress_stream_id: builder.ingress_stream_id,
            ingress_channel: builder.ingress_channel_c_str().map(CStr::to_owned),
            new_leader_timeout_ms: concluded_new_leader_timeout_ms,
            message_timeout_ms: builder.message_timeout_ms,
            last_keep_alive: Instant::now(),
            max_message_length,
            max_payload_length,
            keep_alive_interval_ms,
            regular_assembler: AeronFragmentClosureAssembler::new()
                .map_err(|e| ClusterError::aeron("AeronFragmentClosureAssembler", e))?,
        })
    }

    /// A single-channel REDIRECT adds the leader without waiting. Fan-out
    /// keeps the leader's publication when it is already connected.
    fn follow_redirect(&mut self, endpoint: &str) -> Result<(), ClusterError> {
        self.preferred_endpoint = Some(endpoint.to_owned());
        if self.members.is_empty() && self.active_endpoint.is_none() {
            self.queue_single_redirect(endpoint)?;
        } else {
            self.queue_member_redirect(endpoint)?;
        }
        self.attempt.on_redirect();
        self.step = ConnectStep::SendConnect;
        Ok(())
    }

    fn queue_single_redirect(&mut self, endpoint: &str) -> Result<(), ClusterError> {
        let channel = uri::ingress_channel_for_endpoint(self.builder.ingress_channel_c_str(), endpoint)?;
        let stream_id = self.builder.ingress_stream_id;
        let aeron = self.aeron.as_ref().ok_or_else(|| ClusterError::ReconnectFailed {
            reason: "no aeron client for redirect".into(),
        })?;
        let publication = aeron
            .async_add_exclusive_publication(&channel, stream_id)
            .map_err(|error| ClusterError::reconnect(format!("redirect publication: {error}")))?;
        self.pending_ingress = Some(publication);
        self.ingress = None;
        Ok(())
    }

    fn queue_member_redirect(&mut self, endpoint: &str) -> Result<(), ClusterError> {
        if self.active_endpoint.as_deref() != Some(endpoint) {
            self.ingress = None;
            self.active_endpoint = None;
        }
        self.members.retain(|member| member.endpoint == endpoint);
        let held = self.ingress.is_some() || self.members.iter().any(|member| member.endpoint == endpoint);
        if !held {
            let aeron = self.aeron.as_ref().ok_or_else(|| ClusterError::ReconnectFailed {
                reason: "no aeron client for redirect".into(),
            })?;
            self.members.push(open_member(aeron, &self.builder, endpoint)?);
        }
        Ok(())
    }

    fn adopt_connected_member(&mut self) {
        if self.ingress.is_some() || self.members.is_empty() {
            return;
        }
        if let Some((endpoint, publication)) = take_connected(&mut self.members, self.preferred_endpoint.as_deref()) {
            self.active_endpoint = Some(endpoint);
            self.ingress = Some(publication);
        }
    }

    /// Pick up subscriptions and publications Aeron has finished adding,
    /// without waiting (Java `getSubscription(registrationId)`).
    fn poll_pending_transport(&mut self) -> Result<(), ClusterError> {
        if let Some(add) = &self.pending_egress
            && let Some(egress) = add.poll().map_err(|e| ClusterError::aeron("sub", e))?
        {
            self.egress = Some(egress);
            self.pending_egress = None;
        }
        if let Some(add) = &self.pending_ingress
            && let Some(ingress) = add.poll().map_err(|e| ClusterError::aeron("pub", e))?
        {
            self.ingress = Some(ingress);
            self.pending_ingress = None;
        }
        Ok(())
    }

    /// One egress event and the session id of the image that delivered it.
    fn poll_one_event(&mut self) -> Result<Option<(crate::poller::EgressEvent, Option<i32>)>, ClusterError> {
        match &self.egress {
            Some(egress) => poll_subscription(egress),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AWAITING_LEADER: Awaiting = Awaiting {
        what: "NewLeaderEvent within new_leader_timeout",
        timeout_ms: 5_000,
    };

    #[test]
    fn test_msg_header_total_is_32() -> Result<(), Box<dyn std::error::Error>> {
        // MessageHeader(8) + SessionMessageHeader body(24) = 32
        assert_eq!(MSG_HDR_TOTAL, 32);

        Ok(())
    }

    #[test]
    fn test_session_constants() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::{
            SessionCloseRequestEncoder, SessionKeepAliveEncoder, SessionMessageHeaderEncoder,
        };
        assert_eq!(SessionMessageHeaderEncoder::TEMPLATE_ID, 1);
        assert_eq!(SessionKeepAliveEncoder::TEMPLATE_ID, 5);
        assert_eq!(SessionCloseRequestEncoder::TEMPLATE_ID, 4);

        Ok(())
    }

    #[test]
    fn test_encoded_length_matches_encode_connect_request() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::SessionConnectRequestEncoder;

        let ch = b"aeron:udp?endpoint=localhost:9999";
        let creds = b"user:pass";
        let info = protocol::client_info_bytes(protocol::DEFAULT_CLIENT_NAME);
        let len =
            SessionConnectRequestEncoder::compute_encoded_length_with_message_header(ch.len(), creds.len(), info.len());
        let mut buf = vec![0u8; len];
        use crate::codecs::session::SessionConnectRequestFixedFields;
        let complete = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&SessionConnectRequestFixedFields {
                correlation_id: 0,
                response_stream_id: 102,
                version: Some(0),
            })
            .response_channel(ch)?
            .encoded_credentials(creds)?
            .client_info(&info)?;
        assert_eq!(
            complete.as_bytes_with_header().len(),
            len,
            "EncodedLength compute must equal as_bytes_with_header for connect request"
        );
        Ok(())
    }

    #[test]
    fn test_encoded_length_matches_encode_challenge_response() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::{ChallengeResponseEncoder, ChallengeResponseFixedFields};

        let creds = b"challenge-response-bytes";
        let len = ChallengeResponseEncoder::compute_encoded_length_with_message_header(creds.len());
        let mut buf = vec![0u8; len];
        let complete = ChallengeResponseEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&ChallengeResponseFixedFields {
                correlation_id: 1,
                cluster_session_id: 2,
            })
            .encoded_credentials(creds)?;
        assert_eq!(
            complete.as_bytes_with_header().len(),
            len,
            "EncodedLength compute must equal as_bytes_with_header for challenge response"
        );
        Ok(())
    }

    #[test]
    fn test_keep_alive_buffer_is_stack_sized_by_encoded_length() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::SessionKeepAliveEncoder;
        // Fixed message — the stack array must use the generated ENCODED_LENGTH.
        assert_eq!(
            SessionKeepAliveEncoder::ENCODED_LENGTH,
            8 + SessionKeepAliveEncoder::BLOCK_LENGTH,
        );
        Ok(())
    }

    #[test]
    fn test_pending_close_finalises_to_closed() -> Result<(), Box<dyn std::error::Error>> {
        // T1: PendingClose → Closed via poll → apply_state_transition.
        let mut state = SessionState::PendingClose;
        let mut awaiting = None;
        apply_state_transition(&mut state, &mut awaiting, AWAITING_LEADER, true)?;
        assert_eq!(state, SessionState::Closed);
        Ok(())
    }

    #[test]
    fn test_new_leader_timeout_disconnects() -> Result<(), Box<dyn std::error::Error>> {
        // T7: After deadline, AwaitingNewLeader → Closed + Disconnected error.
        let mut state = SessionState::AwaitingNewLeader;
        let mut awaiting = Some(Instant::now() - Duration::from_secs(6));
        let result = apply_state_transition(&mut state, &mut awaiting, AWAITING_LEADER, false);
        match result {
            Err(ClusterError::Disconnected { .. }) => {}
            other => panic!("expected Disconnected, got {other:?}"),
        }
        assert_eq!(state, SessionState::Closed);
        Ok(())
    }

    #[test]
    fn test_reconnect_prepares_before_swapping_state() -> Result<(), Box<dyn std::error::Error>> {
        // T6: Structural proof — `on_new_leader_event` must run every fallible
        // step BEFORE writing any session field, so a failure leaves the
        // session as it was, and must start the publication add without
        // waiting for it.
        let src = include_str!("client.rs");
        let off = src
            .find("fn on_new_leader_event")
            .ok_or("on_new_leader_event not found")?;
        let end = src[off..].find("\n    }\n").ok_or("end of on_new_leader_event")?;
        let body = &src[off..off + end];
        let first_write = [
            "self.pending_ingress =",
            "self.egress_image =",
            "self.state =",
            "self.awaiting_leader_since =",
            "self.leadership_term_id =",
            "self.leader_member_id =",
        ]
        .iter()
        .filter_map(|write| body.find(write))
        .min()
        .ok_or("no session write")?;
        for step in [
            "parse_leader_endpoint(",
            "ingress_channel_for_endpoint(",
            "async_add_exclusive_publication(",
        ] {
            let at = body.find(step).ok_or(step)?;
            assert!(at < first_write, "{step} must precede the first session write");
        }
        assert_eq!(
            body.matches("add_exclusive_publication(").count(),
            body.matches("async_add_exclusive_publication(").count(),
            "a NewLeaderEvent must not add its publication with a blocking wait"
        );
        Ok(())
    }

    #[test]
    fn test_offer_gathers_without_heap_allocation() -> Result<(), Box<dyn std::error::Error>> {
        // Structural proof — the `offer` method's primary path delegates to
        // `try_claim` (claim-based, allocation-free). Large messages that
        // exceed the max payload use `offer_parts` with a stack-allocated
        // header, avoiding any heap allocation or payload copy.
        let src = include_str!("client.rs");
        let off_start = src
            .find("pub fn offer(&mut self, payload: &[u8])")
            .ok_or("offer signature not found")?;
        let body_snippet = &src[off_start..];
        let next_pub = body_snippet[20..].find("pub fn ").unwrap_or(body_snippet.len());
        let offer_body = &body_snippet[..next_pub + 20];
        assert!(
            offer_body.contains("self.try_claim"),
            "offer must delegate to try_claim as primary path"
        );
        assert!(
            offer_body.contains("offer_parts"),
            "fallback must use offer_parts gather (no heap allocation)"
        );
        assert!(
            !offer_body.contains("vec!["),
            "offer must not allocate on the heap for fragmented messages"
        );
        Ok(())
    }

    #[test]
    fn test_new_leader_timeout_not_yet_elapsed_keeps_awaiting() -> Result<(), Box<dyn std::error::Error>> {
        let mut state = SessionState::AwaitingNewLeader;
        let mut awaiting = Some(Instant::now());
        apply_state_transition(&mut state, &mut awaiting, AWAITING_LEADER, false)?;
        assert_eq!(state, SessionState::AwaitingNewLeader);
        Ok(())
    }
}
