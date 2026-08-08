//! The [`AeronCluster`] client — owns the Aeron transport and drives the
//! full SBE session handshake. Java-parity entry point for the **Ergo Aeron
//! Cluster** client (experimental prototype on `rusteron-client`):
//!
//! → [`cluster/tests/connect_to_cluster.rs`](https://github.com/mimran1980/ergon/blob/main/cluster/tests/connect_to_cluster.rs)
//!
//! Hot path: [`AeronCluster::try_claim`] writes SessionMessageHeader via ergo-sbe
//! into the claim; fill app payload then [`ClusterClaim::commit`].

use std::time::{Duration, Instant};

use rusteron_client::bindings::aeron_controlled_fragment_handler_action_en as AeronAction;
use rusteron_client::{
    Aeron, AeronClaim, AeronContext, AeronControlledFragmentClosureAssembler, AeronExclusivePublication,
    AeronFragmentClosureAssembler, AeronSubscription, Handlers, cformat,
};

use crate::uri;

use crate::codecs::session::{
    AdminRequestEncoder, AdminRequestType, ChallengeResponseEncoder, SessionCloseRequestEncoder,
    SessionConnectRequestEncoder, SessionKeepAliveEncoder, SessionMessageHeaderEncoder,
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

use crate::controlled::{ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction};
use crate::egress::{EgressAdapter, EgressListener};
use crate::error::ClusterError;
use crate::error::PublicationFailure;
use crate::state::SessionState;

/// Populate the `clientInfo` field in `SessionConnectRequest` matching
/// Java's `"name=<clientName> <versionInfo>"` pattern.
fn client_info_bytes() -> Vec<u8> {
    format!("name={} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).into_bytes()
}

/// Header-inclusive SessionMessageHeader (prefer generated `ENCODED_LENGTH`).
const MSG_HDR_TOTAL: usize = SessionMessageHeaderEncoder::ENCODED_LENGTH;

/// Map [`ControlledPollAction`] to Aeron's C enum.
fn to_aeron_action(action: ControlledPollAction) -> AeronAction {
    match action {
        ControlledPollAction::Continue => AeronAction::AERON_ACTION_CONTINUE,
        ControlledPollAction::Abort => AeronAction::AERON_ACTION_ABORT,
        ControlledPollAction::Break => AeronAction::AERON_ACTION_BREAK,
        ControlledPollAction::Commit => AeronAction::AERON_ACTION_COMMIT,
    }
}

/// Context for the dispatch function — set up before each poll cycle.
struct PollCtx<'a, L: EgressListener> {
    adapter: &'a mut EgressAdapter<L>,
    new_leader: &'a mut Option<(i64, i32, String)>,
    decode_err: &'a mut Option<ClusterError>,
    expected_session_id: i64,
    session_closed: &'a mut bool,
}

struct ControlledPollCtx<'a, L: ControlledEgressListener> {
    adapter: &'a mut ControlledEgressAdapter<L>,
    new_leader: &'a mut Option<(i64, i32, String)>,
    decode_err: &'a mut Option<ClusterError>,
    expected_session_id: i64,
    session_closed: &'a mut bool,
}

fn dispatch_regular<L: EgressListener>(ctx: &mut PollCtx<L>, data: &[u8], _hdr: rusteron_client::AeronHeader) {
    // Decode once — the Fragment is used for state tracking AND listener dispatch.
    let frag = match crate::fragment::Fragment::decode(data) {
        Ok(Some(f)) => f,
        Ok(None) => return, // unknown template — forward-compat, not an error
        Err(e) => {
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
        } if ctx.new_leader.is_none() && *cluster_session_id == ctx.expected_session_id => {
            *ctx.new_leader = Some((*leadership_term_id, *leader_member_id, ingress_endpoints.to_string()));
        }
        crate::fragment::Fragment::SessionEvent {
            code: crate::codecs::session::EventCode::CLOSED,
            cluster_session_id,
            ..
        } if *cluster_session_id == ctx.expected_session_id => {
            *ctx.session_closed = true;
        }
        _ => {}
    }
    // Pass the already-decoded fragment — no second decode
    if let Err(e) = ctx.adapter.dispatch_fragment(frag)
        && ctx.decode_err.is_none()
    {
        *ctx.decode_err = Some(e);
    }
}

fn dispatch_controlled<L: ControlledEgressListener>(
    ctx: &mut ControlledPollCtx<L>,
    data: &[u8],
    _hdr: rusteron_client::AeronHeader,
) -> AeronAction {
    // Decode once — shared between state tracking and listener dispatch.
    let frag = match crate::fragment::Fragment::decode(data) {
        Ok(Some(f)) => f,
        Ok(None) => return AeronAction::AERON_ACTION_CONTINUE, // unknown template
        Err(e) => {
            if ctx.decode_err.is_none() {
                *ctx.decode_err = Some(e);
            }
            return AeronAction::AERON_ACTION_ABORT;
        }
    };
    // State tracking — no second decode
    match &frag {
        crate::fragment::Fragment::NewLeader {
            leadership_term_id,
            leader_member_id,
            ingress_endpoints,
            cluster_session_id,
        } if ctx.new_leader.is_none() && *cluster_session_id == ctx.expected_session_id => {
            *ctx.new_leader = Some((*leadership_term_id, *leader_member_id, ingress_endpoints.to_string()));
        }
        crate::fragment::Fragment::SessionEvent {
            code: crate::codecs::session::EventCode::CLOSED,
            cluster_session_id,
            ..
        } if *cluster_session_id == ctx.expected_session_id => {
            *ctx.session_closed = true;
        }
        _ => {}
    }
    match ctx.adapter.dispatch_fragment(frag) {
        Ok(action) => to_aeron_action(action),
        Err(e) => {
            if ctx.decode_err.is_none() {
                *ctx.decode_err = Some(e);
            }
            AeronAction::AERON_ACTION_ABORT
        }
    }
}

