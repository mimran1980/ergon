//! Hand-rolled JSON report (no serde — keeps the dependency budget to the
//! workspace minimum, plan §16). `scripts/test-dpdk-vdev.sh` combines the
//! per-process reports and asserts on the `ok`/`sent`/`received` fields.

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
}

impl Report {
    /// `to_json` is deliberately format-stable: the script greps the fields by
    /// name, so changing this output is a breaking change to the harness.
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"role\": {},\n", json_str(&self.role)));
        s.push_str(&format!("  \"scenario\": {},\n", json_str(&self.scenario)));
        s.push_str(&format!("  \"transport\": {},\n", json_str(&self.transport)));
        s.push_str(&format!("  \"ok\": {},\n", self.ok));
        s.push_str(&format!("  \"sent\": {},\n", self.sent));
        s.push_str(&format!("  \"received\": {},\n", self.received));
        s.push_str(&format!("  \"bytes\": {},\n", self.bytes));
        s.push_str(&format!("  \"duration_ms\": {},\n", self.duration_ms));
        s.push_str(&format!("  \"latency_p50_ns\": {},\n", self.latency_p50_ns));
        s.push_str(&format!("  \"latency_p99_ns\": {},\n", self.latency_p99_ns));
        s.push_str(&format!("  \"latency_max_ns\": {},\n", self.latency_max_ns));
        s.push_str(&format!("  \"offered_per_sec\": {},\n", self.offered_per_sec));
        s.push_str(&format!("  \"delivered_per_sec\": {},\n", self.delivered_per_sec));
        s.push_str(&format!("  \"backpressure_ops\": {},\n", self.backpressure_ops));
        s.push_str(&format!("  \"gaps\": {},\n", self.gaps));
        s.push_str(&format!("  \"clock\": {},\n", json_str(&self.clock)));
        s.push_str(&format!("  \"clock_hz\": {},\n", self.clock_hz));
        s.push_str(&format!("  \"clock_rate\": {},\n", json_str(&self.clock_rate)));
        s.push_str(&format!("  \"clock_eal_hz\": {},\n", self.clock_eal_hz));
        s.push_str(&format!("  \"clock_measured_hz\": {},\n", self.clock_measured_hz));
        s.push_str(&format!("  \"clock_reason\": {},\n", json_str(&self.clock_reason)));
        s.push_str(&format!("  \"cntfrq_hz\": {},\n", self.cntfrq_hz));
        s.push_str("  \"detail\": [\n");
        for (i, d) in self.detail.iter().enumerate() {
            s.push_str(&format!("    {}", json_str(d)));
            if i + 1 < self.detail.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ],\n");
        let err = self
            .error
            .as_deref()
            .map(json_str)
            .unwrap_or_else(|| "null".to_string());
        s.push_str(&format!("  \"error\": {}\n", err));
        s.push_str("}\n");
        s
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The vdev script reads `"<field>": <value>` up to the next comma: the
    /// transport's clock, its rate and `CNTFRQ_EL0` come out that way.
    #[test]
    fn the_report_carries_the_transports_clock() {
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
        let json = report.to_json();
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
    }
}
