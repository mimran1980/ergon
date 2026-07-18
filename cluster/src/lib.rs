//! # Rusteron Cluster Client
//!
//! ⚠️ **TEMPORARY PROTOTYPE.** This is a handwritten Rust reimplementation
//! of the [Aeron Cluster](https://github.com/real-logic/aeron) *client*
//! (no C bindings). It is heavily LLM-assisted, lightly human-reviewed,
//! and less tested than the Java reference.
//!
//! **Delete this crate when official Aeron Cluster C bindings become
//! available.** Bugs in Rusteron's pub/sub layer OR in this
//! reimplementation may cause undefined behaviour, segfaults, or data
//! loss.

// Verify rusteron-client types are accessible across the crate boundary
#[doc(hidden)]
pub mod transport {
    pub use rusteron_client::Aeron;
    pub use rusteron_client::AeronContext;
    pub use rusteron_client::AeronExclusivePublication;
    pub use rusteron_client::AeronPublication;
    pub use rusteron_client::AeronSubscription;
}

pub mod client;
pub mod codecs;
pub mod config;
pub mod connect;
pub mod controlled;
pub mod credentials;
pub mod egress;
pub mod error;
pub mod poller;
pub mod protocol;
pub mod session;
pub mod state;

pub use client::{AeronCluster, AsyncClusterConnect, ClusterClaim};
pub use config::SessionBuilder;
pub use connect::AsyncConnect;
pub use controlled::{ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction};
pub use credentials::{CredentialsSupplier, NullCredentialsSupplier};
pub use error::ClusterError;
pub use poller::{EgressEvent, parse_event, parse_redirect_leader};
pub use state::SessionState;

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_compiles() {
        // Smoke-check the generated codecs are wired into the lib.
        assert_eq!(crate::codecs::cluster_codecs::SBE_SCHEMA_ID, 111);
    }
}
