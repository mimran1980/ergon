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
