use std::path::{Path, PathBuf};

use ergo_aeron_cluster::ClusterError;

/// Locate a jar in the aeron Gradle build output.
///
/// # Errors
///
/// [`ClusterError::ConnectFailed`] when no matching jar is found.
pub fn try_find_jar(name_prefix: &str) -> Result<PathBuf, ClusterError> {
    let aeron = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("aeron");

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
                    && Path::new(&name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
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
