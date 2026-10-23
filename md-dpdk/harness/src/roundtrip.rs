//! What a round-trip client records and writes, shared by `rtt` (through an
//! Aeron media driver) and `floor` (no Aeron: the raw floor), so the two write
//! exactly the same result line and files:
//!
//! - stdout: `rtt,<label>,udp,p50,p90,p99,p99.9,p99.99,max,mean,samples,lost`
//!   (ns; `LABEL` names the run)
//! - `RTT_HISTOGRAM=<path>`: the full histogram, one `value_ns,count` row per
//!   recorded value
//! - `LOAD_SPIKES=<path>`: every measured round trip of at least
//!   `LOAD_SPIKE_NS` (default 100000) as `stream,due_ns,latency_ns,ping`, the
//!   first 100,000 of them, under `# start_realtime_ns=` and `# spikes_total=`
//! - `LOOP_STALLS_DIR=<dir>`: every stall of the client's own polling loop of
//!   at least `LOOP_STALL_NS` (default 20000), as `stalls-ping.csv` /
//!   `stalls-pong.csv` (`end_ns,gap_ns`, under `# start_realtime_ns=`)
//!
//! Everything is preallocated before the measured run, so recording never
//! allocates.

use crate::lab::{Clock, label};
use hdrhistogram::Histogram;

const SPIKE_CAPACITY: usize = 100_000;
const STALL_CAPACITY: usize = 1_000_000;

/// Gaps in the calling loop of at least a threshold, each with when it ended:
/// time this client did not run. Preallocated, so recording never allocates.
pub struct Stalls {
    threshold: u64, // ns
    last: u64,
    t0: u64,
    t0_realtime: u128,
    rows: Vec<(u64, u64)>, // (end, gap) in ns
}

impl Stalls {
    /// `None` unless `LOOP_STALLS_DIR` is set.
    pub fn new(clock: &Clock) -> Option<Self> {
        std::env::var("LOOP_STALLS_DIR").ok().filter(|d| !d.is_empty())?;
        let threshold: u64 = std::env::var("LOOP_STALL_NS").map_or(20_000, |v| v.parse().expect("LOOP_STALL_NS"));
        let t0 = clock.now();
        Some(Self {
            threshold,
            last: t0,
            t0,
            t0_realtime: realtime_ns(),
            rows: Vec::with_capacity(STALL_CAPACITY),
        })
    }

    /// One pass of the loop, at `now`.
    #[inline(always)]
    pub fn pass(&mut self, now: u64) {
        let gap = now.wrapping_sub(self.last);
        if gap >= self.threshold && self.rows.len() < self.rows.capacity() {
            self.rows.push((now.wrapping_sub(self.t0), gap));
        }
        self.last = now;
    }

    /// Writes them to `name` in `LOOP_STALLS_DIR`.
    pub fn write(&self, name: &str) {
        let Some(dir) = std::env::var("LOOP_STALLS_DIR").ok() else {
            return;
        };
        let mut rows = format!("# start_realtime_ns={}\nend_ns,gap_ns\n", self.t0_realtime);
        for (end, gap) in &self.rows {
            rows.push_str(&format!("{end},{gap}\n"));
        }
        let path = format!("{dir}/{name}");
        std::fs::write(&path, rows).unwrap_or_else(|e| panic!("LOOP_STALLS_DIR {path}: {e}"));
    }
}

/// `CLOCK_REALTIME` now, in nanoseconds since the epoch.
pub fn realtime_ns() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

/// The measured run's round trips: the histogram, and the spikes when
/// `LOAD_SPIKES` is set.
pub struct Results {
    histogram: Histogram<u64>,
    spikes_path: Option<String>,
    spike_ns: u64,
    spikes: Vec<(u64, u64, usize)>, // (due, latency, ping) in ns
    spikes_total: u64,
    t0: u64,
    t0_realtime: u128,
}

impl Results {
    /// Empty, from the environment (`LOAD_SPIKES`, `LOAD_SPIKE_NS`); call
    /// [`start`](Self::start) as the measured run begins.
    pub fn new() -> Self {
        let spikes_path = std::env::var("LOAD_SPIKES").ok().filter(|p| !p.is_empty());
        let spike_ns: u64 = std::env::var("LOAD_SPIKE_NS").map_or(100_000, |v| v.parse().expect("LOAD_SPIKE_NS"));
        Self {
            histogram: Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap(),
            spikes: Vec::with_capacity(if spikes_path.is_some() { SPIKE_CAPACITY } else { 0 }),
            spikes_path,
            spike_ns,
            spikes_total: 0,
            t0: 0,
            t0_realtime: 0,
        }
    }

    /// The measured run starts now: spikes' `due_ns` count from here.
    pub fn start(&mut self, clock: &Clock) {
        self.t0 = clock.now();
        self.t0_realtime = realtime_ns();
    }

    /// Ping `ping` (its index in the measured run) came back after `rtt` ns;
    /// `clock` is read only for a spike.
    #[inline(always)]
    pub fn record(&mut self, clock: &Clock, rtt: u64, ping: usize) {
        self.histogram.saturating_record(rtt);
        if rtt >= self.spike_ns {
            self.spikes_total += 1;
            if self.spikes.len() < self.spikes.capacity() {
                self.spikes
                    .push((clock.now().wrapping_sub(self.t0).saturating_sub(rtt), rtt, ping));
            }
        }
    }

    /// The result line (`lost`: pings whose echo never came), as the client
    /// prints it on stdout.
    pub fn line(&self, lost: u64) -> String {
        let q = |quantile| self.histogram.value_at_quantile(quantile);
        format!(
            "rtt,{},udp,{},{},{},{},{},{},{:.0},{},{lost}",
            label(),
            q(0.5),
            q(0.9),
            q(0.99),
            q(0.999),
            q(0.9999),
            self.histogram.max(),
            self.histogram.mean(),
            self.histogram.len()
        )
    }

    /// Prints the result line and writes `RTT_HISTOGRAM` and `LOAD_SPIKES`
    /// (`stream`: the spikes' stream column).
    pub fn report(&self, lost: u64, stream: i32) {
        println!("{}", self.line(lost));
        if let Some(path) = std::env::var("RTT_HISTOGRAM").ok().filter(|p| !p.is_empty()) {
            let mut rows = String::from("value_ns,count\n");
            for v in self.histogram.iter_recorded() {
                rows.push_str(&format!("{},{}\n", v.value_iterated_to(), v.count_at_value()));
            }
            std::fs::write(&path, rows).unwrap_or_else(|e| panic!("RTT_HISTOGRAM {path}: {e}"));
        }
        if let Some(path) = &self.spikes_path {
            // the wall-clock time due_ns counts from, to line spikes up with traces
            let mut rows = format!(
                "# start_realtime_ns={}\n# spikes_total={}\nstream,due_ns,latency_ns,ping\n",
                self.t0_realtime, self.spikes_total
            );
            for (due, latency, ping) in &self.spikes {
                rows.push_str(&format!("{stream},{due},{latency},{ping}\n"));
            }
            std::fs::write(path, rows).unwrap_or_else(|e| panic!("LOAD_SPIKES {path}: {e}"));
        }
    }
}
