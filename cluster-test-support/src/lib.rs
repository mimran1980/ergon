//! Java test harness for ergo-aeron-cluster integration tests.
//!
//! Spawns the official Java Aeron consensus module and archive driver
//! for integration testing. Requires Java 17+ and Gradle-built jars.
//!
//! ⚠️ **Test-only.** Not for production use. PIDs are tracked and
//! processes are killed on drop, but process safety is best-effort.

pub mod archive;
pub mod cluster;
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
