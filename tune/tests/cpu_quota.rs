//! T9, the process's CPU quota (CFS bandwidth control), and `ThrottleWatch`,
//! its throttling counters read while a loop runs: against fixture trees of
//! a pod's, a privileged container's and a cgroup v1 host's cgroup files.
#![allow(missing_docs)]

mod common;

use common::{c9g_tuned, pod};
use ergon_tune::{
    CgroupVersion, CpuQuota, Severity, Status, ThrottleWatch, Throttling, cpu_cgroup, t9_cpu_quota,
};

type R = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------- T9

// ANCHOR: t9_pod
#[test]
fn t9_is_ok_for_a_container_without_a_quota() -> R {
    // What kubelet 1.33+ writes for a Guaranteed pod's container with whole
    // CPUs under the static CPU manager: `max`.
    let f = c9g_tuned("t9-max")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "max 100000\n")?;
    let t9 = t9_cpu_quota(&f.host());
    assert_eq!(t9.id, "T9");
    assert_eq!(t9.status, Status::Ok, "{t9}");
    assert!(t9.message.contains("/sys/fs/cgroup/cpu.max"), "{t9}");
    assert!(t9.message.contains("cpu.max is `max 100000`"), "{t9}");
    // The pod's cgroup above a container's own namespace cannot be seen.
    assert!(t9.message.contains("cpu.stat.local"), "{t9}");
    Ok(())
}

#[test]
fn t9_warns_for_a_whole_cpu_quota_and_names_the_kubernetes_fix() -> R {
    let f = pod("t9-quota")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "200000 100000\n")?
        .cpu_stat("/sys/fs/cgroup", 5000, 12, 34_000)?;
    let t9 = t9_cpu_quota(&f.host());
    assert!(t9.is_warning(), "{t9}");
    for part in [
        "/sys/fs/cgroup/cpu.max is `200000 100000`",
        "2 CPUs per 100 ms",
        "until the period ends",
        "nr_throttled 12 of nr_periods 5000",
        // Kubernetes: the kubelet drops the quota itself from 1.33 ...
        "1.33",
        "cpuManagerPolicy: static",
        "Guaranteed",
        "whole number",
        // ... never the node-wide switch: it lifts every pod's CPU limit ...
        "Never set kubelet `cpuCFSQuota: false`",
        "every pod",
        // ... CRI-O's annotation, which containerd ignores.
        "cpu-quota.crio.io: \"disable\"",
        "containerd",
    ] {
        assert!(t9.message.contains(part), "{part:?} not in {t9}");
    }
    assert!(t9.evidence.contains("sched-bwc.rst"), "{}", t9.evidence);
    assert!(t9.evidence.contains("127525"), "{}", t9.evidence);
    Ok(())
}
// ANCHOR_END: t9_pod

#[test]
fn t9_says_a_fractional_quota_stops_a_lone_spinner_every_period() -> R {
    let f = pod("t9-fraction")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "50000 100000\n")?;
    let t9 = t9_cpu_quota(&f.host());
    assert!(t9.is_warning(), "{t9}");
    for part in [
        "0.5 CPU per 100 ms",
        "50 ms into every period",
        "other 50 ms",
    ] {
        assert!(t9.message.contains(part), "{part:?} not in {t9}");
    }
    let f = pod("t9-fraction-odd")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "150000 100000\n")?;
    assert!(
        t9_cpu_quota(&f.host()).message.contains("1.5 CPUs"),
        "{}",
        t9_cpu_quota(&f.host())
    );
    Ok(())
}

