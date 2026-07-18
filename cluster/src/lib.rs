//! # ergo-aeron-cluster
//!
//! Experimental pure-Rust [Aeron Cluster](https://github.com/real-logic/aeron)
//! *client* on [`rusteron_client`], with **ErgoSBE-generated** session (schema
//! 111) and RFQ (schema 101) codecs.
//!
//! ⚠️ **Prototype.** LLM-assisted and less tested than the Java reference.
//! Bugs in Rusteron pub/sub **or** this reimplementation may cause undefined
//! behaviour, segfaults, or data loss. Replace when official Cluster C client
//! bindings are suitable for your deployment.
//!
//! # Hot path
//!
//! 1. [`AeronCluster::try_claim`] — SessionMessageHeader into the claim via ErgoSBE
//! 2. Egress decode (`egress` / `poller` / `controlled`) — SessionEvent, NewLeader, app
//! 3. Keep-alive encode — periodic
//! 4. Connect / auth / failover — cold path (correctness over nanoseconds)
//!
//! # Codecs
//!
//! Production modules: [`codecs::ergo_codecs`], [`codecs::ergo_codecs_mark`],
//! [`codecs::ergo_rfq_codecs`] (generated in `build.rs` from the Aeron submodule
//! and vendored RFQ XML). Residual sbe-tool trees under [`codecs`] remain for
//! head-to-head benches only.
//!
//! # Quick connect
//!
//! ```rust,ignore
//! use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
//! // SessionBuilder::builder().ingress_channel(...).egress_channel(...)
//! // AeronCluster::connect(&builder, aeron_dir)
//! // client.try_claim(payload_len)?; fill payload; commit
//! // client.poll_egress(&mut adapter, limit)?;
//! ```
//!
//! See the crate [README](https://github.com/mimran1980/ErgoSBE/blob/first_cut/cluster/README.md)
//! for recipes, maintained benches, and the HA sample.

// Verify rusteron-client types are accessible across the crate boundary
#[doc(hidden)]
pub mod transport {
    pub use rusteron_client::Aeron;
    pub use rusteron_client::AeronContext;
    pub use rusteron_client::AeronExclusivePublication;
    pub use rusteron_client::AeronPublication;
    pub use rusteron_client::AeronSubscription;
}

/// High-level cluster client: connect, try_claim, offer, keep-alive, close.
pub mod client;
/// SBE codecs: ErgoSBE production modules + residual sbe-tool trees for benches.
pub mod codecs;
/// [`SessionBuilder`] configuration for connect.
pub mod config;
/// Async connect state machine and connect re-offer cadence helpers.
pub mod connect;
/// Controlled egress poll (Java `ControlledEgressAdapter` analogue).
pub mod controlled;
/// Credential supplier traits for challenge-response auth.
pub mod credentials;
/// Egress adapter + listener dispatch for session and app messages.
pub mod egress;
/// Cluster client error type.
pub mod error;
/// Low-level egress event parse helpers (SessionEvent, NewLeader, redirects).
pub mod poller;
/// Session protocol constants derived from ErgoSBE encoder metadata.
pub mod protocol;
/// Session object wrappers used during connect/lifecycle.
pub mod session;
/// [`SessionState`] machine for connected / new-leader / closed.
pub mod state;

pub use client::{AeronCluster, AsyncClusterConnect, ClusterClaim};
pub use config::SessionBuilder;
pub use connect::{AsyncConnect, connect_reoffer_interval_ms, should_reoffer_connect};
pub use controlled::{ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction};
pub use credentials::{CredentialsSupplier, NullCredentialsSupplier};
pub use error::ClusterError;
pub use poller::{EgressEvent, parse_event, parse_redirect_leader};
pub use state::SessionState;

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_compiles() {
        // Smoke-check ErgoSBE production codecs are wired into the lib.
        assert_eq!(crate::codecs::ergo_codecs::SessionConnectRequestEncoder::SCHEMA_ID, 111);
    }
}
