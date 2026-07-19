//! The [`AeronCluster`] client — owns the Aeron transport and drives the
//! full SBE session handshake. Java-parity entry point for the **Ergo Aeron
//! Cluster** client (experimental prototype on `rusteron-client`):
//!
//! ```ignore
//! let mut client = AeronCluster::connect(builder, aeron_dir)?;
//! client.offer(b"hello")?;
//! client.poll_egress(&mut adapter, 10)?;
//! client.send_keep_alive()?;
//! client.close()?;
//! ```
//!
//! Hot path: [`AeronCluster::try_claim`] writes SessionMessageHeader via ErgoSBE
//! into the claim; fill app payload then [`ClusterClaim::commit`].

use std::time::{Duration, Instant};

use rusteron_client::{cformat, Aeron, AeronClaim, AeronContext, AeronExclusivePublication, AeronSubscription, Handlers};

use crate::codecs::ergo_codecs::{
    ChallengeResponseEncoder, SessionCloseRequestEncoder, SessionConnectRequestEncoder, SessionKeepAliveEncoder,
    SessionMessageHeaderEncoder,
};
use crate::connect::{connect_reoffer_interval_ms, should_reoffer_connect};
use crate::egress::{EgressAdapter, EgressListener};
use crate::error::ClusterError;
use crate::state::SessionState;

/// Header-inclusive SessionMessageHeader (prefer generated `ENCODED_LENGTH`).
const MSG_HDR_TOTAL: usize = SessionMessageHeaderEncoder::ENCODED_LENGTH;

