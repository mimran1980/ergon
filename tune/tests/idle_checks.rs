//! T20, the idle depth of the CPUs that take the data NICs' interrupts:
//! fixture trees that fire it (`intel_idle` with C1E and C6 enabled, an
//! `acpi_idle` C2), find it fine (capped at C1 by `disable`, by `pm_qos`, no
//! cpuidle driver), inform (`acpi_idle` whose deepest state is C1, a hot CPU
//! that is not busy-spinning) and cannot read it.
#![allow(missing_docs)]

mod common;

use common::{Fixture, INTEL_SPR, IdleState, c7i_tuned, c9g_tuned, debian_default};
use ergon_tune::{
    Finding, HotSet, Nic, Role, Severity, Status, T20_MAX_EXIT_LATENCY_US, t20_irq_idle,
    unknown_stops_require,
};

type R = Result<(), Box<dyn std::error::Error>>;

const ALL: [u32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

fn ens6() -> Vec<Nic> {
    vec![Nic::Interface("ens6".to_owned())]
}

fn loop_on(cpu: u32) -> HotSet {
    HotSet::new().with(cpu, Role::Loop)
}

/// A tuned c7i whose every CPU has `intel_idle`'s states, all enabled: the
/// data NIC's IRQs on CPU 0.
fn c7i_deep(name: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let f = c7i_tuned(name)?;
    f.cpuidle("intel_idle", &ALL, &INTEL_SPR)?;
    Ok(f)
}

fn on_cpu(found: &[Finding], cpu: u32) -> Vec<&Finding> {
    let tag = format!("CPU {cpu} ");
    found
        .iter()
        .filter(|f| f.message.starts_with(&tag))
        .collect()
}

#[test]
fn t20_warns_when_the_irq_cpu_may_enter_c1e_or_c6() -> R {
    let f = c7i_deep("t20-warn")?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    let warned: Vec<&Finding> = found.iter().filter(|v| v.is_warning()).collect();
    assert_eq!(warned.len(), 1, "{found:?}");
    let v = warned[0];
    assert_eq!(v.id, "T20");
    for part in [
        "CPU 0 ",
        "ens6-Tx-Rx-0",
        "intel_idle",
        "C1E (exit 2 µs)",
        "C6 (exit 190 µs)",
        // the measured effect, with the fix that removed it
        "p50 43–160 µs",
        "p99 315–559 µs",
        "37–39",
        // per-CPU fixes, the boot-time one, PM QoS
        "echo 1 > /sys/devices/system/cpu/cpu0/cpuidle/state2/disable",
        "echo 1 > /sys/devices/system/cpu/cpu0/cpuidle/state3/disable",
        "echo 1 > /sys/devices/system/cpu/cpu0/power/pm_qos_resume_latency_us",
        "intel_idle.max_cstate=1",
        "/dev/cpu_dma_latency",
        // Kubernetes: the node's setting
        "Kubernetes",
        "privileged",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    // acpi_idle's boot parameter is not offered on an intel_idle host.
    assert!(!v.message.contains("processor.max_cstate"), "{v}");
    // The busy-spinning loop's CPU never idles while it spins: no finding.
    assert!(on_cpu(&found, 2).is_empty(), "{found:?}");
    assert!(v.evidence.contains("aws-20261028-092158"), "{}", v.evidence);
    Ok(())
}

#[test]
fn t20_is_ok_when_the_irq_cpu_is_capped_at_c1() -> R {
    let f = c7i_deep("t20-capped")?;
    f.idle_disable(0, 2, true)?.idle_disable(0, 3, true)?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].status, Status::Ok, "{}", found[0]);
    assert!(found[0].message.contains("CPU 0"), "{}", found[0]);
    // Polling only (state1 off too) is fine as well.
    f.idle_disable(0, 1, true)?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert_eq!(found[0].status, Status::Ok, "{}", found[0]);
    Ok(())
}

#[test]
fn t20_counts_a_per_cpu_resume_latency_limit_as_the_cap() -> R {
    let f = c7i_deep("t20-qos")?;
    let qos = "/sys/devices/system/cpu/cpu0/power/pm_qos_resume_latency_us";
    // 1 µs: the governor keeps to states whose exit latency is at most 1 µs.
    f.file(qos, "1\n")?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert_eq!(found[0].status, Status::Ok, "{}", found[0]);
    assert!(
        found[0].message.contains("pm_qos_resume_latency_us"),
        "{}",
        found[0]
    );
    // n/a: no latency tolerated, polling only.
    f.file(qos, "n/a\n")?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert_eq!(found[0].status, Status::Ok, "{}", found[0]);
    // 0: no limit at all. 100 µs: C6 (190) is out of reach, C1E (2) is not.
    f.file(qos, "0\n")?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert!(found[0].is_warning(), "{}", found[0]);
    f.file(qos, "100\n")?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert!(found[0].is_warning(), "{}", found[0]);
    assert!(found[0].message.contains("C1E (exit 2 µs)"), "{}", found[0]);
    assert!(!found[0].message.contains("C6 (exit"), "{}", found[0]);
    Ok(())
}

