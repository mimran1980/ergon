//! The `ll-latency` report: lines a script can split.
//!
//! It has the same fields as the lab's `rtt` result line (p50, p90, p99,
//! p99.9, p99.99, max, mean), so a sample run reads like a `BENCHMARKS.md`
//! cell.
//!
//! Fields are tab-separated (two spaces here):
//!
//! ```text
//! ll-latency  v1  role=ping  arch=aarch64  cpu_build=generic  counter_hz=1000000000  transport=dpdk-ena  overlay=vdev-local  msg_bytes=32  mode=closed  warmup=100000  samples=1000000  lost=0
//! ll-latency  rtt_ns  p50=…  p90=…  p99=…  p99.9=…  p99.99=…  max=…  mean=…
//! ll-latency  rtt_max_top10  <realtime_ns>:<rtt_ns>,…
//! ll-latency  loop_gap_ns  max=…  over_5us=…  over_20us=…  over_100us=…
//! ll-latency  hot  cpu=…  ran_on=…  voluntary_ctxt_switches=…  nonvoluntary_ctxt_switches=…
//! ll-latency  log  dropped_lines=…  hot_warn_events=…  suppressed=…
//! ```
//!
//! The pong prints every line but the two `rtt_*` ones: it measures no
//! round trip. A run proves the tuning only together with an `ll-verify`
//! `RESULT PASS` from the same window.

use std::fmt::Write as _;

use crate::stats::{LoopGap, Percentiles};

/// What ran.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunInfo {
    /// `ping` or `pong`.
    pub role: String,
    /// `std::env::consts::ARCH`.
    pub arch: String,
    /// The image's CPU build (`LL_CPU_BUILD`: `generic`, `neoverse-v3`…).
    pub cpu_build: String,
    /// The counter rate behind the clock, Hz (`0`: not known).
    pub counter_hz: u64,
    /// `RUSTERON_MEDIA_DRIVER_TRANSPORT` as the app saw it, `-` when unset.
    pub transport: String,
    /// `LL_OVERLAY`, `-` when unset.
    pub overlay: String,
    /// Message length, bytes.
    pub msg_bytes: usize,
    /// Messages a second; 0 is closed loop.
    pub rate: u64,
    /// Round trips before measuring.
    pub warmup: u64,
    /// Round trips measured.
    pub samples: u64,
    /// Echoes that never came back.
    pub lost: u64,
}

/// Where the hot thread ran, and how often it was switched out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HotInfo {
    /// The CPU it was pinned to.
    pub cpu: usize,
    /// The CPU `sched_getcpu()` gave at the end.
    pub ran_on: usize,
    /// `voluntary_ctxt_switches` over the run.
    pub voluntary: u64,
    /// `nonvoluntary_ctxt_switches` over the run.
    pub nonvoluntary: u64,
}

/// What logging dropped and what the hot thread logged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogInfo {
    /// Lines the lossy writer dropped (`ErrorCounter::dropped_lines`).
    pub dropped_lines: u64,
    /// Warning lines the hot thread emitted.
    pub hot_warn_events: u64,
    /// Warning events its rate limits suppressed.
    pub suppressed: u64,
}

/// One app's report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LatencyReport {
    /// What ran.
    pub run: RunInfo,
    /// Round-trip percentiles (ping only).
    pub rtt: Option<Percentiles>,
    /// The largest round trips, `(realtime_ns, rtt_ns)`, largest first.
    pub top: Vec<(u64, u64)>,
    /// The hot loop's gaps.
    pub gap: LoopGap,
    /// The hot thread.
    pub hot: HotInfo,
    /// Logging.
    pub log: LogInfo,
}

