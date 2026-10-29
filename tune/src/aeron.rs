//! T22: Aeron's affinity settings named as before Aeron 1.54.

use crate::finding::{Finding, Severity};

const T22_EVIDENCE: &str = "Aeron 1.54.0 aeron-driver/src/main/c/aeronmd.h (the \
     AERON_THREAD_AFFINITY_* names replace AERON_DRIVER_CPUSET_*; the old names are no longer \
     read) and aeron_driver_context.c (aeron_driver_context_init); not measured here";

/// Each pre-1.54 name and the name Aeron 1.54 reads instead.
const RENAMED: [(&str, &str); 2] = [
    (
        "AERON_DRIVER_CPUSET_AFFINITY",
        "AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS",
    ),
    (
        "AERON_DRIVER_CPUSET_WARNINGS_AS_ERRORS",
        "AERON_THREAD_AFFINITY_FAIL_ON_VALIDATION_ERRORS",
    ),
];

/// T22: no Aeron affinity setting is given by a name Aeron 1.54 no longer
/// reads.
///
/// `env` looks a variable up (`|n| std::env::var(n).ok()` for this process).
/// Aeron 1.54 ignores the old names, so a driver configured with them runs
/// with cpuset offsets off and validation warnings not fatal, whatever they
/// say.
#[must_use]
pub fn t22_aeron_affinity_names(env: impl Fn(&str) -> Option<String>) -> Finding {
    let id = "T22";
    let sev = Severity::Warn;
    let set: Vec<String> = RENAMED
        .iter()
        .filter(|(old, _)| env(old).is_some_and(|v| !v.is_empty()))
        .map(|(old, new)| format!("{old} (now {new})"))
        .collect();
    if set.is_empty() {
        return Finding::ok(
            id,
            sev,
            T22_EVIDENCE,
            "no Aeron affinity setting uses a pre-1.54 name".to_owned(),
        );
    }
    Finding::found(
        id,
        sev,
        T22_EVIDENCE,
        format!(
            "set by a name Aeron 1.54 no longer reads, so ignored: {}. Fix: rename it in the \
             environment (a pod spec's env, a ConfigMap)",
            set.join(", ")
        ),
    )
}
