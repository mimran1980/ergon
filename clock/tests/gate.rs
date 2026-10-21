//! Which hosts get the counter: the rule `calibrate` applies to what it
//! detects, here applied to hosts described by hand.
#![allow(missing_docs)]

use ergon_clock::advanced::{Arch, Fallback, HostFacts, OrderedRead, Os, Trust};

fn x86(clocksource: Option<&str>) -> HostFacts {
    HostFacts {
        clocksource: clocksource.map(str::to_owned),
        invariant_tsc: Some(true),
        rdtscp: Some(true),
        ..HostFacts::unknown(Arch::X86_64, Os::Linux)
    }
}

fn arm(os: Os, clocksource: Option<&str>) -> HostFacts {
    HostFacts {
        clocksource: clocksource.map(str::to_owned),
        register_hz: Some(1_000_000_000),
        ..HostFacts::unknown(Arch::Aarch64, os)
    }
}

/// The kernel keeps time with the counter (x86 `tsc`, Arm
/// `arch_sys_counter`) and, on x86, the TSC is invariant with `rdtscp`:
/// the counter, whichever trust is asked for.
#[test]
fn the_counter_where_the_kernel_keeps_time_with_it() {
    for trust in [Trust::Kernel, Trust::CrossCheck] {
        assert_eq!(x86(Some("tsc")).gate(trust), Ok(()), "{trust:?}");
        // as the kernel writes it: with a newline
        assert_eq!(x86(Some("tsc\n")).gate(trust), Ok(()), "{trust:?}");
        assert_eq!(
            arm(Os::Linux, Some("arch_sys_counter")).gate(trust),
            Ok(()),
            "{trust:?}"
        );
    }
}

/// A TSC that is not invariant (it changes rate or stops in deep C-states),
/// or one without `rdtscp` (the ordered read; a hypervisor can hide it), is
/// never the clock, whatever the kernel uses.
#[test]
fn an_x86_counter_needs_an_invariant_tsc_and_rdtscp() {
    for trust in [Trust::Kernel, Trust::CrossCheck] {
        let varying = HostFacts {
            invariant_tsc: Some(false),
            ..x86(Some("tsc"))
        };
        assert_eq!(varying.gate(trust), Err(Fallback::TscNotInvariant));
        let unknown = HostFacts {
            invariant_tsc: None,
            ..x86(Some("tsc"))
        };
        assert_eq!(unknown.gate(trust), Err(Fallback::TscNotInvariant));
        let hidden = HostFacts {
            rdtscp: Some(false),
            ..x86(Some("tsc"))
        };
        assert_eq!(hidden.gate(trust), Err(Fallback::NoRdtscp));
    }
}

/// The kernel keeping time with something that is not a counter (it judged
/// the TSC unstable, or never had one: `hpet`, `acpi_pm`, `jiffies`) is refused,
/// even with a cross-check: the kernel knows something about it we do not.
#[test]
fn a_kernel_that_distrusts_the_counter_is_believed() {
    for name in ["hpet", "acpi_pm", "jiffies", "refined-jiffies"] {
        for trust in [Trust::Kernel, Trust::CrossCheck] {
            assert_eq!(
                x86(Some(name)).gate(trust),
                Err(Fallback::KernelClocksource(name.to_owned())),
                "{name} {trust:?}"
            );
        }
    }
    // the TSC's name on Arm is no Arm counter
    assert_eq!(
        arm(Os::Linux, Some("tsc")).gate(Trust::CrossCheck),
        Err(Fallback::KernelClocksource("tsc".to_owned()))
    );
}

/// A hypervisor's clock (KVM's pvclock, Hyper-V's TSC page on Azure, Xen)
/// is itself the counter scaled by the hypervisor: the counter only when a
/// cross-check is asked for (`Trust::CrossCheck`), which calibrate then runs.
#[test]
fn a_paravirtual_clock_needs_the_cross_check() {
    for name in ["kvm-clock", "hyperv_clocksource_tsc_page", "xen"] {
        assert_eq!(
            x86(Some(name)).gate(Trust::Kernel),
            Err(Fallback::Paravirt(name.to_owned())),
            "{name}"
        );
        assert_eq!(x86(Some(name)).gate(Trust::CrossCheck), Ok(()), "{name}");
    }
}

/// No clock source to read (Linux without sysfs; macOS, which has no such
/// file): only with the cross-check.
#[test]
fn no_word_from_the_kernel_needs_the_cross_check() {
    assert_eq!(
        x86(None).gate(Trust::Kernel),
        Err(Fallback::ClocksourceUnreadable)
    );
    assert_eq!(x86(None).gate(Trust::CrossCheck), Ok(()));
    assert_eq!(
        arm(Os::MacOs, None).gate(Trust::Kernel),
        Err(Fallback::NoKernelVouch)
    );
    assert_eq!(arm(Os::MacOs, None).gate(Trust::CrossCheck), Ok(()));
    let intel_mac = HostFacts {
        os: Os::MacOs,
        ..x86(None)
    };
    assert_eq!(intel_mac.gate(Trust::Kernel), Err(Fallback::NoKernelVouch));
    assert_eq!(intel_mac.gate(Trust::CrossCheck), Ok(()));
}

/// An architecture whose counter this crate does not read.
#[test]
fn other_architectures_have_no_counter() {
    for trust in [Trust::Kernel, Trust::CrossCheck] {
        let other = HostFacts::unknown(Arch::Other, Os::Linux);
        assert_eq!(other.gate(trust), Err(Fallback::NoCounter));
    }
}

/// The ordered read follows the facts: `rdtscp` on x86-64; on Arm the
/// self-synchronised `CNTVCTSS_EL0` only where the operating system reports
/// `FEAT_ECV`, and `isb; mrs cntvct_el0` where it says no or nothing (an
/// instruction the CPU may lack is never chosen on a guess).
#[test]
fn the_ordered_read_follows_the_facts() {
    let with_ecv = |ecv| HostFacts {
        ecv,
        ..arm(Os::Linux, Some("arch_sys_counter"))
    };
    assert_eq!(with_ecv(Some(true)).ordered_read(), OrderedRead::Cntvctss);
    assert_eq!(with_ecv(Some(false)).ordered_read(), OrderedRead::IsbCntvct);
    assert_eq!(with_ecv(None).ordered_read(), OrderedRead::IsbCntvct);
    let mac = HostFacts {
        ecv: Some(true),
        ..arm(Os::MacOs, None)
    };
    assert_eq!(mac.ordered_read(), OrderedRead::Cntvctss);
    assert_eq!(x86(Some("tsc")).ordered_read(), OrderedRead::Rdtscp);
    // an x86 host's facts never carry ECV; were they to, still rdtscp
    let odd = HostFacts {
        ecv: Some(true),
        ..x86(Some("tsc"))
    };
    assert_eq!(odd.ordered_read(), OrderedRead::Rdtscp);
    assert_eq!(
        HostFacts::unknown(Arch::Other, Os::Linux).ordered_read(),
        OrderedRead::None
    );
    for (read, name) in [
        (OrderedRead::Rdtscp, "rdtscp"),
        (OrderedRead::IsbCntvct, "isb; mrs cntvct_el0"),
        (OrderedRead::Cntvctss, "mrs cntvctss_el0"),
        (OrderedRead::None, "none"),
    ] {
        assert_eq!(read.name(), name);
    }
}
