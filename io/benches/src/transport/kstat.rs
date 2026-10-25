//! Kernel counters an end reads around its traffic, off the hot path.
//!
//! The network namespace's TCP counters (`/proc/net/snmp`,
//! `/proc/net/netstat`, what `nstat` shows) and the calling thread's
//! run-queue wait and involuntary switches. They tell a retransmit the
//! receiver already had (a tail-loss probe or RTO fired because an ACK came
//! late, e.g. while a thread waited for a CPU) from a segment really lost.
#![deny(missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::point::Error;
use super::sockopt::os;

/// The namespace's TCP counters recorded, by `nstat`'s names.
///
/// `Tcp*` from `/proc/net/snmp`, `TcpExt*` from `/proc/net/netstat`. Namespace-wide:
/// every TCP socket in it counts (both ends of a loopback run, a host's
/// ssh session).
pub const NETSTAT: [&str; 14] = [
    "TcpRetransSegs",
    "TcpExtTCPLossProbes",
    "TcpExtTCPLossProbeRecovery",
    "TcpExtTCPDSACKRecv",
    "TcpExtTCPDSACKOldSent",
    "TcpExtTCPTimeouts",
    "TcpExtTCPSpuriousRTOs",
    "TcpExtTCPLostRetransmit",
    "TcpExtTCPFastRetrans",
    "TcpExtTCPSlowStartRetrans",
    "TcpExtTCPSynRetrans",
    "TcpExtTCPRetransFail",
    "TcpExtListenOverflows",
    "TcpExtListenDrops",
];

/// [`NETSTAT`]'s values at one moment, in its order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Netstat(pub [u64; NETSTAT.len()]);

impl Netstat {
    /// Reads them now.
    ///
    /// # Errors
    /// A file cannot be read, or a counter is missing from it.
    pub fn read() -> Result<Self, Error> {
        let mut out = [None; NETSTAT.len()];
        for (file, prefix) in [("/proc/net/snmp", "Tcp"), ("/proc/net/netstat", "TcpExt")] {
            let text = std::fs::read_to_string(file).map_err(|e| os(file, e))?;
            parse(&text, prefix, &mut out);
        }
        let mut v = [0; NETSTAT.len()];
        for (i, got) in out.iter().enumerate() {
            v[i] = got.ok_or_else(|| os("netstat", format!("no {}", NETSTAT[i])))?;
        }
        Ok(Self(v))
    }

    /// Growth from `before` to `self`, by name.
    #[must_use]
    pub fn since(&self, before: &Self) -> [(&'static str, u64); NETSTAT.len()] {
        let mut out = [("", 0); NETSTAT.len()];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (NETSTAT[i], self.0[i].saturating_sub(before.0[i]));
        }
        out
    }
}

/// Fills `out` from one file's header/value line pairs (`Tcp: name ...`
/// then `Tcp: value ...`), for the group `prefix`.
fn parse(text: &str, prefix: &str, out: &mut [Option<u64>; NETSTAT.len()]) {
    let tag = format!("{prefix}:");
    let mut lines = text.lines();
    while let Some(head) = lines.next() {
        let Some(vals) = lines.next() else { break };
        let (Some(names), Some(values)) = (head.strip_prefix(&tag), vals.strip_prefix(&tag)) else {
            continue;
        };
        for (n, v) in names.split_whitespace().zip(values.split_whitespace()) {
            let full = format!("{prefix}{n}");
            if let (Some(i), Ok(v)) = (NETSTAT.iter().position(|k| *k == full), v.parse()) {
                out[i] = Some(v);
            }
        }
    }
}

/// The calling thread's scheduling counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sched {
    /// ns spent runnable but waiting for a CPU (`schedstat`'s second
    /// field, `run_delay`).
    pub run_delay_ns: u64,
    /// Times another task took its CPU (`nonvoluntary_ctxt_switches`).
    pub involuntary_switches: u64,
}

impl Sched {
    /// The calling thread's, now (`/proc/thread-self`).
    ///
    /// # Errors
    /// A file cannot be read or parsed.
    pub fn read() -> Result<Self, Error> {
        let file = "/proc/thread-self/schedstat";
        let s = std::fs::read_to_string(file).map_err(|e| os(file, e))?;
        let run_delay_ns = s
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| os(file, format!("unexpected: {s:?}")))?;
        let file = "/proc/thread-self/status";
        let s = std::fs::read_to_string(file).map_err(|e| os(file, e))?;
        let involuntary_switches = s
            .lines()
            .find_map(|l| l.strip_prefix("nonvoluntary_ctxt_switches:"))
            .and_then(|v| v.trim().parse().ok())
            .ok_or_else(|| os(file, "no nonvoluntary_ctxt_switches"))?;
        Ok(Self {
            run_delay_ns,
            involuntary_switches,
        })
    }

    /// Growth from `before`.
    #[must_use]
    pub const fn since(&self, before: &Self) -> Self {
        Self {
            run_delay_ns: self.run_delay_ns.saturating_sub(before.run_delay_ns),
            involuntary_switches: self
                .involuntary_switches
                .saturating_sub(before.involuntary_switches),
        }
    }
}
