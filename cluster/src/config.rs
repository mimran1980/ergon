//! [`SessionBuilder`]: the channel, stream and timeout settings for connect.
//!
//! It mirrors Java `AeronCluster.Context`. Defaults: ingress stream 101,
//! egress stream 102, 5 s message timeout.
//!
//! Channels are stored as **`CString`** so connect can hand rusteron a `&CStr`
//! without converting through `String`.

use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusteron_client::{Aeron, IdleStrategy};

use crate::protocol::DEFAULT_CLIENT_NAME;
use crate::uri;
use crate::{ClusterError, CredentialsSupplier};

/// Invoked from the Aeron client conductor when the driver reports an error.
pub(crate) type ErrorHandler = Arc<dyn Fn(i32, &str) + Send + Sync>;

/// Builds and connects an [`crate::AeronCluster`].
///
/// Channel setters normalise the URI with `AeronUriStringBuilder` and store it
/// as a `CString`, so connect passes `&CStr` to rusteron without allocating again.
///
/// # Example
///
/// ```rust,no_run
/// use ergo_aeron_cluster::{AeronCluster, ClusterError, SessionBuilder};
///
/// fn connect(aeron_dir: &str) -> Result<AeronCluster, ClusterError> {
///     let builder = SessionBuilder::default()
///         .ingress_channel("aeron:udp?endpoint=localhost:9002")?
///         .egress_channel("aeron:udp?endpoint=localhost:19002")?;
///     AeronCluster::connect(&builder, aeron_dir)
/// }
/// ```
#[derive(Clone)]
#[must_use = "retain the builder; each setter consumes self and returns a new builder"]
pub struct SessionBuilder {
    ingress_c: Option<CString>,
    egress_c: Option<CString>,
    pub(crate) ingress_stream_id: i32,
    pub(crate) egress_stream_id: i32,
    pub(crate) message_timeout_ms: u64,
    /// Deadline (ms) for awaiting a `NewLeaderEvent` before the session is
    /// deemed dead. `None` is Java's NULL: connect uses twice the leader
    /// heartbeat, or 20s when the cluster does not send one.
    pub(crate) new_leader_timeout_ms: Option<u64>,
    pub(crate) credentials: Option<Arc<dyn CredentialsSupplier>>,
    /// Multi-member ingress endpoints: `"0=host:port,1=host:port,..."`.
    pub(crate) ingress_endpoints: Option<String>,
    /// An existing Aeron client. Connect clones it and does not call `start`,
    /// install an error handler or set a client name.
    pub(crate) shared_aeron: Option<Aeron>,
    /// Aeron client name. `None` uses [`DEFAULT_CLIENT_NAME`] when this
    /// builder creates the client.
    pub(crate) client_name: Option<CString>,
    /// Error handler installed only when this builder creates the client.
    pub(crate) error_handler: Option<ErrorHandler>,
    /// Idle strategy for the sync-connect retry loop (Java
    /// `Context.idleStrategy`). `None` spins briefly then sleeps 1µs.
    pub(crate) idle: Option<Arc<Mutex<dyn IdleStrategy + Send + Sync>>>,
}

/// Rejects zero, sub-millisecond and overflowing durations before they reach
/// the protocol poll state machine.
fn checked_timeout_ms(d: Duration, field: &'static str) -> Result<u64, ClusterError> {
    let millis = d.as_millis();
    if millis == 0 {
        return Err(ClusterError::InvalidTimeout {
            phase: field,
            reason: "timeout must be >= 1ms (zero or sub-millisecond rejected)",
        });
    }
    u64::try_from(millis).map_err(|_| ClusterError::InvalidTimeout {
        phase: field,
        reason: "timeout exceeds u64 millisecond range",
    })
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            ingress_c: None,
            egress_c: None,
            ingress_stream_id: 101,
            egress_stream_id: 102,
            message_timeout_ms: 5_000,
            new_leader_timeout_ms: None,
            credentials: None,
            ingress_endpoints: None,
            shared_aeron: None,
            client_name: None,
            error_handler: None,
            idle: None,
        }
    }
}

