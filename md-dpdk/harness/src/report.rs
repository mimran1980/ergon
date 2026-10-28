//! Hand-rolled JSON report (no serde — keeps the dependency budget to the
//! workspace minimum, plan §16). `scripts/test-dpdk-vdev.sh` combines the
//! per-process reports and asserts on the `ok`/`sent`/`received` fields.

use std::fmt::Write as _;

/// Per-process scenario outcome, serialised to the path given by `--report`.
#[derive(Default, Debug)]
pub struct Report {
    pub role: String,
    pub scenario: String,
    pub transport: String,
    pub ok: bool,
    pub sent: u64,
    pub received: u64,
    pub bytes: u64,
    pub duration_ms: u64,
    /// Receiver-side one-way latency histogram (plan §12); zero in count-mode runs.
    pub latency_p50_ns: u64,
    pub latency_p99_ns: u64,
    pub latency_max_ns: u64,
    /// Throughput derived from the perf run's wall clock.
    pub offered_per_sec: f64,
    pub delivered_per_sec: f64,
    /// Offer calls that returned retryable/backpressured (sender-side).
    pub backpressure_ops: u64,
    /// Unrecovered sequence gaps observed by the receiver.
    pub gaps: u64,
    /// The embedded DPDK transport's clock: `counter`, `monotonic`, or
    /// `unknown` (one this harness does not know, named in `clock_reason`);
    /// empty for kernel UDP or a standalone driver.
    pub clock: String,
    /// The counter's rate when `clock` is `counter`, else 0.
    pub clock_hz: u64,
    /// Which rate `clock_hz` is: `eal` (DPDK's) or `measured` (at start,
    /// DPDK's being more than 100 ppm off); empty unless `counter`.
    pub clock_rate: String,
    /// DPDK's rate for the counter (`rte_get_tsc_hz()`) when `clock` is
    /// `counter`, else 0.
    pub clock_eal_hz: u64,
    /// The counter's rate measured at start when `clock` is `counter`, else 0.
    pub clock_measured_hz: u64,
    /// Why `clock` is `monotonic`, or what the `unknown` clock is; empty
    /// otherwise.
    pub clock_reason: String,
    /// `CNTFRQ_EL0` as this process reads it on aarch64, else 0.
    pub cntfrq_hz: u64,
    pub detail: Vec<String>,
    pub error: Option<String>,
    /// The embedded driver's DPDK counters at the end of the run, summed by
    /// `<role>.<dir>.<name>` (`sender.rx.icmp-echo=5;...`), sorted; empty for
    /// kernel UDP. The vdev scenarios assert on them.
    pub dpdk_counters: String,
    /// The embedded driver's distinct error log at the end of the run, one
    /// entry per error, joined by ` | `.
    pub driver_errors: String,
}

