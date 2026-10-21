//! The raw counter reads, and the optional instructions behind proof
//! tokens: `rdtscp` (x86) and `CNTVCTSS_EL0` (Arm `FEAT_ECV`) exist exactly
//! where the operating system reports them.
#![allow(missing_docs)]

use ergon_clock::advanced::HostFacts;

/// Linux's own view of a CPU feature flag, from /proc/cpuinfo (x86's
/// `flags`, Arm's `Features`): an independent source to check detection
/// against. `None` off Linux.
#[cfg(target_os = "linux")]
fn cpuinfo_has(flag: &str) -> Option<bool> {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    let line = cpuinfo
        .lines()
        .find(|l| l.starts_with("flags") || l.starts_with("Features"))?;
    Some(line.split_whitespace().any(|f| f == flag))
}

#[cfg(target_arch = "x86_64")]
mod x86 {
    use ergon_clock::advanced::raw;

    /// `rdtscp` is offered exactly where CPUID reports it, and, on Linux,
    /// exactly where the kernel's flags list it.
    #[test]
    fn rdtscp_exactly_where_reported() {
        let token = raw::Rdtscp::detect();
        assert_eq!(
            token.is_some(),
            super::HostFacts::detect().rdtscp == Some(true)
        );
        #[cfg(target_os = "linux")]
        if let Some(listed) = super::cpuinfo_has("rdtscp") {
            assert_eq!(token.is_some(), listed);
        }
        if let Some(rdtscp) = token {
            let a = rdtscp.read();
            let b = raw::lfence_rdtsc();
            let c = rdtscp.read();
            assert!(a <= b && b <= c, "{a} {b} {c}");
        }
    }

    /// The unordered read advances.
    #[test]
    fn rdtsc_advances() {
        let a = raw::rdtsc();
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(raw::rdtsc() > a);
    }
}

#[cfg(target_arch = "aarch64")]
mod arm {
    use ergon_clock::advanced::raw;

    /// `FEAT_ECV`'s self-synchronised read, `CNTVCTSS_EL0`, is offered
    /// exactly where the operating system reports the feature (Linux
    /// `HWCAP2_ECV`, cross-checked with /proc/cpuinfo's `ecv`; macOS
    /// `hw.optional.arm.FEAT_ECV`), and reads the same counter.
    #[test]
    fn ecv_exactly_where_reported() {
        let token = raw::Ecv::detect();
        assert_eq!(
            token.is_some(),
            super::HostFacts::detect().ecv == Some(true)
        );
        #[cfg(target_os = "linux")]
        if let Some(listed) = super::cpuinfo_has("ecv") {
            assert_eq!(token.is_some(), listed);
        }
        if let Some(ecv) = token {
            let a = raw::isb_cntvct();
            let b = ecv.read();
            let c = raw::isb_cntvct();
            assert!(a <= b && b <= c, "{a} {b} {c}");
        }
    }

    /// The frequency register is set (firmware leaves 0 when it is not)
    /// and the unordered and ordered reads advance together.
    #[test]
    fn the_generic_timer_reads() {
        assert!(raw::cntfrq() >= 1_000, "CNTFRQ_EL0 {}", raw::cntfrq());
        let a = raw::cntvct();
        let b = raw::isb_cntvct();
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(raw::cntvct() > a && raw::isb_cntvct() > b);
    }
}

/// Whatever the architecture, the facts name it consistently with the
/// tokens above (none of them on other architectures).
#[test]
fn facts_and_tokens_agree() {
    let facts = HostFacts::detect();
    assert_eq!(facts.rdtscp.is_some(), cfg!(target_arch = "x86_64"));
    assert_eq!(facts.ecv.is_some(), cfg!(target_arch = "aarch64"));
}