impl core::fmt::Debug for SessionBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SessionBuilder")
            .field(
                "ingress_channel",
                &self
                    .ingress_c
                    .as_ref()
                    .and_then(|c| c.to_str().ok())
                    .unwrap_or("<unset>"),
            )
            .field(
                "egress_channel",
                &self
                    .egress_c
                    .as_ref()
                    .and_then(|c| c.to_str().ok())
                    .unwrap_or("<unset>"),
            )
            .field("ingress_stream_id", &self.ingress_stream_id)
            .field("egress_stream_id", &self.egress_stream_id)
            .field("message_timeout_ms", &self.message_timeout_ms)
            .field("new_leader_timeout_ms", &self.new_leader_timeout_ms)
            .field("ingress_endpoints", &self.ingress_endpoints)
            .field(
                "client_name",
                &self
                    .client_name
                    .as_ref()
                    .and_then(|name| name.to_str().ok())
                    .unwrap_or(DEFAULT_CLIENT_NAME),
            )
            .field(
                "aeron",
                &if self.shared_aeron.is_some() {
                    "<shared>"
                } else {
                    "<owned>"
                },
            )
            .field(
                "error_handler",
                &if self.error_handler.is_some() {
                    "<configured>"
                } else {
                    "<default>"
                },
            )
            .field(
                "credentials",
                &if self.credentials.is_some() {
                    "<configured>"
                } else {
                    "<none>"
                },
            )
            .field(
                "idle_strategy",
                &if self.idle.is_some() {
                    "<configured>"
                } else {
                    "<default>"
                },
            )
            .finish()
    }
}

impl SessionBuilder {
    /// Sets the ingress channel URI.
    ///
    /// # Errors
    ///
    /// A malformed URI fails immediately. A later successful call replaces the
    /// previous value.
    pub fn ingress_channel(mut self, channel: impl AsRef<str>) -> Result<Self, ClusterError> {
        self.ingress_c = Some(uri::channel_cstr(channel.as_ref())?);
        Ok(self)
    }

    /// Sets the egress channel URI.
    ///
    /// # Errors
    ///
    /// A malformed URI fails immediately.
    pub fn egress_channel(mut self, channel: impl AsRef<str>) -> Result<Self, ClusterError> {
        self.egress_c = Some(uri::channel_cstr(channel.as_ref())?);
        Ok(self)
    }

    /// Overrides the ingress stream id (default 101).
    pub const fn ingress_stream_id(mut self, stream_id: i32) -> Self {
        self.ingress_stream_id = stream_id;
        self
    }

    /// Overrides the egress stream id (default 102).
    pub const fn egress_stream_id(mut self, stream_id: i32) -> Self {
        self.egress_stream_id = stream_id;
        self
    }

    /// Sets the deadline for the connect sequence and for a new leader's
    /// ingress publication to connect after its `NewLeaderEvent`. It also sets
    /// the keep-alive interval:
    /// [`AeronCluster::keep_alive_if_due`](crate::AeronCluster::keep_alive_if_due)
    /// sends one every quarter of it, clamped to 50 ms to 1 s. Default: 5 seconds.
    ///
    /// # Errors
    ///
    /// A zero, sub-millisecond or overflowing duration fails immediately.
    pub fn message_timeout(mut self, timeout: Duration) -> Result<Self, ClusterError> {
        self.message_timeout_ms = checked_timeout_ms(timeout, "message_timeout")?;
        Ok(self)
    }

    /// Sets how long to wait for a `NewLeaderEvent` after the current leader
    /// is lost (Java `Context.newLeaderTimeoutNs`). Unset, connect uses twice
    /// the leader heartbeat from the OK `SessionEvent`, or 20 seconds when
    /// that field is absent. A value set here wins. When the wait runs out,
    /// [`crate::AeronCluster::poll`] moves the session to
    /// [`crate::ClusterError::Disconnected`]. Once the event has arrived, the
    /// message timeout bounds the wait instead.
    ///
    /// # Errors
    ///
    /// A zero, sub-millisecond or overflowing duration fails immediately.
    pub fn new_leader_timeout(mut self, timeout: Duration) -> Result<Self, ClusterError> {
        self.new_leader_timeout_ms = Some(checked_timeout_ms(timeout, "new_leader_timeout")?);
        Ok(self)
    }

    /// Uses an existing Aeron client instead of creating one.
    ///
    /// Connect clones `aeron` and does not call `start`, `set_client_name`
    /// or `set_error_handler`. The `aeron_dir` passed to connect is ignored.
    /// The caller's clone keeps the client open after this session drops.
    pub fn aeron(mut self, aeron: Aeron) -> Self {
        self.shared_aeron = Some(aeron);
        self
    }

