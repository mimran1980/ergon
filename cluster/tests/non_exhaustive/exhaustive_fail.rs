//! Exhaustive match without `_` must fail — ClusterError is non-exhaustive.
use ergo_aeron_cluster::ClusterError;

fn classify(err: ClusterError) -> &'static str {
    match err {
        ClusterError::ConnectFailed { .. } => "connect",
        ClusterError::AuthRejected => "auth",
        ClusterError::Timeout { .. } => "timeout",
        ClusterError::NotConnected => "not-connected",
        ClusterError::SessionClosed => "closed",
        ClusterError::Disconnected { .. } => "disconnected",
        ClusterError::ProtocolError { .. } => "protocol",
        ClusterError::Redirect { .. } => "redirect",
        ClusterError::BufferTooSmall { .. } => "buffer",
        ClusterError::Publication { .. } => "pub",
        ClusterError::ReconnectFailed { .. } => "reconnect",
        ClusterError::ChannelUri { .. } => "uri",
        ClusterError::Aeron { .. } => "aeron",
        ClusterError::ListenerPanicked { .. } => "panic",
        ClusterError::InvalidUtf8 { .. } => "utf8",
        ClusterError::InvalidTimeout { .. } => "invalid-timeout",
        ClusterError::PayloadTooLarge { .. } => "payload",
        // Intentionally no `_` arm — must not compile.
    }
}

fn main() {
    let _ = classify(ClusterError::AuthRejected);
}
