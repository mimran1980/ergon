//! The fair backend comparison (`analyse --fair ROOT`): the runs of
//! `MODE=fair` (`scripts/lib-matrix.sh`), each backend against the others at
//! each connection count, with the run id and the environment on top.
//!
//! Layout: `ROOT/fair-env.txt` (one `key=value` a line) and
//! `ROOT/f<round>-<condition>/<run>`, where a run is `md-<backend>-c<N>`
//! (market data on N connections), `rt-<backend>-n<N>-r<rate>` (the order
//! round trip with N-1 feed connections on the loop) or
//! `http-<backend>-p<P>-r<rate>` (HTTP through a pool of P).
//!
//! Market data across backends is compared on the whole path, sender →
//! decoded. Kernel arrival → decoded is shown only within a family: for the
//! interrupt-driven backends the kernel stamps arrival after the interrupt, so
//! it leaves their interrupt delay out.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::matrix::{Run, Tag};
use crate::stats::{SEND_SCHEDULE_HEADER, Summary, WORK_HEADER, send_schedule_row, us, work_row};

/// The backends in report order: interrupt-driven, then polling.
pub const BACKENDS: [&str; 6] = [
    "epoll-spin",
    "mio",
    "uring-spin",
    "recv-busy",
    "recv-busy-pq",
    "uring-sqpoll",
];

/// `polling` for the backends that poll the NIC themselves, `interrupt` for
/// the rest.
#[must_use]
pub fn family(backend: &str) -> &'static str {
    match backend {
        "recv-busy" | "recv-busy-pq" | "uring-sqpoll" => "polling",
        _ => "interrupt",
    }
}

/// What a fair run measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Market data on `conns` connections.
    Md,
    /// The order round trip on 1 connection, `conns - 1` feeds beside it.
    Rt,
    /// HTTP through a pool of `conns`.
    Http,
}

/// A fair run's name, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    /// What it measured.
    pub kind: Kind,
    /// The backend.
    pub backend: String,
    /// Connections: market data's, N for the round trip, the HTTP pool.
    pub conns: u32,
    /// Round trips or requests a second (0: back to back); none for market data.
    pub rate: Option<i64>,
}

/// Parses `md-<b>-c<N>`, `rt-<b>-n<N>-r<rate>` or `http-<b>-p<P>-r<rate>`.
#[must_use]
pub fn parse_run(name: &str) -> Option<Point> {
    let (kind, rest) = if let Some(r) = name.strip_prefix("md-") {
        (Kind::Md, r)
    } else if let Some(r) = name.strip_prefix("rt-") {
        (Kind::Rt, r)
    } else {
        (Kind::Http, name.strip_prefix("http-")?)
    };
    let (rest, rate) = if kind == Kind::Md {
        (rest, None)
    } else {
        let (r, rate) = rest.rsplit_once("-r")?;
        (r, Some(rate.parse().ok()?))
    };
    let mark = match kind {
        Kind::Md => "-c",
        Kind::Rt => "-n",
        Kind::Http => "-p",
    };
    let (backend, conns) = rest.rsplit_once(mark)?;
    if backend.is_empty() {
        return None;
    }
    Some(Point {
        kind,
        backend: backend.to_owned(),
        conns: conns.parse().ok()?,
        rate,
    })
}

/// Parses a fair tag `f<round>-<condition>`.
#[must_use]
pub fn parse_tag(name: &str) -> Option<(u32, &str)> {
    let (round, cond) = name.strip_prefix('f')?.split_once('-')?;
    Some((round.parse().ok()?, cond))
}