    /// Sets the Aeron client name sent in the connect request's `clientInfo`.
    ///
    /// The default is `cluster-client`. The builder rejects a name longer than
    /// 100 bytes or with an interior NUL, and applies it only when it creates
    /// the Aeron client.
    ///
    /// # Errors
    ///
    /// Returns [`ClusterError::ConnectFailed`] when `name` is longer than
    /// 100 bytes or contains a NUL.
    pub fn client_name(mut self, name: impl AsRef<str>) -> Result<Self, ClusterError> {
        let name = name.as_ref();
        if name.len() > 100 {
            return Err(ClusterError::connect("client name length must be <= 100"));
        }
        self.client_name =
            Some(CString::new(name).map_err(|_| ClusterError::connect("client name must not contain NUL"))?);
        Ok(self)
    }

    /// Sets the error handler for the Aeron client this builder creates.
    ///
    /// The handler runs on Aeron's conductor and must not panic; a panic is
    /// caught and written to stderr. It is ignored when [`Self::aeron`]
    /// supplied the client, which has its own handler. The default writes
    /// `ERROR: ({code}): {message}` to stderr.
    pub fn error_handler(mut self, handler: impl Fn(i32, &str) + Send + Sync + 'static) -> Self {
        self.error_handler = Some(Arc::new(handler));
        self
    }

    /// Sets the idle strategy for the connect retry loop (Java
    /// `Context.idleStrategy`), replacing the default spin-then-1µs backoff in
    /// the synchronous handshake.
    pub fn idle_strategy(mut self, strategy: impl IdleStrategy + Send + Sync + 'static) -> Self {
        self.idle = Some(Arc::new(Mutex::new(strategy)));
        self
    }

    /// Sets the credentials supplier for challenge-response authentication.
    pub fn credentials(mut self, supplier: Arc<dyn CredentialsSupplier>) -> Self {
        self.credentials = Some(supplier);
        self
    }

    /// Sets the ingress endpoints of a multi-member cluster: comma-separated
    /// `member_id=host:port` pairs (`"0=host:9002,1=host:9003"`) with no
    /// `aeron:` prefix. Member ids must be unique and every endpoint non-empty.
    ///
    /// # Errors
    ///
    /// A missing `=`, a non-numeric member id, an empty endpoint, an empty map,
    /// an empty entry (leading, trailing or repeated comma) or a duplicate
    /// member id fails here, not at `validate()` or the first `poll()`.
    pub fn ingress_endpoints(mut self, endpoints: impl Into<String>) -> Result<Self, ClusterError> {
        let endpoints = endpoints.into();
        crate::endpoints::parse_ingress_endpoints(&endpoints)?;
        self.ingress_endpoints = Some(endpoints);
        Ok(self)
    }

    /// The ingress channel as a `CStr`, once one has been set.
    #[inline]
    pub fn ingress_channel_c_str(&self) -> Option<&CStr> {
        self.ingress_c.as_deref()
    }

    /// The egress channel as a `CStr`, once one has been set.
    #[inline]
    pub fn egress_channel_c_str(&self) -> Option<&CStr> {
        self.egress_c.as_deref()
    }

    /// Connects synchronously, like [`crate::AeronCluster::connect`].
    ///
    /// # Errors
    ///
    /// As [`crate::AeronCluster::connect`].
    pub fn connect(self, aeron_dir: &str) -> Result<crate::AeronCluster, ClusterError> {
        crate::AeronCluster::connect(&self, aeron_dir)
    }

    /// Starts a poll-driven connect. It is an Aeron state machine, not a Tokio future.
    pub fn connect_async(self, aeron_dir: impl Into<String>) -> crate::AsyncClusterConnect {
        crate::AeronCluster::connect_async(self, aeron_dir)
    }

