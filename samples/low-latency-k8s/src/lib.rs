//! The low-latency Kubernetes sample: an Aeron ping/pong whose hot thread
//! busy-spins on one exclusive CPU while every helper thread (the Aeron
//! client conductor, the `tracing-appender` worker, the stats thread) stays
//! on the node's shared CPUs, plus `ll-probe`, which proves that layout from
//! outside the pods.
//!
//! The layout comes from the `ergon-shared-cpus` NRI plugin
//! (`k8s/nri-shared-cpus`): a Guaranteed pod gets its exclusive CPUs plus a
//! slice of the node's shared CPUs, every thread starts on the shared ones,
//! and the plugin names both sets in `ERGON_ISOLATED_CPUS`,
//! `ERGON_SHARED_CPUS` and `ERGON_HELPER_CPUS`. The app creates every helper
//! while it is still on the shared CPUs and pins only the hot thread
//! ([`cpus`]).
//!
//! Logging is the standard `tracing` stack: `tracing-subscriber`'s `fmt` to
//! stdout through `tracing-appender`'s lossy non-blocking writer
//! ([`logging`]). The event is still formatted on the calling thread; only
//! the `write(2)` moves to the worker. The hot loop logs only rate-limited
//! warnings ([`ratelimit`]).
//!
//! Linux is the target. The parsers, formatters and the logging setup build
//! and test everywhere; pinning and `/proc` reads report
//! [`LlError::Unsupported`] elsewhere.

pub mod costs;
pub mod cpus;
pub mod error;
pub mod logging;
pub mod pingpong;
pub mod probe;
pub mod ratelimit;
pub mod report;
pub mod selfcheck;
pub mod stats;

pub use error::LlError;
