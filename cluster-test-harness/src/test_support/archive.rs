//! Embedded archive media driver, ported from rusteron-archive/testing.rs.

use std::path::PathBuf;
use std::process::{Child, Command};

use super::jar;

/// A Java `ArchivingMediaDriver` child process.
pub struct EmbeddedArchiveDriver {
    process: Child,
    pub aeron_dir: PathBuf,
}

impl EmbeddedArchiveDriver {
    /// Launch an archive media driver bound to localhost.
    pub fn start(base_port: u16) -> Self {
        let aeron_dir = std::env::temp_dir().join(format!("rusteron-archive-test-{}", base_port));
        let _ = std::fs::create_dir_all(&aeron_dir);
        let aeron_all = jar::find_jar("aeron-all-");

        let process = Command::new("java")
            .args([
                "--add-opens",
                "java.base/jdk.internal.misc=ALL-UNNAMED",
                "-cp",
                &aeron_all.display().to_string(),
                "io.aeron.archive.ArchivingMediaDriver",
                &format!("--aeron.dir={}", aeron_dir.display()),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn archive driver");

        std::thread::sleep(std::time::Duration::from_secs(2));
        Self { process, aeron_dir }
    }
}

impl Drop for EmbeddedArchiveDriver {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = std::fs::remove_dir_all(&self.aeron_dir);
    }
}
