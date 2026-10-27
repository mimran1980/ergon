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
//!
//! `RTT_GAP_US=<us>` paces the pings ([`Pace`]): one every that many
//! microseconds, on a fixed schedule (the warm-up's too), instead of each as soon as
//! the last one's echo is back. The client spins meanwhile, polling as it
//! does waiting for an echo, so only the network goes quiet: a sparse flow.
//! A paced run checks `RTT_SECONDS` before every ping.

use crate::lab::{Clock, label};
use hdrhistogram::Histogram;

const SPIKE_CAPACITY: usize = 100_000;
const STALL_CAPACITY: usize = 1_000_000;

/// `RTT_GAP_US`: when the next ping may go. Unset or 0, every ping may go
/// at once (the round trip's own pacing).
pub struct Pace {
    gap: u64, // ns
    next: u64,
}

impl Pace {
    /// From `RTT_GAP_US` (whole microseconds, up to 60 s).
    pub fn from_env() -> Self {
        let gap = match std::env::var("RTT_GAP_US") {
            Err(_) => 0,
            Ok(v) => match v.parse::<u64>() {
                Ok(us) if us <= 60_000_000 => us * 1000,
                _ => panic!("RTT_GAP_US={v}: whole microseconds, 0 to 60000000"),
            },
        };
        Self { gap, next: 0 }
    }

    /// Whether pings are paced: the run's budget is then checked every ping.
    #[inline(always)]
    pub const fn paced(&self) -> bool {
        self.gap != 0
    }

    /// Whether the next ping may go at `now` (the clock's).
    #[inline(always)]
    pub const fn ready(&self, now: u64) -> bool {
        self.gap == 0 || now >= self.next
    }

    /// A ping went at `at`: the next is due a gap after this one was due,
    /// so the loop's own latency between pings never adds up (a run of
    /// `RTT_SECONDS` holds its seconds over the gap of pings); a ping half
    /// a gap late or more (a lost one's timeout, a stall) starts the
    /// schedule again a gap after it, never a burst to catch up.
    #[inline(always)]
    pub const fn sent(&mut self, at: u64) {
        let due = self.next;
        self.next = if due != 0 && at.wrapping_sub(due) < self.gap / 2 {
            due.wrapping_add(self.gap)
        } else {
            at.wrapping_add(self.gap)
        };
    }
}

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

#[cfg(test)]
mod tests {
    use super::Pace;

    const GAP: u64 = 1_000_000; // 1 ms

    /// Each ping sent 3 µs after it was due (the loop's own latency): the
    /// schedule stays a gap apart from the first, so 20,000 pings at 1 ms
    /// take 20 s, not 20 s and 60 ms (which `RTT_SECONDS=20` would cut 60
    /// pings short of).
    #[test]
    fn a_paced_schedule_does_not_drift_by_the_loops_latency() {
        let mut pace = Pace { gap: GAP, next: 0 };
        let mut at = 5_000;
        for _ in 0..20_000 {
            assert!(pace.ready(at));
            pace.sent(at);
            at = pace.next + 3_000;
        }
        assert_eq!(pace.next, 5_000 + 20_000 * GAP);
    }

    /// A ping half a gap late or more (a lost one's timeout, a stall): the
    /// schedule starts again a gap after it, no burst of pings to catch up;
    /// one a little late keeps it.
    #[test]
    fn a_late_ping_restarts_the_schedule_without_a_burst() {
        let mut pace = Pace { gap: GAP, next: 0 };
        pace.sent(1_000);
        assert_eq!(pace.next, 1_000 + GAP);
        let late = pace.next + 50 * GAP;
        assert!(pace.ready(late));
        pace.sent(late);
        assert_eq!(pace.next, late + GAP);
        assert!(!pace.ready(late + GAP - 1));
        let a_little = pace.next + GAP / 4;
        pace.sent(a_little);
        assert_eq!(pace.next, late + 2 * GAP);
    }

    /// Unpaced (`RTT_GAP_US` 0 or unset): every ping may go at once.
    #[test]
    fn unpaced_is_always_ready() {
        let mut pace = Pace { gap: 0, next: 0 };
        pace.sent(10);
        assert!(!pace.paced());
        assert!(pace.ready(0) && pace.ready(10));
    }
}
