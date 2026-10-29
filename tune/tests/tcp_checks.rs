//! T21, TCP autocorking: fixture trees that fire it (on, as a warning for
//! a latency-sensitive TCP caller, information otherwise), find it fine (off,
//! or a kernel without it) and cannot read it.
#![allow(missing_docs)]

mod common;

use common::{Fixture, c7i_tuned};
use ergon_tune::{Severity, Status, t21_tcp_autocorking, unknown_stops_require};

type R = Result<(), Box<dyn std::error::Error>>;

const AUTOCORK: &str = "/proc/sys/net/ipv4/tcp_autocorking";

fn with(name: &str, value: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let f = c7i_tuned(name)?;
    f.file(AUTOCORK, value)?;
    Ok(f)
}

#[test]
fn t21_warns_while_autocorking_is_on_for_latency_sensitive_tcp() -> R {
    let f = with("t21-warn", "1\n")?;
    let v = t21_tcp_autocorking(&f.host(), true);
    assert_eq!(v.id, "T21");
    assert!(v.is_warning(), "{v}");
    for part in [
        "net.ipv4.tcp_autocorking is 1",
        "TCP_NODELAY",
        // the measured counts, uncapped and with the IRQ CPU at C1
        "TCPAutoCorking",
        "49k–164k",
        "42–149",
        // the fix, where it applies, and Kubernetes
        "sysctl -w net.ipv4.tcp_autocorking=0",
        "network namespace",
        "securityContext.sysctls",
        "--allowed-unsafe-sysctls",
        "hostNetwork",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    assert!(v.evidence.contains("tcp_should_autocork"), "{}", v.evidence);
    Ok(())
}

#[test]
fn t21_only_informs_a_caller_without_latency_sensitive_tcp() -> R {
    let f = with("t21-info", "1\n")?;
    let v = t21_tcp_autocorking(&f.host(), false);
    assert!(v.is_finding(), "{v}");
    assert_eq!(v.severity, Severity::Info, "{v}");
    Ok(())
}

#[test]
fn t21_is_ok_when_off_or_absent() -> R {
    let f = with("t21-off", "0\n")?;
    let v = t21_tcp_autocorking(&f.host(), true);
    assert_eq!(v.status, Status::Ok, "{v}");
    // A kernel before 3.14: no such sysctl, its directory there.
    f.remove(AUTOCORK)?;
    let v = t21_tcp_autocorking(&f.host(), true);
    assert_eq!(v.status, Status::Ok, "{v}");
    assert!(v.message.contains("no tcp_autocorking"), "{v}");
    Ok(())
}

#[test]
fn t21_is_unknown_when_unreadable_never_a_warning() -> R {
    let f = with("t21-bad", "yes\n")?;
    let v = t21_tcp_autocorking(&f.host(), true);
    assert!(v.is_unknown(), "{v}");
    // No /proc/sys/net/ipv4 at all.
    let f = c7i_tuned("t21-none")?;
    let v = t21_tcp_autocorking(&f.host(), true);
    assert!(v.is_unknown(), "{v}");
    // The network namespace's setting, not a property of this process's
    // CPUs: an unknown never stops a `require` start.
    assert!(!unknown_stops_require("T21"));
    Ok(())
}
