//! Java test harness for `ergo-aeron-cluster` integration tests.
//!
//! Spawns the official Java Aeron consensus module (via `ClusterLauncher`)
//! for integration testing. Requires a JDK, 17 or newer, and Gradle-built
//! jars under the `aeron/` submodule (`just build-aeron-jars` from the repo
//! root). CI runs it on Temurin 25, the newest LTS in Aeron's own test
//! matrix; [`jvm`] picks `$JAVA_HOME/bin/java` when `JAVA_HOME` is set, else
//! `java` on `PATH`, and checks its version before the first launch.
//!
//! ⚠️ **Test-only.** Not for production use. Child PIDs are tracked and
//! processes are killed on drop, but process safety is best-effort.
//!
//! # Jars and their Aeron release
//!
//! [`jar`] locates `aeron-all-`, `aeron-cluster-`, and sample jars under
//! known `aeron/*/build/libs` paths. `scripts/check-aeron-version.sh` fails
//! unless a checked-out `aeron/` is at the commit pinned in
//! `scripts/aeron-pin.txt` with no local changes to tracked files and every
//! jar in those paths is named for that release (it reads jar names, not
//! contents, so a jar built before such changes were undone still passes
//! until `just build-aeron-jars` rebuilds it). The jars have no byte hashes:
//! the same commit built by another JDK, or another update of one, gives
//! different bytes.
//!
//! # Spawn failure modes
//!
//! | Failure | Typical cause |
//! |---------|----------------|
//! | `cannot run`, or `is Java 11; ... need Java 17 or newer` | No JDK, or one older than 17: set `JAVA_HOME` to a JDK 25 |
//! | `failed to spawn ClusterLauncher` | `java` vanished between the version check and the launch |
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
/// The `java` the harness runs, its version check, and its JVM flags.
pub mod jvm;

pub use archive::EmbeddedArchiveDriver;
pub use cluster::TestCluster;
pub use jar::{find_jar, try_find_jar};
