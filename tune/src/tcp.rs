//! T21: TCP autocorking (`net.ipv4.tcp_autocorking`), read in this
//! process's network namespace.

use crate::finding::{Finding, Severity};
use crate::host::{Host, ReadError, first_line};

const AUTOCORK: &str = "/proc/sys/net/ipv4/tcp_autocorking";

const T21_EVIDENCE: &str = "Linux net/ipv4/tcp.c tcp_should_autocork (a send of less than \
     size_goal is held while an earlier skb of the socket is still in the qdisc or the NIC's \
     queue, sk_wmem_alloc above one skb's truesize; TCP_NODELAY does not stop it) and \
     Documentation/networking/ip-sysctl.rst (tcp_autocorking); not measured here";

/// T21: TCP autocorking is off (`net.ipv4.tcp_autocorking` 0).
///
/// `latency_tcp`: the caller sends latency-sensitive TCP (a warning when on;
/// information otherwise). The sysctl is per network namespace, so this is
/// the namespace the process runs in (a pod's own, or the node's under
/// `hostNetwork`). Absent on kernels before 3.14: ok.
#[must_use]
pub fn t21_tcp_autocorking(host: &Host, latency_tcp: bool) -> Finding {
    let id = "T21";
    let sev = if latency_tcp {
        Severity::Warn
    } else {
        Severity::Info
    };
    let value = match host.read_optional(AUTOCORK) {
        Ok(None) => {
            return Finding::ok(
                id,
                sev,
                T21_EVIDENCE,
                "the kernel has no tcp_autocorking (before Linux 3.14): nothing corks".to_owned(),
            );
        }
        Ok(Some(t)) => {
            let line = first_line(&t);
            match line.parse::<u64>() {
                Ok(v) => v,
                Err(_) => {
                    return Finding::unknown(
                        id,
                        sev,
                        T21_EVIDENCE,
                        "cannot tell whether TCP autocorking is on".to_owned(),
                        ReadError::Malformed {
                            path: AUTOCORK.to_owned(),
                            detail: "not a number",
                        },
                    );
                }
            }
        }
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                T21_EVIDENCE,
                "cannot tell whether TCP autocorking is on".to_owned(),
                e,
            );
        }
    };
    if value == 0 {
        return Finding::ok(
            id,
            sev,
            T21_EVIDENCE,
            "net.ipv4.tcp_autocorking is 0: a small send goes out at once".to_owned(),
        );
    }
    Finding::found(
        id,
        sev,
        T21_EVIDENCE,
        format!(
            "net.ipv4.tcp_autocorking is {value}: a small send is held (corked) while the \
             socket's previous one is still queued in the qdisc or the NIC and not yet freed, \
             even with TCP_NODELAY, and goes out only when that one's transmit completes (a TX \
             interrupt, or the next send); with the IRQ CPU in a deep idle state (T20) that \
             completion waits for its wake. Fix: `sysctl -w net.ipv4.tcp_autocorking=0`. It is per network \
             namespace: set it in the namespace the process sends from. Kubernetes: a pod with \
             its own network sets it in securityContext.sysctls (name \
             net.ipv4.tcp_autocorking, value \"0\"), which the kubelet admits only when listed \
             in --allowed-unsafe-sysctls (or allowedUnsafeSysctls in its config); a \
             hostNetwork pod shares the node's namespace, so set it on the node"
        ),
    )
}