    /// Checks that the required fields are set and the channel URIs are valid.
    ///
    /// # Errors
    ///
    /// [`ClusterError::ConnectFailed`] when neither an ingress channel nor ingress
    /// endpoints are set, the egress channel is not set, the ingress endpoints do
    /// not parse, or an IPC ingress channel comes with ingress endpoints.
    pub fn validate(&self) -> Result<(), ClusterError> {
        let has_ingress = self.ingress_c.is_some();
        let has_endpoints = self.ingress_endpoints.as_ref().is_some_and(|s| !s.is_empty());
        if !has_ingress && !has_endpoints {
            return Err(ClusterError::connect(
                "ingress_channel or ingress_endpoints is required",
            ));
        }
        if self.egress_c.is_none() {
            return Err(ClusterError::connect("egress_channel is required"));
        }
        if has_endpoints {
            let _ = crate::endpoints::parse_ingress_endpoints(self.ingress_endpoints.as_deref().unwrap_or(""))?;
            if self
                .ingress_c
                .as_ref()
                .is_some_and(|channel| channel.as_bytes().starts_with(b"aeron:ipc"))
            {
                return Err(ClusterError::connect(
                    "IPC plus ingress endpoints is a configuration error",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn egress_for_aeron(&self) -> Result<&CString, ClusterError> {
        self.egress_c
            .as_ref()
            .ok_or_else(|| ClusterError::connect("egress channel missing (call validate first)"))
    }

    /// The ingress channel connect starts on: the configured channel, else the first endpoint.
    pub(crate) fn resolve_initial_ingress_for_aeron(&self) -> Result<CString, ClusterError> {
        if let Some(c) = self.ingress_c.as_ref() {
            return Ok(c.clone());
        }
        if let Some(ref map) = self.ingress_endpoints {
            let eps = crate::endpoints::parse_ingress_endpoints(map)?;
            return uri::udp_endpoint_cstr(&eps[0].endpoint);
        }
        Err(ClusterError::connect(
            "no ingress_channel or ingress_endpoints to resolve",
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_match_java() {
        let b = SessionBuilder::default();
        assert_eq!(b.ingress_stream_id, 101);
        assert_eq!(b.egress_stream_id, 102);
        assert_eq!(b.message_timeout_ms, 5_000);
        assert!(b.new_leader_timeout_ms.is_none());
    }

    #[test]
    fn test_validate_rejects_empty_channels() {
        let b = SessionBuilder::default();
        assert!(b.validate().is_err());
    }

    #[test]
    fn test_validate_accepts_configured_channels() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:9020")?;
        b.validate()?;
        assert!(b.ingress_channel_c_str().is_some());
        assert!(b.egress_channel_c_str().is_some());
        Ok(())
    }

    #[test]
    fn ingress_endpoints_rejects_malformed_maps_at_the_setter() {
        for bad in [
            "",
            "0localhost:9012",                    // missing '='
            "x=localhost:9012",                   // non-numeric member id
            "0=",                                 // empty endpoint
            "0=localhost:9012,,1=localhost:9112", // empty comma-separated entry
            ",0=localhost:9012",                  // leading comma
            "0=localhost:9012,",                  // trailing comma
            "0=localhost:9012,0=localhost:9112",  // duplicate member id
        ] {
            assert!(
                SessionBuilder::default().ingress_endpoints(bad).is_err(),
                "expected {bad:?} to be rejected at the setter"
            );
        }
    }

    #[test]
    fn ingress_endpoints_accepts_valid_unsorted_map() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default().ingress_endpoints("1=host-b:9013,0=host-a:9012")?;
        assert_eq!(
            b.ingress_endpoints.as_deref(),
            Some("1=host-b:9013,0=host-a:9012"),
            "the setter stores the map as given; member-ID resolution sorts on use"
        );
        Ok(())
    }

    #[test]
    fn cstr_accessors_borrow_cached_storage() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel(uri::AERON_IPC_STREAM.to_str()?)?
            .egress_channel(uri::AERON_IPC_STREAM.to_str()?)?;
        b.validate()?;
        let a = b.ingress_channel_c_str().ok_or("missing")?.as_ptr();
        let a2 = b.ingress_channel_c_str().ok_or("missing")?.as_ptr();
        assert_eq!(a, a2, "cached CString must be stable across calls");
        Ok(())
    }

    #[test]
    fn test_validate_endpoints_without_ingress_channel() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_endpoints("0=localhost:9002,1=localhost:9102")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?;
        b.validate()?;
        let c = b.resolve_initial_ingress_for_aeron()?;
        let s = c.to_str()?;
        assert!(s.contains("localhost:9002"), "{s}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_missing_ingress() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default().egress_channel("aeron:udp?endpoint=localhost:19002")?;
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("ingress"), "{err}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_missing_egress() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default().ingress_channel("aeron:udp?endpoint=localhost:9010")?;
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("egress"), "{err}");
        Ok(())
    }

    #[test]
    fn timeout_rejects_zero_and_sub_millisecond() -> Result<(), Box<dyn std::error::Error>> {
        for d in [Duration::ZERO, Duration::from_nanos(1), Duration::from_nanos(999_999)] {
            let err = SessionBuilder::default()
                .ingress_channel("aeron:udp?endpoint=localhost:9010")?
                .egress_channel("aeron:udp?endpoint=localhost:19002")?
                .message_timeout(d)
                .expect_err("zero/sub-ms must fail at the setter");
            assert!(
                matches!(
                    err,
                    ClusterError::InvalidTimeout {
                        phase: "message_timeout",
                        ..
                    }
                ),
                "got {err:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn timeout_accepts_one_millisecond() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::from_millis(1))?
            .new_leader_timeout(Duration::from_millis(1))?;
        b.validate()?;
        assert_eq!(b.message_timeout_ms, 1);
        assert_eq!(b.new_leader_timeout_ms, Some(1));
        Ok(())
    }

    #[test]
    fn timeout_rejects_duration_max() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::MAX)
            .expect_err("Duration::MAX must fail at the setter");
        assert!(
            matches!(
                err,
                ClusterError::InvalidTimeout {
                    phase: "message_timeout",
                    ..
                }
            ),
            "got {err:?}"
        );
        Ok(())
    }

    #[test]
    fn replacing_a_timeout_cannot_retain_a_stale_error() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::ZERO)
            .expect_err("zero must fail immediately");
        assert!(matches!(
            err,
            ClusterError::InvalidTimeout {
                phase: "message_timeout",
                ..
            }
        ));
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::from_secs(5))?;
        b.validate()?;
        Ok(())
    }

    #[test]
    fn new_leader_timeout_same_validation() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .new_leader_timeout(Duration::from_nanos(1))
            .expect_err("sub-ms new_leader must fail at the setter");
        assert!(
            matches!(
                err,
                ClusterError::InvalidTimeout {
                    phase: "new_leader_timeout",
                    ..
                }
            ),
            "got {err:?}"
        );
        Ok(())
    }

