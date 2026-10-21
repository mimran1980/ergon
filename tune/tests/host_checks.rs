//! The host checks T1-T8 against fixture trees of five hosts: what each
//! finds, what it reports as fine, and what it cannot tell.
#![allow(missing_docs)]

mod common;

use std::path::Path;

use common::{Fixture, azure, c7i_tuned, c9g_tuned, debian_default, irq, pod};
use ergon_tune::{
    Arch, CheckId, Cloud, Component, CpuSet, Finding, FsMagic, HotSet, Nic, Platform, ReadError,
    Role, SchedPolicy, Severity, Status, Thread, busy_poll_sysctls, carrier_changes, nic_irqs,
    t1_isolation, t2_nic_irqs, t3_thp, t4_aeron_dir, t4_from, t5_numa_balancing, t6_nic_node,
    t7_sched_policy, t8_smt_siblings, t10_smt_on, thread_affinity, thread_policy,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn findings(v: &[Finding]) -> Vec<&Finding> {
    v.iter().filter(|f| f.is_finding()).collect()
}

fn unknowns(v: &[Finding]) -> Vec<&Finding> {
    v.iter().filter(|f| f.is_unknown()).collect()
}

/// Exactly one result, `Ok`, for `id`.
fn assert_only_ok(v: &[Finding], id: CheckId) {
    assert_eq!(v.len(), 1, "{v:#?}");
    assert_eq!(v[0].id, id);
    assert_eq!(v[0].status, Status::Ok, "{v:#?}");
    assert_ne!(v[0].message, "");
}

// ---------------------------------------------------------------- T1

// ANCHOR: t1_tuned
#[test]
fn t1_is_ok_on_tuned_hosts() -> R {
    let f = c9g_tuned("t1-c9g")?;
    assert_only_ok(
        &t1_isolation(&f.host(), &HotSet::new().with(3, Role::Loop)),
        CheckId::T1,
    );
    let f = c7i_tuned("t1-c7i")?;
    let hot = HotSet::new().with(2, Role::Sender).with(3, Role::Receiver);
    let v = t1_isolation(&f.host(), &hot);
    assert_only_ok(&v, CheckId::T1);
    assert!(
        v[0].message
            .ends_with("isolated, nohz_full and outside default_smp_affinity"),
        "{v:#?}"
    );
    Ok(())
}
// ANCHOR_END: t1_tuned

// ANCHOR: t1_untuned
#[test]
fn t1_warns_separately_for_isolation_nohz_and_default_affinity_on_an_untuned_host() -> R {
    let f = debian_default("t1-debian")?;
    let v = t1_isolation(&f.host(), &HotSet::new().with(2, Role::Loop));
    let found = findings(&v);
    assert_eq!(found.len(), 3, "{v:#?}");
    assert!(
        found
            .iter()
            .all(|f| f.id == CheckId::T1 && f.severity == Severity::Warn)
    );
    let fix = "isolcpus=nohz,domain,managed_irq,2 nohz_full=2 rcu_nocbs=2 irqaffinity=0-1,3-7";
    let domain = found
        .iter()
        .find(|f| f.message.contains("not isolated"))
        .ok_or("domain")?;
    assert!(
        domain.message.contains("CPU 2 (loop)"),
        "{}",
        domain.message
    );
    assert!(
        domain.message.contains("120 and 162 a minute to 4 and 0.5"),
        "{}",
        domain.message
    );
    assert!(domain.message.contains(fix), "{}", domain.message);
    let nohz = found
        .iter()
        .find(|f| f.message.contains("not nohz_full"))
        .ok_or("nohz")?;
    assert!(nohz.message.contains("1,002"), "{}", nohz.message);
    assert!(nohz.message.contains("60,120"), "{}", nohz.message);
    assert!(nohz.message.contains(fix), "{}", nohz.message);
    // The tick rate is the kernel's CONFIG_HZ (250 on Debian), not always
    // 1,000: the 1,003/s is AWS's kernel.
    assert!(nohz.message.contains("CONFIG_HZ"), "{}", nohz.message);
    assert!(
        !nohz.message.contains("about 1,000 times"),
        "{}",
        nohz.message
    );
    let irq = found
        .iter()
        .find(|f| f.message.contains("default_smp_affinity"))
        .ok_or("default affinity")?;
    assert!(
        irq.message
            .contains("echo fb > /proc/irq/default_smp_affinity"),
        "{}",
        irq.message
    );
    Ok(())
}
// ANCHOR_END: t1_untuned

#[test]
fn t1_names_a_kernel_without_no_hz_full() -> R {
    let f = debian_default("t1-nonohz")?;
    f.remove("/sys/devices/system/cpu/nohz_full")?;
    let v = t1_isolation(&f.host(), &HotSet::new().with(2, Role::Loop));
    let nohz = v
        .iter()
        .find(|f| f.is_finding() && f.message.contains("NO_HZ_FULL"))
        .ok_or("no NO_HZ_FULL finding")?;
    assert!(
        nohz.message.contains("CONFIG_NO_HZ_FULL"),
        "{}",
        nohz.message
    );
    assert!(unknowns(&v).is_empty(), "{v:#?}");
    Ok(())
}

#[test]
fn t1_counts_a_cgroup_isolated_partition_as_isolated() -> R {
    let f = debian_default("t1-cgroup")?;
    f.file("/sys/fs/cgroup/cpuset.cpus.isolated", "2\n")?
        .file("/sys/devices/system/cpu/nohz_full", "2\n")?
        .file("/proc/irq/default_smp_affinity", "fb\n")?;
    assert_only_ok(
        &t1_isolation(&f.host(), &HotSet::new().with(2, Role::Loop)),
        CheckId::T1,
    );
    Ok(())
}

#[test]
fn t1_does_not_ask_nohz_full_of_an_sqpoll_cpu() -> R {
    // ergon-io measured no difference with the SQPOLL CPU out of nohz_full
    // (io/benches/BENCHMARK.md, Host and NIC settings on AWS): a host whose
    // SQPOLL CPU is isolated but ticking is tuned.
    let f = c7i_tuned("t1-sqpoll")?;
    f.file("/sys/devices/system/cpu/nohz_full", "1-3\n")?;
    let hot = HotSet::new().with(2, Role::Loop).with(6, Role::SqPoll);
    let v = t1_isolation(&f.host(), &hot);
    assert_only_ok(&v, CheckId::T1);
    assert!(
        v[0].message
            .contains("nohz_full but for CPU 6 (SQPOLL thread)"),
        "{v:#?}"
    );
    // The media driver's agents are asked, whatever their role.
    for role in [Role::Sender, Role::Receiver, Role::SharedAgent] {
        let v = t1_isolation(&f.host(), &HotSet::new().with(6, role));
        assert!(
            v.iter().any(|f| f.is_warning()
                && f.message.contains("CPU 6 (")
                && f.message.contains("is not nohz_full")),
            "{role:?}: {v:#?}"
        );
    }
    // Still checked for isolation and the default IRQ affinity.
    f.file("/sys/devices/system/cpu/isolated", "1-3\n")?;
    let v = t1_isolation(&f.host(), &hot);
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v:#?}");
    assert!(
        found[0]
            .message
            .contains("CPU 6 (SQPOLL thread) is not isolated"),
        "{}",
        found[0].message
    );
    // A loop on that CPU is asked.
    let v = t1_isolation(&f.host(), &HotSet::new().with(6, Role::Loop));
    assert!(
        v.iter()
            .any(|f| f.is_warning() && f.message.contains("CPU 6 (loop) is not nohz_full")),
        "{v:#?}"
    );
    // So is a CPU shared by the loop and the SQPOLL thread.
    let hot = HotSet::new().with(6, Role::Loop).with(6, Role::SqPoll);
    assert!(
        t1_isolation(&f.host(), &hot)
            .iter()
            .any(|f| f.is_warning() && f.message.contains("not nohz_full")),
    );
    // Without NO_HZ_FULL, only the loop's CPU is reported.
    f.remove("/sys/devices/system/cpu/nohz_full")?
        .file("/sys/devices/system/cpu/isolated", "1-3,5-7\n")?;
    let v = t1_isolation(
        &f.host(),
        &HotSet::new().with(2, Role::Loop).with(6, Role::SqPoll),
    );
    let nohz: Vec<_> = findings(&v)
        .into_iter()
        .filter(|f| f.message.contains("not nohz_full"))
        .collect();
    assert_eq!(nohz.len(), 1, "{v:#?}");
    assert!(nohz[0].message.contains("CPU 2 (loop)"), "{v:#?}");
    Ok(())
}

#[test]
fn t1_checks_only_busy_spinning_cpus() -> R {
    // The conductor and the EAL lcore sit on housekeeping CPU 0 on purpose.
    let f = c7i_tuned("t1-roles")?;
    let hot = HotSet::new()
        .with(0, Role::Conductor)
        .with(0, Role::EalLcore)
        .with(4, Role::Housekeeping)
        .with(2, Role::Sender);
    assert_only_ok(&t1_isolation(&f.host(), &hot), CheckId::T1);
    let v = t1_isolation(&f.host(), &HotSet::new().with(0, Role::Conductor));
    assert_only_ok(&v, CheckId::T1);
    assert!(v[0].message.contains("no busy-spinning CPU"), "{v:#?}");
    Ok(())
}

#[test]
fn t1_is_unknown_never_a_warning_when_unreadable() -> R {
    let f = debian_default("t1-unknown")?;
    f.remove("/sys/devices/system/cpu")?;
    f.remove("/proc/irq")?;
    let v = t1_isolation(&f.host(), &HotSet::new().with(2, Role::Loop));
    assert!(findings(&v).is_empty(), "{v:#?}");
    assert!(!unknowns(&v).is_empty(), "{v:#?}");
    for u in unknowns(&v) {
        let Status::Unknown(reason) = &u.status else {
            return Err("not unknown".into());
        };
        assert!(reason.contains("not present"), "{reason}");
    }
    let f = debian_default("t1-malformed")?;
    f.file("/sys/devices/system/cpu/isolated", "two\n")?;
    let v = t1_isolation(&f.host(), &HotSet::new().with(2, Role::Loop));
    assert!(
        v.iter()
            .any(|f| matches!(&f.status, Status::Unknown(r) if r.contains("not a CPU list"))),
        "{v:#?}"
    );
    assert!(
        !v.iter()
            .any(|f| f.is_finding() && f.message.contains("not isolated")),
        "{v:#?}"
    );
    Ok(())
}

// ---------------------------------------------------------------- T2

#[test]
fn t2_is_ok_when_every_nic_irq_is_on_housekeeping() -> R {
    let f = c9g_tuned("t2-ok")?;
    let v = t2_nic_irqs(
        &f.host(),
        &Nic::Interface("ens6".into()),
        &HotSet::new().with(3, Role::Loop),
    );
    assert_only_ok(&v, CheckId::T2);
    assert!(v[0].message.contains("3 interrupts"), "{v:#?}");
    Ok(())
}

#[test]
fn t2_warns_for_each_irq_that_may_fire_on_a_hot_cpu() -> R {
    let f = debian_default("t2-debian")?;
    let v = t2_nic_irqs(
        &f.host(),
        &Nic::Interface("ens5".into()),
        &HotSet::new().with(2, Role::Loop),
    );
    let found = findings(&v);
    // The management IRQ (0-7) and queue 2's (2).
    assert_eq!(found.len(), 2, "{v:#?}");
    let q2 = found
        .iter()
        .find(|f| f.message.contains("IRQ 27"))
        .ok_or("IRQ 27")?;
    assert_eq!(q2.severity, Severity::Warn);
    assert!(q2.message.contains("ens5-Tx-Rx-2"), "{}", q2.message);
    assert!(q2.message.contains("CPU 2 (loop)"), "{}", q2.message);
    assert!(q2.message.contains("462–547 a"), "{}", q2.message);
    assert!(
        q2.message
            .contains("echo 0-1,3-7 > /proc/irq/27/smp_affinity_list"),
        "{}",
        q2.message
    );
    // ENA re-places its queue IRQs at every reset: queue i on CPU i modulo
    // the online CPUs (ena_setup_io_intr), isolated CPUs included.
    assert!(
        q2.message
            .contains("CPU i modulo the number of online CPUs"),
        "{}",
        q2.message
    );
    assert!(found.iter().any(|f| f.message.contains("IRQ 24")), "{v:#?}");
    Ok(())
}

#[test]
fn t2_reports_an_irq_still_delivered_to_a_hot_cpu_as_information() -> R {
    let f = c9g_tuned("t2-effective")?;
    f.irq(&irq(52, "ens6-Tx-Rx-1", "0", Some("3")))?;
    let v = t2_nic_irqs(
        &f.host(),
        &Nic::Interface("ens6".into()),
        &HotSet::new().with(3, Role::Loop),
    );
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v:#?}");
    assert_eq!(found[0].severity, Severity::Info);
    assert!(
        found[0].message.contains("moves at its next interrupt"),
        "{}",
        found[0].message
    );
    Ok(())
}

#[test]
fn t2_follows_an_azure_interface_to_its_vf() -> R {
    let f = azure("t2-azure")?;
    let hot = HotSet::new().with(4, Role::Receiver);
    let irqs = nic_irqs(&f.host(), &Nic::Interface("eth1".into()))?;
    assert_eq!(irqs.iter().map(|i| i.irq).collect::<Vec<_>>(), [70, 71, 72]);
    assert_eq!(irqs[2].device, "7870:00:00.0");
    assert_eq!(irqs[2].action.as_deref(), Some("mana_q1@pci:7870:00:00.0"));
    let v = t2_nic_irqs(&f.host(), &Nic::Interface("eth1".into()), &hot);
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v:#?}");
    assert!(
        found[0]
            .message
            .contains("IRQ 72 (mana_q1@pci:7870:00:00.0"),
        "{}",
        found[0].message
    );
    assert!(!found[0].message.contains("ENA"), "{}", found[0].message);
    Ok(())
}

#[test]
fn t2_follows_a_vlan_or_bond_to_the_nics_below_it() -> R {
    // ens5.100 is a VLAN on ens5 (no device of its own); bond0 bonds
    // ens5 and ens6, and bond0.7 is a VLAN on the bond.
    let f = debian_default("t2-vlan")?;
    f.pci(
        "0000:00:06.0",
        "ena",
        0,
        &[irq(30, "ens6-Tx-Rx-0", "0", Some("0"))],
    )?
    .netdev("ens6", 3, Some(("pci", "0000:00:06.0")))?
    .netdev("ens5.100", 4, None)?
    .link("/sys/class/net/ens5.100/lower_ens5", "../ens5")?
    .netdev("bond0", 5, None)?
    .link("/sys/class/net/bond0/lower_ens5", "../ens5")?
    .link("/sys/class/net/bond0/lower_ens6", "../ens6")?
    .netdev("bond0.7", 6, None)?
    .link("/sys/class/net/bond0.7/lower_bond0", "../bond0")?;
    let hot = HotSet::new().with(2, Role::Loop);
    let direct = t2_nic_irqs(&f.host(), &Nic::Interface("ens5".into()), &hot);
    let vlan = t2_nic_irqs(&f.host(), &Nic::Interface("ens5.100".into()), &hot);
    assert_eq!(findings(&vlan).len(), 2, "{vlan:#?}");
    assert_eq!(
        vlan.iter().map(|f| &f.message).collect::<Vec<_>>(),
        direct.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
    let irqs = nic_irqs(&f.host(), &Nic::Interface("bond0.7".into()))?;
    assert_eq!(
        irqs.iter().map(|i| i.irq).collect::<Vec<_>>(),
        [24, 25, 26, 27, 28, 30]
    );
    // A bridge of veths (no NIC below it) stays unknown.
    f.netdev("veth1", 8, None)?
        .netdev("br0", 9, None)?
        .link("/sys/class/net/br0/lower_veth1", "../veth1")?;
    let v = t2_nic_irqs(&f.host(), &Nic::Interface("br0".into()), &hot);
    assert!(
        matches!(&v[0].status, Status::Unknown(r) if r.contains("no device")),
        "{v:#?}"
    );
    Ok(())
}

#[test]
fn t2_takes_a_dpdk_port_by_pci_address() -> R {
    let f = c7i_tuned("t2-dpdk")?;
    f.pci(
        "0000:00:07.0",
        "igb_uio",
        -1,
        &[irq(90, "igb_uio", "0-7", Some("0"))],
    )?;
    let v = t2_nic_irqs(
        &f.host(),
        &Nic::Pci("0000:00:07.0".into()),
        &HotSet::new().with(2, Role::Sender),
    );
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v:#?}");
    assert!(
        found[0].message.contains("IRQ 90 (igb_uio"),
        "{}",
        found[0].message
    );
    assert!(
        found[0]
            .message
            .contains("echo 0,4 > /proc/irq/90/smp_affinity_list"),
        "{}",
        found[0].message
    );
    Ok(())
}

#[test]
fn t2_is_unknown_for_interfaces_it_cannot_map() -> R {
    let hot = HotSet::new().with(2, Role::Loop);
    // A pod's veth: no device.
    let f = pod("t2-pod")?;
    let v = t2_nic_irqs(&f.host(), &Nic::Interface("eth0".into()), &hot);
    assert_eq!(v.len(), 1, "{v:#?}");
    assert!(
        matches!(&v[0].status, Status::Unknown(r) if r.contains("no device")),
        "{v:#?}"
    );
    // Not in this namespace, or not an interface at all.
    f.file("/sys/class/net/bonding_masters", "\n")?;
    for name in ["ens9", "bonding_masters", "../../devices", ""] {
        let v = t2_nic_irqs(&f.host(), &Nic::Interface(name.into()), &hot);
        assert!(v.len() == 1 && v[0].is_unknown(), "{name}: {v:#?}");
    }
    // Azure without a VF: a VMBus device only.
    let f = azure("t2-novf")?;
    let v = t2_nic_irqs(&f.host(), &Nic::Interface("eth3".into()), &hot);
    assert!(
        matches!(&v[0].status, Status::Unknown(r) if r.contains("no PCI device") && r.contains("VMBus")),
        "{v:#?}"
    );
    // A virtio NIC (KVM): its PCI device is the virtio device's parent,
    // which the check does not follow.
    f.dir("/sys/bus/virtio/devices/virtio0")?
        .link(
            "/sys/bus/virtio/devices/virtio0/subsystem",
            "../../../../bus/virtio",
        )?
        .netdev("eth9", 9, Some(("virtio", "virtio0")))?;
    let v = t2_nic_irqs(&f.host(), &Nic::Interface("eth9".into()), &hot);
    assert!(
        matches!(&v[0].status, Status::Unknown(r) if r.contains("virtio") && !r.contains("VMBus")),
        "{v:#?}"
    );
    // One IRQ's affinity unreadable: unknown for it, the others still checked.
    let f = debian_default("t2-unreadable")?;
    f.remove("/proc/irq/27/smp_affinity_list")?;
    let v = t2_nic_irqs(&f.host(), &Nic::Interface("ens5".into()), &hot);
    assert_eq!(unknowns(&v).len(), 1, "{v:#?}");
    assert_eq!(findings(&v).len(), 1, "{v:#?}");
    Ok(())
}

#[test]
fn t2_says_so_when_a_device_has_no_interrupts() -> R {
    // A DPDK port whose interrupts its process has not enabled (vfio-pci):
    // no msi_irqs and no legacy IRQ with a handler.
    let f = c7i_tuned("t2-noirq")?;
    f.pci("0000:00:08.0", "vfio-pci", -1, &[])?;
    let v = t2_nic_irqs(
        &f.host(),
        &Nic::Pci("0000:00:08.0".into()),
        &HotSet::new().with(2, Role::Sender),
    );
    assert_only_ok(&v, CheckId::T2);
    assert!(v[0].message.contains("has no interrupts"), "{v:#?}");
    assert!(!v[0].message.contains("()"), "{v:#?}");
    Ok(())
}

// ---------------------------------------------------------------- T3

#[test]
fn t3_is_ok_with_every_size_off() -> R {
    let f = c9g_tuned("t3-ok")?;
    assert_only_ok(&t3_thp(&f.host(), Component::Md), CheckId::T3);
    // A size only shmem can use (order 1, Linux 6.12 on arm64): no
    // `enabled`, only `shmem_enabled`.
    f.file(
        "/sys/kernel/mm/transparent_hugepage/hugepages-8kB/shmem_enabled",
        "always inherit within_size advise [never] deny force\n",
    )?;
    assert_only_ok(&t3_thp(&f.host(), Component::Md), CheckId::T3);
    Ok(())
}

#[test]
fn t3_warns_for_aeron_processes_and_informs_for_ergon_io() -> R {
    let f = debian_default("t3-always")?;
    for (component, severity) in [
        (Component::Md, Severity::Warn),
        (Component::Rt, Severity::Warn),
        (Component::Io, Severity::Info),
    ] {
        let v = t3_thp(&f.host(), component);
        assert_eq!(v.len(), 1, "{v:#?}");
        assert!(v[0].is_finding());
        assert_eq!(v[0].severity, severity, "{component:?}");
        assert!(v[0].message.contains("`always`"), "{}", v[0].message);
        // The md-dpdk THP figures have no archived data: the finding
        // says so instead of quoting them.
        assert!(
            v[0].message.contains("not measured with archived data"),
            "{}",
            v[0].message
        );
        assert!(!v[0].message.contains("1,148"), "{}", v[0].message);
        assert!(
            v[0].message
                .contains("echo never > /sys/kernel/mm/transparent_hugepage/enabled"),
            "{}",
            v[0].message
        );
    }
    Ok(())
}

#[test]
fn t3_says_madvise_was_not_measured_on_its_own() -> R {
    let f = debian_default("t3-madvise")?;
    f.thp("madvise", &[("2048", "inherit")])?;
    let v = t3_thp(&f.host(), Component::Md);
    assert!(v[0].is_finding());
    assert!(v[0].message.contains("`madvise`"), "{}", v[0].message);
    assert!(v[0].message.contains("not measured"), "{}", v[0].message);
    Ok(())
}

#[test]
fn t3_finds_an_mthp_size_turned_on_when_the_top_level_is_off() -> R {
    let f = c9g_tuned("t3-mthp")?;
    f.file(
        "/sys/kernel/mm/transparent_hugepage/hugepages-64kB/enabled",
        "[always] inherit madvise never\n",
    )?;
    let v = t3_thp(&f.host(), Component::Rt);
    assert_eq!(findings(&v).len(), 1, "{v:#?}");
    assert!(
        v[0].message
            .contains("echo never > /sys/kernel/mm/transparent_hugepage/hugepages-64kB/enabled"),
        "{}",
        v[0].message
    );
    // `madvise` at one size counts too (khugepaged still runs under it).
    f.file(
        "/sys/kernel/mm/transparent_hugepage/hugepages-64kB/enabled",
        "always inherit [madvise] never\n",
    )?;
    let v = t3_thp(&f.host(), Component::Rt);
    assert_eq!(findings(&v).len(), 1, "{v:#?}");
    assert!(v[0].message.contains("hugepages-64kB `madvise`"), "{v:#?}");
    // `inherit` under a top level of `never` is off.
    f.file(
        "/sys/kernel/mm/transparent_hugepage/hugepages-64kB/enabled",
        "always [inherit] madvise never\n",
    )?;
    assert_only_ok(&t3_thp(&f.host(), Component::Rt), CheckId::T3);
    Ok(())
}

#[test]
fn t3_without_thp_and_unreadable() -> R {
    let f = c9g_tuned("t3-nothp")?;
    f.remove("/sys/kernel/mm/transparent_hugepage")?;
    assert_only_ok(&t3_thp(&f.host(), Component::Md), CheckId::T3);
    f.remove("/sys/kernel/mm")?;
    let v = t3_thp(&f.host(), Component::Md);
    assert!(v.len() == 1 && v[0].is_unknown(), "{v:#?}");
    let f = c9g_tuned("t3-malformed")?;
    f.file(
        "/sys/kernel/mm/transparent_hugepage/enabled",
        "always madvise never\n",
    )?;
    let v = t3_thp(&f.host(), Component::Md);
    assert!(v.len() == 1 && v[0].is_unknown(), "{v:#?}");
    Ok(())
}

// ---------------------------------------------------------------- T4

#[test]
fn t4_accepts_memory_filesystems_and_warns_for_disk_ones() {
    let dir = Path::new("/dev/shm/aeron");
    for magic in [0x0102_1994, 0x9584_58f6, 0x8584_58f6] {
        let v = t4_from(dir, Ok(FsMagic(magic)));
        assert_eq!(v.status, Status::Ok, "{magic:#x}: {v:?}");
    }
    let v = t4_from(Path::new("/var/lib/aeron"), Ok(FsMagic(0xEF53)));
    assert!(v.is_warning(), "{v:?}");
    assert!(v.message.contains("/var/lib/aeron"), "{}", v.message);
    assert!(v.message.contains("ext4"), "{}", v.message);
    assert!(v.message.contains("/dev/shm"), "{}", v.message);
    let v = t4_from(dir, Ok(FsMagic(0x794c_7630)));
    assert!(v.is_warning() && v.message.contains("overlayfs"), "{v:?}");
    let v = t4_from(dir, Ok(FsMagic(0x1234_5678)));
    assert!(v.is_warning() && v.message.contains("0x12345678"), "{v:?}");
}

#[test]
fn t4_is_unknown_when_statfs_cannot_answer() -> R {
    let v = t4_from(Path::new("/aeron"), Err(ReadError::NotLinux("statfs")));
    assert!(v.is_unknown(), "{v:?}");
    // A fixture tree is not the running host: statfs would describe the
    // test machine's disk, so it is not asked.
    let f = c9g_tuned("t4-fixture")?;
    let v = t4_aeron_dir(&f.host(), Path::new("/dev/shm/aeron"));
    assert!(v.is_unknown(), "{v:?}");
    Ok(())
}

// ---------------------------------------------------------------- T5

#[test]
fn t5_applies_only_to_multi_node_hosts() -> R {
    let f = c9g_tuned("t5-single")?;
    f.file("/proc/sys/kernel/numa_balancing", "1\n")?;
    let v = t5_numa_balancing(&f.host());
    assert_eq!(v.status, Status::Ok, "{v:?}");
    assert!(v.message.contains("one NUMA node"), "{}", v.message);
    let f = pod("t5-pod")?;
    let v = t5_numa_balancing(&f.host());
    assert!(v.is_warning(), "{v:?}");
    assert!(
        v.message.contains("sysctl -w kernel.numa_balancing=0"),
        "{}",
        v.message
    );
    f.file("/proc/sys/kernel/numa_balancing", "0\n")?;
    assert_eq!(t5_numa_balancing(&f.host()).status, Status::Ok);
    Ok(())
}

#[test]
fn t5_without_numa_balancing_or_numa_and_unreadable() -> R {
    let f = pod("t5-nofile")?;
    f.remove("/proc/sys/kernel/numa_balancing")?;
    assert_eq!(t5_numa_balancing(&f.host()).status, Status::Ok);
    f.remove("/proc/sys")?;
    assert!(t5_numa_balancing(&f.host()).is_unknown());
    let f = c9g_tuned("t5-nonuma")?;
    f.remove("/sys/devices/system/node")?;
    assert_eq!(t5_numa_balancing(&f.host()).status, Status::Ok);
    Ok(())
}

// ---------------------------------------------------------------- T6

#[test]
fn t6_warns_when_the_nic_is_on_another_node() -> R {
    let f = pod("t6-remote")?;
    f.pci("0000:00:06.0", "igb_uio", 1, &[])?;
    let hot = HotSet::new().with(2, Role::Sender).with(3, Role::Receiver);
    let v = t6_nic_node(&f.host(), &Nic::Pci("0000:00:06.0".into()), &hot);
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v:#?}");
    assert!(found[0].is_warning());
    assert!(
        found[0].message.contains("0000:00:06.0 is on node 1"),
        "{}",
        found[0].message
    );
    assert!(
        found[0].message.contains("CPU 2 (sender)"),
        "{}",
        found[0].message
    );
    assert!(found[0].message.contains("4-7"), "{}", found[0].message);
    f.file("/sys/bus/pci/devices/0000:00:06.0/numa_node", "0\n")?;
    assert_only_ok(
        &t6_nic_node(&f.host(), &Nic::Pci("0000:00:06.0".into()), &hot),
        CheckId::T6,
    );
    f.file("/sys/bus/pci/devices/0000:00:06.0/numa_node", "-1\n")?;
    let v = t6_nic_node(&f.host(), &Nic::Pci("0000:00:06.0".into()), &hot);
    assert!(v.len() == 1 && v[0].is_unknown(), "{v:#?}");
    Ok(())
}

#[test]
fn t6_does_not_apply_on_one_node() -> R {
    let f = c9g_tuned("t6-single")?;
    let v = t6_nic_node(
        &f.host(),
        &Nic::Interface("ens6".into()),
        &HotSet::new().with(3, Role::Loop),
    );
    assert_only_ok(&v, CheckId::T6);
    assert!(v[0].message.contains("one NUMA node"), "{v:#?}");
    Ok(())
}

// ---------------------------------------------------------------- T7

#[test]
fn t7_reads_the_policy_past_a_command_name_with_spaces_and_parentheses() -> R {
    let f = c9g_tuned("t7-ok")?;
    assert_eq!(
        thread_policy(&f.host(), Thread::Current)?,
        SchedPolicy::Other
    );
    assert_eq!(
        thread_affinity(&f.host(), Thread::Current)?,
        CpuSet::of([3])
    );
    let v = t7_sched_policy(&f.host(), Thread::Current, Role::Loop);
    assert_eq!(v.status, Status::Ok, "{v:?}");
    // A field read one off would give rt_priority (0, SCHED_OTHER) here.
    f.thread(1, "3")?;
    assert_eq!(
        thread_policy(&f.host(), Thread::Current)?,
        SchedPolicy::Fifo
    );
    Ok(())
}

#[test]
fn t7_warns_for_fifo_and_round_robin() -> R {
    for (policy, name) in [(1, "SCHED_FIFO"), (2, "SCHED_RR")] {
        let f = c7i_tuned(&format!("t7-{policy}"))?;
        f.thread(policy, "2")?
            .file("/proc/sys/kernel/sched_rt_runtime_us", "950000\n")?;
        let v = t7_sched_policy(&f.host(), Thread::Current, Role::Loop);
        assert!(v.is_warning(), "{v:?}");
        assert!(v.message.contains(name), "{}", v.message);
        assert!(v.message.contains("533–947 local-timer"), "{}", v.message);
        assert!(v.message.contains("950000"), "{}", v.message);
    }
    Ok(())
}

#[test]
fn t7_reads_another_thread_by_id_and_is_unknown_when_unreadable() -> R {
    let f = c7i_tuned("t7-task")?;
    f.thread(6, "2")?;
    let stat = std::fs::read_to_string(f.root().join("proc/thread-self/stat"))?;
    f.file("/proc/self/task/4243/stat", &stat)?.thread(0, "2")?;
    assert_eq!(
        thread_policy(&f.host(), Thread::Task(4243))?,
        SchedPolicy::Deadline
    );
    assert_eq!(
        thread_policy(&f.host(), Thread::Current)?,
        SchedPolicy::Other
    );
    assert!(t7_sched_policy(&f.host(), Thread::Task(4244), Role::Sender).is_unknown());
    f.file("/proc/thread-self/stat", "4242 (loop) S 1 2 3\n")?;
    assert!(t7_sched_policy(&f.host(), Thread::Current, Role::Loop).is_unknown());
    f.file("/proc/thread-self/stat", "4242 loop S 1 2 3\n")?;
    assert!(thread_policy(&f.host(), Thread::Current).is_err());
    Ok(())
}

// ---------------------------------------------------------------- T8

#[test]
fn t8_never_fires_without_smt() -> R {
    let f = c9g_tuned("t8-graviton")?;
    let hot = HotSet::new().with(2, Role::Sender).with(3, Role::Receiver);
    let v = t8_smt_siblings(&f.host(), &hot);
    assert_only_ok(&v, CheckId::T8);
    assert!(v[0].message.contains("Graviton"), "{v:#?}");
    Ok(())
}

#[test]
fn t8_is_ok_when_siblings_are_isolated_and_idle() -> R {
    let f = c7i_tuned("t8-ok")?;
    let hot = HotSet::new().with(2, Role::Sender).with(3, Role::Receiver);
    assert_only_ok(&t8_smt_siblings(&f.host(), &hot), CheckId::T8);
    Ok(())
}

#[test]
fn t8_warns_once_for_two_busy_threads_on_one_core() -> R {
    let f = c7i_tuned("t8-pair")?;
    let hot = HotSet::new().with(2, Role::Sender).with(6, Role::Receiver);
    let v = t8_smt_siblings(&f.host(), &hot);
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v:#?}");
    assert!(found[0].is_warning());
    assert!(
        found[0]
            .message
            .contains("CPU 2 (sender) and CPU 6 (receiver) are hyperthreads of one core"),
        "{}",
        found[0].message
    );
    // The conductor on the sibling counts as another of the component's.
    let hot = HotSet::new().with(2, Role::Loop).with(6, Role::Conductor);
    assert!(
        t8_smt_siblings(&f.host(), &hot)
            .iter()
            .any(Finding::is_warning)
    );
    // So do two busy threads on one CPU.
    let hot = HotSet::new().with(2, Role::Sender).with(2, Role::Receiver);
    let v = t8_smt_siblings(&f.host(), &hot);
    assert!(
        v.iter()
            .any(|f| f.is_warning() && f.message.contains("both on CPU 2")),
        "{v:#?}"
    );
    Ok(())
}

#[test]
fn t8_allows_sqpoll_on_the_loops_sibling() -> R {
    let f = c7i_tuned("t8-sqpoll")?;
    let hot = HotSet::new().with(2, Role::Loop).with(6, Role::SqPoll);
    assert_only_ok(&t8_smt_siblings(&f.host(), &hot), CheckId::T8);
    Ok(())
}

#[test]
fn t8_informs_when_the_sibling_is_housekeeping_or_not_isolated() -> R {
    let f = c7i_tuned("t8-info")?;
    // The runtime's loop with a background CPU on its sibling.
    let hot = HotSet::new()
        .with(1, Role::Loop)
        .with(5, Role::Housekeeping);
    let v = t8_smt_siblings(&f.host(), &hot);
    assert_eq!(findings(&v).len(), 1, "{v:#?}");
    assert_eq!(findings(&v)[0].severity, Severity::Info);
    // A sibling outside isolation (Azure CPU 0 is housekeeping).
    let f = azure("t8-azure")?;
    let v = t8_smt_siblings(&f.host(), &HotSet::new().with(1, Role::Loop));
    assert_eq!(findings(&v).len(), 1, "{v:#?}");
    assert_eq!(findings(&v)[0].severity, Severity::Info);
    assert!(findings(&v)[0].message.contains("CPU 0"), "{v:#?}");
    Ok(())
}

#[test]
fn t8_is_unknown_without_topology() -> R {
    let f = c7i_tuned("t8-unknown")?;
    f.remove("/sys/devices/system/cpu/cpu2/topology")?;
    let v = t8_smt_siblings(&f.host(), &HotSet::new().with(2, Role::Loop));
    assert!(v.len() == 1 && v[0].is_unknown(), "{v:#?}");
    Ok(())
}

// ---------------------------------------------------------------- readers

#[test]
fn platform_detection_from_dmi_and_cpuinfo() -> R {
    let p = Platform::detect(&c9g_tuned("p-c9g")?.host());
    assert_eq!((p.arch, p.cloud), (Arch::Aarch64, Cloud::Aws));
    assert!(p.is_graviton());
    assert_eq!(p.product.as_deref(), Some("c9g.2xlarge"));
    let p = Platform::detect(&azure("p-azure")?.host());
    assert_eq!((p.arch, p.cloud), (Arch::X86_64, Cloud::Azure));
    assert!(!p.is_graviton());
    let p = Platform::detect(&Fixture::empty("p-empty")?.host());
    assert_eq!(
        (p.arch, p.cloud, p.product),
        (Arch::Unknown, Cloud::Other, None)
    );
    Ok(())
}

#[test]
fn busy_poll_sysctls_are_unknown_in_a_pods_namespace() -> R {
    let b = busy_poll_sysctls(&azure("bp-azure")?.host())?;
    assert_eq!((b.busy_read, b.busy_poll), (50, 50));
    assert!(busy_poll_sysctls(&pod("bp-pod")?.host()).is_err());
    Ok(())
}

#[test]
fn carrier_changes_reads_the_counter() -> R {
    let f = c9g_tuned("carrier")?;
    assert_eq!(carrier_changes(&f.host(), "ens6")?, 2);
    f.file("/sys/class/net/ens6/carrier_changes", "4\n")?;
    assert_eq!(carrier_changes(&f.host(), "ens6")?, 4);
    assert!(carrier_changes(&f.host(), "ens9").is_err());
    Ok(())
}

// ANCHOR: one_line
#[test]
fn findings_print_as_one_line_each() {
    let f = t4_from(Path::new("/var/lib/aeron"), Ok(FsMagic(0xEF53)));
    let line = f.to_string();
    assert!(line.starts_with("T4 warn: "), "{line}");
    assert!(line.contains("(evidence: "), "{line}");
    assert!(!line.contains('\n'), "{line}");
    let u = t4_from(Path::new("/aeron"), Err(ReadError::NotLinux("statfs")));
    assert!(u.to_string().starts_with("T4 unknown: "), "{u}");
}
// ANCHOR_END: one_line

// ---------------------------------------------------------------- T10

#[test]
fn t10_is_ok_on_graviton() -> R {
    let f = c9g_tuned("t10-c9g")?;
    let v = t10_smt_on(&f.host());
    assert!(v.id == CheckId::T10 && v.status == Status::Ok, "{v:#?}");
    assert!(v.message.contains("Graviton"), "{}", v.message);
    Ok(())
}

#[test]
fn t10_is_ok_when_smt_is_off_on_x86() -> R {
    let f = c7i_tuned("t10-off")?;
    f.file("/sys/devices/system/cpu/smt/active", "0\n")?;
    let v = t10_smt_on(&f.host());
    assert!(v.id == CheckId::T10 && v.status == Status::Ok, "{v:#?}");
    assert!(v.message.contains("SMT is off"), "{}", v.message);
    Ok(())
}

#[test]
fn t10_warns_when_smt_is_on_and_names_every_way_to_turn_it_off() -> R {
    let f = c7i_tuned("t10-on")?;
    f.file("/sys/devices/system/cpu/smt/active", "1\n")?;
    let v = t10_smt_on(&f.host());
    assert!(v.is_warning(), "{v:#?}");
    let m = &v.message;
    // What it means in Kubernetes, with the measured numbers.
    assert!(m.contains("`cpu: 2` is one physical core"), "{m}");
    assert!(m.contains("a busy sibling slows the loop"), "{m}");
    assert!(
        m.contains("10 connections 7.5–8.4 / 29–33 / 96–349 against 5.2–6.0 / 20–24 / 77–109"),
        "{m}"
    );
    // EKS/EC2: the exact launch-template CpuOptions.
    assert!(
        m.contains("\"CpuOptions\": {\"CoreCount\": <the type's cores>, \"ThreadsPerCore\": 1}"),
        "{m}"
    );
    assert!(
        m.contains("not possible on T2, C7a, M7a, R7a, Mac or Graviton"),
        "{m}"
    );
    // Self-managed: the boot argument and the sysfs control.
    assert!(m.contains("boot with `nosmt`"), "{m}");
    assert!(m.contains("nosmt=force"), "{m}");
    assert!(
        m.contains("echo off > /sys/devices/system/cpu/smt/control"),
        "{m}"
    );
    assert!(m.contains("cpu_manager_state"), "{m}");
    // Azure: sizes without SMT (AMD Fasv6/Falsv6/Famsv6, Cobalt Dpsv6/Dplsv6).
    assert!(
        m.contains("Fasv6")
            && m.contains("Falsv6")
            && m.contains("Famsv6")
            && m.contains("Dpsv6")
            && m.contains("Dplsv6"),
        "{m}"
    );
    // The switch is on the printed line, not in the finding.
    assert!(!m.contains("ERGON_HOST_CHECKS"), "{m}");
    Ok(())
}

#[test]
fn t10_falls_back_to_the_sibling_lists() -> R {
    // c7i_tuned writes no smt/active and pairs c with c%4,c%4+4: SMT is
    // found from the thread_siblings_lists alone.
    let f = c7i_tuned("t10-fallback")?;
    let v = t10_smt_on(&f.host());
    assert!(v.is_warning(), "{v:#?}");
    // One thread per core, no smt/active: SMT is off.
    let g = Fixture::empty("t10-one-thread")?;
    g.platform("Intel", "m5.large", false)?
        .cpus(4, |c| c.to_string())?;
    let v = t10_smt_on(&g.host());
    assert!(v.id == CheckId::T10 && v.status == Status::Ok, "{v:#?}");
    Ok(())
}

#[test]
fn t10_is_unknown_without_smt_or_topology() -> R {
    // x86, but neither smt/active nor a thread_siblings_list to fall back on.
    let f = Fixture::empty("t10-empty")?;
    f.platform("Intel", "m5.large", false)?;
    let v = t10_smt_on(&f.host());
    assert!(v.is_unknown(), "{v:#?}");
    // A bad smt/active is unknown too, not a warning.
    let f = c7i_tuned("t10-bad")?;
    f.file("/sys/devices/system/cpu/smt/active", "sometimes\n")?;
    let v = t10_smt_on(&f.host());
    assert!(v.is_unknown(), "{v:#?}");
    // A host of no known architecture and no SMT data: unknown, never a
    // warning (a Linux x86 host always has one of the two files).
    let f = Fixture::empty("t10-noarch")?;
    let v = t10_smt_on(&f.host());
    assert!(v.is_unknown(), "{v:#?}");
    Ok(())
}
