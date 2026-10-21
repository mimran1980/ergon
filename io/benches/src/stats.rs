//! Percentile summaries used by the analysis binary, and the work accounting
//! that sits beside them.

use std::collections::HashMap;
use std::fmt::Write as _;

/// A summary of a sample, values in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    /// Sample size.
    pub n: usize,
    /// Mean.
    pub mean: f64,
    /// Median.
    pub p50: i64,
    /// 90th percentile.
    pub p90: i64,
    /// 99th percentile.
    pub p99: i64,
    /// 99.9th percentile.
    pub p999: i64,
    /// 99.99th percentile (needs about a million samples to mean much).
    pub p9999: i64,
    /// Minimum.
    pub min: i64,
    /// Maximum.
    pub max: i64,
}

/// Nearest-rank percentile of a sorted slice (`q` in 0..=1).
#[must_use]
pub fn percentile(sorted: &[i64], q: f64) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (q * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// Summarises `v` (sorted in place). `None` when empty.
#[must_use]
pub fn summarize(v: &mut [i64]) -> Option<Summary> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    let sum: f64 = v.iter().map(|&x| x as f64).sum();
    Some(Summary {
        n: v.len(),
        mean: sum / v.len() as f64,
        p50: percentile(v, 0.50),
        p90: percentile(v, 0.90),
        p99: percentile(v, 0.99),
        p999: percentile(v, 0.999),
        p9999: percentile(v, 0.9999),
        min: v[0],
        max: v[v.len() - 1],
    })
}

/// Formats nanoseconds as microseconds with two decimals.
#[must_use]
pub fn us(ns: i64) -> String {
    format!("{:.2}", ns as f64 / 1000.0)
}

/// A markdown table row: `label | n | mean | p50 | p90 | p99 | p99.9 | p99.99 | max` (µs).
#[must_use]
pub fn row(label: &str, s: &Summary) -> String {
    format!(
        "| {label} | {} | {:.2} | {} | {} | {} | {} | {} | {} |",
        s.n,
        s.mean / 1000.0,
        us(s.p50),
        us(s.p90),
        us(s.p99),
        us(s.p999),
        us(s.p9999),
        us(s.max)
    )
}

/// Header matching [`row`].
pub const HEADER: &str = "| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |\n|---|---|---|---|---|---|---|---|---|";

/// The work a run planned and what became of it.
///
/// Percentiles are over the samples that completed and were recorded. These
/// counts sit beside them: without them a reader cannot tell a clean run from
/// one that never issued, never got back or discarded part of its work, and a
/// shorter sample can read as a faster one. A count the path does not produce
/// is `None` and is printed as "not yet measured", never as zero.
///
/// Where every count is present, `issued = completed + censored` and
/// `scheduled >= issued`. The counts cover the whole run; a report that drops a
/// warm-up shows `n` smaller than `completed` and says so. On a receive-only
/// run the send-side counts are the sender's and are `None` here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Work {
    /// Sends the run's schedule called for over its length. The schedule never
    /// slips, so every slot is a send the client owed; a slot not filled is
    /// unsent work. A back-to-back closed loop has no schedule beyond its
    /// replies, so this equals [`Work::issued`].
    pub scheduled: Option<u64>,
    /// Sends that went out.
    pub issued: Option<u64>,
    /// Sends with a recorded sample: the percentile's `n` before a warm-up is
    /// dropped.
    pub completed: Option<u64>,
    /// Sends that went out when the next slot on the schedule was already due,
    /// so the run owed more than one when it issued them. Not merely past
    /// their own slot: on a nanosecond clock every send is past its slot by
    /// the loop's detection latency, and that count would be [`Work::issued`]
    /// itself. The size of the slip is the run's max lag.
    pub late: Option<u64>,
    /// Send attempts the transport refused. On a WebSocket or HTTP client the
    /// send queue grows instead of refusing, and a send failure fails the run,
    /// so this is 0 on every run that reported; on the Aeron load generator it
    /// is the back-pressured offers.
    pub rejected: Option<u64>,
    /// Issued sends with no recorded sample: no reply came back before the run
    /// ended, or the reply was discarded because the record vector was full.
    /// This is what the percentiles leave out. On a receive-only run it is the
    /// messages that arrived but produced no sample; messages that never
    /// arrived are the sender's to account for.
    pub censored: Option<u64>,
}

