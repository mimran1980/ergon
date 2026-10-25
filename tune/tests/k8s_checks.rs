//! The Kubernetes checks K1-K7 against fixture trees: what each finds in a
//! pod, what it reports as fine, and what it cannot tell. Outside a pod
//! ([`in_pod`]) the group returns nothing.
#![allow(missing_docs)]

mod common;

use common::Fixture;
use ergon_tune::{
    Finding, HotSet, Needs, Nic, Role, Status, Thread, in_pod, k8s_checks, t7_sched_policy,
    t9_cpu_quota,
};

type R = Result<(), Box<dyn std::error::Error>>;

/// A pod: x86, 8 CPUs with SMT pairs 0-4, 1-5, 2-6, 3-7, and a cgroup at
/// `path`. `cpus` is the container's cpuset.
fn pod_at(
    name: &str,
    path: &str,
    cpus: Option<&str>,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let f = Fixture::empty(name)?;
    f.platform("Amazon EC2", "c7i.2xlarge", false)?
        .cpus(8, |c| format!("{},{}", c % 4, c % 4 + 4))?
        .thread(0, "2")?
        .cgroup_v2(path)?;
    if let Some(cpus) = cpus {
        let dir = format!("/sys/fs/cgroup{}", path.trim_end_matches('/'));
        f.file(
            &format!("{dir}/cpuset.cpus.effective"),
            &format!("{cpus}\n"),
        )?;
    }
    Ok(f)
}

fn hot() -> HotSet {
    HotSet::new().with(2, Role::Loop)
}

const fn needs() -> Needs {
    Needs::new(Thread::Current, Role::Loop)
}

fn by_id<'a>(v: &'a [Finding], id: &str) -> Result<&'a Finding, Box<dyn std::error::Error>> {
    v.iter()
        .find(|f| f.id == id)
        .ok_or_else(|| format!("{id} missing from {v:#?}").into())
}

const QOS_BURSTABLE: &str = "/kubepods/burstable/pod8f3c2a1b";

// ---------------------------------------------------------------- in_pod

#[test]
fn the_group_only_runs_in_a_pod() -> R {
    let f = pod_at("k-notpod", "/pod-alone", Some("2"))?;
    assert!(!in_pod(&f.host()), "a bare cgroup is not a pod");
    assert_eq!(k8s_checks(&f.host(), &hot(), &needs()), [] as [Finding; 0]);
    let f = pod_at("k-pod", QOS_BURSTABLE, Some("2"))?;
    assert!(in_pod(&f.host()), "a kubepods cgroup path is a pod");
    assert_ne!(k8s_checks(&f.host(), &hot(), &needs()), [] as [Finding; 0]);
    // The service-account mount counts too.
    let f = pod_at("k-sa", "/pod-alone", Some("2"))?;
    f.dir("/var/run/secrets/kubernetes.io/serviceaccount")?;
    assert!(in_pod(&f.host()));
    Ok(())
}

// ---------------------------------------------------------------- K1 QoS

