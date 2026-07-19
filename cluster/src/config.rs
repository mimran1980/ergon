//! [`SessionBuilder`] — channel/stream/timeout configuration for connect.
//!
//! Mirrors Java `AeronCluster.Context`. Defaults: ingress stream 101, egress
//! stream 102, 10s message timeout.
//!
//! Public channel accessors return **`&str`** (zero-cost borrows of stored
//! UTF-8). [`CString`] is kept only privately for rusteron FFI reuse on connect.

use std::borrow::Cow;
use std::ffi::CString;
use std::sync::Arc;
use std::time::Duration;

use crate::uri;
use crate::{ClusterError, CredentialsSupplier};

/// Builds and connects an [`crate::AeronCluster`].
///
/// Channel setters normalize URIs via
/// [`AeronUriStringBuilder`](rusteron_client::AeronUriStringBuilder) into
/// UTF-8 strings. FFI `CString`s are cached privately for connect.
///
/// # Example
///
/// ```rust,ignore
/// use ergo_aeron_cluster::SessionBuilder;
/// let client = SessionBuilder::builder()
///     .ingress_channel("aeron:udp?endpoint=localhost:9002")
///     .egress_channel("aeron:udp?endpoint=localhost:19002")
///     .connect(aeron_dir)?;
/// ```
#[derive(Clone)]
pub struct SessionBuilder {
    /// Normalized ingress channel URI (UTF-8).
    pub(crate) ingress_channel: String,
    /// Normalized egress channel URI (UTF-8).
    pub(crate) egress_channel: String,
    /// Private FFI cache of ingress (rusteron only).
    ingress_c: Option<CString>,
    /// Private FFI cache of egress (rusteron only).
    egress_c: Option<CString>,
    pub(crate) ingress_stream_id: i32,
    pub(crate) egress_stream_id: i32,
    pub(crate) message_timeout_ms: u64,
    pub(crate) credentials: Option<Arc<dyn CredentialsSupplier>>,
    /// Multi-member ingress endpoints: `"0=host:port,1=host:port,..."`.
    pub(crate) ingress_endpoints: Option<String>,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            ingress_channel: String::new(),
            egress_channel: String::new(),
            ingress_c: None,
            egress_c: None,
            ingress_stream_id: 101,
            egress_stream_id: 102,
            message_timeout_ms: 10_000,
            credentials: None,
            ingress_endpoints: None,
        }
    }
}

impl SessionBuilder {
    pub fn builder() -> Self {
        Self::default()
    }

    /// Set the ingress channel URI (normalized to UTF-8; FFI cache filled).
    pub fn ingress_channel(mut self, channel: impl Into<String>) -> Self {
        let raw = channel.into();
        match uri::channel_uri(&raw) {
            Ok(normalized) => {
                self.ingress_c = Some(uri::to_c_string(&normalized));
                self.ingress_channel = normalized;
            }
            Err(_) => {
                self.ingress_channel = raw;
                self.ingress_c = None;
            }
        }
        self
    }

    /// Set the egress channel URI (normalized to UTF-8; FFI cache filled).
    pub fn egress_channel(mut self, channel: impl Into<String>) -> Self {
        let raw = channel.into();
        match uri::channel_uri(&raw) {
            Ok(normalized) => {
                self.egress_c = Some(uri::to_c_string(&normalized));
                self.egress_channel = normalized;
            }
            Err(_) => {
                self.egress_channel = raw;
                self.egress_c = None;
            }
        }
        self
    }

    pub fn ingress_stream_id(mut self, stream_id: i32) -> Self {
        self.ingress_stream_id = stream_id;
        self
    }

    pub fn egress_stream_id(mut self, stream_id: i32) -> Self {
        self.egress_stream_id = stream_id;
        self
    }

    pub fn message_timeout(mut self, timeout: Duration) -> Self {
        self.message_timeout_ms = timeout.as_millis() as u64;
        self
    }

    /// Set the credentials supplier for challenge/response auth.
    pub fn credentials(mut self, supplier: Arc<dyn CredentialsSupplier>) -> Self {
        self.credentials = Some(supplier);
        self
    }

    /// Set multi-member ingress endpoints (`"0=host:port,1=host:port"`).
    ///
    /// First-connect opens against the lowest member id, then follows REDIRECT
    /// / NewLeader. An explicit [`Self::ingress_channel`] overrides the initial
    /// publication URI.
    pub fn ingress_endpoints(mut self, endpoints: impl Into<String>) -> Self {
        self.ingress_endpoints = Some(endpoints.into());
        self
    }

    /// Ingress channel as UTF-8 (empty if only `ingress_endpoints` was set).
    /// Zero-cost borrow of the stored string.
    #[inline]
    pub fn ingress_channel_str(&self) -> &str {
        &self.ingress_channel
    }

    /// Egress channel as UTF-8. Zero-cost borrow.
    #[inline]
    pub fn egress_channel_str(&self) -> &str {
        &self.egress_channel
    }