/// `fair-env.txt` as key → value, in file order.
fn env(root: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(root.join("fair-env.txt"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

fn cond_order(c: &str) -> (u8, &str) {
    match c {
        "irqcore" => (0, c),
        "irqlocal" => (1, c),
        _ => (2, c),
    }
}

/// One row's cells: n, p50, p99, p99.9, max.
fn cells(s: &Summary) -> String {
    format!(
        "{} | {} | {} | {} | {}",
        s.n,
        us(s.p50),
        us(s.p99),
        us(s.p999),
        us(s.max)
    )
}

/// The work behind one comparison, beside it: a backend row's percentiles are
/// over `completed` alone and drop the warm-up, so `n` is not the run's work.
/// Every run in the comparison gets a row; one recorded before the counts were
/// is "not yet measured" in every column, never zeros.
///
/// Two accounts, never printed as one. `work.late` counts the paced **order
/// slots** filled when the next slot was already due. The sender's schedule
/// counts **batches** across all the traffic it had to send, and says whether
/// the offered load was the load the run planned. A comparison whose rows
/// disagree on that is not a like-for-like ranking, so it names those rows.
fn work_beside(out: &mut String, rows: &[(usize, u32, String, &str, &Run)]) {
    if rows.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "### Work behind the comparison\n\n\
         Every percentile above is over `completed` alone, and the tables drop the warm-up, so `n` is \
         smaller still. `censored` is the issued work they leave out. `scheduled − issued` is work the \
         schedule called for that never went out (early termination, or a schedule that outran the \
         client). `rejected` is send attempts the transport refused: capacity exhaustion. `not yet \
         measured` is a count this run did not record, never a zero.\n\n\
         {WORK_HEADER}"
    );
    let unmeasured = "not yet measured | ".repeat(6);
    for (_, _, _, name, r) in rows {
        let label = format!("`{name}`");
        match r.work {
            Some(w) => {
                let _ = writeln!(out, "{}", work_row(&label, &w));
            }
            None => {
                let _ = writeln!(out, "| {label} | {unmeasured}");
            }
        }
    }
    let _ = writeln!(out);

    // The second account. `late` above counts the order slots — **not the
    // order slots'** share of the traffic, which is what this measures.
    let _ = writeln!(
        out,
        "### Sender schedule behind the comparison\n\n\
         `late >100 µs` counts the **batches** the sender began over 100 µs after their due time, \
         across all the traffic it had to send (the order connection and the feeds together, the \
         first message of each due time) — **not the order slots** that `late` above counts. A run \
         can report `late = 2` and 39 464 late batches at once. Traffic sent late is not the traffic \
         planned (a saturated sender also slows down with a slow receiver), so the receiver measures \
         a load the run was not offered on purpose. `not yet measured` is a run with no sender \
         account, never a zero.\n\n\
         {SEND_SCHEDULE_HEADER}"
    );
    let mut slipped: Vec<String> = Vec::new();
    for (_, _, _, name, r) in rows {
        let label = format!("`{name}`");
        let s = r.send.unwrap_or_default();
        let _ = writeln!(out, "{}", send_schedule_row(&label, &s));
        if s.verdict().starts_with("over 1%") {
            slipped.push(label);
        }
    }
    let _ = writeln!(out);
    if !slipped.is_empty() {
        let _ = writeln!(
            out,
            "Not the planned traffic: {}. Do not treat those rows as a like-for-like ranking.\n",
            slipped.join(", ")
        );
    }
}

/// Every point of a kind: (condition, connections, rate) → rows of
/// (backend order, round, backend, run name, run).
type Groups<'a> =
    BTreeMap<((u8, String), u32, Option<i64>), Vec<(usize, u32, String, &'a str, &'a Run)>>;

fn groups<'a>(tags: &'a [Tag], kind: Kind) -> Groups<'a> {
    let mut g: Groups<'a> = BTreeMap::new();
    for t in tags {
        let Some((round, cond)) = parse_tag(&t.name) else {
            continue;
        };
        for (name, run) in &t.runs {
            let Some(p) = parse_run(name) else { continue };
            if p.kind != kind {
                continue;
            }
            let order = BACKENDS
                .iter()
                .position(|b| *b == p.backend)
                .unwrap_or(BACKENDS.len());
            let (o, c) = cond_order(cond);
            g.entry(((o, c.to_owned()), p.conns, p.rate))
                .or_default()
                .push((order, round, p.backend, name.as_str(), run));
        }
    }
    for rows in g.values_mut() {
        rows.sort_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));
    }
    g
}

