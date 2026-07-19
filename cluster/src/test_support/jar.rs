use std::path::PathBuf;

/// Locate a jar in the aeron Gradle build output.
pub fn find_jar(name_prefix: &str) -> PathBuf {
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
                    && name.ends_with(".jar")
                    && !name.contains("sources")
                    && !name.contains("javadoc")
                {
                    return entry.path();
                }
            }
        }
    }
    panic!("jar with prefix '{name_prefix}' not found. Run just build first.");
}

/// Compute SHA-256 of a file. Uses `shasum -a 256` (macOS) or `sha256sum`.
pub fn sha256(path: &PathBuf) -> String {
    let output = std::process::Command::new(if cfg!(target_os = "macos") {
        "shasum"
    } else {
        "sha256sum"
    })
    .args(if cfg!(target_os = "macos") {
        &["-a", "256"] as &[&str]
    } else {
        &[]
    })
    .arg(path)
    .output()
    .expect("shasum/sha256sum not found");
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_aeron_all_jar() {
        let path = find_jar("aeron-all-");
        assert!(path.exists());
    }

    #[test]
    fn test_find_aeron_cluster_jar() {
        let path = find_jar("aeron-cluster-");
        assert!(path.exists());
    }

    #[test]
    fn test_find_aeron_archive_jar() {
        let path = find_jar("aeron-archive-");
        assert!(path.exists());
    }
}