    /// Multi-member endpoints map as UTF-8, if set.
    #[inline]
    pub fn ingress_endpoints_str(&self) -> Option<&str> {
        self.ingress_endpoints.as_deref()
    }

    /// Synchronous connect — equivalent to [`crate::AeronCluster::connect`].
    pub fn connect(self, aeron_dir: &str) -> Result<crate::AeronCluster, ClusterError> {
        crate::AeronCluster::connect(&self, aeron_dir)
    }

    /// Poll-driven Aeron async connect (not Tokio).
    pub fn connect_async(self, aeron_dir: impl Into<String>) -> crate::AsyncClusterConnect {
        crate::AeronCluster::connect_async(self, aeron_dir)
    }

    /// Validate required fields and that channel URIs are valid.
    pub fn validate(&self) -> Result<(), ClusterError> {
        let has_ingress = !self.ingress_channel.is_empty();
        let has_endpoints = self.ingress_endpoints.as_ref().is_some_and(|s| !s.is_empty());
        if !has_ingress && !has_endpoints {
            return Err(ClusterError::connect(
                "ingress_channel or ingress_endpoints is required",
            ));
        }
        if self.egress_channel.is_empty() {
            return Err(ClusterError::connect("egress_channel is required"));
        }
        if has_ingress && self.ingress_c.is_none() {
            // Re-run for a precise URI error.
            let _ = uri::channel_uri(&self.ingress_channel)?;
            return Err(ClusterError::connect(format!(
                "invalid ingress_channel URI: {}",
                self.ingress_channel
            )));
        }
        if has_endpoints {
            let _ = crate::endpoints::parse_ingress_endpoints(self.ingress_endpoints.as_deref().unwrap_or(""))?;
        }
        if self.egress_c.is_none() {
            let _ = uri::channel_uri(&self.egress_channel)?;
            return Err(ClusterError::connect(format!(
                "invalid egress_channel URI: {}",
                self.egress_channel
            )));
        }
        Ok(())
    }

    /// Initial ingress channel as UTF-8 (borrow when already stored).
    pub fn resolve_initial_ingress_uri(&self) -> Result<Cow<'_, str>, ClusterError> {
        if !self.ingress_channel.is_empty() {
            return Ok(Cow::Borrowed(&self.ingress_channel));
        }
        if let Some(ref map) = self.ingress_endpoints {
            let eps = crate::endpoints::parse_ingress_endpoints(map)?;
            let owned = uri::udp_endpoint_uri(&eps[0].endpoint)?;
            return Ok(Cow::Owned(owned));
        }
        Err(ClusterError::connect(
            "no ingress_channel or ingress_endpoints to resolve",
        ))
    }

    /// FFI form of egress channel (private cache; clones the `CString` handle).
    pub(crate) fn egress_for_aeron(&self) -> Result<&CString, ClusterError> {
        self.egress_c
            .as_ref()
            .ok_or_else(|| ClusterError::connect("egress channel missing (call validate first)"))
    }

    /// FFI form of the initial ingress channel for connect.
    pub(crate) fn resolve_initial_ingress_for_aeron(&self) -> Result<CString, ClusterError> {
        if let Some(c) = self.ingress_c.as_ref() {
            return Ok(c.clone());
        }
        let uri = self.resolve_initial_ingress_uri()?;
        Ok(uri::to_c_string(uri.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_match_java() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default();
        assert_eq!(b.ingress_stream_id, 101);
        assert_eq!(b.egress_stream_id, 102);
        assert_eq!(b.message_timeout_ms, 10_000);
        Ok(())
    }

    #[test]
    fn test_validate_rejects_empty_channels() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default();
        assert!(b.validate().is_err());
        Ok(())
    }

    #[test]
    fn test_validate_accepts_configured_channels() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")
            .egress_channel("aeron:udp?endpoint=localhost:9020");
        b.validate()?;
        assert!(!b.ingress_channel_str().is_empty());
        assert!(!b.egress_channel_str().is_empty());
        Ok(())
    }

    #[test]
    fn str_accessors_are_zero_cost_borrows() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::builder()
            .ingress_channel(uri::IPC)
            .egress_channel(uri::IPC);
        b.validate()?;
        // Same pointer as internal storage — no copy.
        assert_eq!(b.ingress_channel_str(), "aeron:ipc");
        assert_eq!(b.egress_channel_str(), "aeron:ipc");
        assert!(std::ptr::eq(
            b.ingress_channel_str().as_ptr(),
            b.ingress_channel.as_ptr()
        ));
        Ok(())
    }

    #[test]
    fn test_validate_endpoints_without_ingress_channel() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::builder()
            .ingress_endpoints("0=localhost:9002,1=localhost:9102")
            .egress_channel("aeron:udp?endpoint=localhost:19002");
        b.validate()?;
        let uri = b.resolve_initial_ingress_uri()?;
        assert!(uri.contains("localhost:9002"), "{uri}");
        Ok(())
    }
}