#[test]
fn k1_is_ok_for_a_guaranteed_pod() -> R {
    let f = pod_at("k1-guaranteed", "/kubepods/pod8f3c2a1b", Some("2"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k1 = by_id(&v, "K1")?;
    assert_eq!(k1.status, Status::Ok, "{v:#?}");
    assert!(k1.message.contains("Guaranteed"), "{}", k1.message);
    // The systemd slice spelling of the same class.
    let f = pod_at(
        "k1-guaranteed-slice",
        "/kubepods.slice/kubepods-pod8f3c2a1b.slice",
        Some("2"),
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    assert_eq!(by_id(&v, "K1")?.status, Status::Ok, "{v:#?}");
    Ok(())
}

#[test]
fn k1_warns_for_burstable_and_besteffort() -> R {
    let f = pod_at("k1-burstable", QOS_BURSTABLE, Some("2"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k1 = by_id(&v, "K1")?;
    assert!(k1.is_warning(), "{v:#?}");
    let m = &k1.message;
    assert!(m.contains("this pod is Burstable"), "{m}");
    assert!(m.contains("Fix (pod spec):"), "{m}");
    assert!(m.contains("`resources.requests`"), "{m}");
    assert!(!m.contains("Fix (node)"), "{m}");
    let f = pod_at(
        "k1-besteffort",
        "/kubepods/besteffort/pod8f3c2a1b",
        Some("2"),
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    assert!(by_id(&v, "K1")?.is_warning(), "{v:#?}");
    assert!(
        by_id(&v, "K1")?.message.contains("BestEffort"),
        "{}",
        by_id(&v, "K1")?.message
    );
    Ok(())
}

#[test]
fn k1_is_unknown_when_the_cgroup_path_hides_the_pod() -> R {
    let f = pod_at("k1-namespaced", "/", Some("2"))?;
    // The service-account is missing and the path is namespaced: still a pod
    // only if the path says kubepods. Here it does not, so nothing runs.
    // Make it a pod by the mount, then the QoS cannot be told.
    assert_eq!(k8s_checks(&f.host(), &hot(), &needs()), [] as [Finding; 0]);
    f.dir("/var/run/secrets/kubernetes.io/serviceaccount")?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    assert!(by_id(&v, "K1")?.is_unknown(), "{v:#?}");
    Ok(())
}

// ------------------------------------------------------- K2 exclusive CPUs

#[test]
fn k2_warns_when_the_pod_has_the_shared_pool() -> R {
    let f = pod_at("k2-shared", QOS_BURSTABLE, Some("0-7"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k2 = by_id(&v, "K2")?;
    assert!(k2.is_warning(), "{v:#?}");
    let m = &k2.message;
    assert!(m.contains("shared pool"), "{m}");
    assert!(m.contains("Fix (node):"), "{m}");
    assert!(m.contains("cpuManagerPolicy: static"), "{m}");
    assert!(m.contains("full-pcpus-only"), "{m}");
    assert!(m.contains("Fix (pod spec):"), "{m}");
    Ok(())
}

#[test]
fn k2_is_ok_on_exclusive_cpus() -> R {
    let f = pod_at("k2-exclusive", QOS_BURSTABLE, Some("2-3,6-7"))?;
    // Exclusivity has to be evidenced: a proper subset of the node's CPUs is
    // also what a restricted shared pool looks like from inside the pod.
    f.file(
        &format!("/sys/fs/cgroup{QOS_BURSTABLE}/cpuset.cpus.partition"),
        "isolated\n",
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k2 = by_id(&v, "K2")?;
    assert_eq!(k2.status, Status::Ok, "{v:#?}");
    assert!(k2.message.contains("exclusive"), "{}", k2.message);
    assert!(k2.message.contains("partition=isolated"), "{}", k2.message);
    Ok(())
}

#[test]
fn k2_is_ok_when_the_exclusive_file_matches_the_cpuset() -> R {
    let f = pod_at("k2-exclusive-file", QOS_BURSTABLE, Some("2-3,6-7"))?;
    f.file(
        &format!("/sys/fs/cgroup{QOS_BURSTABLE}/cpuset.cpus.exclusive"),
        "2-3,6-7\n",
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k2 = by_id(&v, "K2")?;
    assert_eq!(k2.status, Status::Ok, "{v:#?}");
    assert!(k2.message.contains("exclusive"), "{}", k2.message);
    Ok(())
}

#[test]
fn k2_does_not_call_a_restricted_cpuset_exclusive_without_evidence() -> R {
    // A proper subset of the node's online CPUs, with nothing that says who
    // owns them: exactly what a restricted shared pool looks like.
    let f = pod_at("k2-unevidenced", QOS_BURSTABLE, Some("2-3,6-7"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k2 = by_id(&v, "K2")?;
    assert!(
        !matches!(k2.status, Status::Ok),
        "exclusivity must not be claimed without evidence: {v:#?}"
    );
    assert!(k2.is_unknown(), "{v:#?}");
    Ok(())
}

#[test]
fn k2_is_unknown_without_a_cpuset() -> R {
    let f = pod_at("k2-nocpuset", QOS_BURSTABLE, None)?;
    f.file("/sys/fs/cgroup/cpu.max", "max 100000\n")?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    assert!(by_id(&v, "K2")?.is_unknown(), "{v:#?}");
    Ok(())
}

// -------------------------------------------------- K3 a sibling outside

#[test]
fn k3_warns_when_the_smt_sibling_is_outside_the_pod() -> R {
    // The loop is on CPU 2; its sibling is CPU 6.
    let f = pod_at("k3-outside", QOS_BURSTABLE, Some("2-3"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k3 = by_id(&v, "K3")?;
    assert!(k3.is_warning(), "{v:#?}");
    let m = &k3.message;
    assert!(m.contains("CPU 2 (loop): CPU 6"), "{m}");
    assert!(m.contains("T10's figures"), "{m}");
    assert!(m.contains("Fix (node): turn SMT off"), "{m}");
    assert!(m.contains("Fix (pod spec):"), "{m}");
    Ok(())
}

#[test]
fn k3_is_ok_when_the_pod_owns_the_siblings() -> R {
    let f = pod_at("k3-inside", QOS_BURSTABLE, Some("2,6"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k3 = by_id(&v, "K3")?;
    assert_eq!(k3.status, Status::Ok, "{v:#?}");
    assert!(
        k3.message.contains("siblings are in this pod's cpuset"),
        "{}",
        k3.message
    );
    Ok(())
}

#[test]
fn k3_is_ok_without_smt() -> R {
    let f = pod_at("k3-nosmt", QOS_BURSTABLE, Some("2"))?;
    for c in 0..8 {
        f.file(
            &format!("/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list"),
            &format!("{c}\n"),
        )?;
    }
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k3 = by_id(&v, "K3")?;
    assert_eq!(k3.status, Status::Ok, "{v:#?}");
    assert!(
        k3.message.contains("no hot CPU has an SMT sibling"),
        "{}",
        k3.message
    );
    Ok(())
}

#[test]
fn k3_is_unknown_when_the_topology_is_unreadable() -> R {
    // A missing thread_siblings_list is not "SMT is off". Without it the
    // check cannot tell whether a sibling is in the pod.
    let f = pod_at("k3-notopo", QOS_BURSTABLE, Some("2"))?;
    f.remove("/sys/devices/system/cpu/cpu2/topology/thread_siblings_list")?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k3 = by_id(&v, "K3")?;
    assert!(
        !matches!(k3.status, Status::Ok),
        "unreadable topology must not pass: {v:#?}"
    );
    assert!(k3.is_unknown(), "{v:#?}");
    Ok(())
}

// ----------------------------------------------------- K4 CPU for helpers

#[test]
fn k4_warns_when_there_is_one_cpu_and_no_helpers_named() -> R {
    let f = pod_at("k4-one", QOS_BURSTABLE, Some("2"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k4 = by_id(&v, "K4")?;
    assert!(k4.is_warning(), "{v:#?}");
    let m = &k4.message;
    assert!(m.contains("one CPU (2)"), "{m}");
    assert!(m.contains("ERGON_SHARED_CPUS or ERGON_HELPER_CPUS"), "{m}");
    assert!(m.contains("Fix (pod spec):"), "{m}");
    Ok(())
}

#[test]
fn k4_is_ok_with_a_second_cpu() -> R {
    // 2 and 6 are SMT siblings, the book's `x86 with SMT, cpu: 2` row: the
    // spare is the loop's other hyperthread, so the message has to say the
    // helpers share that core rather than claim a whole core of their own.
    let f = pod_at("k4-two", QOS_BURSTABLE, Some("2,6"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k4 = by_id(&v, "K4")?;
    assert_eq!(k4.status, Status::Ok, "{v:#?}");
    let m = &k4.message;
    assert!(m.contains("for helpers"), "{m}");
    assert!(m.contains("SMT sibling"), "{m}");
    Ok(())
}

#[test]
fn k4_names_a_spare_on_another_core() -> R {
    // 2 and 3 are different cores (2's sibling is 6, 3's is 7), so the
    // helpers get a core of their own.
    let f = pod_at("k4-cores", QOS_BURSTABLE, Some("2,3"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k4 = by_id(&v, "K4")?;
    assert_eq!(k4.status, Status::Ok, "{v:#?}");
    let m = &k4.message;
    assert!(m.contains("for helpers"), "{m}");
    assert!(!m.contains("SMT sibling"), "{m}");
    Ok(())
}

#[test]
fn k4_warns_when_every_cpu_holds_a_spinning_role() -> R {
    // Two CPUs, but both busy-spin: a sender on 6 as well as the loop on 2.
    // The helpers have nowhere to go, and counting the CPUs is not enough.
    let f = pod_at("k4-full", QOS_BURSTABLE, Some("2,6"))?;
    let hot = HotSet::new().with(2, Role::Loop).with(6, Role::Sender);
    let v = k8s_checks(&f.host(), &hot, &needs());
    let k4 = by_id(&v, "K4")?;
    assert!(k4.is_warning(), "{v:#?}");
    let m = &k4.message;
    assert!(m.contains("sender"), "{m}");
    assert!(m.contains("ERGON_SHARED_CPUS or ERGON_HELPER_CPUS"), "{m}");
    assert!(m.contains("Fix (pod spec):"), "{m}");
    Ok(())
}

#[test]
fn k4_accepts_a_cpu_that_only_runs_non_spinning_roles() -> R {
    // The conductor does not busy-spin, so CPU 6 is still free for helpers.
    let f = pod_at("k4-conductor", QOS_BURSTABLE, Some("2,6"))?;
    let hot = HotSet::new().with(2, Role::Loop).with(6, Role::Conductor);
    let v = k8s_checks(&f.host(), &hot, &needs());
    let k4 = by_id(&v, "K4")?;
    assert_eq!(k4.status, Status::Ok, "{v:#?}");
    assert!(k4.message.contains("for helpers"), "{}", k4.message);
    Ok(())
}

// ----------------------------------------------------- K5 capabilities

#[test]
fn k5_warns_only_for_what_the_caller_uses() -> R {
    let f = pod_at("k5-caps", QOS_BURSTABLE, Some("2"))?;
    f.file(
        "/proc/self/status",
        "Name:\tergon\nCapEff:\t0000000000000000\n",
    )?;
    // Nothing needed: no warning.
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k5 = by_id(&v, "K5")?;
    assert_eq!(k5.status, Status::Ok, "{v:#?}");
    // Busy polling needs CAP_NET_ADMIN (bit 12).
    let v = k8s_checks(&f.host(), &hot(), &needs().busy_polls());
    let k5 = by_id(&v, "K5")?;
    assert!(k5.is_warning(), "{v:#?}");
    assert!(k5.message.contains("NET_ADMIN"), "{}", k5.message);
    assert!(
        k5.message.contains("securityContext.capabilities.add"),
        "{}",
        k5.message
    );
    assert!(!k5.message.contains("SYS_NICE"), "{}", k5.message);
    // The capability is there: no warning. Bit 12 = 0x1000.
    f.file(
        "/proc/self/status",
        "Name:\tergon\nCapEff:\t0000000000001000\n",
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs().busy_polls());
    assert_eq!(by_id(&v, "K5")?.status, Status::Ok, "{v:#?}");
    Ok(())
}

#[test]
fn k5_takes_an_unlimited_memlock_for_mlockall() -> R {
    let f = pod_at("k5-mlock", QOS_BURSTABLE, Some("2"))?;
    f.file(
        "/proc/self/status",
        "Name:\tergon\nCapEff:\t0000000000000000\n",
    )?;
    f.file(
        "/proc/self/limits",
        "Limit                     Soft Limit           Hard Limit           Units\n\
         Max locked memory         65536                65536                bytes\n",
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs().mlock());
    let k5 = by_id(&v, "K5")?;
    assert!(k5.is_warning(), "{v:#?}");
    assert!(k5.message.contains("IPC_LOCK"), "{}", k5.message);
    f.file(
        "/proc/self/limits",
        "Limit                     Soft Limit           Hard Limit           Units\n\
         Max locked memory         unlimited            unlimited            bytes\n",
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs().mlock());
    assert_eq!(by_id(&v, "K5")?.status, Status::Ok, "{v:#?}");
    Ok(())
}

// ----------------------------------------------------- K6 memory / pressure

#[test]
fn k6_is_ok_with_no_memory_high_and_no_pressure() -> R {
    let f = pod_at("k6-quiet", QOS_BURSTABLE, Some("2"))?;
    let dir = format!("/sys/fs/cgroup{QOS_BURSTABLE}");
    f.file(&format!("{dir}/memory.high"), "max\n")?;
    f.file(
        &format!("{dir}/cpu.pressure"),
        "some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k6 = by_id(&v, "K6")?;
    assert_eq!(k6.status, Status::Ok, "{v:#?}");
    assert!(k6.message.contains("no memory.high"), "{}", k6.message);
    Ok(())
}

#[test]
fn k6_warns_on_memory_high_and_on_pressure() -> R {
    let f = pod_at("k6-high", QOS_BURSTABLE, Some("2"))?;
    let dir = format!("/sys/fs/cgroup{QOS_BURSTABLE}");
    f.file(&format!("{dir}/memory.high"), "800000000\n")?;
    f.file(
        &format!("{dir}/memory.pressure"),
        "some avg10=2.50 avg60=1.00 avg300=0.10 total=98765\n",
    )?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k6 = by_id(&v, "K6")?;
    assert!(k6.is_warning(), "{v:#?}");
    let m = &k6.message;
    assert!(m.contains("memory.high is `800000000`"), "{m}");
    assert!(m.contains("memory.pressure some avg10 2.5"), "{m}");
    assert!(m.contains("Fix (pod spec):"), "{m}");
    assert!(m.contains("Fix (node):"), "{m}");
    Ok(())
}

#[test]
fn k6_is_unknown_when_there_is_no_pressure_evidence() -> R {
    // No memory.high and no PSI file is not "no pressure": it is a kernel or
    // a cgroup that will not say. The check must not report success.
    let f = pod_at("k6-nofiles", QOS_BURSTABLE, Some("2"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k6 = by_id(&v, "K6")?;
    assert!(
        !matches!(k6.status, Status::Ok),
        "missing pressure evidence must not pass: {v:#?}"
    );
    assert!(k6.is_unknown(), "{v:#?}");
    Ok(())
}

// --------------------------------------------------------------- K7 NUMA

#[test]
fn k7_is_ok_without_a_nic_and_on_the_nics_node() -> R {
    let f = pod_at("k7-nonic", QOS_BURSTABLE, Some("2"))?;
    f.numa(&["0-3", "4-7"], Some("1"))?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    let k7 = by_id(&v, "K7")?;
    assert_eq!(k7.status, Status::Ok, "{v:#?}");
    assert!(k7.message.contains("no NIC named"), "{}", k7.message);
    // The NIC on the CPUs' node.
    f.pci("0000:00:06.0", "igb_uio", 0, &[])?;
    let v = k8s_checks(
        &f.host(),
        &hot(),
        &needs().with_nic(Nic::Pci("0000:00:06.0".into())),
    );
    let k7 = by_id(&v, "K7")?;
    assert_eq!(k7.status, Status::Ok, "{v:#?}");
    assert!(k7.message.contains("(node 0)"), "{}", k7.message);
    Ok(())
}

#[test]
fn k7_warns_when_the_cpus_are_on_another_node() -> R {
    let f = pod_at("k7-remote", QOS_BURSTABLE, Some("4"))?;
    f.numa(&["0-3", "4-7"], Some("1"))?;
    f.pci("0000:00:06.0", "igb_uio", 0, &[])?;
    let v = k8s_checks(
        &f.host(),
        &hot(),
        &needs().with_nic(Nic::Pci("0000:00:06.0".into())),
    );
    let k7 = by_id(&v, "K7")?;
    assert!(k7.is_warning(), "{v:#?}");
    let m = &k7.message;
    assert!(m.contains("NIC"), "{m}");
    assert!(m.contains("node 0"), "{m}");
    assert!(m.contains("Fix (node):"), "{m}");
    assert!(m.contains("topologyManagerPolicy: single-numa-node"), "{m}");
    Ok(())
}

// --------------------------------------------- T7 and T9 are in the group

#[test]
fn the_group_reuses_t7_and_t9() -> R {
    let f = pod_at("k-reuse", QOS_BURSTABLE, Some("2"))?;
    let dir = format!("/sys/fs/cgroup{QOS_BURSTABLE}");
    f.file(&format!("{dir}/cpu.max"), "200000 100000\n")?;
    let v = k8s_checks(&f.host(), &hot(), &needs());
    assert_eq!(
        by_id(&v, "T9")?.message,
        t9_cpu_quota(&f.host()).message,
        "T9 runs in the group"
    );
    assert_eq!(
        by_id(&v, "T7")?.message,
        t7_sched_policy(&f.host(), Thread::Current, Role::Loop).message,
        "T7 runs in the group"
    );
    assert!(by_id(&v, "T9")?.is_warning(), "{v:#?}");
    Ok(())
}
