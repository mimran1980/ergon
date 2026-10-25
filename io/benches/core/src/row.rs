//! What an arm prints and writes, in md-dpdk/harness's formats so
//! md-dpdk/benches/cloud/analyse.py and spikes.py read it unchanged.
//!
//! The result row is `load`'s (md-dpdk/harness/src/bin/load.rs), every
//! latency in ns from the intended send time, then the suite's own columns:
//!
//! ```text
//! <load|oneway>,<label>,<channel>,<size>,<rate>,<burst>,p50,p90,p99,p99.9,p99.99,max,mean,
//! samples,sent,received,backpressured,achieved,max_send_lag,scheduled,late,censored,
//! min,lost,reordered,dup,retrans,wire_bytes,busy_threads,stream0_p50,stream0_p99,stream0_p99.99,stream0_max
//! ```
//!
//! `burst` is always 1. `sent` and `scheduled` include the warm-up, as
//! `load`'s do; `late` and `max_send_lag` cover the measured sends only.
//! `censored` counts the latencies only known to be at least what was
//! recorded (the histogram's bound, or owed without a reply). `retrans` is
//! empty where the transport has no retransmit counter. `oneway` rows have
//! the same columns, measured one way on one host's counter.
//!
//! Beside it: the histogram ([`Latency::csv`](crate::hist::Latency::csv)),
//! [`Spikes`] and [`Stalls`], as md-dpdk/harness/src/roundtrip.rs writes
//! them.

use crate::Error;
use crate::clock::TscClock;
use crate::hist::Summary;
use crate::loss::LossCounts;
use crate::pace::PaceStats;
use std::fmt::Write;
use std::path::Path;

/// The row's first column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A round trip (`load`).
    Load,
    /// One way, same host (`oneway`).
    Oneway,
}

impl RowKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Oneway => "oneway",
        }
    }
}

/// One point's result.
#[derive(Debug, Clone, PartialEq)]
pub struct Row<'a> {
    /// `load` or `oneway`.
    pub kind: RowKind,
    /// The run's label (lab.sh's mode).
    pub label: &'a str,
    /// The channel column: `udp` where lab.sh rewrites it to `udp-<point>`.
    pub channel: &'a str,
    /// Payload bytes per message.
    pub size: usize,
    /// Offered msg/s.
    pub rate: u64,
    /// Every stream's latency from the intended send time.
    pub latency: Summary,
    /// Stream 0's alone.
    pub stream0: Summary,
    /// The sender against its schedule.
    pub pace: PaceStats,
    /// Replies (or one-way messages) received, warm-up included.
    pub received: u64,
    /// Sends that met back-pressure at least once.
    pub backpressured: u64,
    /// The msg/s achieved over the measured window.
    pub achieved: f64,
    /// Every stream's loss accounting summed.
    pub loss: LossCounts,
    /// Retransmits counted by the transport; `None` where it has none.
    pub retrans: Option<u64>,
    /// Bytes on the wire per message (payload, framing and headers).
    pub wire_bytes: u64,
    /// Busy-spinning threads the arm runs.
    pub busy_threads: u32,
}

fn field(v: &str) -> Result<&str, Error> {
    if v.is_empty() || v.contains([',', '\n', '\r']) {
        return Err(Error::Field(v.to_owned()));
    }
    Ok(v)
}

impl Row<'_> {
    /// The row, without a line break.
    ///
    /// # Errors
    /// [`Error::Field`] for a label or channel that is empty or holds a comma
    /// or line break.
    pub fn line(&self) -> Result<String, Error> {
        let (l, s) = (&self.latency, &self.stream0);
        let retrans = self.retrans.map(|r| r.to_string()).unwrap_or_default();
        Ok(format!(
            "{},{},{},{},{},1,{},{},{},{},{},{},{:.0},{},{},{},{},{:.0},{},{},{},{},{},{},{},{},{retrans},{},{},{},{},{},{}",
            self.kind.name(),
            field(self.label)?,
            field(self.channel)?,
            self.size,
            self.rate,
            l.p50,
            l.p90,
            l.p99,
            l.p999,
            l.p9999,
            l.max,
            l.mean,
            l.count,
            self.pace.sent,
            self.received,
            self.backpressured,
            self.achieved,
            self.pace.max_send_lag_ns,
            self.pace.scheduled,
            self.pace.late,
            l.censored,
            l.min,
            self.loss.lost,
            self.loss.reordered,
            self.loss.duplicate,
            self.wire_bytes,
            self.busy_threads,
            s.p50,
            s.p99,
            s.p9999,
            s.max,
        ))
    }
}

