//! The JVM the harness runs Aeron's Java on: which `java`, its version check,
//! and the options every Java process gets
//! ([`test_options`](crate::test_support::jvm::test_options)).
//!
//! The binary is `$JAVA_HOME/bin/java` when `JAVA_HOME` is set, else `java` on
//! `PATH`. The first launch asks it for its `java.specification.version`; one
//! older than [`MIN_JAVA`](crate::test_support::jvm::MIN_JAVA), or a `java`
//! that will not run, fails that launch
//! and every later one with an error naming the binary and how to choose
//! another.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use ergo_aeron_cluster::ClusterError;

/// The oldest Java Aeron 1.53.3 runs on: its jars are Java 17 bytecode.
pub const MIN_JAVA: u32 = 17;

/// The Java the harness is tested on and the book recommends: Java 25, the
/// newest LTS release in Aeron 1.53.3's own CI matrix (17, 21 and 25).
pub const RECOMMENDED_JAVA: u32 = 25;

/// `jvm/test.jvm`, the java argument file every Java process the harness
/// starts reads: Aeron's own test settings (its `build.gradle`), so the
/// checks stay on and nothing spins.
#[must_use]
pub fn test_options() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("jvm")
        .join("test.jvm")
}

/// The `java` binary: `$JAVA_HOME/bin/java`, else `java` on `PATH`.
#[must_use]
pub fn java_path() -> PathBuf {
    match std::env::var_os("JAVA_HOME") {
        Some(home) if !home.is_empty() => PathBuf::from(home).join("bin").join("java"),
        _ => PathBuf::from("java"),
    }
}

/// The major version in `java -XshowSettings:properties -version` output:
/// its `java.specification.version`, which is `25` on Java 25 and `1.8` on
/// Java 8.
#[must_use]
pub fn parse_java_major(settings: &str) -> Option<u32> {
    let value = settings
        .lines()
        .find_map(|line| line.trim().strip_prefix("java.specification.version = "))?
        .trim();
    let value = value.strip_prefix("1.").unwrap_or(value);
    value.split('.').next()?.parse().ok()
}

fn probe(java: &Path) -> Result<u32, String> {
    let how = format!(
        "set JAVA_HOME to a JDK {RECOMMENDED_JAVA} (Temurin {RECOMMENDED_JAVA} recommended; \
         Aeron runs on {MIN_JAVA} or newer) or put one first on PATH"
    );
    let output = Command::new(java)
        .args(["-XshowSettings:properties", "-version"])
        .output()
        .map_err(|e| format!("cannot run `{}`: {e}: {how}", java.display()))?;
    let settings = String::from_utf8_lossy(&output.stderr);
    let major = parse_java_major(&settings).ok_or_else(|| {
        format!(
            "`{} -XshowSettings:properties -version` ({}) printed no java.specification.version: {how}",
            java.display(),
            output.status
        )
    })?;
    if major < MIN_JAVA {
        return Err(format!(
            "`{}` is Java {major}; Aeron Archive and Cluster need Java {MIN_JAVA} or newer: {how}",
            java.display()
        ));
    }
    Ok(major)
}

fn checked() -> &'static Result<(PathBuf, u32), String> {
    static CHECKED: OnceLock<Result<(PathBuf, u32), String>> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let java = java_path();
        probe(&java).map(|major| (java, major))
    })
}

/// The major version of the `java` the harness runs, after its check.
///
/// # Errors
///
/// [`ClusterError::ConnectFailed`] when the `java` will not run or is older
/// than [`MIN_JAVA`].
pub fn try_java_major() -> Result<u32, ClusterError> {
    checked()
        .as_ref()
        .map(|(_, major)| *major)
        .map_err(|e| ClusterError::connect(e.clone()))
}

/// A command for the checked `java`, with no flags: for a test that brings
/// its own, as `tests/jvm_settings.rs` does with the recommended ones.
///
/// # Errors
///
/// As [`try_java_major`].
pub fn try_java() -> Result<Command, ClusterError> {
    checked()
        .as_ref()
        .map(|(java, _)| Command::new(java))
        .map_err(|e| ClusterError::connect(e.clone()))
}

/// A command for the checked `java` reading [`test_options`].
///
/// # Errors
///
/// As [`try_java_major`].
pub fn try_command() -> Result<Command, ClusterError> {
    let mut options = OsString::from("@");
    options.push(test_options());
    let mut command = try_java()?;
    command.arg(options);
    Ok(command)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_specification_version_of_modern_and_legacy_java() {
        let java25 = "Property settings:\n    java.specification.version = 25\n    java.vm.specification.version = 25\n";
        assert_eq!(parse_java_major(java25), Some(25));
        let java8 = "    java.specification.name = Java Platform API Specification\n    java.specification.version = 1.8\n";
        assert_eq!(parse_java_major(java8), Some(8));
        assert_eq!(parse_java_major("java version \"27\""), None);
    }

    #[test]
    fn the_java_the_harness_runs_reports_its_version() -> Result<(), Box<dyn std::error::Error>> {
        let major = try_java_major()?;
        eprintln!("harness java: {} (Java {major})", java_path().display());
        Ok(())
    }

    #[test]
    fn the_test_options_open_what_agrona_and_checksums_need()
    -> Result<(), Box<dyn std::error::Error>> {
        let options = std::fs::read_to_string(test_options())?;
        for line in [
            "--add-opens java.base/jdk.internal.misc=ALL-UNNAMED",
            "--add-opens java.base/java.util.zip=ALL-UNNAMED",
            "-Dagrona.strict.alignment.checks=true",
        ] {
            assert!(options.lines().any(|l| l == line), "test.jvm lacks {line}");
        }
        Ok(())
    }

    #[test]
    fn a_java_that_will_not_run_names_itself_and_the_fix() {
        let missing = Path::new("/nonexistent/jdk/bin/java");
        let Err(error) = probe(missing) else {
            panic!("a missing java passed the check");
        };
        assert!(error.contains("/nonexistent/jdk/bin/java"), "{error}");
        assert!(error.contains("JAVA_HOME"), "{error}");
    }
}