/// Meta keys of [`Work`], in report order.
const WORK_KEYS: [&str; 6] = [
    "work.scheduled",
    "work.issued",
    "work.completed",
    "work.late",
    "work.rejected",
    "work.censored",
];

impl Work {
    /// The counts of a round trip. `rejected` is 0 (see its field).
    #[must_use]
    pub const fn round_trip(
        scheduled: u64,
        issued: u64,
        completed: u64,
        late: u64,
        censored: u64,
    ) -> Self {
        Self {
            scheduled: Some(scheduled),
            issued: Some(issued),
            completed: Some(completed),
            late: Some(late),
            rejected: Some(0),
            censored: Some(censored),
        }
    }

    /// The counts of a round trip whose schedule is known: `interval_ns` is 0
    /// when unpaced (then `scheduled` is the sends made: there is no schedule
    /// beyond the replies), `first_due_ns` is when the first message was due
    /// (0 before the first send), and `end_real_ns` is when the run ended
    /// (wall clock), so the schedule's slots can be counted. The schedule never
    /// slips, so every slot is a send owed and a slot not filled is unsent
    /// work.
    #[must_use]
    pub fn paced(
        end_real_ns: i64,
        first_due_ns: i64,
        interval_ns: i64,
        issued: u64,
        completed: u64,
        late: u64,
    ) -> Self {
        let censored = issued.saturating_sub(completed);
        let scheduled = if interval_ns == 0 {
            issued
        } else if first_due_ns == 0 {
            0
        } else {
            let span = (end_real_ns - first_due_ns).max(0);
            u64::try_from(1 + span / interval_ns).unwrap_or(0)
        };
        Self::round_trip(scheduled, issued, completed, late, censored)
    }

    /// Whether a send issued at `now` for a slot at `due` counts as
    /// [`Work::late`]: the next slot (`due + interval_ns`) is already due, so
    /// the run owed more than one when it issued this. A send merely past its
    /// own slot is not late — on a nanosecond clock every send is, by the
    /// loop's detection latency, and that count would be [`Work::issued`]
    /// itself. `interval_ns` 0 is an unpaced run, which has no slots.
    #[must_use]
    pub const fn is_late(now: i64, due: i64, interval_ns: i64) -> bool {
        interval_ns != 0 && now >= due + interval_ns
    }

    /// A receive-only run: what arrived and what produced no sample. The
    /// sender's schedule is not visible here.
    #[must_use]
    pub const fn received(completed: u64, censored: u64) -> Self {
        Self {
            scheduled: None,
            issued: None,
            completed: Some(completed),
            late: None,
            rejected: None,
            censored: Some(censored),
        }
    }

    /// `issued = completed + censored` and `scheduled >= issued`, wherever both
    /// sides of each are known. `Ok` when they hold or when a count is missing.
    /// The reason names the `work.*` keys, so it can be pasted under a table.
    pub fn check(&self) -> Result<(), String> {
        if let (Some(i), Some(c), Some(x)) = (self.issued, self.completed, self.censored)
            && i != c + x
        {
            return Err(format!(
                "work.issued {i} is not work.completed {c} + work.censored {x}"
            ));
        }
        if let (Some(s), Some(i)) = (self.scheduled, self.issued)
            && s < i
        {
            return Err(format!("work.scheduled {s} is below work.issued {i}"));
        }
        Ok(())
    }

    /// Appends the counts as `work.*` keys (a `None` count is left out).
    pub fn write_meta(&self, m: &mut String) {
        let vals = [
            self.scheduled,
            self.issued,
            self.completed,
            self.late,
            self.rejected,
            self.censored,
        ];
        for (k, v) in WORK_KEYS.iter().zip(vals) {
            if let Some(v) = v {
                let _ = writeln!(m, "{k}={v}");
            }
        }
    }

