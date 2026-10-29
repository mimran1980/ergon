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

const T23_EVIDENCE: &str = "a sized 2 MiB hugetlbfs is what lets Aeron allocate its log buffers \
     (an unsized mount reports no free space); book/src/transports/comparison.md#hugepages (the \
     IPC 2 MiB directory was not separated from tmpfs); k8s/nri-latency-pods/README.md";

/// Page size Aeron must be told when `AERON_DIR` is on the plugin's 2 MiB mount.
pub const AERON_HUGEPAGE_BYTES: &str = "2097152";

/// T23: a directory on hugetlbfs has the page size and non-sparse terms set.
///
/// `on_hugepages` is whether `AERON_DIR` itself is on hugetlbfs. Elsewhere
/// the check is ok. `env` looks a variable up. The nri-shared-cpus plugin
/// sets both properties when `hugepages.ergon.io/<container>` is `2M`. It
/// does not set `AERON_DIR`, and it does not fill an address, a gateway or
/// a PCI address.
#[must_use]
pub fn t23_aeron_hugepage_env(on_hugepages: bool, env: impl Fn(&str) -> Option<String>) -> Finding {
    let id = "T23";
    let sev = Severity::Warn;
    if !on_hugepages {
        return Finding::ok(
            id,
            sev,
            T23_EVIDENCE,
            "the Aeron directory is not on hugetlbfs, so the 2 MiB page-size properties do not apply"
                .to_owned(),
        );
    }
    let page = env("AERON_FILE_PAGE_SIZE").unwrap_or_default();
    let sparse = env("AERON_TERM_BUFFER_SPARSE_FILE").unwrap_or_default();
    let page_ok = page.trim() == AERON_HUGEPAGE_BYTES;
    let sparse_ok = matches!(sparse.trim().to_ascii_lowercase().as_str(), "false" | "0");
    if page_ok && sparse_ok {
        return Finding::ok(
            id,
            sev,
            T23_EVIDENCE,
            format!(
                "AERON_FILE_PAGE_SIZE={AERON_HUGEPAGE_BYTES} and AERON_TERM_BUFFER_SPARSE_FILE=false"
            ),
        );
    }
    Finding::found(
        id,
        sev,
        T23_EVIDENCE,
        format!(
            "AERON_DIR is on hugetlbfs with AERON_FILE_PAGE_SIZE={page:?} and \
             AERON_TERM_BUFFER_SPARSE_FILE={sparse:?}. An unsized mount, or a page size Aeron was \
             not told, refuses the log buffers; sparse terms fault a page in on first touch. Fix: \
             AERON_FILE_PAGE_SIZE={AERON_HUGEPAGE_BYTES} and AERON_TERM_BUFFER_SPARSE_FILE=false. \
             hugepages.ergon.io/<container>=2M with a hugepages-2Mi limit sets both. The process \
             still sets AERON_DIR to $ERGON_HUGEPAGES_DIR/aeron. The address, the gateway and the \
             PCI address stay unset"
        ),
    )
}