impl LatencyReport {
    /// The report's lines, without line ends.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let r = &self.run;
        let mode = if r.rate == 0 {
            "closed".to_owned()
        } else {
            format!("open:{}/s", r.rate)
        };
        let mut lines = vec![format!(
            "ll-latency\tv1\trole={}\tarch={}\tcpu_build={}\tcounter_hz={}\ttransport={}\toverlay={}\tmsg_bytes={}\tmode={mode}\twarmup={}\tsamples={}\tlost={}",
            r.role,
            r.arch,
            r.cpu_build,
            r.counter_hz,
            r.transport,
            r.overlay,
            r.msg_bytes,
            r.warmup,
            r.samples,
            r.lost
        )];
        if let Some(p) = &self.rtt {
            lines.push(format!(
                "ll-latency\trtt_ns\tp50={}\tp90={}\tp99={}\tp99.9={}\tp99.99={}\tmax={}\tmean={}",
                p.p50, p.p90, p.p99, p.p999, p.p9999, p.max, p.mean
            ));
            let top: Vec<String> = self
                .top
                .iter()
                .map(|(at, rtt)| format!("{at}:{rtt}"))
                .collect();
            lines.push(format!("ll-latency\trtt_max_top10\t{}", top.join(",")));
        }
        let g = &self.gap;
        lines.push(format!(
            "ll-latency\tloop_gap_ns\tmax={}\tover_5us={}\tover_20us={}\tover_100us={}",
            g.max_ns, g.over_5us, g.over_20us, g.over_100us
        ));
        let h = &self.hot;
        lines.push(format!(
            "ll-latency\thot\tcpu={}\tran_on={}\tvoluntary_ctxt_switches={}\tnonvoluntary_ctxt_switches={}",
            h.cpu, h.ran_on, h.voluntary, h.nonvoluntary
        ));
        let l = &self.log;
        lines.push(format!(
            "ll-latency\tlog\tdropped_lines={}\thot_warn_events={}\tsuppressed={}",
            l.dropped_lines, l.hot_warn_events, l.suppressed
        ));
        lines
    }

    /// The lines joined, each ending in `\n`.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        for line in self.lines() {
            let _ = writeln!(out, "{line}");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LatencyReport {
        LatencyReport {
            run: RunInfo {
                role: "ping".into(),
                arch: "aarch64".into(),
                cpu_build: "generic".into(),
                counter_hz: 1_000_000_000,
                transport: "dpdk-ena".into(),
                overlay: "vdev-local".into(),
                msg_bytes: 32,
                rate: 0,
                warmup: 10,
                samples: 100,
                lost: 0,
            },
            rtt: Some(Percentiles {
                p50: 20_000,
                p90: 21_000,
                p99: 25_000,
                p999: 40_000,
                p9999: 60_000,
                max: 90_000,
                mean: 20_400,
                count: 100,
            }),
            top: vec![
                (1_700_000_000_000_000_002, 90_000),
                (1_700_000_000_000_000_001, 60_000),
            ],
            gap: {
                let mut gap = LoopGap::default();
                gap.max_ns = 3_000;
                gap
            },
            hot: HotInfo {
                cpu: 4,
                ran_on: 4,
                voluntary: 1,
                nonvoluntary: 0,
            },
            log: LogInfo {
                dropped_lines: 0,
                hot_warn_events: 1,
                suppressed: 3,
            },
        }
    }

    #[test]
    fn ping_report_golden() {
        let want = "\
ll-latency\tv1\trole=ping\tarch=aarch64\tcpu_build=generic\tcounter_hz=1000000000\ttransport=dpdk-ena\toverlay=vdev-local\tmsg_bytes=32\tmode=closed\twarmup=10\tsamples=100\tlost=0
ll-latency\trtt_ns\tp50=20000\tp90=21000\tp99=25000\tp99.9=40000\tp99.99=60000\tmax=90000\tmean=20400
ll-latency\trtt_max_top10\t1700000000000000002:90000,1700000000000000001:60000
ll-latency\tloop_gap_ns\tmax=3000\tover_5us=0\tover_20us=0\tover_100us=0
ll-latency\thot\tcpu=4\tran_on=4\tvoluntary_ctxt_switches=1\tnonvoluntary_ctxt_switches=0
ll-latency\tlog\tdropped_lines=0\thot_warn_events=1\tsuppressed=3
";
        assert_eq!(sample().text(), want);
    }

    #[test]
    fn pong_and_open_loop_reports() {
        let mut r = sample();
        r.run.role = "pong".into();
        r.run.rate = 10_000;
        r.rtt = None;
        r.top.clear();
        let lines = r.lines();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("\tmode=open:10000/s\t"), "{}", lines[0]);
        assert!(lines.iter().all(|l| !l.contains("rtt_")));
    }
}
