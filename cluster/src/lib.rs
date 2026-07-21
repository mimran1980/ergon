//! # ergo-aeron-cluster
//!
//! Experimental pure-Rust [Aeron Cluster](https://github.com/real-logic/aeron)
//! *client* on [`rusteron_client`] **0.2** (latest 0.2.x), with
//! **ErgoSBE-generated** session (schema 111) and RFQ (schema 101) codecs.
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
//! Production modules: [`codecs::session`], [`codecs::mark`],
//! [`codecs::rfq`] (generated in `build.rs` from the Aeron submodule
//! and vendored RFQ XML). sbe-tool reference runtime lives at
//! `cluster/benches/reference_sbe/` (Criterion-private — never imported
//! from library, test, or example code).
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
/// Generated Aeron protocol codecs — hidden from docs, not part of the public API.
#[doc(hidden)]
pub mod codecs;
/// [`SessionBuilder`] configuration for connect.
pub mod config;
/// Async connect state machine and connect re-offer cadence helpers.
///
/// **Aeron poll-driven async only** — not Tokio/`async`/`await`. Drive
/// [`AsyncClusterConnect::poll`] from your event loop (same model as Java
/// `AeronCluster.AsyncConnect`).
pub mod connect;
/// Controlled egress poll (Java `ControlledEgressAdapter` analogue).
pub mod controlled;
/// Credential supplier traits for challenge-response auth.
pub mod credentials;
/// Equal-work ErgoSBE decode helpers for session / new-leader frames.
pub mod decode;
/// Egress adapter + listener dispatch for session and app messages.
pub mod egress;
/// Shared fragment decode — canonical AnyMessage dispatch used by egress,
/// controlled, and poller paths. Not public API.
pub(crate) mod fragment;
/// Multi-member ingress endpoint maps (`0=host:port,…`).
pub mod endpoints;
/// Cluster client error type.
pub mod error;
/// Poll-loop idle helpers ([`rusteron_client::IdleStrategy`]).
pub mod idle;
/// Low-level egress event parse helpers (SessionEvent, NewLeader, redirects).
pub mod poller;
/// Session object wrappers used during connect/lifecycle.
pub mod session;
/// [`SessionState`] machine for connected / new-leader / closed.
pub mod state;
/// Aeron channel URI helpers ([`AeronUriStringBuilder`](rusteron_client::AeronUriStringBuilder)).
pub mod uri;

pub use client::{AeronCluster, AsyncClusterConnect, ClusterClaim};
pub use config::SessionBuilder;
pub use connect::{connect_reoffer_interval_ms, should_reoffer_connect};
pub use controlled::{ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction};
pub use credentials::{CredentialsSupplier, EchoChallengeCredentials, NullCredentialsSupplier, StaticCredentials};
pub use decode::{
    NewLeaderEventView, SessionEventView, SessionMessageHeaderView, decode_new_leader_event, decode_session_event,
    decode_session_message_header,
};
pub use egress::{EgressAdapter, EgressListener, NullListener};
pub use endpoints::{IngressEndpoint, endpoint_for_member, next_endpoint_index, parse_ingress_endpoints};
pub use error::{ClusterError, ClusterResult, PublicationFailure};
pub use idle::{
    BusySpinIdleStrategy, ClusterBackoffIdle, ClusterIdleStrategy, NoOpIdleStrategy, SleepingIdleStrategy,
    YieldingIdleStrategy, default_idle, poll_connect_once, poll_connect_until_done, poll_egress_idle,
};
pub use poller::{EgressEvent, parse_event, parse_redirect_leader};
pub use state::SessionState;
pub use uri::AERON_IPC_STREAM;

/// Java Aeron Cluster spawn harness (integration tests / examples only).
///
/// Enable with `--features test-harness` (requires Java 17+ and
/// `just build-aeron-jars`). Not for production and not published as a separate crate.
#[cfg(feature = "test-harness")]
pub mod test_support;

#[cfg(feature = "test-harness")]
pub use test_support::{EmbeddedArchiveDriver, TestCluster};

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_compiles() -> Result<(), Box<dyn std::error::Error>> {
        // Smoke-check ErgoSBE production codecs are wired into the lib.
        assert_eq!(crate::codecs::session::SessionConnectRequestEncoder::SCHEMA_ID, 111);

        Ok(())
    }
}
