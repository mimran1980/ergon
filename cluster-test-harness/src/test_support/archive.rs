//! Embedded archive media driver, ported from rusteron-archive/testing.rs.

use std::path::PathBuf;
use std::process::Child;

use ergo_aeron_cluster::ClusterError;

use super::{jar, jvm};

/// A Java `ArchivingMediaDriver` child process.
pub struct EmbeddedArchiveDriver {
    process: Child,
    /// The driver's Aeron directory, removed when the driver drops.
    pub aeron_dir: PathBuf,
}

impl EmbeddedArchiveDriver {
    /// Launch an archive media driver bound to localhost.
    ///
    /// # Errors
    ///
    /// [`ClusterError::ConnectFailed`] when the `aeron-all` jar is missing,
    /// the `java` fails its check, or the driver will not spawn.
    pub fn start(base_port: u16) -> Result<Self, ClusterError> {
        let aeron_dir = std::env::temp_dir().join(format!("rusteron-archive-test-{base_port}"));
        let _ = std::fs::create_dir_all(&aeron_dir);
        let aeron_all = jar::try_find_jar("aeron-all-")?;

        let process = jvm::try_command()?
            .args([
                "-cp",
                &aeron_all.display().to_string(),
                "io.aeron.archive.ArchivingMediaDriver",
                &format!("--aeron.dir={}", aeron_dir.display()),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ClusterError::connect(format!("failed to spawn archive driver: {e}")))?;

        std::thread::sleep(std::time::Duration::from_secs(2));
        Ok(Self { process, aeron_dir })
    }
}

impl Drop for EmbeddedArchiveDriver {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = std::fs::remove_dir_all(&self.aeron_dir);
    }
}