fn conns_text(kind: Kind, n: u32) -> String {
    match kind {
        Kind::Md if n == 1 => "1 connection".to_owned(),
        Kind::Md => format!("{n} connections"),
        Kind::Rt if n == 1 => "N = 1 (the order connection alone)".to_owned(),
        Kind::Rt => format!("N = {n} (1 order connection + {} feed connections)", n - 1),
        Kind::Http if n == 1 => "pool of 1 connection".to_owned(),
        Kind::Http => format!("pool of {n} connections"),
    }
}

fn rate_text(rate: Option<i64>) -> String {
    match rate {
        None => String::new(),
        Some(0) => ", back to back".to_owned(),
        Some(r) => format!(", {r}/s"),
    }
}

/// The fair report; `skipped` lists runs left out (failed points first).
#[must_use]
pub fn report(root: &Path, tags: &[Tag], skipped: &[(String, String)]) -> String {
    let mut out = String::new();
    let env = env(root);
    let get = |k: &str| {
        env.iter()
            .find(|(key, _)| key == k)
            .map_or("unknown", |(_, v)| v.as_str())
    };
    let _ = writeln!(
        out,
        "# Fair backend comparison\n\n\
         - **Run:** {} ({} {}, {} priority)\n\
         - **Environment:** receiver {} ({}, {}), sender {}, kernel {}, image {}\n\
         - **Receiver CPUs:** layout {}; housekeeping {}, NIC interrupts {}, loop {}, SQPOLL {}, spare {}; boot profile {}\n\
         - **Every figure:** µs, as n (samples) | p50 | p99 | p99.9 | max, per round; the connection count is in each heading.\n",
        get("run"),
        get("cloud"),
        get("region"),
        get("priority"),
        get("receiver"),
        get("receiver_cpu"),
        get("arch"),
        get("sender"),
        get("kernel"),
        get("image"),
        get("layout"),
        get("housekeeping_cpu"),
        get("irq_cpu"),
        get("loop_cpu"),
        get("sqpoll_cpu"),
        get("spare_cpu"),
        get("receiver_boot"),
    );
    let shared: Vec<&str> = env
        .iter()
        .filter(|(k, _)| k.starts_with("setting."))
        .map(|(_, v)| v.as_str())
        .collect();
    if !shared.is_empty() {
        let _ = writeln!(
            out,
            "**Settings every backend shares** (held, and checked before and after every point):\n"
        );
        for s in shared {
            let _ = writeln!(out, "- {s}");
        }
        let _ = writeln!(out);
    }
    let held: Vec<String> = env
        .iter()
        .filter(|(k, _)| k.starts_with("receiver.") || k.starts_with("sender."))
        .map(|(k, v)| format!("`{k}={v}`"))
        .collect();
    if !held.is_empty() {
        let _ = writeln!(out, "**As found at the start:** {}\n", held.join(", "));
    }
    let own: Vec<(&str, &str)> = env
        .iter()
        .filter_map(|(k, v)| Some((k.strip_prefix("backend.")?, v.as_str())))
        .collect();
    if !own.is_empty() {
        let _ = writeln!(out, "**What each backend sets for itself:**\n");
        for (b, d) in own {
            let _ = writeln!(out, "- `{b}`: {d}");
        }
        let _ = writeln!(out);
    }
    let (failed, other): (Vec<_>, Vec<_>) = skipped
        .iter()
        .partition(|(_, why)| why.starts_with("failed:"));
    if !failed.is_empty() {
        let _ = writeln!(
            out,
            "**Failed points, left out** (a setting drifted, or the run did not finish):\n"
        );
        for (run, why) in failed {
            let _ = writeln!(out, "- `{run}`: {why}");
        }
        let _ = writeln!(out);
    }
    if !other.is_empty() {
        let _ = writeln!(out, "**Runs that could not be loaded:**\n");
        for (run, why) in other {
            let _ = writeln!(out, "- `{run}`: {why}");
        }
        let _ = writeln!(out);
    }
    market_data(&mut out, tags);
    round_trips(&mut out, tags, Kind::Rt);
    round_trips(&mut out, tags, Kind::Http);
    out
}