/// Latencies of at least a threshold, each with when it was due:
/// `stream,due_ns,latency_ns,ping` under `# start_realtime_ns=` and
/// `# spikes_total=`. Preallocated; past capacity only the total grows.
#[derive(Debug, Clone)]
pub struct Spikes {
    threshold_ns: u64,
    rows: Vec<(u32, u64, u64, u64)>,
    total: u64,
}

impl Spikes {
    /// Spikes of at least `threshold_ns` (the suite's default: 100 000),
    /// keeping the first `capacity` (100 000).
    #[must_use]
    pub fn new(threshold_ns: u64, capacity: usize) -> Self {
        Self {
            threshold_ns,
            rows: Vec::with_capacity(capacity),
            total: 0,
        }
    }

    /// Message `ping` of `stream`, due `due_ns` after the run's start, took
    /// `latency_ns`.
    #[inline]
    pub fn record(&mut self, stream: u32, due_ns: u64, latency_ns: u64, ping: u64) {
        if latency_ns >= self.threshold_ns {
            self.total += 1;
            if self.rows.len() < self.rows.capacity() {
                self.rows.push((stream, due_ns, latency_ns, ping));
            }
        }
    }

    /// The file, `start_realtime_ns` being when `due_ns` counts from.
    #[must_use]
    pub fn csv(&self, start_realtime_ns: u128) -> String {
        let mut out = format!(
            "# start_realtime_ns={start_realtime_ns}\n# spikes_total={}\nstream,due_ns,latency_ns,ping\n",
            self.total
        );
        for (stream, due, latency, ping) in &self.rows {
            let _ = writeln!(out, "{stream},{due},{latency},{ping}");
        }
        out
    }

    /// Writes [`csv`](Self::csv) to `path`.
    ///
    /// # Errors
    /// The write's.
    pub fn write(&self, path: &Path, start_realtime_ns: u128) -> std::io::Result<()> {
        std::fs::write(path, self.csv(start_realtime_ns))
    }
}

/// Gaps of at least a threshold between passes of a polling loop, each with
/// when it ended: `end_ns,gap_ns` under `# start_realtime_ns=`. Time the
/// loop did not run. Preallocated; past capacity, dropped.
#[derive(Debug, Clone)]
pub struct Stalls {
    threshold_ns: u64,
    t0: u64,
    last: u64,
    rows: Vec<(u64, u64)>,
}

impl Stalls {
    /// Gaps of at least `threshold_ns` (the suite's default: 20 000), the
    /// first `capacity` (1 000 000), from raw `t0`.
    #[must_use]
    pub fn new(threshold_ns: u64, capacity: usize, t0: u64) -> Self {
        Self {
            threshold_ns,
            t0,
            last: t0,
            rows: Vec::with_capacity(capacity),
        }
    }

    /// One pass of the loop at raw `now`.
    #[inline]
    pub fn pass(&mut self, clock: &TscClock, now: u64) {
        let gap = clock.delta_ns(self.last, now);
        if gap >= self.threshold_ns && self.rows.len() < self.rows.capacity() {
            self.rows.push((clock.delta_ns(self.t0, now), gap));
        }
        self.last = now;
    }

    /// The file, `start_realtime_ns` being raw `t0`'s wall-clock time.
    #[must_use]
    pub fn csv(&self, start_realtime_ns: u128) -> String {
        let mut out = format!("# start_realtime_ns={start_realtime_ns}\nend_ns,gap_ns\n");
        for (end, gap) in &self.rows {
            let _ = writeln!(out, "{end},{gap}");
        }
        out
    }

    /// Writes [`csv`](Self::csv) to `path`.
    ///
    /// # Errors
    /// The write's.
    pub fn write(&self, path: &Path, start_realtime_ns: u128) -> std::io::Result<()> {
        std::fs::write(path, self.csv(start_realtime_ns))
    }
}
