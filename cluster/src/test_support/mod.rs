//! Java test harness for `ergo-aeron-cluster` integration tests.
//!
//! Spawns the official Java Aeron consensus module (via `ClusterLauncher`)
//! for integration testing. Requires **Java 17+** and Gradle-built jars under
//! the `aeron/` submodule (`just build-aeron-jars` from the repo root).
//!
//! ⚠️ **Test-only.** Not for production use. Child PIDs are tracked and
//! processes are killed on drop, but process safety is best-effort.
//!
//! # Jars and integrity
//!
//! [`jar`] locates `aeron-all-`, `aeron-cluster-`, and sample jars under
//! known `aeron/*/build/libs` paths. Optional checksums live in
//! `test-jars.sha256` (crate root) for drift detection after jar rebuilds.
//!
//! # Spawn failure modes
//!
//! | Failure | Typical cause |
//! |---------|----------------|
//! | `failed to spawn ClusterLauncher` | `java` missing from `PATH` |
//! | `ClusterLauncher did not emit CLUSTER_READY` | Port bind clash, missing jars, classpath incomplete |
//! | Jar not found | Aeron submodule not built — run `just build-aeron-jars` |
//! | Tests hang / flaky | Previous node left ports busy; kill stale Java cluster processes |
//!
//! Do **not** commit `aeron-cluster-[0-9]/` runtime directories created by launches.

/// Embedded archive driver helpers for archive-related tests.
pub mod archive;
/// Multi-node cluster spawn, kill, and restart-keep-dirs.
pub mod cluster;
/// Locate Aeron jars on the local filesystem.
pub mod jar;

pub use archive::EmbeddedArchiveDriver;
pub use cluster::TestCluster;

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_compiles() {
        assert!(true);
    }
}