    /// The counts a `meta.txt` map holds. `Ok(None)` when the run wrote none
    /// of them (a run recorded before the counts were).
    ///
    /// An absent key is "not yet measured": a path that never produced that
    /// count. A key that is present but not a number is **malformed metadata**,
    /// not missing data — the two must not collapse into the same gap, or
    /// `work.scheduled=garbage` is printed beside the percentiles as an honest
    /// hole while the run had reported something unreadable.
    ///
    /// Counts that contradict each other are reported, not accepted: a run
    /// that claims `1 scheduled, 8 issued, 7 completed, 5 censored` is
    /// malformed metadata, and its numbers must not sit beside percentiles.
    pub fn from_meta(m: &HashMap<String, String>) -> Result<Option<Self>, String> {
        let get = |k: &str| -> Result<Option<u64>, String> {
            match m.get(k) {
                None => Ok(None),
                Some(v) => v
                    .parse()
                    .map(Some)
                    .map_err(|_| format!("{k}={v} is not a count")),
            }
        };
        let w = Self {
            scheduled: get(WORK_KEYS[0])?,
            issued: get(WORK_KEYS[1])?,
            completed: get(WORK_KEYS[2])?,
            late: get(WORK_KEYS[3])?,
            rejected: get(WORK_KEYS[4])?,
            censored: get(WORK_KEYS[5])?,
        };
        if [
            w.scheduled,
            w.issued,
            w.completed,
            w.late,
            w.rejected,
            w.censored,
        ]
        .iter()
        .all(Option::is_none)
        {
            return Ok(None);
        }
        w.check()?;
        Ok(Some(w))
    }

    /// The one loader the report paths share: the map's counts, checked
    /// against the records the run wrote to `records.bin`, **before** any
    /// warm-up is dropped. `work.completed` is the sends with a recorded
    /// sample, so it must equal `record_count`; a run that claims more
    /// completions than it recorded is lying about its own sample, and a
    /// shorter sample can read as a faster one.
    pub fn load(m: &HashMap<String, String>, record_count: usize) -> Result<Option<Self>, String> {
        let w = Self::from_meta(m)?;
        if let Some(w) = &w
            && let (Some(c), Ok(n)) = (w.completed, u64::try_from(record_count))
            && c != n
        {
            return Err(format!(
                "work.completed {c} is not the {n} records in records.bin"
            ));
        }
        Ok(w)
    }

    /// The counts as table cells: their values, or "not yet measured".
    #[must_use]
    pub fn cells(&self) -> String {
        let c = |v: Option<u64>| match v {
            Some(n) => n.to_string(),
            None => "not yet measured".to_owned(),
        };
        format!(
            "{} | {} | {} | {} | {} | {}",
            c(self.scheduled),
            c(self.issued),
            c(self.completed),
            c(self.late),
            c(self.rejected),
            c(self.censored)
        )
    }
}

/// A markdown table row: `label | scheduled | issued | completed | late |
/// rejected | censored`.
#[must_use]
pub fn work_row(label: &str, w: &Work) -> String {
    format!("| {label} | {} |", w.cells())
}

/// Header matching [`work_row`].
pub const WORK_HEADER: &str = "| | scheduled | issued | completed | late | rejected | censored |\n\
     |---|---|---|---|---|---|---|";

/// The sender's batch schedule: how far behind its plan the traffic went.
///
/// A different population from [`Work`], and the two must never be printed as
/// one. [`Work::late`] counts the paced **order slots** — a slot filled when
/// the next was already due — and says nothing about the rest of the sender's
/// traffic. This counts **batches** across everything the sender sends: the
/// order connection and the feeds together, one batch per due time, the first
/// message of each. `late_100us` is a batch that began over 100 µs after its
/// due time, so the traffic was not the planned traffic; a saturated sender
/// also slows down with a slow receiver, which then measures a load it was not
/// offered on purpose.
///
/// A run can report `work.late = 2` and 39 464 late batches at once. Only the
/// second says the offered traffic was not the plan.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SendSchedule {
    /// Messages the sender put on the wire.
    pub sent: Option<u64>,
    /// Due times it had traffic for (the first message of each).
    pub batches: Option<u64>,
    /// Batches that began over 100 µs late.
    pub late_100us: Option<u64>,
    /// The worst batch's lateness, in microseconds.
    pub late_max_us: Option<u64>,
}

