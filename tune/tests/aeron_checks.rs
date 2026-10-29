//! T22, Aeron's affinity settings by their pre-1.54 names: set, they are
//! ignored, so it warns; unset or set by their 1.54 names, it is fine.
#![allow(missing_docs)]

use ergon_tune::{
    AERON_HUGEPAGE_BYTES, Severity, Status, t22_aeron_affinity_names, t23_aeron_hugepage_env,
};

fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        vars.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| (*v).to_owned())
    }
}

#[test]
fn t22_warns_for_each_old_name_set() {
    let v = t22_aeron_affinity_names(env(&[
        ("AERON_DRIVER_CPUSET_AFFINITY", "true"),
        ("AERON_DRIVER_CPUSET_WARNINGS_AS_ERRORS", "true"),
    ]));
    assert_eq!(v.id, "T22");
    assert!(v.is_warning(), "{v}");
    for part in [
        "AERON_DRIVER_CPUSET_AFFINITY",
        "AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS",
        "AERON_DRIVER_CPUSET_WARNINGS_AS_ERRORS",
        "AERON_THREAD_AFFINITY_FAIL_ON_VALIDATION_ERRORS",
        "1.54",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
}

#[test]
fn t22_is_ok_without_old_names() {
    let v = t22_aeron_affinity_names(env(&[("AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS", "true")]));
    assert_eq!(v.status, Status::Ok, "{v}");
    assert_eq!(v.severity, Severity::Warn, "{v}");
    // An empty value is not a setting.
    let v = t22_aeron_affinity_names(env(&[("AERON_DRIVER_CPUSET_AFFINITY", "")]));
    assert_eq!(v.status, Status::Ok, "{v}");
}

#[test]
fn t23_is_ok_off_hugetlbfs() {
    let v = t23_aeron_hugepage_env(false, env(&[]));
    assert_eq!(v.status, Status::Ok, "{v}");
    assert!(v.message.contains("not on hugetlbfs"), "{v}");
}

#[test]
fn t23_is_ok_when_both_properties_are_set() {
    let v = t23_aeron_hugepage_env(
        true,
        env(&[
            ("AERON_FILE_PAGE_SIZE", AERON_HUGEPAGE_BYTES),
            ("AERON_TERM_BUFFER_SPARSE_FILE", "false"),
        ]),
    );
    assert_eq!(v.status, Status::Ok, "{v}");
    assert_eq!(v.severity, Severity::Warn, "{v}");
    // `0` is the same non-sparse setting as `false`.
    let zero = t23_aeron_hugepage_env(
        true,
        env(&[
            ("AERON_FILE_PAGE_SIZE", AERON_HUGEPAGE_BYTES),
            ("AERON_TERM_BUFFER_SPARSE_FILE", "0"),
        ]),
    );
    assert_eq!(zero.status, Status::Ok, "{zero}");
}

#[test]
fn t23_warns_when_the_page_size_or_sparse_flag_is_missing() {
    let missing = t23_aeron_hugepage_env(true, env(&[]));
    assert!(missing.is_warning(), "{missing}");
    for part in [
        "AERON_FILE_PAGE_SIZE=2097152",
        "AERON_TERM_BUFFER_SPARSE_FILE=false",
        "hugepages.ergon.io",
        "AERON_DIR",
        "gateway",
        "PCI",
    ] {
        assert!(missing.message.contains(part), "{part:?} not in {missing}");
    }
    let sparse = t23_aeron_hugepage_env(
        true,
        env(&[
            ("AERON_FILE_PAGE_SIZE", AERON_HUGEPAGE_BYTES),
            ("AERON_TERM_BUFFER_SPARSE_FILE", "true"),
        ]),
    );
    assert!(sparse.is_warning(), "{sparse}");
    let page = t23_aeron_hugepage_env(
        true,
        env(&[
            ("AERON_FILE_PAGE_SIZE", "4096"),
            ("AERON_TERM_BUFFER_SPARSE_FILE", "0"),
        ]),
    );
    assert!(page.is_warning(), "{page}");
}