#[test]
fn t9_reads_the_cgroup_and_every_parent_visible_here() -> R {
    // A privileged container (no cgroup namespace) sees the host's tree:
    // the container has no quota, its pod has one.
    let pod_dir = "/kubepods.slice/kubepods-pod1234.slice";
    let path = format!("{pod_dir}/cri-containerd-abc.scope");
    let f = pod("t9-parent")?;
    f.cgroup_v2(&path)?
        .file(&format!("/sys/fs/cgroup{path}/cpu.max"), "max 100000\n")?
        .file(
            &format!("/sys/fs/cgroup{pod_dir}/cpu.max"),
            "300000 100000\n",
        )?;
    let cg = cpu_cgroup(&f.host())?;
    assert_eq!(cg.version, CgroupVersion::V2);
    assert_eq!(cg.dir, format!("/sys/fs/cgroup{path}"));
    assert_eq!(
        cg.limits,
        vec![
            CpuQuota {
                dir: format!("/sys/fs/cgroup{path}"),
                quota_us: None,
                period_us: 100_000,
            },
            CpuQuota {
                dir: format!("/sys/fs/cgroup{pod_dir}"),
                quota_us: Some(300_000),
                period_us: 100_000,
            },
        ]
    );
    let t9 = t9_cpu_quota(&f.host());
    assert!(t9.is_warning(), "{t9}");
    assert!(t9.message.contains("parent cgroup"), "{t9}");
    assert!(
        t9.message.contains(&format!(
            "/sys/fs/cgroup{pod_dir}/cpu.max is `300000 100000`"
        )),
        "{t9}"
    );
    assert!(t9.message.contains("3 CPUs"), "{t9}");
    Ok(())
}

#[test]
fn t9_reads_the_namespace_root_when_the_path_is_not_visible() -> R {
    // A path from outside this cgroup namespace: the container's own cgroup
    // is the one mounted at /sys/fs/cgroup.
    let f = pod("t9-root")?;
    f.file("/proc/self/cgroup", "0::/../../kubepods/x\n")?
        .file("/sys/fs/cgroup/cpu.max", "100000 100000\n")?;
    let cg = cpu_cgroup(&f.host())?;
    assert_eq!(cg.dir, "/sys/fs/cgroup");
    let t9 = t9_cpu_quota(&f.host());
    assert!(t9.is_warning(), "{t9}");
    assert!(
        t9.message
            .contains("this process's cgroup: /sys/fs/cgroup/cpu.max is `100000 100000`"),
        "{t9}"
    );
    // A path that is simply not mounted here.
    f.file("/proc/self/cgroup", "0::/kubepods/x\n")?;
    assert_eq!(cpu_cgroup(&f.host())?.dir, "/sys/fs/cgroup");
    Ok(())
}

#[test]
fn t9_reads_cgroup_v1() -> R {
    // runc mounts the container's own cpu directory at /sys/fs/cgroup/cpu.
    let f = pod("t9-v1")?;
    f.file(
        "/proc/self/cgroup",
        "12:memory:/kubepods/burstable/pod1/abc\n4:cpu,cpuacct:/kubepods/burstable/pod1/abc\n0::/\n",
    )?
    .file("/sys/fs/cgroup/cpu/cpu.cfs_quota_us", "150000\n")?
    .file("/sys/fs/cgroup/cpu/cpu.cfs_period_us", "100000\n")?
    .file(
        "/sys/fs/cgroup/cpu/cpu.stat",
        "nr_periods 10\nnr_throttled 4\nthrottled_time 9000000\n",
    )?;
    let cg = cpu_cgroup(&f.host())?;
    assert_eq!(cg.version, CgroupVersion::V1);
    let t9 = t9_cpu_quota(&f.host());
    assert!(t9.is_warning(), "{t9}");
    for part in [
        "/sys/fs/cgroup/cpu/cpu.cfs_quota_us is `150000` per 100000 µs",
        "1.5 CPUs",
        "nr_throttled 4 of nr_periods 10",
        "echo -1 > /sys/fs/cgroup/cpu/cpu.cfs_quota_us",
    ] {
        assert!(t9.message.contains(part), "{part:?} not in {t9}");
    }
    f.file("/sys/fs/cgroup/cpu/cpu.cfs_quota_us", "-1\n")?;
    assert_eq!(t9_cpu_quota(&f.host()).status, Status::Ok);
    Ok(())
}

#[test]
fn t9_is_ok_when_the_cpu_controller_is_off_for_the_cgroup() -> R {
    // A plain host: a login session's scope, no cpu controller below the
    // root, and the root cgroup has no cpu.max.
    let f = c9g_tuned("t9-nocontroller")?;
    f.cgroup_v2("/user.slice/user-1000.slice/session-3.scope")?;
    let t9 = t9_cpu_quota(&f.host());
    assert_eq!(t9.status, Status::Ok, "{t9}");
    assert!(t9.message.contains("cpu controller"), "{t9}");
    Ok(())
}