    #[test]
    fn ipc_plus_ingress_endpoints_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel(uri::AERON_IPC_STREAM.to_str()?)?
            .ingress_endpoints("0=localhost:9002")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .validate()
            .expect_err("IPC plus endpoints");
        assert!(err.to_string().contains("IPC"), "{err}");
        Ok(())
    }

    #[test]
    fn client_name_rejects_a_long_name_and_a_nul() -> Result<(), Box<dyn std::error::Error>> {
        let long = "n".repeat(101);
        assert!(SessionBuilder::default().client_name(long).is_err());
        assert!(SessionBuilder::default().client_name("bad\0name").is_err());
        let b = SessionBuilder::default().client_name("desk")?;
        assert_eq!(b.client_name.as_deref().and_then(|n| n.to_str().ok()), Some("desk"));
        Ok(())
    }

    #[test]
    fn malformed_channel_fails_at_the_setter() {
        let err = SessionBuilder::default()
            .ingress_channel("not a uri")
            .expect_err("malformed URI must fail at the setter");
        assert_ne!(err.to_string(), "");
    }

    #[test]
    fn debug_redacts_credentials_and_idle() -> Result<(), Box<dyn std::error::Error>> {
        let secret = b"super-secret-password-xyz";
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .credentials(std::sync::Arc::new(crate::StaticCredentials::new(secret.to_vec())));
        let dbg = format!("{b:?}");
        assert!(
            !dbg.contains("super-secret"),
            "Debug must not leak credential text: {dbg}"
        );
        assert!(
            !dbg.as_bytes().windows(secret.len()).any(|w| w == secret),
            "Debug must not leak credential bytes"
        );
        assert!(
            dbg.contains("<configured>"),
            "credentials should show as configured: {dbg}"
        );
        assert!(dbg.contains("ingress_channel"), "safe fields remain: {dbg}");
        assert!(
            dbg.contains("localhost:9010"),
            "channel URI should remain visible: {dbg}"
        );
        Ok(())
    }
}
