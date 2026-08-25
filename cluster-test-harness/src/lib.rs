//! Unpublished Java Aeron Cluster spawn harness.
//!
//! Owns [`TestCluster`], [`EmbeddedArchiveDriver`], and the `ClusterLauncher`
//! Java adapter. The published [`ergo_aeron_cluster`] crate does not advertise
//! a `test-harness` feature.

#![allow(missing_docs)]

pub mod test_support;
mod uri;

pub use test_support::{EmbeddedArchiveDriver, TestCluster, jar};
pub use uri::{channel_cstr, udp_endpoint_cstr};