/// Map an ErgoSBE encode error into the cluster error enum.
fn enc_err<E: std::fmt::Debug>(e: E) -> ClusterError {
    ClusterError::Publication {
        reason: format!("sbe encode: {e:?}"),
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
    /// Configured ingress stream id, retained so a `NewLeaderEvent`
    /// reconnect uses the same stream the session was established on
    /// (the post-connect path no longer has the builder).
    ingress_stream_id: i32,
}

impl AeronCluster {
    /// Connect to a cluster whose media driver lives at `aeron_dir`.
    /// Performs the full SBE handshake: SessionConnectRequest → poll
    /// egress for SessionEvent(OK) → extract session/leadership IDs.
    pub fn connect(builder: &crate::SessionBuilder, aeron_dir: &str) -> Result<Self, ClusterError> {
        builder.validate()?;

        let dir_cstr = cformat!("{aeron_dir}");
        let ctx = AeronContext::new().map_err(|e| ClusterError::ConnectFailed {
            reason: format!("AeronContext: {e}"),
        })?;
        ctx.set_dir(&dir_cstr).map_err(|e| ClusterError::ConnectFailed {
            reason: format!("set_dir: {e}"),
        })?;
        let aeron = Aeron::new(&ctx).map_err(|e| ClusterError::ConnectFailed {
            reason: format!("Aeron::new: {e}"),
        })?;
        aeron.start().map_err(|e| ClusterError::ConnectFailed {
            reason: format!("Aeron::start: {e}"),
        })?;

        let egress_cstr = cformat!("{}", builder.egress_channel);
        let ingress_cstr = cformat!("{}", builder.ingress_channel);

        let egress = aeron
            .add_subscription(
                &egress_cstr,
                builder.egress_stream_id,
                Handlers::NONE,
                Handlers::NONE,
                Duration::from_secs(5),
            )
            .map_err(|e| ClusterError::ConnectFailed {
                reason: format!("add_subscription: {e}"),
            })?;

        let ingress = aeron
            .add_exclusive_publication(&ingress_cstr, builder.ingress_stream_id, Duration::from_secs(5))
            .map_err(|e| ClusterError::ConnectFailed {
                reason: format!("add_exclusive_publication: {e}"),
            })?;

        let mut client = Self {
            _aeron: aeron,
            ingress,
            egress,
            cluster_session_id: -1,
            leadership_term_id: -1,
            leader_member_id: -1,
            state: SessionState::Closed,
            ingress_stream_id: builder.ingress_stream_id,
        };

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
            .and_then(|c| c.encoded_credentials())
            .unwrap_or_default();

        self.send_connect_request(builder, &creds)?;
        let mut last_offer = Instant::now();
        let reoffer_ms = connect_reoffer_interval_ms(builder.message_timeout_ms);

        let deadline = Instant::now() + Duration::from_millis(builder.message_timeout_ms);
        let mut captured: Option<crate::poller::EgressEvent> = None;
        while Instant::now() < deadline {
            let _ = self.egress.poll_fn(
                |data, _hdr| {
                    if captured.is_none() {
                        captured = crate::poller::parse_event(data);
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
                    use crate::codecs::ergo_codecs::EventCode;
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
                        .and_then(|c| c.on_challenge(&encoded_challenge))
                        .unwrap_or_default();
                    self.send_challenge_response(correlation_id, cluster_session_id, &resp)?;
                    // keep polling for the resulting SessionEvent
                }
                _ => {
                    // No event: re-offer connect if the interval elapsed so a
                    // pre-election peer that neither leads nor redirects does
                    // not burn the full timeout on a single silent offer.
                    if should_reoffer_connect(last_offer, Instant::now(), reoffer_ms) {
                        let _ = self.try_offer_connect_request(builder, &creds);
                        last_offer = Instant::now();
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(50));
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
        let deadline = Instant::now() + Duration::from_millis(builder.message_timeout_ms);
        while Instant::now() < deadline {
            if self.ingress.offer_raw(&buf, Handlers::NONE) > 0 {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
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
        let mut buf = vec![0u8; 512];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0).map_err(enc_err)?;
        let _ = enc
            .correlation_id(0)
            .response_stream_id(builder.egress_stream_id)
            .version(0);
        let _ = enc
            .response_channel(builder.egress_channel.as_bytes())
            .map_err(enc_err)?
            .encoded_credentials(credentials)
            .map_err(enc_err)?
            .client_info(b"")
            .map_err(enc_err)?;
        Ok(buf)
    }

    fn send_challenge_response(
        &mut self,
        correlation_id: i64,
        cluster_session_id: i64,
        credentials: &[u8],
    ) -> Result<(), ClusterError> {
        let mut buf = vec![0u8; 512];
        let mut enc = ChallengeResponseEncoder::wrap_and_apply_header(&mut buf, 0).map_err(enc_err)?;
        let _ = enc
            .correlation_id(correlation_id)
            .cluster_session_id(cluster_session_id);
        let _ = enc.encoded_credentials(credentials).map_err(enc_err)?;
        let r = self.ingress.offer_raw(&buf, Handlers::NONE);
        if r <= 0 {
            return Err(ClusterError::Publication {
                reason: format!("challenge response offer returned {r}"),
            });
        }
        Ok(())
    }

    /// Recreate the ingress publication pointed at a new leader endpoint.
    fn reconnect_ingress(&mut self, builder: &crate::SessionBuilder, endpoint: &str) -> Result<(), ClusterError> {
        // Close the old publication then open a new one to the leader.
        let cstr = cformat!("aeron:udp?endpoint={endpoint}");
        let new_pub = self
            ._aeron
            .add_exclusive_publication(&cstr, builder.ingress_stream_id, Duration::from_secs(5))
            .map_err(|e| ClusterError::ConnectFailed {
                reason: format!("redirect pub: {e}"),
            })?;
        self.ingress = new_pub;
        Ok(())
    }

    /// Publish an application message. Prepends the SessionMessageHeader
    /// (leadershipTermId + clusterSessionId + timestamp).
    pub fn offer(&mut self, payload: &[u8]) -> Result<i64, ClusterError> {
        if self.state != SessionState::Connected {
            return Err(ClusterError::NotConnected);
        }
        let mut buf = vec![0u8; MSG_HDR_TOTAL + payload.len()];
        let _ = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
            .map_err(enc_err)?
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id)
            .timestamp(0);
        buf[MSG_HDR_TOTAL..].copy_from_slice(payload);

        let r = self.ingress.offer_raw(&buf, Handlers::NONE);
        if r > 0 {
            Ok(r)
        } else {
            Err(ClusterError::Publication {
                reason: format!("offer returned {r} (backpressure / not connected)"),
            })
        }
    }

    /// Send a SessionKeepAlive to hold the session open.
    pub fn send_keep_alive(&mut self) -> Result<(), ClusterError> {
        let mut buf = vec![0u8; 8 + SessionKeepAliveEncoder::BLOCK_LENGTH];
        let _ = SessionKeepAliveEncoder::wrap_and_apply_header(&mut buf, 0)
            .map_err(enc_err)?
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id);
        let r = self.ingress.offer_raw(&buf, Handlers::NONE);
        if r < 0 {
            return Err(ClusterError::Publication {
                reason: format!("keep-alive offer returned {r}"),
            });
        }
        Ok(())
    }

    /// Send a SessionCloseRequest and mark the session PendingClose.
    pub fn close(&mut self) -> Result<(), ClusterError> {
        if self.state == SessionState::Closed {
            return Err(ClusterError::SessionClosed);
        }
        let mut buf = vec![0u8; 8 + SessionCloseRequestEncoder::BLOCK_LENGTH];
        let _ = SessionCloseRequestEncoder::wrap_and_apply_header(&mut buf, 0)
            .map_err(enc_err)?
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
    /// Handles `NewLeaderEvent` internally: updates the session's
    /// leadership term / leader id and reconnects the ingress
    /// publication to the new leader. Returns fragments polled.
    pub fn poll_egress<L: EgressListener>(
        &mut self,
        adapter: &mut EgressAdapter<L>,
        limit: usize,
    ) -> Result<i32, ClusterError> {
        // Capture any NewLeaderEvent during the poll so we can act on
        // it after (avoids &mut self inside the poll_fn closure). Capture the
        // first decode error too — the closure is infallible, so we buffer it
        // and surface it after the batch instead of dropping it with `let _ =`.
        let mut new_leader: Option<(i64, i32, String)> = None;
        let mut decode_err: Option<ClusterError> = None;
        let n = self
            .egress
            .poll_fn(
                |data, _hdr| {
                    if let Some(crate::poller::EgressEvent::NewLeader {
                        leadership_term_id,
                        leader_member_id,
                        ingress_endpoints,
                        ..
                    }) = crate::poller::parse_event(data)
                        && new_leader.is_none()
                    {
                        new_leader = Some((leadership_term_id, leader_member_id, ingress_endpoints));
                    }
                    if let Err(e) = adapter.on_fragment(data)
                        && decode_err.is_none()
                    {
                        decode_err = Some(e);
                    }
                },
                limit,
            )
            .map_err(|e| ClusterError::ConnectFailed {
                reason: format!("poll_fn: {e}"),
            })?;

        if let Some((term, member, endpoints)) = new_leader {
            self.leadership_term_id = term;
            self.leader_member_id = member;
            self.state = SessionState::AwaitingNewLeaderConnection;
            // Resolve the NEW leader's endpoint by member id — the endpoints
            // list is in id order, so a position-based parse would reconnect
            // to the dead leader. The session continues; only the ingress
            // publication is redirected to the new leader.
            let ep = crate::poller::parse_leader_endpoint(&endpoints, member).ok_or_else(|| {
                ClusterError::ReconnectFailed {
                    reason: format!("NewLeaderEvent listed no endpoint for leader member {member}: {endpoints}"),
                }
            })?;
            let cstr = cformat!("aeron:udp?endpoint={ep}");
            let pub_ = self
                ._aeron
                .add_exclusive_publication(&cstr, self.ingress_stream_id, Duration::from_secs(5))
                .map_err(|e| ClusterError::ReconnectFailed {
                    reason: format!("new-leader publication to member {member}: {e}"),
                })?;
            self.ingress = pub_;
            self.state = SessionState::Connected;
        }
        // Surface a buffered decode error AFTER the NewLeaderEvent reconnect —
        // failover must complete even if an unrelated fragment in the same batch
        // was malformed.
        if let Some(e) = decode_err {
            return Err(e);
        }
        Ok(n)
    }

    pub fn cluster_session_id(&self) -> i64 {
        self.cluster_session_id
    }
    pub fn leadership_term_id(&self) -> i64 {
        self.leadership_term_id
    }
    pub fn leader_member_id(&self) -> i32 {
        self.leader_member_id
    }

    /// Begin a poll-driven async connect. Returns an
    /// [`AsyncClusterConnect`] whose `poll()` advances the handshake.
    pub fn connect_async(builder: crate::SessionBuilder, aeron_dir: impl Into<String>) -> AsyncClusterConnect {
        AsyncClusterConnect::new(builder, aeron_dir.into())
    }
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Zero-copy publish: claim a region of the ingress term buffer,
    /// write the SessionMessageHeader into the first 32 bytes via ErgoSBE,
    /// and expose the remaining `payload_len` bytes for the caller to fill
    /// directly. Mirrors Java `AeronCluster.tryClaim(length, claim)`.
    ///
    /// # Errors
    ///
    /// - [`ClusterError::NotConnected`] if the session is not Connected
    /// - [`ClusterError::Publication`] on claim failure / backpressure
    /// - [`ClusterError::BufferTooSmall`] if the claim buffer is shorter than
    ///   the 32-byte session header
    ///
    /// # Hot path
    ///
    /// This is the HFT publish path. On success there is no temp buffer copy of
    /// the application payload — fill [`ClusterClaim::payload_mut`] then
    /// [`ClusterClaim::commit`]. Abort with [`ClusterClaim::abort`] (or drop).
    ///
    /// ```rust,ignore
    /// let mut claim = client.try_claim(app_len)?;
    /// claim.payload_mut().copy_from_slice(&app_bytes);
    /// claim.commit()?;
    /// ```
    pub fn try_claim(&mut self, payload_len: usize) -> Result<ClusterClaim, ClusterError> {
        if self.state != SessionState::Connected {
            return Err(ClusterError::NotConnected);
        }
        let total = MSG_HDR_TOTAL + payload_len;
        let mut claim = self
            .ingress
            .try_claim_owned(total)
            .map_err(|e| ClusterError::Publication {
                reason: format!("try_claim: {e}"),
            })?;

        // Write the SessionMessageHeader (schema 111) into the claim's
        // first 32 bytes via the ErgoSBE encoder.
        if claim.data().len() < MSG_HDR_TOTAL {
            return Err(ClusterError::BufferTooSmall {
                needed: MSG_HDR_TOTAL,
                actual: claim.data().len(),
            });
        }
        let _ = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut claim.data()[0..MSG_HDR_TOTAL], 0)
            .map_err(enc_err)?
            .leadership_term_id(self.leadership_term_id)
            .cluster_session_id(self.cluster_session_id)
            .timestamp(0);

        Ok(ClusterClaim { claim, payload_len })
    }
}

/// A zero-copy claim on the ingress publication. The caller writes the
/// application payload into `payload_mut()` (the bytes after the 32-byte
/// SessionMessageHeader) then calls `commit()`.
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
    pub fn commit(self) -> Result<i64, ClusterError> {
        self.claim.commit().map_err(|e| ClusterError::ConnectFailed {
            reason: format!("commit: {e}"),
        })
    }

    /// Abort the claim, discarding it as padding.
    pub fn abort(self) -> Result<(), ClusterError> {
        self.claim.abort().map_err(|e| ClusterError::ConnectFailed {
            reason: format!("abort: {e}"),
        })
    }
}

/// Poll-driven async connect — mirrors Java `AeronCluster.AsyncConnect`.
///
/// Created by [`crate::SessionBuilder::connect_async`]. Call `poll()`
/// repeatedly; it returns `Ok(true)` while more steps remain, `Ok(false)`
/// when the connect has completed (then call `finish()` to get the
/// [`AeronCluster`]). Handles challenge-response and redirect between polls.
pub struct AsyncClusterConnect {
    aeron: Option<Aeron>,
    ingress: Option<AeronExclusivePublication>,
    egress: Option<AeronSubscription>,
    builder: crate::SessionBuilder,
    aeron_dir: String,
    credentials: Vec<u8>,
    step: AsyncStep,
    connect_sent: bool,
    /// Wall-clock of last SessionConnectRequest offer attempt (success or not).
    last_connect_offer: Instant,
    reoffer_interval_ms: u64,
    deadline: Instant,
    cluster_session_id: i64,
    leadership_term_id: i64,
    leader_member_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AsyncStep {
    CreateTransport,
    SendConnect,
    PollResponse,
    Done,
}

impl AsyncClusterConnect {
    pub(crate) fn new(builder: crate::SessionBuilder, aeron_dir: String) -> Self {
        let timeout_ms = builder.message_timeout_ms;
        let creds = builder
            .credentials
            .as_ref()
            .and_then(|c| c.encoded_credentials())
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
            step: AsyncStep::CreateTransport,
            connect_sent: false,
            last_connect_offer: past,
            reoffer_interval_ms: connect_reoffer_interval_ms(timeout_ms),
            deadline: Instant::now() + Duration::from_millis(timeout_ms),
            cluster_session_id: -1,
            leadership_term_id: -1,
            leader_member_id: -1,
        }
    }

    /// Current connect step.
    pub fn step(&self) -> &'static str {
        match self.step {
            AsyncStep::CreateTransport => "create_transport",
            AsyncStep::SendConnect => "send_connect",
            AsyncStep::PollResponse => "poll_response",
            AsyncStep::Done => "done",
        }
    }

    /// True once the connect has completed and `finish()` can be called.
    pub fn is_complete(&self) -> bool {
        self.step == AsyncStep::Done
    }

    /// Advance the connect by one unit of work. Returns `Ok(true)` if
    /// more polling is needed, `Ok(false)` once complete.
    pub fn poll(&mut self) -> Result<bool, ClusterError> {
        if Instant::now() > self.deadline {
            return Err(ClusterError::Timeout {
                phase: "async_connect",
                after_ms: 0,
            });
        }
        match self.step {
            AsyncStep::CreateTransport => {
                self.builder.validate()?;
                let dir_cstr = cformat!("{}", self.aeron_dir);
                let ctx = AeronContext::new().map_err(|e| ClusterError::ConnectFailed {
                    reason: format!("ctx: {e}"),
                })?;
                ctx.set_dir(&dir_cstr).map_err(|e| ClusterError::ConnectFailed {
                    reason: format!("set_dir: {e}"),
                })?;
                let aeron = Aeron::new(&ctx).map_err(|e| ClusterError::ConnectFailed {
                    reason: format!("new: {e}"),
                })?;
                aeron.start().map_err(|e| ClusterError::ConnectFailed {
                    reason: format!("start: {e}"),
                })?;
                let egr = cformat!("{}", self.builder.egress_channel);
                let ing = cformat!("{}", self.builder.ingress_channel);
                let egress = aeron
                    .add_subscription(
                        &egr,
                        self.builder.egress_stream_id,
                        Handlers::NONE,
                        Handlers::NONE,
                        Duration::from_secs(5),
                    )
                    .map_err(|e| ClusterError::ConnectFailed {
                        reason: format!("sub: {e}"),
                    })?;
                let ingress = aeron
                    .add_exclusive_publication(&ing, self.builder.ingress_stream_id, Duration::from_secs(5))
                    .map_err(|e| ClusterError::ConnectFailed {
                        reason: format!("pub: {e}"),
                    })?;
                self.aeron = Some(aeron);
                self.ingress = Some(ingress);
                self.egress = Some(egress);
                self.step = AsyncStep::SendConnect;
                Ok(true)
            }
            AsyncStep::SendConnect => {
                // Retry the offer across polls until the publication
                // connects and the connect request lands.
                if self.encode_and_send_connect()? {
                    self.step = AsyncStep::PollResponse;
                }
                Ok(true)
            }
            AsyncStep::PollResponse => {
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
                            use crate::codecs::ergo_codecs::EventCode;
                            match code {
                                EventCode::OK => {
                                    self.cluster_session_id = cluster_session_id;
                                    self.leadership_term_id = leadership_term_id;
                                    self.leader_member_id = leader_member_id;
                                    self.step = AsyncStep::Done;
                                    return Ok(false);
                                }
                                EventCode::AUTHENTICATIONREJECTED => {
                                    return Err(ClusterError::AuthRejected);
                                }
                                EventCode::REDIRECT => {
                                    if let Some((member_id, ep)) = crate::poller::parse_redirect_leader(&detail) {
                                        let c = cformat!("aeron:udp?endpoint={ep}");
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
                                            .map_err(|e| ClusterError::ReconnectFailed {
                                                reason: format!("redirect publication: {e}"),
                                            })?;
                                        self.ingress = Some(p);
                                        self.leader_member_id = member_id;
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
                                .and_then(|c| c.on_challenge(&encoded_challenge))
                                .unwrap_or_default();
                            self.send_challenge_response(correlation_id, cluster_session_id, &resp)?;
                        }
                        _ => {}
                    }
                } else if should_reoffer_connect(self.last_connect_offer, Instant::now(), self.reoffer_interval_ms) {
                    // Pre-election / silent non-leader: re-offer connect.
                    self.connect_sent = false;
                    let _ = self.encode_and_send_connect()?;
                }
                Ok(true)
            }
            AsyncStep::Done => Ok(false),
        }
    }

    /// Consume the in-progress connect and yield the connected client.
    pub fn finish(self) -> Result<AeronCluster, ClusterError> {
        if self.step != AsyncStep::Done {
            return Err(ClusterError::ConnectFailed {
                reason: "connect not complete".into(),
            });
        }
        Ok(AeronCluster {
            _aeron: self.aeron.unwrap(),
            ingress: self.ingress.unwrap(),
            egress: self.egress.unwrap(),
            cluster_session_id: self.cluster_session_id,
            leadership_term_id: self.leadership_term_id,
            leader_member_id: self.leader_member_id,
            state: SessionState::Connected,
            ingress_stream_id: self.builder.ingress_stream_id,
        })
    }

    fn encode_and_send_connect(&mut self) -> Result<bool, ClusterError> {
        if self.connect_sent {
            return Ok(true);
        }
        let mut buf = vec![0u8; 512];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0).map_err(enc_err)?;
        let _ = enc
            .correlation_id(0)
            .response_stream_id(self.builder.egress_stream_id)
            .version(0);
        let _ = enc
            .response_channel(self.builder.egress_channel.as_bytes())
            .map_err(enc_err)?
            .encoded_credentials(&self.credentials)
            .map_err(enc_err)?
            .client_info(b"")
            .map_err(enc_err)?;
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
        let mut buf = vec![0u8; 512];
        let mut enc = ChallengeResponseEncoder::wrap_and_apply_header(&mut buf, 0).map_err(enc_err)?;
        let _ = enc.correlation_id(cid).cluster_session_id(csid);
        let _ = enc.encoded_credentials(creds).map_err(enc_err)?;
        if let Some(ingress) = &self.ingress {
            let r = ingress.offer_raw(&buf, Handlers::NONE);
            if r <= 0 {
                return Err(ClusterError::Publication {
                    reason: format!("challenge response offer returned {r}"),
                });
            }
        }
        Ok(())
    }

    fn poll_one_event(&mut self) -> Result<Option<crate::poller::EgressEvent>, ClusterError> {
        let mut ev: Option<crate::poller::EgressEvent> = None;
        if let Some(egress) = &self.egress {
            let _ = egress.poll_fn(
                |data, _hdr| {
                    if ev.is_none() {
                        ev = crate::poller::parse_event(data);
                    }
                },
                1,
            );
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
        use crate::codecs::ergo_codecs::{
            SessionCloseRequestEncoder, SessionKeepAliveEncoder, SessionMessageHeaderEncoder,
        };
        assert_eq!(SessionMessageHeaderEncoder::TEMPLATE_ID, 1);
        assert_eq!(SessionKeepAliveEncoder::TEMPLATE_ID, 5);
        assert_eq!(SessionCloseRequestEncoder::TEMPLATE_ID, 4);
    
        Ok(())
    }
}