fn market_data(out: &mut String, tags: &[Tag]) {
    let g = groups(tags, Kind::Md);
    if g.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "## Market data, sender → decoded: every backend against the others\n\n\
         The whole path: the sender's send time to decoded, NIC, interrupt and loop included. Two hosts' clocks, so \
         each value includes their offset (the bound column: chrony's estimate); the backends of one table ran \
         minutes apart, so compare them with each other, not with other tables.\n"
    );
    for (((_, cond), n, rate), rows) in &g {
        let _ = writeln!(
            out,
            "### {cond}, {}{}\n\n| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |\n|---|---|---|---|---|---|---|---|---|",
            conns_text(Kind::Md, *n),
            rate_text(*rate)
        );
        for (_, round, b, _, run) in rows {
            let Some(s) = run.one_way.as_ref() else {
                continue;
            };
            let bound = run.clock_bound_ns.map_or_else(|| "unknown".to_owned(), us);
            let _ = writeln!(
                out,
                "| `{b}` | {} | {round} | {} | {bound} |",
                family(b),
                cells(s)
            );
        }
        let _ = writeln!(out);
        work_beside(out, rows);
    }
    let _ = writeln!(
        out,
        "## Market data, kernel arrival → decoded: within a family only\n\n\
         One clock (the receiver's), so exact, but not comparable across families: for an interrupt-driven \
         backend the kernel stamps arrival after the interrupt, so this leaves its interrupt delay out. mio reads \
         without kernel timestamps and has no row here.\n"
    );
    for fam in ["polling", "interrupt"] {
        for (((_, cond), n, _), rows) in &g {
            let rows: Vec<_> = rows
                .iter()
                .filter(|(_, _, b, _, r)| family(b) == fam && r.latency.is_some())
                .collect();
            if rows.is_empty() {
                continue;
            }
            let _ = writeln!(
                out,
                "### {fam} backends, {cond}, {}\n\n| backend | round | n | p50 | p99 | p99.9 | max |\n|---|---|---|---|---|---|---|",
                conns_text(Kind::Md, *n)
            );
            for (_, round, b, _, run) in rows {
                if let Some(s) = run.latency.as_ref() {
                    let _ = writeln!(out, "| `{b}` | {round} | {} |", cells(s));
                }
            }
            let _ = writeln!(out);
        }
    }
}

fn round_trips(out: &mut String, tags: &[Tag], kind: Kind) {
    let g = groups(tags, kind);
    if g.is_empty() {
        return;
    }
    let (title, intro) = if kind == Kind::Rt {
        (
            "The WebSocket order round trip with feed connections on the same loop",
            "One order connection, echoed by the server, while N-1 more connections on the same loop carry the \
             market data (decoded, not timed). One clock (the receiver's): send to echo decoded. \"From due\" times \
             each paced round trip from its slot on the schedule, so it includes any wait behind a stall.",
        )
    } else {
        (
            "The HTTP order round trip through a pool",
            "One request in flight, each on the pool's next connection in turn, every pooled connection on the \
             loop; the server answers every connection from one thread. One clock (the receiver's). \"From due\" \
             as above.",
        )
    };
    let _ = writeln!(out, "## {title}\n\n{intro}\n");
    for (((_, cond), n, rate), rows) in &g {
        let _ = writeln!(
            out,
            "### {cond}, {}{}\n\n| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |\n|---|---|---|---|---|---|---|---|---|---|",
            conns_text(kind, *n),
            rate_text(*rate)
        );
        for (_, round, b, _, run) in rows {
            let Some(s) = run.latency.as_ref() else {
                continue;
            };
            let due = run.due.as_ref().map_or_else(
                || "– | –".to_owned(),
                |d| format!("{} | {}", us(d.p99), us(d.max)),
            );
            let _ = writeln!(
                out,
                "| `{b}` | {} | {round} | {} | {due} |",
                family(b),
                cells(s)
            );
        }
        let _ = writeln!(out);
        work_beside(out, rows);
    }
}