#[test]
fn t9_is_unknown_never_a_warning_when_unreadable() -> R {
    let f = c9g_tuned("t9-unknown")?;
    // No /proc/self/cgroup.
    let t9 = t9_cpu_quota(&f.host());
    assert!(t9.is_unknown() && !t9.is_warning(), "{t9}");
    // A cpu.max that does not parse.
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "lots 100000\n")?;
    let t9 = t9_cpu_quota(&f.host());
    assert!(t9.is_unknown(), "{t9}");
    assert!(t9.to_string().contains("/sys/fs/cgroup/cpu.max"), "{t9}");
    // Neither a cpu line nor a unified one.
    f.file("/proc/self/cgroup", "1:name=systemd:/x\n")?;
    assert!(t9_cpu_quota(&f.host()).is_unknown());
    // A unified line but no cgroup filesystem mounted.
    f.file("/proc/self/cgroup", "0::/\n")?
        .remove("/sys/fs/cgroup")?;
    assert!(t9_cpu_quota(&f.host()).is_unknown());
    Ok(())
}

// ---------------------------------------------------------------- ThrottleWatch

// ANCHOR: throttle_watch
#[test]
fn the_throttle_watch_reports_each_growth_once() -> R {
    let f = pod("t9-watch")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "200000 100000\n")?
        .cpu_stat("/sys/fs/cgroup", 100, 0, 0)?
        .file("/sys/fs/cgroup/cpu.stat.local", "throttled_usec 0\n")?;
    // Before the loop: opens the files and takes the first sample.
    let mut watch = ThrottleWatch::open(&f.host())?;
    assert_eq!(watch.dir(), "/sys/fs/cgroup");
    assert_eq!(watch.note(), None);
    assert_eq!(
        watch.last(),
        Throttling {
            periods: Some(100),
            throttled: Some(0),
            throttled_us: Some(0),
            local_throttled_us: Some(0),
        }
    );
    // Housekeeping, off the hot path: nothing new, nothing to say.
    assert_eq!(watch.check(), None);

    // The cgroup's own quota stopped it: both files grow, as in the kernel.
    f.cpu_stat("/sys/fs/cgroup", 160, 3, 12_500)?
        .file("/sys/fs/cgroup/cpu.stat.local", "throttled_usec 12500\n")?;
    let found = watch.check().ok_or("no finding for 3 throttled periods")?;
    assert_eq!(found.id, "T9");
    assert!(found.is_warning(), "{found}");
    for part in [
        "throttled in 3 of the last 60 periods",
        "12.5 ms",
        "nr_throttled 0 → 3",
        "Never set kubelet `cpuCFSQuota: false`",
    ] {
        assert!(found.message.contains(part), "{part:?} not in {found}");
    }
    assert_eq!(watch.check(), None, "the same counters again");
    Ok(())
}
// ANCHOR_END: throttle_watch

/// Printing `cpu.stat` first flushes the cgroup's CPU statistics under a
/// lock every cgroup shares (Linux 6.12), and the watch runs on the busy
/// thread's housekeeping. `cpu.stat.local` counts every stop, by the
/// cgroup's own quota too, so `cpu.stat` is read only after it grew.
#[test]
fn cpu_stat_is_read_only_after_cpu_stat_local_grew() -> R {
    let f = pod("t9-watch-lazy")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "200000 100000\n")?
        .cpu_stat("/sys/fs/cgroup", 100, 0, 0)?
        .file("/sys/fs/cgroup/cpu.stat.local", "throttled_usec 0\n")?;
    let mut watch = ThrottleWatch::open(&f.host())?;
    // A cpu.stat that would not parse is not seen while cpu.stat.local
    // holds still.
    f.file(
        "/sys/fs/cgroup/cpu.stat",
        "nr_periods 1\nnr_throttled lots\n",
    )?;
    assert_eq!(watch.check(), None, "cpu.stat was read");
    assert_eq!(watch.last().periods, Some(100));
    // cpu.stat.local grew: cpu.stat is read, and gives the periods.
    f.cpu_stat("/sys/fs/cgroup", 160, 3, 12_500)?
        .file("/sys/fs/cgroup/cpu.stat.local", "throttled_usec 12500\n")?;
    let found = watch.check().ok_or("no finding")?;
    assert!(
        found
            .message
            .contains("throttled in 3 of the last 60 periods"),
        "{found}"
    );
    Ok(())
}