#[test]
fn t20_warns_for_every_cpu_an_interrupt_lands_on() -> R {
    // ENA's queue i on CPU i: CPUs 0-3 take the queues' interrupts (their
    // effective affinity), the management IRQ's list (every CPU) aside.
    let f = debian_default("t20-spread")?;
    f.cpuidle("intel_idle", &ALL, &INTEL_SPR)?;
    let found = t20_irq_idle(
        &f.host(),
        &[Nic::Interface("ens5".to_owned())],
        &HotSet::new(),
    );
    let warned: Vec<u32> = (0..8)
        .filter(|c| on_cpu(&found, *c).iter().any(|v| v.is_warning()))
        .collect();
    assert_eq!(warned, vec![0, 1, 2, 3], "{found:?}");
    Ok(())
}

#[test]
fn t20_informs_for_a_hot_cpu_that_sleeps() -> R {
    // The media driver's conductor (not a busy-spinner) on CPU 3 may sleep
    // between duty cycles: information, not a warning.
    let f = c7i_deep("t20-conductor")?;
    f.idle_disable(0, 2, true)?.idle_disable(0, 3, true)?;
    let hot = loop_on(2).with(3, Role::Conductor);
    let found = t20_irq_idle(&f.host(), &ens6(), &hot);
    let v = on_cpu(&found, 3);
    assert_eq!(v.len(), 1, "{found:?}");
    assert_eq!(v[0].severity, Severity::Info, "{}", v[0]);
    assert!(v[0].is_finding(), "{}", v[0]);
    assert!(v[0].message.contains("conductor"), "{}", v[0]);
    assert!(!found.iter().any(Finding::is_warning), "{found:?}");
    Ok(())
}

const ACPI_C1: [IdleState; 2] = [("POLL", 0, false), ("C1", 1, false)];
const ACPI_C2: [IdleState; 3] = [("POLL", 0, false), ("C1", 1, false), ("C2", 18, false)];

#[test]
fn t20_informs_on_acpi_idle_whose_deepest_state_is_c1() -> R {
    let f = c7i_tuned("t20-acpi-c1")?;
    f.cpuidle("acpi_idle", &ALL, &ACPI_C1)?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert_eq!(found.len(), 1, "{found:?}");
    let v = &found[0];
    assert!(v.is_finding() && v.severity == Severity::Info, "{v}");
    for part in ["CPU 0 ", "acpi_idle", "C1", "nothing deeper to cap"] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    Ok(())
}

#[test]
fn t20_warns_on_acpi_idle_with_a_deeper_state() -> R {
    let f = c7i_tuned("t20-acpi-c2")?;
    f.cpuidle("acpi_idle", &ALL, &ACPI_C2)?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    let v = &found[0];
    assert!(v.is_warning(), "{v}");
    for part in [
        "C2 (exit 18 µs)",
        "processor.max_cstate=1",
        "echo 1 > /sys/devices/system/cpu/cpu0/cpuidle/state2/disable",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    assert!(!v.message.contains("intel_idle.max_cstate"), "{v}");
    Ok(())
}

#[test]
fn t20_is_ok_without_a_cpuidle_driver() -> R {
    // Graviton: Debian's cloud-arm64 kernel builds no cpuidle driver (WFI).
    let f = c9g_tuned("t20-none")?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(3));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].status, Status::Ok, "{}", found[0]);
    assert!(
        found[0].message.contains("no cpuidle driver"),
        "{}",
        found[0]
    );
    // A kernel without cpuidle at all: no directory.
    f.remove("/sys/devices/system/cpu/cpuidle")?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(3));
    assert_eq!(found[0].status, Status::Ok, "{}", found[0]);
    Ok(())
}

#[test]
fn t20_is_unknown_when_it_cannot_read_never_a_warning() -> R {
    // A state's exit latency that is not a number.
    let f = c7i_deep("t20-bad")?;
    f.file(
        "/sys/devices/system/cpu/cpu0/cpuidle/state3/latency",
        "lots\n",
    )?;
    let found = t20_irq_idle(&f.host(), &ens6(), &loop_on(2));
    assert!(found[0].is_unknown(), "{}", found[0]);
    assert!(!found.iter().any(Finding::is_warning), "{found:?}");
    // A NIC that is not there: unknown, and the hot CPUs still checked.
    let f = c7i_deep("t20-nonic")?;
    let hot = loop_on(2).with(3, Role::Conductor);
    let found = t20_irq_idle(&f.host(), &[Nic::Interface("eth9".to_owned())], &hot);
    assert!(found.iter().any(Finding::is_unknown), "{found:?}");
    assert!(
        on_cpu(&found, 3).iter().any(|v| v.is_finding()),
        "{found:?}"
    );
    // An unknown T20 never stops a `require` start: idle states are the
    // node's, not a property of this process.
    assert!(!unknown_stops_require("T20"));
    Ok(())
}

#[test]
fn t20_threshold_is_c1s_exit_latency() {
    // C1 (1 µs) is allowed; C1E (2 µs), the first state the lab's cap
    // disabled, is not.
    assert_eq!(T20_MAX_EXIT_LATENCY_US, 1);
}
