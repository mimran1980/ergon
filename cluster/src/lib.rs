//! # ergo-aeron-cluster
//!
//! Experimental Rust **client** for [Aeron Cluster](https://github.com/real-logic/aeron)
//! on [`rusteron_client`] **0.2** (latest 0.2.x), using **ergo-sbe-generated**
//! session codecs (schema 111).
//!
//! ⚠️ **Prototype.** LLM-assisted and less tested than the Java reference.
//! Bugs in Rusteron pub/sub **or** this reimplementation may cause undefined
//! behaviour, segfaults, or data loss. Replace when official Cluster C client
//! bindings are suitable for your deployment.
//!
//! ## Client-only — the Java process *is* the cluster
//!
//! This crate implements the **client** side of the Aeron Cluster protocol
//! only — parity with Java
//! [`io.aeron.cluster.client`](https://github.com/real-logic/aeron/tree/master/aeron-cluster/src/main/java/io/aeron/cluster/client)
//! (connect, offer/`try_claim`, poll egress, leader failover, keep-alive,
//! admin snapshot, challenge-response auth). It does **not** implement — and
//! never will — the cluster **server**: no consensus module (Raft), no
//! clustered-service container, no leader election, no snapshots/recovery, no
//! archive, no backup node, no `ClusterTool` CLI. You run all of that as the
//! **Java Aeron process**; this client connects to it over the standard Aeron
//! wire protocol.
//!
//! # Hot path
//!
//! 1. [`AeronCluster::try_claim`] — SessionMessageHeader into the claim via ergo-sbe
//! 2. Egress decode (`egress` / `poller` / `controlled`) — SessionEvent, NewLeader, app
//! 3. Keep-alive encode — periodic
//! 4. Connect / auth / failover — cold path (correctness over nanoseconds)
//!
//! # Codecs
//!
//! Production modules: `codecs::session` (schema 111) and `codecs::mark`,
//! generated in `build.rs` from the vendored Aeron schemas. The sbe-tool
//! reference runtime lives at `cluster/benches/reference_sbe/`
//! (Criterion-private — never imported from library, test, or example code).
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
//! See the crate [README](https://github.com/mimran1980/ergon/blob/first_cut/cluster/README.md)
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
/// SBE codecs: ergo-sbe production modules + residual sbe-tool trees for benches.
/// Generated Aeron protocol codecs — hidden from docs, not part of the public API.
#[doc(hidden)]
pub mod codecs;
/// [`SessionBuilder`] configuration for connect.
pub mod config;
/// Controlled egress poll (Java `ControlledEgressAdapter` analogue).
pub mod controlled;
/// Credential supplier traits for challenge-response auth.
pub mod credentials;
/// Egress adapter + listener dispatch for session and app messages.
pub mod egress;
/// Multi-member ingress endpoint maps (`0=host:port,…`).
pub mod endpoints;
/// Cluster client error type.
pub mod error;
/// Shared fragment decode — canonical AnyMessage dispatch used by egress,
/// controlled, and poller paths. Not public API.
pub(crate) mod fragment;
/// Poll-loop idle helpers ([`rusteron_client::IdleStrategy`]).
pub mod idle;
/// Low-level egress event parse helpers (SessionEvent, NewLeader, redirects).
pub mod poller;
/// [`SessionState`] machine for connected / new-leader / closed.
pub mod state;
/// Aeron channel URI helpers (`AeronUriStringBuilder`).
pub mod uri;

pub use client::{AeronCluster, AsyncClusterConnect, ClusterClaim};
pub use config::SessionBuilder;
pub use controlled::{ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction};
pub use credentials::{CredentialsSupplier, NullCredentialsSupplier, StaticCredentials};
pub use egress::{EgressAdapter, EgressListener, NullListener};
pub use endpoints::{IngressEndpoint, parse_ingress_endpoints};
pub use error::{ClusterError, PublicationFailure};
pub use idle::{default_idle, poll_connect_until_done};
pub use poller::{EgressEvent, parse_event};
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
        // Smoke-check ergo-sbe production codecs are wired into the lib.
        assert_eq!(crate::codecs::session::SessionConnectRequestEncoder::SCHEMA_ID, 111);

        Ok(())
    }
}