/// Map a raw offer return: `Ok(pos)` if `r > 0`, else typed publication error.
#[inline]
fn offer_result(context: &'static str, r: i64) -> Result<i64, ClusterError> {
    if r > 0 {
        Ok(r)
    } else {
        Err(ClusterError::from_offer_raw(context, r))
    }
}

#[inline]
fn tolerate_retryable_keep_alive(result: Result<(), ClusterError>) -> Result<(), ClusterError> {
    match result {
        Err(error) if error.is_retryable() => Ok(()),
        result => result,
    }
}

/// A connected Aeron cluster client. Owns the Aeron client, ingress
/// exclusive publication, and egress subscription.
pub struct AeronCluster {
    _aeron: Aeron,
    ingress: AeronExclusivePublication,
    egress: AeronSubscription,
    cluster_session_id: i64,
    leadership_term_id: i64,
    leader_member_id: i32,
    state: SessionState,
    /// Wall-clock when the session entered `AwaitingNewLeader` (None while
    /// Connected). Drives the `new_leader_timeout` enforced by
    /// [`Self::poll_state_changes`].
    awaiting_leader_since: Option<Instant>,
    /// Configured ingress stream id, retained so a `NewLeaderEvent`
    /// reconnect uses the same stream the session was established on.
    ingress_stream_id: i32,
    /// `newLeaderTimeout` from the builder (ms). Enforced by
    /// [`Self::poll_state_changes`].
    new_leader_timeout_ms: u64,
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
    /// rusteron fragment assembler for controlled egress.
    controlled_assembler: AeronControlledFragmentClosureAssembler,
}

impl Drop for AeronCluster {
    fn drop(&mut self) {
        if self.state != SessionState::Closed && self.state != SessionState::PendingClose {
            let _ = self.close();
        }
    }
}

impl AeronCluster {
    /// Connect to a cluster whose media driver lives at `aeron_dir`.
    /// Performs the full SBE handshake: SessionConnectRequest → poll
    /// egress for SessionEvent(OK) → extract session/leadership IDs.
    pub fn connect(builder: &crate::SessionBuilder, aeron_dir: &str) -> Result<Self, ClusterError> {
        builder.validate()?;

        let dir_cstr = cformat!("{aeron_dir}");
        let ctx = AeronContext::new().map_err(|e| ClusterError::aeron("AeronContext", e))?;
        ctx.set_dir(&dir_cstr).map_err(|e| ClusterError::aeron("set_dir", e))?;
        let aeron = Aeron::new(&ctx).map_err(|e| ClusterError::aeron("Aeron::new", e))?;
        aeron.start().map_err(|e| ClusterError::aeron("Aeron::start", e))?;

        // Builder already holds CString; pass &CStr to rusteron with no second alloc.
        let egress_c = builder.egress_for_aeron()?;
        let ingress_c = builder.resolve_initial_ingress_for_aeron()?;

        let egress = aeron
            .add_subscription(
                egress_c,
                builder.egress_stream_id,
                Handlers::NONE,
                Handlers::NONE,
                Duration::from_secs(5),
            )
            .map_err(|e| ClusterError::aeron("add_subscription", e))?;

        let ingress = aeron
            .add_exclusive_publication(&ingress_c, builder.ingress_stream_id, Duration::from_secs(5))
            .map_err(|e| ClusterError::aeron("add_exclusive_publication", e))?;

        let mut client = Self {
            _aeron: aeron,
            ingress,
            egress,
            cluster_session_id: -1,
            leadership_term_id: -1,
            leader_member_id: -1,
            state: SessionState::Closed,
            awaiting_leader_since: None,
            ingress_stream_id: builder.ingress_stream_id,
            new_leader_timeout_ms: builder.new_leader_timeout_ms,
            last_keep_alive: Instant::now(),
            keep_alive_interval_ms: connect_reoffer_interval_ms(builder.message_timeout_ms),
            max_message_length: 0,
            max_payload_length: 0,
            regular_assembler: AeronFragmentClosureAssembler::new()
                .map_err(|e| ClusterError::aeron("AeronFragmentClosureAssembler", e))?,
            controlled_assembler: AeronControlledFragmentClosureAssembler::new()
                .map_err(|e| ClusterError::aeron("AeronControlledFragmentClosureAssembler", e))?,
        };
        client.cache_publication_limits();

        client.handshake(builder)?;
        Ok(client)
    }

