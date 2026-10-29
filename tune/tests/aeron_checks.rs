//! T22, Aeron's affinity settings by their pre-1.54 names: set, they are
//! ignored, so it warns; unset or set by their 1.54 names, it is fine.
#![allow(missing_docs)]

use ergon_tune::{Severity, Status, t22_aeron_affinity_names};

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