#[test]
fn the_throttle_watch_sees_a_parents_quota_in_cpu_stat_local() -> R {
    let f = pod("t9-watch-local")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "max 100000\n")?
        .cpu_stat("/sys/fs/cgroup", 0, 0, 0)?
        .file("/sys/fs/cgroup/cpu.stat.local", "throttled_usec 0\n")?;
    let mut watch = ThrottleWatch::open(&f.host())?;
    f.file("/sys/fs/cgroup/cpu.stat.local", "throttled_usec 4000\n")?;
    let found = watch
        .check()
        .ok_or("no finding for the parent's throttling")?;
    assert!(found.is_warning(), "{found}");
    assert!(found.message.contains("parent cgroup"), "{found}");
    assert!(found.message.contains("4 ms"), "{found}");
    Ok(())
}

#[test]
fn the_throttle_watch_notes_what_it_cannot_see() -> R {
    // Before Linux 6.6 there is no cpu.stat.local.
    let f = pod("t9-watch-note")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "max 100000\n")?
        .cpu_stat("/sys/fs/cgroup", 0, 0, 0)?;
    let watch = ThrottleWatch::open(&f.host())?;
    let note = watch.note().ok_or("no note")?;
    assert!(
        note.contains("cpu.stat.local") && note.contains("6.6"),
        "{note}"
    );
    // The cpu controller off: cpu.stat has no throttling counters.
    f.file(
        "/sys/fs/cgroup/cpu.stat",
        "usage_usec 1\nuser_usec 1\nsystem_usec 0\n",
    )?;
    let watch = ThrottleWatch::open(&f.host())?;
    assert_eq!(watch.last().throttled, None);
    assert!(
        watch
            .note()
            .is_some_and(|n| n.contains("nr_throttled") && n.contains("cpu controller")),
        "{:?}",
        watch.note()
    );
    Ok(())
}

#[test]
fn the_throttle_watch_reads_cgroup_v1_nanoseconds() -> R {
    let f = pod("t9-watch-v1")?;
    f.file("/proc/self/cgroup", "4:cpu,cpuacct:/kubepods/pod1/abc\n")?
        .file("/sys/fs/cgroup/cpu/cpu.cfs_quota_us", "-1\n")?
        .file("/sys/fs/cgroup/cpu/cpu.cfs_period_us", "100000\n")?
        .file(
            "/sys/fs/cgroup/cpu/cpu.stat",
            "nr_periods 1\nnr_throttled 0\nthrottled_time 0\n",
        )?;
    let mut watch = ThrottleWatch::open(&f.host())?;
    f.file(
        "/sys/fs/cgroup/cpu/cpu.stat",
        "nr_periods 3\nnr_throttled 2\nthrottled_time 7000000\n",
    )?;
    let found = watch.check().ok_or("no finding")?;
    assert!(found.message.contains("7 ms"), "{found}");
    assert_eq!(watch.last().throttled_us, Some(7000));
    Ok(())
}

#[test]
fn a_failed_read_is_one_unknown_never_a_warning() -> R {
    let f = pod("t9-watch-bad")?;
    f.cgroup_v2("/")?
        .file("/sys/fs/cgroup/cpu.max", "max 100000\n")?
        .cpu_stat("/sys/fs/cgroup", 0, 0, 0)?;
    let mut watch = ThrottleWatch::open(&f.host())?;
    f.file(
        "/sys/fs/cgroup/cpu.stat",
        "nr_periods 1\nnr_throttled lots\n",
    )?;
    let found = watch.check().ok_or("no unknown")?;
    assert!(found.is_unknown() && !found.is_warning(), "{found}");
    assert_eq!(found.severity, Severity::Warn);
    assert_eq!(watch.check(), None, "said once");
    // It recovers when the file reads again.
    f.cpu_stat("/sys/fs/cgroup", 2, 1, 100)?;
    assert!(watch.check().is_some_and(|f| f.is_warning()));
    Ok(())
}

#[test]
fn the_throttle_watch_cannot_open_without_a_cgroup() -> R {
    let f = c9g_tuned("t9-watch-none")?;
    assert!(ThrottleWatch::open(&f.host()).is_err());
    Ok(())
}