    /// Send the SessionConnectRequest and poll egress for the result,
    /// handling challenge-response and leader redirect along the way.
    ///
    /// While waiting, re-offers the connect request on a cadence of
    /// [`connect_reoffer_interval_ms`] so a first offer that lands on a
    /// pre-election / silent non-leader peer does not stall until timeout.
    fn handshake(&mut self, builder: &crate::SessionBuilder) -> Result<(), ClusterError> {
        let creds: Vec<u8> = builder
            .credentials
            .as_ref()
            .and_then(|c| c.encoded_credentials().map(|cow| cow.into_owned()))
            .unwrap_or_default();

        self.send_connect_request(builder, &creds)?;
        let mut last_offer = Instant::now();
        let reoffer_ms = connect_reoffer_interval_ms(builder.message_timeout_ms);

        let deadline = checked_deadline("connect", builder.message_timeout_ms)?;
        let idle_clone = builder.idle.clone();
        let mut captured: Option<crate::poller::EgressEvent> = None;
        while Instant::now() < deadline {
            let _ = self.egress.poll_fn(
                |data, _hdr| {
                    if captured.is_none() {
                        captured = crate::poller::parse_event(data).ok().flatten();
                    }
                },
                1,
            );

            match captured.take() {
                Some(crate::poller::EgressEvent::SessionEvent {
                    code,
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    detail,
                    ..
                }) => {
                    use crate::codecs::session::EventCode;
                    match code {
                        EventCode::OK => {
                            self.cluster_session_id = cluster_session_id;
                            self.leadership_term_id = leadership_term_id;
                            self.leader_member_id = leader_member_id;
                            self.state = SessionState::Connected;
                            return Ok(());
                        }
                        EventCode::AUTHENTICATIONREJECTED => {
                            return Err(ClusterError::AuthRejected);
                        }
                        EventCode::REDIRECT => {
                            // Resolve the LEADER's endpoint by member id — the
                            // detail lists all members in id order, so a
                            // position-based parse would redirect back to the
                            // follower we just asked (an infinite loop).
                            if let Some(ep) = crate::poller::parse_leader_endpoint(&detail, leader_member_id) {
                                self.reconnect_ingress(builder, &ep)?;
                                self.send_connect_request(builder, &creds)?;
                                last_offer = Instant::now();
                            }
                            // keep polling
                        }
                        _ => { /* keep polling */ }
                    }
                }
                Some(crate::poller::EgressEvent::Challenge {
                    correlation_id,
                    cluster_session_id,
                    encoded_challenge,
                }) => {
                    // Respond with credentials from the supplier.
                    let resp: Vec<u8> = builder
                        .credentials
                        .as_ref()
                        .and_then(|c| c.on_challenge(&encoded_challenge).map(|cow| cow.into_owned()))
                        .unwrap_or_default();
                    self.send_challenge_response(correlation_id, cluster_session_id, &resp)?;
                    // keep polling for the resulting SessionEvent
                }
                _ => {
                    // No event: re-offer connect if the interval elapsed so a
                    // pre-election peer that neither leads nor redirects does
                    // not burn the full timeout on a single silent offer.
                    if last_offer.elapsed() >= Duration::from_millis(reoffer_ms) {
                        match self.try_offer_connect_request(builder, &creds) {
                            Ok(true) => {}
                            Ok(false) => {}
                            Err(e) if e.is_retryable() => { /* keep polling */ }
                            Err(e) => return Err(e), // fatal — surface immediately
                        }
                        last_offer = Instant::now();
                    }
                }
            }
            if let Some(ref idle) = idle_clone {
                idle.lock().expect("idle mutex poisoned").idle(0);
            } else {
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        Err(ClusterError::ConnectFailed {
            reason: "timeout waiting for SessionEvent(OK)".into(),
        })
    }

    /// Encode SessionConnectRequest and retry offer until it lands or timeout.
    fn send_connect_request(
        &mut self,
        builder: &crate::SessionBuilder,
        credentials: &[u8],
    ) -> Result<(), ClusterError> {
        let buf = Self::encode_connect_request(builder, credentials)?;
        let deadline = checked_deadline("connect_request", builder.message_timeout_ms)?;
        let idle_clone = builder.idle.clone();
        while Instant::now() < deadline {
            if self.ingress.offer_raw(&buf, Handlers::NONE) > 0 {
                return Ok(());
            }
            if let Some(ref idle) = idle_clone {
                idle.lock().expect("idle mutex poisoned").idle(0);
            } else {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        Err(ClusterError::ConnectFailed {
            reason: "connect request offer timed out".into(),
        })
    }

    /// Best-effort single offer of a fresh SessionConnectRequest (re-offer path).
    fn try_offer_connect_request(
        &mut self,
        builder: &crate::SessionBuilder,
        credentials: &[u8],
    ) -> Result<bool, ClusterError> {
        let buf = Self::encode_connect_request(builder, credentials)?;
        Ok(self.ingress.offer_raw(&buf, Handlers::NONE) > 0)
    }

    fn encode_connect_request(builder: &crate::SessionBuilder, credentials: &[u8]) -> Result<Vec<u8>, ClusterError> {
        let ch = builder.egress_channel_bytes();
        let info = client_info_bytes();
        let len = SessionConnectRequestEncoder::compute_encoded_length_with_message_header(
            ch.len(),
            credentials.len(),
            info.len(),
        );
        let mut buf = vec![0u8; len];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(0)
            .response_stream_id(builder.egress_stream_id)
            .version(0);
        let _ = enc
            .response_channel(ch)?
            .encoded_credentials(credentials)?
            .client_info(&client_info_bytes())?;
        Ok(buf)
    }

    fn send_challenge_response(
        &mut self,
        correlation_id: i64,
        cluster_session_id: i64,
        credentials: &[u8],
    ) -> Result<(), ClusterError> {
        let len = ChallengeResponseEncoder::compute_encoded_length_with_message_header(credentials.len());
        let mut buf = vec![0u8; len];
        let mut enc = ChallengeResponseEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(correlation_id)
            .cluster_session_id(cluster_session_id);
        let _ = enc.encoded_credentials(credentials)?;
        let r = self.ingress.offer_raw(&buf, Handlers::NONE);
        offer_result("challenge_response", r).map(|_| ())
    }

    /// Prepare a fresh ingress publication + both fragment assemblers from an
    /// endpoint without touching any session field. The caller atomically swaps
    /// the result into `self` when ready. Mirrors Java's ingress rebuild
    /// discipline (prepare before commit).
    fn prepare_reconnect_ingress(
        aeron: &Aeron,
        endpoint: &str,
        stream_id: i32,
    ) -> Result<
        (
            AeronExclusivePublication,
            AeronFragmentClosureAssembler,
            AeronControlledFragmentClosureAssembler,
        ),
        ClusterError,
    > {
        let cstr = uri::udp_endpoint_cstr(endpoint)?;
        let new_pub = aeron
            .add_exclusive_publication(&cstr, stream_id, Duration::from_secs(5))
            .map_err(|e| ClusterError::reconnect(format!("ingress to {endpoint}: {e}")))?;
        let new_regular = AeronFragmentClosureAssembler::new()
            .map_err(|e| ClusterError::aeron("AeronFragmentClosureAssembler", e))?;
        let new_controlled = AeronControlledFragmentClosureAssembler::new()
            .map_err(|e| ClusterError::aeron("AeronControlledFragmentClosureAssembler", e))?;
        Ok((new_pub, new_regular, new_controlled))
    }

    /// Recreate the ingress publication pointed at a new leader endpoint
    /// (connect-time redirect). Uses [`Self::prepare_reconnect_ingress`] so
    /// the publication AND assemblers are refreshed atomically.
    fn reconnect_ingress(&mut self, builder: &crate::SessionBuilder, endpoint: &str) -> Result<(), ClusterError> {
        let (new_pub, new_regular, new_controlled) =
            Self::prepare_reconnect_ingress(&self._aeron, endpoint, builder.ingress_stream_id)?;
        self.ingress = new_pub;
        self.regular_assembler = new_regular;
        self.controlled_assembler = new_controlled;
        self.cache_publication_limits();
        Ok(())
    }

    /// Cache max_message_length and max_payload_length from the ingress
    /// publication so size checks don't cross the FFI boundary on every offer.
    fn cache_publication_limits(&mut self) {
        if let Ok(constants) = self.ingress.get_constants() {
            self.max_message_length = constants.max_message_length;
            self.max_payload_length = constants.max_payload_length;
        }
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
        if self.max_message_length > 0 && total > self.max_message_length {
            return Err(ClusterError::PayloadTooLarge {
                operation: "offer",
                requested: total,
                maximum: self.max_message_length,
            });
        }
        // Fast path: zero-alloc claim when the payload fits in one fragment.
        if payload.len() <= self.max_payload_length || self.max_payload_length == 0 {
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

    /// Publish a sub-range of application data — equivalent to Java
    /// `offer(DirectBuffer, int offset, int length)`. The slice
    /// `payload[offset..offset+length]` is bounds-checked then published
    /// via the allocation-free [`Self::offer`] path.
    pub fn offer_range(&mut self, payload: &[u8], offset: usize, length: usize) -> Result<i64, ClusterError> {
        let Some(end) = offset.checked_add(length) else {
            return Err(ClusterError::BufferTooSmall {
                needed: offset.saturating_add(length),
                actual: payload.len(),
            });
        };
        if end > payload.len() {
            return Err(ClusterError::BufferTooSmall {
                needed: end,
                actual: payload.len(),
            });
        }
        self.offer(&payload[offset..end])
    }

    /// Send a SessionKeepAlive to hold the session open.
    ///
    /// Encoded into a stack array sized to the generated
    /// [`SessionKeepAliveEncoder::ENCODED_LENGTH`] — no heap allocation.
    pub fn send_keep_alive(&mut self) -> Result<(), ClusterError> {
        let mut buf = [0u8; SessionKeepAliveEncoder::ENCODED_LENGTH];
        SessionKeepAliveEncoder::wrap_and_apply_header(&mut buf, 0)
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id);
        let r = self.ingress.offer_raw(&buf, Handlers::NONE);
        self.track_ingress_publication_result(r).map(|_| ())
    }

    /// Send keep-alive if the interval has elapsed since the last send.
    /// Called from `poll_egress` — mirrors Java's automatic session keep-alive.
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

    /// Classify an ingress publication result (raw offer/try_claim return
    /// or commit position). On a fatal sentinel — [`PublicationFailure::Closed`]
    /// or [`PublicationFailure::MaxPositionExceeded`] — the session is
    /// transitioned to `AwaitingNewLeader` so a subsequent
    /// `NewLeaderEvent` can reconnect. Java `trackIngressPublicationResult`
    /// analogue. Non-fatal retryable sentinels (NotConnected/BackPressured)
    /// are returned as an error but do not change session state.
    pub fn track_ingress_publication_result(&mut self, result: i64) -> Result<i64, ClusterError> {
        if result > 0 {
            return Ok(result);
        }
        let failure = PublicationFailure::from_raw(result);
        if matches!(
            failure,
            PublicationFailure::Closed | PublicationFailure::MaxPositionExceeded
        ) && self.state == SessionState::Connected
        {
            self.state = SessionState::AwaitingNewLeader;
            self.awaiting_leader_since = Some(Instant::now());
        }
        Err(ClusterError::Publication {
            failure,
            context: "track_ingress",
        })
    }

    /// Mirror [`Self::track_ingress_publication_result`] for the typed
    /// `ClusterError::Publication` path used by `offer` / `try_claim`: on a
    /// fatal sentinel (Closed / MaxPositionExceeded) transition a Connected
    /// session to `AwaitingNewLeader` so a `NewLeaderEvent` can recover it.
    /// Retryable sentinels (NotConnected / BackPressured / AdminAction) leave
    /// the session intact.
    fn track_publication_error(&mut self, err: ClusterError) -> ClusterError {
        if let ClusterError::Publication { failure, .. } = &err
            && self.state == SessionState::Connected
            && matches!(
                *failure,
                PublicationFailure::Closed | PublicationFailure::MaxPositionExceeded
            )
        {
            self.state = SessionState::AwaitingNewLeader;
            self.awaiting_leader_since = Some(Instant::now());
        }
        err
    }

    /// Track the result of an `offer_parts` call. Accepts the mapped result
    /// from `offer_parts().map_err(|e| ClusterError::from_offer_error(...))`.
    fn track_ingress_publication_result_offer(
        &mut self,
        result: Result<i64, ClusterError>,
    ) -> Result<i64, ClusterError> {
        match result {
            Ok(pos) if pos >= 0 => Ok(pos),
            Ok(neg) => {
                let failure = PublicationFailure::from_raw(neg);
                if matches!(
                    failure,
                    PublicationFailure::Closed | PublicationFailure::MaxPositionExceeded
                ) && self.state == SessionState::Connected
                {
                    self.state = SessionState::AwaitingNewLeader;
                    self.awaiting_leader_since = Some(Instant::now());
                }
                Err(ClusterError::Publication {
                    failure,
                    context: "offer",
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Send an AdminRequest (e.g. snapshot) on the ingress publication.
    ///
    /// Responses arrive as admin events on the egress listener
    /// ([`crate::EgressListener::on_admin_response`]).
    pub fn send_admin_request(
        &mut self,
        correlation_id: i64,
        request_type: AdminRequestType,
        payload: &[u8],
    ) -> Result<i64, ClusterError> {
        if self.state != SessionState::Connected {
            return Err(ClusterError::NotConnected);
        }
        let len = AdminRequestEncoder::compute_encoded_length_with_message_header(payload.len());
        let mut buf = vec![0u8; len];
        let mut enc = AdminRequestEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id)
            .correlation_id(correlation_id)
            .request_type(request_type);
        let complete = enc.payload(payload)?;
        let bytes = complete.as_bytes_with_header();
        let r = self.ingress.offer_raw(bytes, Handlers::NONE);
        offer_result("admin_request", r)
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
        let _ = self.ingress.offer_raw(&buf, Handlers::NONE);
        Ok(())
    }

    /// Poll egress and dispatch decoded messages through `adapter`.
    /// Fragments are reassembled into complete logical messages before
    /// decoding. Application messages not matching the connected
    /// `cluster_session_id` are silently dropped.
    ///
    /// Handles `NewLeaderEvent` internally: updates the session's
    /// leadership term / leader id, recreates assemblers, and reconnects
    /// ingress to the new leader. Returns fragments polled.
    pub fn poll_egress<L: EgressListener>(
        &mut self,
        adapter: &mut EgressAdapter<L>,
        limit: usize,
    ) -> Result<i32, ClusterError> {
        adapter.set_expected_session_id(self.cluster_session_id);
        tolerate_retryable_keep_alive(self.keep_alive_if_due())?;
        let mut new_leader: Option<(i64, i32, String)> = None;
        let mut decode_err: Option<ClusterError> = None;
        let mut session_closed = false;
        let mut ctx = PollCtx {
            adapter,
            new_leader: &mut new_leader,
            decode_err: &mut decode_err,
            expected_session_id: self.cluster_session_id,
            session_closed: &mut session_closed,
        };
        let n = self
            .regular_assembler
            .poll(&self.egress, &mut ctx, dispatch_regular::<L>, limit)
            .map_err(|e| ClusterError::aeron("poll", e))?;
        if let Some((term, member, endpoints)) = new_leader {
            self.on_new_leader_event(term, member, &endpoints)?;
        }
        if let Some(e) = decode_err {
            return Err(e);
        }
        if session_closed && self.state == SessionState::Connected {
            self.state = SessionState::PendingClose;
        }
        self.poll_state_changes()?;
        Ok(n)
    }

    /// Controlled variant — application messages are dispatched through a
    /// [`ControlledEgressListener`] whose `on_message` returns a
    /// [`ControlledPollAction`] for backpressure. Lifecycle, challenge, and
    /// admin callbacks are no-ops by default.
    pub fn poll_egress_controlled<L: ControlledEgressListener>(
        &mut self,
        adapter: &mut ControlledEgressAdapter<L>,
        fragment_limit: usize,
    ) -> Result<i32, ClusterError> {
        adapter.set_expected_session_id(self.cluster_session_id);
        tolerate_retryable_keep_alive(self.keep_alive_if_due())?;
        let mut new_leader: Option<(i64, i32, String)> = None;
        let mut decode_err: Option<ClusterError> = None;
        let mut session_closed = false;
        let mut ctx = ControlledPollCtx {
            adapter,
            new_leader: &mut new_leader,
            decode_err: &mut decode_err,
            expected_session_id: self.cluster_session_id,
            session_closed: &mut session_closed,
        };
        let n = self
            .controlled_assembler
            .poll(&self.egress, &mut ctx, dispatch_controlled::<L>, fragment_limit)
            .map_err(|e| ClusterError::aeron("controlled_poll", e))?;
        if let Some((term, member, endpoints)) = new_leader {
            self.on_new_leader_event(term, member, &endpoints)?;
        }
        if let Some(e) = decode_err {
            return Err(e);
        }
        if session_closed && self.state == SessionState::Connected {
            self.state = SessionState::PendingClose;
        }
        self.poll_state_changes()?;
        Ok(n)
    }

    /// Handle a `NewLeaderEvent` **atomically**: resolve the new leader
    /// endpoint, prepare a fresh ingress publication + assemblers *before*
    /// any session field is touched, then commit the leadership swap in one
    /// step. Uses [`Self::prepare_reconnect_ingress`] so the connect-time
    /// redirect and the active-session failover share one uniform code path.
    fn on_new_leader_event(&mut self, term: i64, member: i32, endpoints: &str) -> Result<(), ClusterError> {
        let ep =
            crate::poller::parse_leader_endpoint(endpoints, member).ok_or_else(|| ClusterError::ReconnectFailed {
                reason: format!("NewLeaderEvent listed no endpoint for leader member {member}: {endpoints}"),
            })?;
        let (new_pub, new_regular, new_controlled) =
            Self::prepare_reconnect_ingress(&self._aeron, &ep, self.ingress_stream_id)?;
        // All preparation succeeded — commit the leadership swap in one step.
        self.leadership_term_id = term;
        self.leader_member_id = member;
        self.ingress = new_pub;
        self.regular_assembler = new_regular;
        self.controlled_assembler = new_controlled;
        self.state = SessionState::Connected;
        self.awaiting_leader_since = None;
        self.cache_publication_limits();
        Ok(())
    }

    /// The cluster-assigned session id (set after connect completes).
    pub fn cluster_session_id(&self) -> i64 {
        self.cluster_session_id
    }
    /// The current leadership term (set after connect completes).
    pub fn leadership_term_id(&self) -> i64 {
        self.leadership_term_id
    }
    /// The current leader's member id (set after connect completes).
    pub fn leader_member_id(&self) -> i32 {
        self.leader_member_id
    }

    /// True when the ingress publication has a connected subscriber (the
    /// leader). Java `publication.isConnected()` analogue for
    /// backpressure-aware callers.
    #[inline]
    pub fn is_ingress_connected(&self) -> bool {
        self.ingress.is_connected()
    }

    /// True when the ingress publication is closed (fatal for this handle).
    #[inline]
    pub fn is_ingress_closed(&self) -> bool {
        self.ingress.is_closed()
    }

    /// Current ingress publication position.
    #[inline]
    pub fn ingress_position(&self) -> i64 {
        self.ingress.position()
    }

    /// True when the egress subscription still has a live image from the
    /// cluster (image count > 0). Going false while `Connected` is the
    /// leader-loss signal consumed by [`Self::poll_state_changes`].
    #[inline]
    pub fn is_egress_connected(&self) -> bool {
        self.egress.image_count().is_ok_and(|c| c > 0)
    }

    /// Request the cluster take a snapshot (Java
    /// `sendAdminRequestToTakeASnapshot`). The response arrives on the egress
    /// listener as [`crate::EgressListener::on_admin_response`].
    pub fn send_admin_request_to_take_snapshot(&mut self, correlation_id: i64) -> Result<i64, ClusterError> {
        self.send_admin_request(correlation_id, AdminRequestType::SNAPSHOT, &[])
    }

    /// Drive pure session-state transitions without polling fragments (Java
    /// `pollStateChanges`): leader-loss detection, `newLeaderTimeout`
    /// enforcement, and `PendingClose` → `Closed` finalisation. Call this from
    /// your event loop alongside [`Self::poll_egress`].
    pub fn poll_state_changes(&mut self) -> Result<(), ClusterError> {
        let connected = self.is_egress_connected();
        apply_state_transition(
            &mut self.state,
            &mut self.awaiting_leader_since,
            self.new_leader_timeout_ms,
            connected,
        )
    }

    /// Begin a poll-driven async connect. Returns an
    /// [`AsyncClusterConnect`] whose `poll()` advances the handshake.
    pub fn connect_async(builder: crate::SessionBuilder, aeron_dir: impl Into<String>) -> AsyncClusterConnect {
        AsyncClusterConnect::new(builder, aeron_dir.into())
    }
    /// Current session state: `Connected`, `NotConnected`, `Closed`.
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
    /// - [`ClusterError::PayloadTooLarge`] if header + payload exceeds the
    ///   publication's max message length
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
            return Err(ClusterError::NotConnected);
        }
        let total = MSG_HDR_TOTAL
            .checked_add(payload_len)
            .ok_or(ClusterError::PayloadTooLarge {
                operation: "try_claim",
                requested: payload_len,
                maximum: self.max_message_length,
            })?;
        if self.max_message_length > 0 && total > self.max_message_length {
            return Err(ClusterError::PayloadTooLarge {
                operation: "try_claim",
                requested: total,
                maximum: self.max_message_length,
            });
        }
        let mut claim = self
            .ingress
            .try_claim_owned(total)
            .map_err(|e| self.track_publication_error(ClusterError::from_offer_error("try_claim", e)))?;

        // Write the SessionMessageHeader (schema 111) into the claim's
        // first 32 bytes via the ergon encoder.
        if claim.data().len() < MSG_HDR_TOTAL {
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

/// Pure state-machine transition logic, extracted for unit testing.
/// Called by [`AeronCluster::poll_state_changes`].
pub(crate) fn apply_state_transition(
    state: &mut SessionState,
    awaiting_leader_since: &mut Option<Instant>,
    new_leader_timeout_ms: u64,
    egress_connected: bool,
) -> Result<(), ClusterError> {
    match *state {
        SessionState::Connected if !egress_connected => {
            *state = SessionState::AwaitingNewLeader;
            *awaiting_leader_since = Some(Instant::now());
        }
        SessionState::AwaitingNewLeader => {
            if let Some(since) = *awaiting_leader_since
                && since.elapsed() >= Duration::from_millis(new_leader_timeout_ms)
            {
                *state = SessionState::Closed;
                return Err(ClusterError::Disconnected {
                    reason: format!(
                        "no NewLeaderEvent within new_leader_timeout ({}ms)",
                        new_leader_timeout_ms
                    ),
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
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let start = MSG_HDR_TOTAL;
        &mut self.claim.data()[start..start + self.payload_len]
    }

    /// Stream position Aeron assigned to this claim.
    pub fn position(&self) -> i64 {
        self.claim.position()
    }

    /// Commit the claimed bytes, publishing them to the cluster.
    /// Note: commit errors cannot be classified as retryable — commit
    /// returns a generic Aeron error (`AeronCError`), not a publication
    /// sentinel like `offer`/`try_claim`. The caller should treat a failed
    /// commit as terminal for this claim.
    pub fn commit(self) -> Result<i64, ClusterError> {
        self.claim.commit().map_err(|e| ClusterError::aeron("claim_commit", e))
    }

    /// Abort the claim, discarding it as padding.
    pub fn abort(self) -> Result<(), ClusterError> {
        self.claim.abort().map_err(|e| ClusterError::aeron("claim_abort", e))
    }
}

/// Poll-driven **Aeron** async connect — mirrors Java `AeronCluster.AsyncConnect`.
///
/// Not Tokio/`async`/`await`. Drive [`Self::poll`] from your event loop until
/// [`Self::is_complete`], then [`Self::finish`]. Handles challenge-response and
/// redirect between polls.
#[must_use = "poll until is_complete(), then finish() to obtain the cluster client"]
pub struct AsyncClusterConnect {
    aeron: Option<Aeron>,
    ingress: Option<AeronExclusivePublication>,
    egress: Option<AeronSubscription>,
    builder: crate::SessionBuilder,
    aeron_dir: String,
    credentials: Vec<u8>,
    step: ConnectStep,
    connect_sent: bool,
    /// Wall-clock of last SessionConnectRequest offer attempt (success or not).
    last_connect_offer: Instant,
    reoffer_interval_ms: u64,
    started: Instant,
    /// Deadline computed on first poll (lazy to avoid panicking in `new`).
    deadline: Option<Instant>,
    cluster_session_id: i64,
    leadership_term_id: i64,
    leader_member_id: i32,
}

/// State machine step for [`AsyncClusterConnect`]. Each call to
/// [`AsyncClusterConnect::poll`] may advance one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectStep {
    /// Building the Aeron transport (client, publication, subscription).
    CreateTransport,
    /// Offering the session-connect request.
    SendConnect,
    /// Waiting for the NewLeaderEvent / OK response.
    PollResponse,
    /// Connect complete — [`AsyncClusterConnect::finish`] may be called.
    Done,
}

impl AsyncClusterConnect {
    pub(crate) fn new(builder: crate::SessionBuilder, aeron_dir: String) -> Self {
        let timeout_ms = builder.message_timeout_ms;
        let creds = builder
            .credentials
            .as_ref()
            .and_then(|c| c.encoded_credentials().map(|cow| cow.into_owned()))
            .unwrap_or_default();
        // Epoch-like past so the first SendConnect is not blocked by re-offer gate.
        let past = Instant::now()
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or_else(Instant::now);
        Self {
            aeron: None,
            ingress: None,
            egress: None,
            builder,
            aeron_dir,
            credentials: creds,
            step: ConnectStep::CreateTransport,
            connect_sent: false,
            last_connect_offer: past,
            reoffer_interval_ms: connect_reoffer_interval_ms(timeout_ms),
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
    /// [`#[non_exhaustive]`](ConnectStep) so future releases may add steps
    /// without a semver break.
    pub fn step(&self) -> ConnectStep {
        self.step
    }

    /// True once the connect has completed and `finish()` can be called.
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
                self.builder.validate()?;
                let dir_cstr = cformat!("{}", self.aeron_dir);
                let ctx = AeronContext::new().map_err(|e| ClusterError::aeron("ctx", e))?;
                ctx.set_dir(&dir_cstr).map_err(|e| ClusterError::aeron("set_dir", e))?;
                let aeron = Aeron::new(&ctx).map_err(|e| ClusterError::aeron("new", e))?;
                aeron.start().map_err(|e| ClusterError::aeron("start", e))?;
                let egr = self.builder.egress_for_aeron()?;
                let ing = self.builder.resolve_initial_ingress_for_aeron()?;
                let egress = aeron
                    .add_subscription(
                        egr,
                        self.builder.egress_stream_id,
                        Handlers::NONE,
                        Handlers::NONE,
                        Duration::from_secs(5),
                    )
                    .map_err(|e| ClusterError::aeron("sub", e))?;
                let ingress = aeron
                    .add_exclusive_publication(&ing, self.builder.ingress_stream_id, Duration::from_secs(5))
                    .map_err(|e| ClusterError::aeron("pub", e))?;
                self.aeron = Some(aeron);
                self.ingress = Some(ingress);
                self.egress = Some(egress);
                self.step = ConnectStep::SendConnect;
                Ok(true)
            }
            ConnectStep::SendConnect => {
                // Retry the offer across polls until the publication
                // connects and the connect request lands.
                if self.encode_and_send_connect()? {
                    self.step = ConnectStep::PollResponse;
                }
                Ok(true)
            }
            ConnectStep::PollResponse => {
                if let Some(ev) = self.poll_one_event()? {
                    use crate::poller::EgressEvent;
                    match ev {
                        EgressEvent::SessionEvent {
                            code,
                            cluster_session_id,
                            leadership_term_id,
                            leader_member_id,
                            detail,
                            ..
                        } => {
                            use crate::codecs::session::EventCode;
                            match code {
                                EventCode::OK => {
                                    self.cluster_session_id = cluster_session_id;
                                    self.leadership_term_id = leadership_term_id;
                                    self.leader_member_id = leader_member_id;
                                    self.step = ConnectStep::Done;
                                    return Ok(false);
                                }
                                EventCode::AUTHENTICATIONREJECTED => {
                                    return Err(ClusterError::AuthRejected);
                                }
                                EventCode::REDIRECT => {
                                    if let Some(ep) = crate::poller::parse_leader_endpoint(&detail, leader_member_id) {
                                        let c = uri::udp_endpoint_cstr(&ep)?;
                                        let aeron =
                                            self.aeron.as_ref().ok_or_else(|| ClusterError::ReconnectFailed {
                                                reason: "no aeron client for redirect".into(),
                                            })?;
                                        let p = aeron
                                            .add_exclusive_publication(
                                                &c,
                                                self.builder.ingress_stream_id,
                                                Duration::from_secs(5),
                                            )
                                            .map_err(|e| {
                                                ClusterError::reconnect(format!("redirect publication: {e}"))
                                            })?;
                                        self.ingress = Some(p);
                                        self.leader_member_id = leader_member_id;
                                        self.connect_sent = false;
                                        self.encode_and_send_connect()?;
                                    }
                                }
                                _ => {}
                            }
                        }
                        EgressEvent::Challenge {
                            correlation_id,
                            cluster_session_id,
                            encoded_challenge,
                        } => {
                            let resp = self
                                .builder
                                .credentials
                                .as_ref()
                                .and_then(|c| c.on_challenge(&encoded_challenge).map(|cow| cow.into_owned()))
                                .unwrap_or_default();
                            self.send_challenge_response(correlation_id, cluster_session_id, &resp)?;
                        }
                        _ => {}
                    }
                } else if self.last_connect_offer.elapsed() >= Duration::from_millis(self.reoffer_interval_ms) {
                    // Pre-election / silent non-leader: re-offer connect.
                    self.connect_sent = false;
                    let _ = self.encode_and_send_connect()?;
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
        let Self {
            aeron,
            ingress,
            egress,
            builder,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            ..
        } = self;
        let mut client = AeronCluster {
            _aeron: aeron.ok_or_else(|| ClusterError::connect("async connect finished without Aeron client"))?,
            ingress: ingress
                .ok_or_else(|| ClusterError::connect("async connect finished without ingress publication"))?,
            egress: egress
                .ok_or_else(|| ClusterError::connect("async connect finished without egress subscription"))?,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            state: SessionState::Connected,
            awaiting_leader_since: None,
            ingress_stream_id: builder.ingress_stream_id,
            new_leader_timeout_ms: builder.new_leader_timeout_ms,
            last_keep_alive: Instant::now(),
            max_message_length: 0,
            max_payload_length: 0,
            keep_alive_interval_ms,
            regular_assembler: AeronFragmentClosureAssembler::new()
                .map_err(|e| ClusterError::aeron("AeronFragmentClosureAssembler", e))?,
            controlled_assembler: AeronControlledFragmentClosureAssembler::new()
                .map_err(|e| ClusterError::aeron("AeronControlledFragmentClosureAssembler", e))?,
        };
        client.cache_publication_limits();
        Ok(client)
    }

    fn encode_and_send_connect(&mut self) -> Result<bool, ClusterError> {
        if self.connect_sent {
            return Ok(true);
        }
        let ch = self.builder.egress_channel_bytes();
        let info = client_info_bytes();
        let len = SessionConnectRequestEncoder::compute_encoded_length_with_message_header(
            ch.len(),
            self.credentials.len(),
            info.len(),
        );
        let mut buf = vec![0u8; len];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(0)
            .response_stream_id(self.builder.egress_stream_id)
            .version(0);
        let _ = enc
            .response_channel(ch)?
            .encoded_credentials(&self.credentials)?
            .client_info(&client_info_bytes())?;
        // Always stamp the attempt so re-offer cadence advances even under
        // backpressure (avoids tight spin when publication is not connected).
        self.last_connect_offer = Instant::now();
        if let Some(ingress) = &self.ingress
            && ingress.offer_raw(&buf, Handlers::NONE) > 0
        {
            self.connect_sent = true;
            return Ok(true);
        }
        Ok(false)
    }

    fn send_challenge_response(&mut self, cid: i64, csid: i64, creds: &[u8]) -> Result<(), ClusterError> {
        let len = ChallengeResponseEncoder::compute_encoded_length_with_message_header(creds.len());
        let mut buf = vec![0u8; len];
        let mut enc = ChallengeResponseEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(cid).cluster_session_id(csid);
        let _ = enc.encoded_credentials(creds)?;
        if let Some(ingress) = &self.ingress {
            let r = ingress.offer_raw(&buf, Handlers::NONE);
            offer_result("challenge_response", r)?;
        }
        Ok(())
    }

    fn poll_one_event(&mut self) -> Result<Option<crate::poller::EgressEvent>, ClusterError> {
        let mut ev: Option<crate::poller::EgressEvent> = None;
        let mut err: Option<ClusterError> = None;
        if let Some(egress) = &self.egress {
            let _ = egress.poll_fn(
                |data, _hdr| {
                    if ev.is_none() && err.is_none() {
                        match crate::poller::parse_event(data) {
                            Ok(e) => ev = e,
                            Err(e) => err = Some(e),
                        }
                    }
                },
                1,
            );
        }
        if let Some(e) = err {
            return Err(e);
        }
        Ok(ev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn retryable_keep_alive_failure_does_not_block_egress_poll() -> Result<(), Box<dyn std::error::Error>> {
        for code in [-1, -2, -3] {
            tolerate_retryable_keep_alive(Err(ClusterError::from_offer_raw("track_ingress", code)))?;
        }
        for code in [-4, -5] {
            assert!(tolerate_retryable_keep_alive(Err(ClusterError::from_offer_raw("track_ingress", code))).is_err());
        }
        Ok(())
    }

    #[test]
    fn test_encoded_length_matches_encode_connect_request() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::SessionConnectRequestEncoder;

        let ch = b"aeron:udp?endpoint=localhost:9999";
        let creds = b"user:pass";
        let info = client_info_bytes();
        let len =
            SessionConnectRequestEncoder::compute_encoded_length_with_message_header(ch.len(), creds.len(), info.len());
        let mut buf = vec![0u8; len];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(0).response_stream_id(102).version(0);
        let complete = enc
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
        use crate::codecs::session::ChallengeResponseEncoder;

        let creds = b"challenge-response-bytes";
        let len = ChallengeResponseEncoder::compute_encoded_length_with_message_header(creds.len());
        let mut buf = vec![0u8; len];
        let mut enc = ChallengeResponseEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(1).cluster_session_id(2);
        let complete = enc.encoded_credentials(creds)?;
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
        // T1: PendingClose → Closed via poll_state_changes → apply_state_transition.
        let mut state = SessionState::PendingClose;
        let mut awaiting = None;
        apply_state_transition(&mut state, &mut awaiting, 5_000, true)?;
        assert_eq!(state, SessionState::Closed);
        Ok(())
    }

    #[test]
    fn test_new_leader_timeout_disconnects() -> Result<(), Box<dyn std::error::Error>> {
        // T7: After deadline, AwaitingNewLeader → Closed + Disconnected error.
        let mut state = SessionState::AwaitingNewLeader;
        let mut awaiting = Some(Instant::now() - Duration::from_secs(6));
        let result = apply_state_transition(&mut state, &mut awaiting, 5_000, false);
        match result {
            Err(ClusterError::Disconnected { .. }) => {}
            other => panic!("expected Disconnected, got {other:?}"),
        }
        assert_eq!(state, SessionState::Closed);
        Ok(())
    }

    #[test]
    fn test_reconnect_prepares_before_swapping_state() -> Result<(), Box<dyn std::error::Error>> {
        // T6: Structural proof — `on_new_leader_event` must call
        // `parse_leader_endpoint` and `prepare_reconnect_ingress` BEFORE
        // writing any leadership field.  Failure must leave prior state intact.
        let src = include_str!("client.rs");
        let off = src
            .find("fn on_new_leader_event")
            .ok_or("on_new_leader_event not found")?;
        let snippet = &src[off..src[off..].find("Ok(())").unwrap_or(src.len()) + off];
        let parse_call = snippet.find("parse_leader_endpoint(");
        let prepare_call = snippet.find("prepare_reconnect_ingress(");
        let first_assign = snippet.find("self.leadership_term_id").unwrap_or(usize::MAX);
        assert!(parse_call.is_some(), "must call parse_leader_endpoint");
        assert!(prepare_call.is_some(), "must call prepare_reconnect_ingress");
        assert!(
            parse_call.unwrap() < first_assign,
            "parse_leader_endpoint must precede leadership_term_id write"
        );
        assert!(
            prepare_call.unwrap() < first_assign,
            "prepare_reconnect_ingress (pub+assemblers) must precede state swap"
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
        apply_state_transition(&mut state, &mut awaiting, 5_000, false)?;
        assert_eq!(state, SessionState::AwaitingNewLeader);
        Ok(())
    }
}