/// Meta keys of [`SendSchedule`], in report order.
const SEND_KEYS: [&str; 4] = [
    "send.sent",
    "send.batches",
    "send.late_100us",
    "send.late_max_us",
];

impl SendSchedule {
    /// The share of batches that began over 100 µs late, in percent. `None`
    /// when there were no batches to judge.
    #[must_use]
    pub fn late_pct(&self) -> Option<f64> {
        match (self.late_100us, self.batches) {
            (Some(l), Some(b)) if b > 0 => Some(100.0 * l as f64 / b as f64),
            _ => None,
        }
    }

    /// Whether the traffic was the planned traffic. The harness warns past 1%
    /// of batches; this is the same rule, so a table and a run log agree.
    #[must_use]
    pub fn verdict(&self) -> &'static str {
        match self.late_pct() {
            None => "not yet measured",
            Some(p) if p > 1.0 => "over 1%: not the planned traffic",
            Some(_) => "ok",
        }
    }

    /// The counts as table cells: their values, then the verdict.
    #[must_use]
    pub fn cells(&self) -> String {
        let c = |v: Option<u64>| match v {
            Some(n) => n.to_string(),
            None => "not yet measured".to_owned(),
        };
        format!(
            "{} | {} | {} | {} | {}",
            c(self.sent),
            c(self.batches),
            c(self.late_100us),
            c(self.late_max_us),
            self.verdict()
        )
    }

    /// The schedule a `meta.txt` map holds. `Ok(None)` when the run wrote
    /// none of it: a path that has no sender — a receive-only point, or a
    /// client recorded before the counts were.
    ///
    /// Present but partial is **malformed**, not an honest hole. `sent` and
    /// `batches` alone cannot say whether the traffic was the planned
    /// traffic, so reporting them would read as a clean account.
    pub fn from_meta(m: &HashMap<String, String>) -> Result<Option<Self>, String> {
        let get = |k: &str| -> Result<Option<u64>, String> {
            match m.get(k) {
                None => Ok(None),
                Some(v) => v
                    .parse()
                    .map(Some)
                    .map_err(|_| format!("{k}={v} is not a count")),
            }
        };
        if SEND_KEYS.iter().all(|k| !m.contains_key(*k)) {
            return Ok(None);
        }
        // Unreadable first, then incomplete: a run that wrote `late_100us=lots`
        // and forgot `sent` is lying about a number, which outranks the hole.
        let mut vals = [None; 4];
        for (i, k) in SEND_KEYS.iter().enumerate() {
            vals[i] = get(k)?;
        }
        for (i, k) in SEND_KEYS.iter().enumerate() {
            if vals[i].is_none() {
                return Err(format!("{k} is missing: a sender schedule is all or none"));
            }
        }
        Ok(Some(Self {
            sent: vals[0],
            batches: vals[1],
            late_100us: vals[2],
            late_max_us: vals[3],
        }))
    }
}

/// A markdown table row: `label | sent | batches | late >100 µs | latest µs |
/// verdict`, the late count carrying its share of the batches so a reader
/// never has to work out which of two runs was further behind.
#[must_use]
pub fn send_schedule_row(label: &str, s: &SendSchedule) -> String {
    let late = match (s.late_100us, s.late_pct()) {
        (Some(n), Some(p)) => format!("{n} ({p:.2}%)"),
        (Some(n), None) => n.to_string(),
        _ => "not yet measured".to_owned(),
    };
    let c = |v: Option<u64>| match v {
        Some(n) => n.to_string(),
        None => "not yet measured".to_owned(),
    };
    format!(
        "| {label} | {} | {} | {late} | {} | {} |",
        c(s.sent),
        c(s.batches),
        c(s.late_max_us),
        s.verdict()
    )
}

/// Header matching [`send_schedule_row`].
pub const SEND_SCHEDULE_HEADER: &str = "| | sent | batches | late >100 µs | latest µs | verdict |\n\
     |---|---|---|---|---|---|";
