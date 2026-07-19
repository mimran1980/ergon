//! [`SessionBuilder`] — channel/stream/timeout configuration for connect.
//!
//! Mirrors Java `AeronCluster.Context`. Defaults: ingress stream 101, egress
//! stream 102, 10s message timeout.

use std::sync::Arc;
use std::time::Duration;

use crate::{ClusterError, CredentialsSupplier};

/// Builds and connects an [`crate::AeronCluster`].
///
/// Mirrors `AeronCluster.Context` in the Java client. All channel and
/// stream-ID defaults match the upstream Java defaults.
///
/// # Example
///
/// ```rust,ignore
/// use ergo_aeron_cluster::SessionBuilder;
/// let builder = SessionBuilder::builder()
///     .ingress_channel("aeron:udp?endpoint=localhost:9002".into())
///     .egress_channel("aeron:udp?endpoint=localhost:19002".into())
///     .ingress_stream_id(101)
///     .egress_stream_id(102);
/// ```
#[derive(Clone)]
pub struct SessionBuilder {
    pub(crate) ingress_channel: String,
    pub(crate) egress_channel: String,
    pub(crate) ingress_stream_id: i32,
    pub(crate) egress_stream_id: i32,
    pub(crate) message_timeout_ms: u64,
    pub(crate) credentials: Option<Arc<dyn CredentialsSupplier>>,
    /// Multi-member ingress endpoints: `"0=host:port,1=host:port,..."`.
    /// When set, the client connects to the leader member's endpoint.
    pub(crate) ingress_endpoints: Option<String>,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            ingress_channel: String::new(),
            egress_channel: String::new(),
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

    pub fn ingress_channel(mut self, channel: impl Into<String>) -> Self {
        self.ingress_channel = channel.into();
        self
    }

    pub fn egress_channel(mut self, channel: impl Into<String>) -> Self {
        self.egress_channel = channel.into();
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
    pub fn ingress_endpoints(mut self, endpoints: impl Into<String>) -> Self {
        self.ingress_endpoints = Some(endpoints.into());
        self
    }

    /// Validate required fields are set.
    pub fn validate(&self) -> Result<(), ClusterError> {
        if self.ingress_channel.is_empty() {
            return Err(ClusterError::ConnectFailed {
                reason: "ingress_channel is required".into(),
            });
        }
        if self.egress_channel.is_empty() {
            return Err(ClusterError::ConnectFailed {
                reason: "egress_channel is required".into(),
            });
        }
        Ok(())
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
        assert!(b.validate().is_ok());
    
        Ok(())
    }
}