impl Report {
    /// `to_json` is deliberately format-stable: the script greps the fields by
    /// name, so changing this output is a breaking change to the harness.
    pub fn to_json(&self) -> Result<String, std::fmt::Error> {
        let mut s = String::new();
        s.push_str("{\n");
        writeln!(s, "  \"role\": {},", json_str(&self.role))?;
        writeln!(s, "  \"scenario\": {},", json_str(&self.scenario))?;
        writeln!(s, "  \"transport\": {},", json_str(&self.transport))?;
        writeln!(s, "  \"ok\": {},", self.ok)?;
        writeln!(s, "  \"sent\": {},", self.sent)?;
        writeln!(s, "  \"received\": {},", self.received)?;
        writeln!(s, "  \"bytes\": {},", self.bytes)?;
        writeln!(s, "  \"duration_ms\": {},", self.duration_ms)?;
        writeln!(s, "  \"latency_p50_ns\": {},", self.latency_p50_ns)?;
        writeln!(s, "  \"latency_p99_ns\": {},", self.latency_p99_ns)?;
        writeln!(s, "  \"latency_max_ns\": {},", self.latency_max_ns)?;
        writeln!(s, "  \"offered_per_sec\": {},", self.offered_per_sec)?;
        writeln!(s, "  \"delivered_per_sec\": {},", self.delivered_per_sec)?;
        writeln!(s, "  \"backpressure_ops\": {},", self.backpressure_ops)?;
        writeln!(s, "  \"gaps\": {},", self.gaps)?;
        writeln!(s, "  \"clock\": {},", json_str(&self.clock))?;
        writeln!(s, "  \"clock_hz\": {},", self.clock_hz)?;
        writeln!(s, "  \"clock_rate\": {},", json_str(&self.clock_rate))?;
        writeln!(s, "  \"clock_eal_hz\": {},", self.clock_eal_hz)?;
        writeln!(s, "  \"clock_measured_hz\": {},", self.clock_measured_hz)?;
        writeln!(s, "  \"clock_reason\": {},", json_str(&self.clock_reason))?;
        writeln!(s, "  \"cntfrq_hz\": {},", self.cntfrq_hz)?;
        writeln!(s, "  \"dpdk_counters\": {},", json_str(&self.dpdk_counters))?;
        writeln!(s, "  \"driver_errors\": {},", json_str(&self.driver_errors))?;
        s.push_str("  \"detail\": [\n");
        for (i, d) in self.detail.iter().enumerate() {
            write!(s, "    {}", json_str(d))?;
            if i + 1 < self.detail.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ],\n");
        let err = self.error.as_deref().map_or_else(|| "null".to_string(), json_str);
        writeln!(s, "  \"error\": {err}")?;
        s.push_str("}\n");
        Ok(s)
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The key a DPDK counter's label sums under: `<role>.<dir>.<name>` from
/// `rusteron-dpdk role=<role> bdf=<bdf> port=<n> q=<n> dir=<dir> <name> ...`
/// (port-info appends its details after the name); `None` for any other label.
pub fn dpdk_counter_key(label: &str) -> Option<String> {
    let rest = label.strip_prefix("rusteron-dpdk role=")?;
    let role = rest.split_whitespace().next()?;
    let (_, after_dir) = rest.split_once(" dir=")?;
    let mut words = after_dir.split_whitespace();
    let dir = words.next()?;
    let name = words.next()?;
    Some(format!("{role}.{dir}.{name}"))
}

/// `key=value` pairs, summed per key and sorted, joined by `;`.
pub fn sum_counters(values: impl IntoIterator<Item = (String, i64)>) -> String {
    let mut sums = std::collections::BTreeMap::<String, i64>::new();
    for (key, value) in values {
        *sums.entry(key).or_default() += value;
    }
    sums.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpdk_counter_labels_key_by_role_dir_and_name() {
        assert_eq!(
            dpdk_counter_key("rusteron-dpdk role=sender bdf=net_tap0 port=0 q=0 dir=rx icmp-echo").as_deref(),
            Some("sender.rx.icmp-echo")
        );
        assert_eq!(
            dpdk_counter_key("rusteron-dpdk role=receiver bdf=net_tap1 port=1 q=0 dir=tx arp-miss").as_deref(),
            Some("receiver.tx.arp-miss")
        );
        assert_eq!(
            dpdk_counter_key(
                "rusteron-dpdk role=sender bdf=net_tap0 port=0 q=0 dir=- port-info driver=net_tap mac=02:00:00:00:00:01"
            )
            .as_deref(),
            Some("sender.-.port-info")
        );
        assert_eq!(dpdk_counter_key("NAKs sent: 0"), None);
        assert_eq!(dpdk_counter_key("rusteron-dpdk role=sender"), None);
    }

    #[test]
    fn counters_sum_per_key_sorted() {
        let line = sum_counters([
            ("sender.tx.pkts".to_string(), 3),
            ("receiver.rx.pkts".to_string(), 2),
            ("sender.tx.pkts".to_string(), 4),
        ]);
        assert_eq!(line, "receiver.rx.pkts=2;sender.tx.pkts=7");
    }

    #[test]
    fn the_report_carries_counters_and_errors() -> Result<(), std::fmt::Error> {
        let report = Report {
            dpdk_counters: "sender.rx.icmp-echo=5".to_string(),
            driver_errors: "next hop 10.9.0.254 does not answer ARP".to_string(),
            ..Report::default()
        };
        let json = report.to_json()?;
        assert!(json.contains("\"dpdk_counters\": \"sender.rx.icmp-echo=5\","), "{json}");
        assert!(
            json.contains("\"driver_errors\": \"next hop 10.9.0.254 does not answer ARP\","),
            "{json}"
        );
        Ok(())
    }

    /// The vdev script reads `"<field>": <value>` up to the next comma: the
    /// transport's clock, its rate and `CNTFRQ_EL0` come out that way.
    #[test]
    fn the_report_carries_the_transports_clock() -> Result<(), std::fmt::Error> {
        let report = Report {
            transport: "dpdk".to_string(),
            clock: "counter".to_string(),
            clock_hz: 1_000_000_000,
            clock_rate: "eal".to_string(),
            clock_eal_hz: 1_000_000_000,
            clock_measured_hz: 1_000_000_017,
            cntfrq_hz: 1_000_000_000,
            ..Report::default()
        };
        let json = report.to_json()?;
        for line in [
            "\"clock\": \"counter\",",
            "\"clock_hz\": 1000000000,",
            "\"clock_rate\": \"eal\",",
            "\"clock_eal_hz\": 1000000000,",
            "\"clock_measured_hz\": 1000000017,",
            "\"clock_reason\": \"\",",
            "\"cntfrq_hz\": 1000000000,",
        ] {
            assert!(json.contains(line), "{line} in {json}");
        }
        Ok(())
    }
}
