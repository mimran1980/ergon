//! The real readers against the running host's own `/proc` and `/sys` (in
//! CI, or a Linux container): every check returns ok, a finding or unknown,
//! and the readers whose answers can be cross-checked agree with the kernel.
//! Elsewhere than Linux every reader reports unknown and nothing fires.
#![allow(missing_docs)]

use std::path::Path;

use ergon_tune::{
    Component, Finding, Host, HotSet, Needs, Nic, ReadError, Role, Thread, k8s_checks,
    t1_isolation, t2_nic_irqs, t3_thp, t4_aeron_dir, t5_numa_balancing, t6_nic_node,
    t7_sched_policy, t8_smt_siblings, t9_cpu_quota, t10_smt_on, thread_affinity, thread_policy,
};

// ANCHOR: run_all
/// Every host check, with `hot` and each interface the host shows.
fn run_all(host: &Host, cpus: &HotSet, interfaces: &[String]) -> Vec<Finding> {
    let mut all = Vec::new();
    all.extend(t1_isolation(host, cpus));
    for c in [Component::Md, Component::Io, Component::Rt] {
        all.extend(t3_thp(host, c));
    }
    all.push(t4_aeron_dir(
        host,
        Path::new("/dev/shm/ergon-tune-live/aeron"),
    ));
    all.push(t5_numa_balancing(host));
    all.push(t7_sched_policy(host, Thread::Current, Role::Loop));
    all.extend(t8_smt_siblings(host, cpus));
    all.push(t9_cpu_quota(host));
    all.push(t10_smt_on(host));
    all.extend(k8s_checks(
        host,
        cpus,
        &Needs::new(Thread::Current, Role::Loop),
    ));
    for i in interfaces {
        let nic = Nic::Interface(i.clone());
        all.extend(t2_nic_irqs(host, &nic, cpus));
        all.extend(t6_nic_node(host, &nic, cpus));
    }
    all
}
// ANCHOR_END: run_all

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    use ergon_tune::CheckId::{T1, T3, T4, T5, T7, T8, T9, T10};

    type R = Result<(), Box<dyn std::error::Error>>;
    use ergon_tune::{
        Arch, FsMagic, SchedPolicy, Status, ThrottleWatch, busy_poll_sysctls, carrier_changes,
        cpu_cgroup, fs_magic, nic_irqs,
    };

    fn interfaces() -> Result<Vec<String>, std::io::Error> {
        // Directories only: the bonding driver's `bonding_masters` is a file.
        let mut v: Vec<String> = std::fs::read_dir("/sys/class/net")?
            .filter_map(|e| {
                let e = e.ok()?;
                e.path().is_dir().then_some(())?;
                e.file_name().to_str().map(str::to_owned)
            })
            .collect();
        v.sort();
        Ok(v)
    }

    #[test]
    fn the_readers_agree_with_the_kernel() -> R {
        let host = Host::system();
        assert_eq!(thread_policy(&host, Thread::Current)?, SchedPolicy::Other);
        let affinity = thread_affinity(&host, Thread::Current)?;
        // std counts the same mask through sched_getaffinity (and may cap
        // it further by the cgroup's quota).
        let parallel = std::thread::available_parallelism()?.get();
        assert!(affinity.len() >= parallel, "{affinity} against {parallel}");
        assert_eq!(fs_magic(&host, Path::new("/dev/shm"))?, FsMagic::TMPFS);
        assert_eq!(
            fs_magic(&host, Path::new("/dev/shm/ergon-tune-not-created/x"))?,
            FsMagic::TMPFS
        );
        // A relative directory not created yet is on the working directory's
        // filesystem, not the root's.
        assert_eq!(
            fs_magic(&host, Path::new("ergon-tune-not-created/aeron"))?,
            fs_magic(&host, Path::new("."))?
        );
        let arch = ergon_tune::Platform::detect(&host).arch;
        if cfg!(target_arch = "x86_64") {
            assert_eq!(arch, Arch::X86_64);
        } else if cfg!(target_arch = "aarch64") {
            assert_eq!(arch, Arch::Aarch64);
        }
        for i in interfaces()? {
            carrier_changes(&host, &i)?;
            match nic_irqs(&host, &Nic::Interface(i.clone())) {
                Ok(_) | Err(ReadError::NoDevice { .. }) => {}
                Err(e) => return Err(format!("{i}: {e}").into()),
            }
        }
        // The host's network namespace has them; a container's own has not.
        match busy_poll_sysctls(&host) {
            Ok(_) => {}
            Err(ReadError::Io { kind, .. }) => assert_eq!(kind, std::io::ErrorKind::NotFound),
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    #[test]
    fn every_host_check_answers_on_the_running_host() -> R {
        let host = Host::system();
        let affinity = thread_affinity(&host, Thread::Current)?;
        let cpu = affinity.iter().next().ok_or("no CPU in the affinity")?;
        let mut cpus = HotSet::new().with(cpu, Role::Loop);
        if let Some(other) = affinity.iter().nth(1) {
            cpus.push(other, Role::SqPoll);
        }
        let all = run_all(&host, &cpus, &interfaces()?);
        for f in &all {
            assert!(!f.message.is_empty(), "{f:?}");
            println!("{f}");
        }
        // The files every Linux has: no unknown for these.
        for f in all
            .iter()
            .filter(|f| [T1, T3, T5, T8, T9, T10].contains(&f.id))
        {
            assert!(!f.is_unknown(), "{f}");
        }
        for id in [T4, T7] {
            let f = all.iter().find(|f| f.id == id).ok_or("missing")?;
            assert_eq!(f.status, Status::Ok, "{f}");
        }
        Ok(())
    }

    /// T9's reader against std's, which caps `available_parallelism` by
    /// the same quota, and the throttle watch against the kernel's files.
    #[test]
    fn the_cpu_quota_agrees_with_std_and_the_counters_only_grow() -> R {
        let host = Host::system();
        let cg = cpu_cgroup(&host)?;
        println!("{cg:?}");
        let affinity = thread_affinity(&host, Thread::Current)?.len();
        let parallel = std::thread::available_parallelism()?.get();
        // std rounds each quota down to whole CPUs, at least one.
        let ours = cg
            .limits
            .iter()
            .filter_map(|q| usize::try_from(q.quota_us? / q.period_us).ok())
            .map(|n| n.max(1))
            .min();
        if parallel < affinity {
            assert_eq!(
                ours,
                Some(parallel),
                "std caps {affinity} CPUs at {parallel}"
            );
        }
        let mut watch = ThrottleWatch::open(&host)?;
        if let Some(note) = watch.note() {
            println!("{note}");
        }
        let (before, now) = (watch.last(), watch.sample()?);
        for (a, b) in [
            (before.periods, now.periods),
            (before.throttled, now.throttled),
            (before.throttled_us, now.throttled_us),
            (before.local_throttled_us, now.local_throttled_us),
        ] {
            assert_eq!(a.is_some(), b.is_some(), "{before:?} {now:?}");
            assert!(b >= a, "{before:?} {now:?}");
        }
        if let Some(f) = watch.check() {
            assert_eq!(f.id, T9);
            println!("{f}");
        }
        Ok(())
    }

    #[cfg(feature = "nic")]
    #[test]
    fn ethtool_netlink_round_trips_with_the_kernel() -> R {
        use ergon_tune::{
            Ethtool, EthtoolCache, EthtoolError, Netlink, n1_adaptive_rx, n2_deferred_irqs,
            n3_azure_vf, n4_rx_ring,
        };
        const EOPNOTSUPP: i32 = 95;
        const ENODEV: i32 = 19;
        let host = Host::system();
        let mut n = Netlink::new();
        // No interface has this index: the kernel's ethtool header parser
        // refuses it, so the request reached ethtool and was understood.
        assert_eq!(
            n.rings(&host, 0x7fff_fff0),
            Err(EthtoolError::Kernel(ENODEV))
        );
        assert_eq!(
            n.coalesce(&host, 0x7fff_fff0),
            Err(EthtoolError::Kernel(ENODEV))
        );
        let lo: u32 = std::fs::read_to_string("/sys/class/net/lo/ifindex")?
            .trim()
            .parse()?;
        for r in [
            n.rings(&host, lo).map(|_| ()),
            n.coalesce(&host, lo).map(|_| ()),
        ] {
            assert!(
                matches!(r, Ok(()) | Err(EthtoolError::Kernel(EOPNOTSUPP))),
                "{r:?}"
            );
        }
        let mut cache = EthtoolCache::new();
        for i in interfaces()? {
            let (rings, coalesce) = (cache.rings(&host, &i), cache.coalesce(&host, &i));
            println!("{i}: rings {rings:?}, coalesce {coalesce:?}");
            for r in [rings.map(|_| ()), coalesce.map(|_| ())] {
                assert!(
                    matches!(r, Ok(()) | Err(EthtoolError::Kernel(_))),
                    "{i}: {r:?}"
                );
            }
            // ANCHOR: nic_checks
            let mut v = vec![
                n1_adaptive_rx(&host, &i, &mut cache),
                n3_azure_vf(&host, &i),
                n4_rx_ring(&host, &i, &mut cache),
            ];
            v.extend(n2_deferred_irqs(&host, &i, Some(false)));
            // ANCHOR_END: nic_checks
            for f in &v {
                assert!(!f.message.is_empty(), "{f:?}");
                println!("{f}");
            }
        }
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
#[test]
fn elsewhere_every_reader_is_unknown_and_nothing_fires() {
    let host = Host::system();
    assert_eq!(
        thread_policy(&host, Thread::Current),
        Err(ReadError::NotLinux("the host's /sys and /proc"))
    );
    assert!(thread_affinity(&host, Thread::Current).is_err());
    let cpus = HotSet::new().with(0, Role::Loop);
    let all = run_all(&host, &cpus, &["lo".to_owned(), "en0".to_owned()]);
    assert_ne!(all.len(), 0);
    for f in &all {
        assert!(f.is_unknown(), "{f}");
    }
}
