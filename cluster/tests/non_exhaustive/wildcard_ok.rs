//! External consumer: wildcard arm documents the non-exhaustive contract.
use ergo_aeron_cluster::ClusterError;

fn classify(err: ClusterError) -> &'static str {
    match err {
        ClusterError::AuthRejected => "auth",
        ClusterError::NotConnected => "not-connected",
        // Required: ClusterError is #[non_exhaustive] and may grow variants.
        _ => "other",
    }
}

fn main() {
    let _ = classify(ClusterError::AuthRejected);
}
