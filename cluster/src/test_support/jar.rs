use std::path::{Path, PathBuf};

use crate::ClusterError;

/// Locate a jar in the aeron Gradle build output.
///
/// Returns [`ClusterError::ConnectFailed`] when no matching jar is found
/// (prefer this over panicking in CI). The legacy panic path is available
/// via [`find_jar`].
pub fn try_find_jar(name_prefix: &str) -> Result<PathBuf, ClusterError> {
    let aeron = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("aeron");

    for dir in &[
        aeron.join("aeron-all/build/libs"),
        aeron.join("aeron-cluster/build/libs"),
        aeron.join("aeron-archive/build/libs"),
        aeron.join("aeron-samples/build/libs"),
    ] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(name_prefix)
                    && name.ends_with(".jar")
                    && !name.contains("sources")
                    && !name.contains("javadoc")
                {
                    return Ok(entry.path());
                }
            }
        }
    }
    Err(ClusterError::connect(format!(
        "jar with prefix '{name_prefix}' not found. Run `just build-aeron-jars` first."
    )))
}

/// Locate a jar in the aeron Gradle build output (panics if missing).
pub fn find_jar(name_prefix: &str) -> PathBuf {
    try_find_jar(name_prefix).unwrap_or_else(|e| panic!("{e}"))
}

/// Compute SHA-256 of a file. Uses `shasum -a 256` (macOS) or `sha256sum`.
pub fn try_sha256(path: &Path) -> Result<String, ClusterError> {
    let (bin, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("shasum", &["-a", "256"])
    } else {
        ("sha256sum", &[])
    };
    let output = std::process::Command::new(bin)
        .args(args)
        .arg(path)
        .output()
        .map_err(|e| ClusterError::connect(format!("{bin} failed: {e}")))?;
    if !output.status.success() {
        return Err(ClusterError::connect(format!("{bin} exited {}", output.status)));
    }
    let s = String::from_utf8_lossy(&output.stdout);
    Ok(s.split_whitespace().next().unwrap_or("unknown").to_string())
}

/// Compute SHA-256 of a file (panics on tool failure).
pub fn sha256(path: &PathBuf) -> String {
    try_sha256(path).unwrap_or_else(|e| panic!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_aeron_all_jar() -> Result<(), Box<dyn std::error::Error>> {
        let path = try_find_jar("aeron-all-")?;
        assert!(path.exists());
        Ok(())
    }

    #[test]
    fn test_find_aeron_cluster_jar() -> Result<(), Box<dyn std::error::Error>> {
        let path = try_find_jar("aeron-cluster-")?;
        assert!(path.exists());
        Ok(())
    }

    #[test]
    fn test_find_aeron_archive_jar() -> Result<(), Box<dyn std::error::Error>> {
        let path = try_find_jar("aeron-archive-")?;
        assert!(path.exists());
        Ok(())
    }
}
