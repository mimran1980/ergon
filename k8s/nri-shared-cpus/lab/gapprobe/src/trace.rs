//! The kernel-trace report: what ran on, interrupted or was sent to the
//! probe's CPU during a spin, from the text of tracefs `trace` (events with
//! the `mono` trace clock), joined to the spin's gap log by
//! `CLOCK_REALTIME`.
//!
//! The lab enables, filtered to the probe's CPU N: `sched_switch` (on N),
//! `sched_wakeup`/`sched_wakeup_new`/`sched_waking` (target N), and
//! `sched_migrate_task` (to or from N); `ipi_send_cpu`, `ipi_send_cpumask`,
//! `ipi_raise` and `csd_queue_cpu` (sent to N, from any CPU, each with a
//! `stacktrace` trigger: the sender's kernel stack); `ipi_entry`,
//! `irq_handler_entry`, `softirq_entry`, `csd_function_entry`,
//! `hrtimer_expire_entry`, `timer_expire_entry`, `tick_stop` and
//! `workqueue_execute_start` (on N); `workqueue_queue_work` (queued for N,
//! with a stack). This module reads whichever of them the trace holds.

use std::fmt::Write as _;

pub use crate::procfs::ParseError;

/// One trace event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Event {
    /// the task the event was recorded in
    pub comm: String,
    /// its pid (thread id)
    pub pid: u32,
    /// the CPU it was recorded on
    pub cpu: usize,
    /// trace clock (`mono`) ns; microsecond resolution in the text
    pub ts_ns: u64,
    /// `sched_switch`, `ipi_send_cpu`, ...
    pub name: String,
    /// what follows `name: `
    pub body: String,
    /// the kernel stack a `stacktrace` trigger recorded with it, innermost first
    pub stack: Vec<String>,
}

/// The text of tracefs `trace`, parsed: the events, and any `[LOST n
/// EVENTS]` notice (a buffer that overran: the report says so first).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trace {
    /// in file order (time order)
    pub events: Vec<Event>,
    /// `(cpu, events lost)` for each notice
    pub lost: Vec<(usize, u64)>,
}

/// `sec.usec` as ns.
fn ts(s: &str) -> Option<u64> {
    let (a, b) = s.split_once('.')?;
    let sec: u64 = a.parse().ok()?;
    let frac: u64 = b.parse().ok()?;
    let scale = 10_u64.checked_pow(9_u32.checked_sub(u32::try_from(b.len()).ok()?)?)?;
    Some(sec * 1_000_000_000 + frac * scale)
}

/// `<comm>-<pid> [<cpu>] [flags] <sec.usec>: <rest>`.
fn event_line(line: &str) -> Option<Event> {
    let open = line.find(" [")?;
    let close = open + line.get(open..)?.find(']')?;
    let cpu: usize = line.get(open + 2..close)?.trim().parse().ok()?;
    let who = line.get(..open)?.trim();
    let dash = who.rfind('-')?;
    let comm = who.get(..dash)?.to_string();
    let pid: u32 = who.get(dash + 1..)?.parse().ok()?;
    let mut rest = line.get(close + 1..)?.trim_start();
    // the irq-info flags column, when there is one
    let first = rest.split_whitespace().next()?;
    if ts(first.trim_end_matches(':')).is_none() || !first.ends_with(':') {
        rest = rest.get(first.len()..)?.trim_start();
    }
    let (t, rest) = rest.split_once(": ")?;
    let ts_ns = ts(t.trim())?;
    let rest = rest.trim_start();
    if rest.starts_with("<stack trace>") {
        return Some(Event {
            comm,
            pid,
            cpu,
            ts_ns,
            name: "<stack trace>".into(),
            ..Event::default()
        });
    }
    let (name, body) = rest.split_once(':').unwrap_or((rest, ""));
    Some(Event {
        comm,
        pid,
        cpu,
        ts_ns,
        name: name.trim().to_string(),
        body: body.trim().to_string(),
        stack: Vec::new(),
    })
}

/// `CPU:3 [LOST 12 EVENTS]`
fn lost_line(line: &str) -> Option<(usize, u64)> {
    let l = line.trim();
    let cpu = l
        .strip_prefix("CPU:")?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    let n = l
        .split("[LOST ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some((cpu, n))
}

/// Parses tracefs `trace` text. Comment lines are skipped; a `<stack
/// trace>` entry and its ` => frame` lines go to the latest event recorded
/// on the same CPU (the trigger writes the stack right after its event).
///
/// # Errors
/// A line that is neither a comment, an event, a stack frame nor a lost
/// notice.
pub fn parse(text: &str) -> Result<Trace, ParseError> {
    let mut t = Trace::default();
    // the event a stack being read belongs to
    let mut stack_of: Option<usize> = None;
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(f) = line.trim_start().strip_prefix("=> ") {
            if let Some(e) = stack_of.and_then(|k| t.events.get_mut(k)) {
                let name = f.split('+').next().unwrap_or(f).trim();
                e.stack.push(name.to_string());
            }
            continue;
        }
        if let Some(l) = lost_line(line) {
            t.lost.push(l);
            stack_of = None;
            continue;
        }
        let e = event_line(line)
            .ok_or_else(|| ParseError(format!("trace line {}: {line:?}", i + 1)))?;
        if e.name == "<stack trace>" {
            stack_of = t.events.iter().rposition(|x| x.cpu == e.cpu);
        } else {
            stack_of = None;
            t.events.push(e);
        }
    }
    Ok(t)
}

/// The value of `key=` in an event body: up to the next space, or for a
/// `comm`-like key up to the next ` <word>=` (a comm may hold spaces).
#[must_use]
pub fn field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("{key}=");
    let mut from = 0;
    let start = loop {
        let at = from + body.get(from..)?.find(&pat)?;
        if at == 0 || body.as_bytes().get(at - 1) == Some(&b' ') {
            break at + pat.len();
        }
        from = at + pat.len();
    };
    let rest = body.get(start..)?;
    let end = if key.ends_with("comm") {
        // the next " key=" or " ==>"
        rest.match_indices(' ')
            .map(|(i, _)| i)
            .find(|&i| {
                let after = rest.get(i + 1..).unwrap_or("");
                after.starts_with("==>")
                    || after
                        .split_whitespace()
                        .next()
                        .is_some_and(|w| w.contains('=') && !w.starts_with('='))
            })
            .unwrap_or(rest.len())
    } else {
        rest.find(' ').unwrap_or(rest.len())
    };
    rest.get(..end)
}

/// Whether CPU `cpu` is in a cpulist (`2`, `0-3`, `1,4-6`).
#[must_use]
pub fn in_cpulist(list: &str, cpu: usize) -> Option<bool> {
    let mut any = false;
    for part in list.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (a, b) = part.split_once('-').unwrap_or((part, part));
        let (a, b): (usize, usize) = (a.parse().ok()?, b.parse().ok()?);
        any |= (a..=b).contains(&cpu);
    }
    Some(any)
}

/// Whether CPU `cpu` is in a hex bitmask (`00000000,00000004`).
#[must_use]
pub fn in_hexmask(mask: &str, cpu: usize) -> Option<bool> {
    let words: Vec<&str> = mask.split(',').collect();
    let word = cpu / 32;
    let w = words.len().checked_sub(word + 1).and_then(|i| words.get(i));
    match w {
        Some(w) => Some(u32::from_str_radix(w.trim(), 16).ok()? & (1 << (cpu % 32)) != 0),
        None => Some(false),
    }
}

/// A gap from `gaps.csv`: `CLOCK_REALTIME` ns it ended, and its length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gap {
    /// when the loop saw it end
    pub end_ns: u64,
    /// its length
    pub gap_ns: u64,
}

/// Parses `end_realtime_ns,gap_ns` lines (a header line is skipped).
///
/// # Errors
/// A line that is not two numbers.
pub fn parse_gaps(csv: &str) -> Result<Vec<Gap>, ParseError> {
    let mut out = Vec::new();
    for (i, l) in csv.lines().enumerate() {
        if l.is_empty() || l.starts_with("end_") {
            continue;
        }
        let (a, b) = l
            .split_once(',')
            .ok_or_else(|| ParseError(format!("gaps line {}: {l:?}", i + 1)))?;
        match (a.trim().parse(), b.trim().parse()) {
            (Ok(end_ns), Ok(gap_ns)) => out.push(Gap { end_ns, gap_ns }),
            _ => return Err(ParseError(format!("gaps line {}: {l:?}", i + 1))),
        }
    }
    Ok(out)
}

/// What the report is about.
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// the parsed trace
    pub trace: Trace,
    /// the probe's CPU
    pub cpu: usize,
    /// the spin thread's host tid; `None`: the `gp-hot` comm in `sched_switch`
    pub hot_tid: Option<u32>,
    /// `CLOCK_REALTIME` - `CLOCK_MONOTONIC`, ns, read as the trace started
    pub offset_ns: i128,
    /// the same, read as it stopped (NTP slews realtime), if read
    pub offset_end_ns: Option<i128>,
    /// the spin, `CLOCK_REALTIME` ns (0, 0: the whole trace)
    pub window: (u64, u64),
    /// the spin's gaps
    pub gaps: Vec<Gap>,
    /// the spin's involuntary context switches from `/proc` (`nivcsw`), if known
    pub nivcsw: Option<u64>,
    /// gaps at or above this are each listed with what overlapped them
    pub min_gap_ns: u64,
    /// slack around a gap when matching events to it (the text has µs resolution)
    pub slack_ns: u64,
    /// events the lab enabled, and the ones the kernel did not have
    pub enabled: Vec<String>,
    /// events asked for that the kernel did not have
    pub missing: Vec<String>,
}

/// An event of interest on, or sent to, the probe's CPU.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Item {
    /// `CLOCK_REALTIME` ns
    at: u64,
    /// one-line description, the cause when it names one
    what: String,
}

/// A symbol without its `+0x../0x..` offset.
fn sym(s: &str) -> &str {
    s.split('+').next().unwrap_or(s)
}

fn rt(e: &Event, offset: i128) -> u64 {
    u64::try_from(i128::from(e.ts_ns) + offset).unwrap_or(0)
}

fn us(ns: u64) -> String {
    format!("{}.{:02}", ns / 1000, (ns % 1000) / 10)
}

/// The first frames of a stack, past the tracing ones.
fn frames(stack: &[String], n: usize) -> String {
    let s: Vec<&str> = stack
        .iter()
        .map(String::as_str)
        .filter(|f| {
            !f.starts_with("trace_")
                && !f.starts_with("__traceiter")
                && !f.starts_with("__trace_")
                && *f != "event_triggers_call"
                && !f.starts_with("stacktrace_trigger")
                && !f.starts_with("__ftrace_trace_stack")
                && !f.starts_with("__event_trigger")
        })
        .take(n)
        .collect();
    if s.is_empty() {
        "no stack".into()
    } else {
        s.join(" < ")
    }
}

/// A sent IPI or queued cross-call: is it aimed at `cpu`? (`None`: the
/// event is not one; the kernel filter is trusted when the target cannot
/// be read.)
fn sent_to(e: &Event, cpu: usize) -> Option<bool> {
    match e.name.as_str() {
        "ipi_send_cpu" | "csd_queue_cpu" => Some(
            field(&e.body, "cpu")
                .and_then(|c| c.parse::<usize>().ok())
                .is_none_or(|c| c == cpu),
        ),
        "ipi_send_cpumask" => Some(
            field(&e.body, "cpumask")
                .and_then(|m| in_cpulist(m, cpu))
                .unwrap_or(true),
        ),
        "ipi_raise" => Some(
            field(&e.body, "target_mask")
                .and_then(|m| in_hexmask(m, cpu))
                .unwrap_or(true),
        ),
        _ => None,
    }
}

/// A short name of an event on the probe's CPU, for the gap list.
fn on_cpu_label(e: &Event) -> Option<String> {
    let f = |k: &str| field(&e.body, k).unwrap_or("?").to_string();
    Some(match e.name.as_str() {
        "irq_handler_entry" => format!("irq {} ({})", f("name"), f("irq")),
        "softirq_entry" => format!(
            "softirq {}",
            e.body
                .split_once("action=")
                .map_or(e.body.as_str(), |x| x.1.trim_end_matches(']'))
        ),
        "ipi_entry" => format!("ipi {}", e.body.trim_matches(|c| c == '(' || c == ')')),
        "csd_function_entry" => format!("cross-call {}", sym(&f("func"))),
        "hrtimer_expire_entry" => format!("hrtimer {}", sym(&f("function"))),
        "timer_expire_entry" => format!("timer {}", sym(&f("function"))),
        "tick_stop" => format!("tick_stop {}", e.body),
        "workqueue_execute_start" => {
            format!("work {}", e.body.rsplit("function ").next().unwrap_or("?"))
        }
        "sched_switch" => format!("switch {} -> {}", f("prev_comm"), f("next_comm")),
        _ => return None,
    })
}

/// The report: markdown, every cause named that the trace can name.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render(input: &Input) -> String {
    let cpu = input.cpu;
    let off = input.offset_ns;
    let (w0, w1) = input.window;
    let inside = |e: &Event| {
        let t = rt(e, off);
        (w0 == 0 && w1 == 0) || (t >= w0 && t <= w1)
    };
    let ev: Vec<&Event> = input.trace.events.iter().filter(|e| inside(e)).collect();
    // the spin thread: the tid given when the trace shows it on the CPU;
    // else the `gp-hot` thread the trace shows (a tid from another pid
    // namespace does not match); else the tid given (a trace with no
    // switch of it at all)
    let switch_pids = |e: &Event| -> [Option<u32>; 2] {
        [
            field(&e.body, "prev_pid").and_then(|p| p.parse().ok()),
            field(&e.body, "next_pid").and_then(|p| p.parse().ok()),
        ]
    };
    let on_cpu: Vec<&&Event> = ev
        .iter()
        .filter(|e| e.name == "sched_switch" && e.cpu == cpu)
        .collect();
    let by_comm = on_cpu.iter().find_map(|e| {
        if field(&e.body, "prev_comm") == Some("gp-hot") {
            switch_pids(e)[0]
        } else if field(&e.body, "next_comm") == Some("gp-hot") {
            switch_pids(e)[1]
        } else {
            None
        }
    });
    let seen = |t: u32| on_cpu.iter().any(|e| switch_pids(e).contains(&Some(t)));
    let (hot, hot_said) = match (input.hot_tid, by_comm) {
        (Some(t), _) if seen(t) => (Some(t), format!("tid {t} (host)")),
        (Some(t), Some(c)) if c != t => (
            Some(c),
            format!("tid {c} (gp-hot in the trace; the tid given, {t}, is not in it)"),
        ),
        (Some(t), _) => (Some(t), format!("tid {t} (host)")),
        (None, Some(c)) => (Some(c), format!("tid {c} (gp-hot in the trace)")),
        (None, None) => (None, "not found in the trace".to_string()),
    };
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# CPU {cpu}: what ran on it, interrupted it or was sent to it\n"
    );
    let _ = writeln!(out, "| item | value |\n|---|---|");
    let _ = writeln!(out, "| spin thread | {hot_said} |");
    let _ = writeln!(
        out,
        "| window (CLOCK_REALTIME ns) | {} |",
        if w0 == 0 && w1 == 0 {
            "the whole trace".to_string()
        } else {
            format!(
                "{w0} to {w1} ({} s)",
                (w1.saturating_sub(w0)) / 1_000_000_000
            )
        }
    );
    let _ = writeln!(
        out,
        "| clock | trace `mono` + {off} ns = CLOCK_REALTIME ({}); the text has µs resolution, events are matched to gaps within {} |",
        input.offset_end_ns.map_or_else(
            || "read at the start only".to_string(),
            |e| format!("{} ns of drift by the end", e - off)
        ),
        us(input.slack_ns) + " µs"
    );
    let _ = writeln!(
        out,
        "| events enabled | {} |",
        if input.enabled.is_empty() {
            "not listed".to_string()
        } else {
            input.enabled.join(", ")
        }
    );
    if !input.missing.is_empty() {
        let _ = writeln!(
            out,
            "| events this kernel lacks | {} |",
            input.missing.join(", ")
        );
    }
    let lost: u64 = input.trace.lost.iter().map(|l| l.1).sum();
    let _ = writeln!(
        out,
        "| events lost | {} |",
        if lost == 0 {
            "none".to_string()
        } else {
            format!("**{lost}** (buffer overran: the lists below are incomplete)")
        }
    );

    // switches away from the spin thread
    let mut switches: Vec<(u64, &Event, bool, Option<u64>)> = Vec::new();
    for (i, e) in ev.iter().enumerate() {
        if e.name != "sched_switch" || e.cpu != cpu {
            continue;
        }
        let prev: Option<u32> = field(&e.body, "prev_pid").and_then(|p| p.parse().ok());
        if hot.is_none() || prev != hot {
            continue;
        }
        let involuntary = field(&e.body, "prev_state").is_some_and(|s| s.starts_with('R'));
        let back = ev.get(i + 1..).unwrap_or(&[]).iter().find(|x| {
            x.name == "sched_switch"
                && x.cpu == cpu
                && field(&x.body, "next_pid").and_then(|p| p.parse().ok()) == hot
        });
        switches.push((rt(e, off), e, involuntary, back.map(|b| rt(b, off))));
    }
    let invol = switches.iter().filter(|s| s.2).count();
    let _ = writeln!(
        out,
        "| involuntary switches of the spin thread | {invol} in the trace; {} from /proc nonvoluntary_ctxt_switches |",
        input
            .nivcsw
            .map_or_else(|| "not read".to_string(), |n| n.to_string())
    );
    let _ = writeln!(out, "| voluntary switches | {} |", switches.len() - invol);

    let wakeup_of = |pid: &str, before: u64| -> Option<&Event> {
        ev.iter()
            .filter(|w| {
                matches!(
                    w.name.as_str(),
                    "sched_wakeup" | "sched_wakeup_new" | "sched_waking"
                ) && field(&w.body, "pid") == Some(pid)
                    && rt(w, off) <= before
            })
            .max_by_key(|w| (w.ts_ns, w.name == "sched_waking"))
            .copied()
    };
    let gap_over = |a: u64, b: u64| -> Option<Gap> {
        input
            .gaps
            .iter()
            .filter(|g| {
                let gs = g.end_ns.saturating_sub(g.gap_ns);
                g.end_ns + input.slack_ns >= a && gs <= b + input.slack_ns
            })
            .max_by_key(|g| g.gap_ns)
            .copied()
    };

    let _ = writeln!(out, "\n## Switches away from the spin thread\n");
    if switches.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let _ = writeln!(
            out,
            "| at (CLOCK_REALTIME ns) | kind | prev_state | ran instead (comm/pid) | work it ran | off CPU µs | woken by (comm/pid on CPU) | waker's stack | gap µs |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|");
        for (at, e, involuntary, back) in &switches {
            let next = field(&e.body, "next_comm").unwrap_or("?");
            let next_pid = field(&e.body, "next_pid").unwrap_or("?");
            let w = wakeup_of(next_pid, *at);
            let gap = gap_over(*at, back.unwrap_or(*at));
            let until = back.unwrap_or(u64::MAX);
            let mut ran: Vec<String> = Vec::new();
            for x in ev.iter().filter(|x| {
                x.name == "workqueue_execute_start"
                    && x.cpu == cpu
                    && x.pid.to_string() == next_pid
                    && rt(x, off) >= *at
                    && rt(x, off) <= until
            }) {
                let f = x.body.rsplit("function ").next().unwrap_or("?").to_string();
                if !ran.contains(&f) {
                    ran.push(f);
                }
            }
            let woken = w.map_or_else(
                || "no wakeup in the trace".into(),
                |w| {
                    if Some(w.pid) == hot && w.cpu == cpu {
                        format!(
                            "a timer or interrupt on CPU {cpu}, in the spin thread's context ({})",
                            w.name
                        )
                    } else {
                        format!("{}/{} on CPU {} ({})", w.comm, w.pid, w.cpu, w.name)
                    }
                },
            );
            let _ = writeln!(
                out,
                "| {at} | {} | {} | {next}/{next_pid} | {} | {} | {woken} | {} | {} |",
                if *involuntary {
                    "involuntary"
                } else {
                    "voluntary"
                },
                field(&e.body, "prev_state").unwrap_or("?"),
                if ran.is_empty() {
                    "-".to_string()
                } else {
                    ran.join(", ")
                },
                back.map_or_else(
                    || "not back in the trace".into(),
                    |b| us(b.saturating_sub(*at))
                ),
                w.map_or_else(|| "-".into(), |w| frames(&w.stack, 6)),
                gap.map_or_else(|| "no gap logged".into(), |g| us(g.gap_ns)),
            );
        }
    }

    // IPIs and cross-calls sent to the CPU
    let sent: Vec<&&Event> = ev
        .iter()
        .filter(|e| sent_to(e, cpu) == Some(true))
        .collect();
    let _ = writeln!(out, "\n## IPIs and cross-calls sent to CPU {cpu}\n");
    if sent.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let _ = writeln!(
            out,
            "| at (CLOCK_REALTIME ns) | event | from CPU | sender (comm/pid) | callsite | callback | sender's stack |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|---|---|");
        for e in &sent {
            let reason = if e.name == "ipi_raise" {
                e.body
                    .split_once('(')
                    .map_or("", |x| x.1)
                    .trim_end_matches(')')
                    .to_string()
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {}/{} | {} | {} | {} |",
                rt(e, off),
                e.name,
                e.cpu,
                e.comm,
                e.pid,
                field(&e.body, "callsite").map_or(reason, |c| sym(c).to_string()),
                field(&e.body, "callback")
                    .or_else(|| field(&e.body, "func"))
                    .map_or("-", sym),
                frames(&e.stack, 8),
            );
        }
        // by callsite
        let mut by: Vec<(String, usize)> = Vec::new();
        for e in &sent {
            let k = format!(
                "{} {} from {}",
                e.name,
                field(&e.body, "callsite")
                    .or_else(|| field(&e.body, "func"))
                    .map_or("", sym),
                frames(&e.stack, 3)
            );
            match by.iter_mut().find(|x| x.0 == k) {
                Some(x) => x.1 += 1,
                None => by.push((k, 1)),
            }
        }
        by.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let _ = writeln!(out, "\nBy sender path:\n");
        for (k, n) in by {
            let _ = writeln!(out, "- {n} x {k}");
        }
    }

    // work queued for the CPU, and run there
    let _ = writeln!(out, "\n## Work items run on CPU {cpu}\n");
    let works: Vec<&&Event> = ev
        .iter()
        .filter(|e| e.name == "workqueue_execute_start" && e.cpu == cpu)
        .collect();
    if works.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let _ = writeln!(
            out,
            "| at (CLOCK_REALTIME ns) | function | worker | queued by (comm/pid on CPU) | queuer's stack |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|");
        for e in &works {
            let ptr = e
                .body
                .split_whitespace()
                .nth(2)
                .map(|p| p.trim_end_matches(':'));
            let q = ev
                .iter()
                .filter(|q| {
                    q.name == "workqueue_queue_work"
                        && q.ts_ns <= e.ts_ns
                        && ptr.is_some()
                        && field(&q.body, "struct") == ptr
                })
                .max_by_key(|q| q.ts_ns);
            let _ = writeln!(
                out,
                "| {} | {} | {}/{} | {} | {} |",
                rt(e, off),
                e.body.rsplit("function ").next().unwrap_or("?"),
                e.comm,
                e.pid,
                q.map_or_else(
                    || "not in the trace".into(),
                    |q| format!("{}/{} on CPU {}", q.comm, q.pid, q.cpu)
                ),
                q.map_or_else(|| "-".into(), |q| frames(&q.stack, 6)),
            );
        }
    }

    // timers, ticks, interrupts on the CPU, counted
    let _ = writeln!(out, "\n## Timers, ticks and interrupts on CPU {cpu}\n");
    let mut counts: Vec<(String, usize)> = Vec::new();
    for e in ev
        .iter()
        .filter(|e| e.cpu == cpu && e.name != "sched_switch" && e.name != "workqueue_execute_start")
    {
        if let Some(l) = on_cpu_label(e) {
            match counts.iter_mut().find(|x| x.0 == l) {
                Some(x) => x.1 += 1,
                None => counts.push((l, 1)),
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if counts.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let _ = writeln!(out, "| event | count |\n|---|---|");
        for (l, n) in counts {
            let _ = writeln!(out, "| {l} | {n} |");
        }
    }

    // each long gap with what overlapped it
    let mut items: Vec<Item> = Vec::new();
    for e in ev.iter().filter(|e| e.cpu == cpu) {
        if let Some(l) = on_cpu_label(e) {
            let what = if e.name == "sched_switch" {
                let next_pid = field(&e.body, "next_pid").unwrap_or("?");
                match wakeup_of(next_pid, rt(e, off)) {
                    Some(w) if field(&e.body, "next_pid").and_then(|p| p.parse().ok()) != hot => {
                        format!("{l} (woken by {}/{} on CPU {})", w.comm, w.pid, w.cpu)
                    }
                    _ => l,
                }
            } else {
                l
            };
            items.push(Item {
                at: rt(e, off),
                what,
            });
        }
    }
    for e in &sent {
        items.push(Item {
            at: rt(e, off),
            what: format!(
                "{} from CPU {} {}/{} via {}",
                e.name,
                e.cpu,
                e.comm,
                e.pid,
                field(&e.body, "callsite")
                    .map_or_else(|| frames(&e.stack, 2), |c| sym(c).to_string())
            ),
        });
    }
    let max = input.gaps.iter().map(|g| g.gap_ns).max().unwrap_or(0);
    let mut long: Vec<Gap> = input
        .gaps
        .iter()
        .filter(|g| g.gap_ns >= input.min_gap_ns || g.gap_ns == max)
        .copied()
        .collect();
    long.sort_by_key(|g| g.end_ns);
    let _ = writeln!(
        out,
        "\n## Gaps of {} µs or more, with what overlapped each\n",
        us(input.min_gap_ns)
    );
    let mut unexplained = 0;
    if long.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let _ = writeln!(
            out,
            "| gap ended (CLOCK_REALTIME ns) | gap µs | on CPU {cpu} or sent to it, within the gap |\n|---|---|---|"
        );
        for g in &long {
            let a = g
                .end_ns
                .saturating_sub(g.gap_ns)
                .saturating_sub(input.slack_ns);
            let b = g.end_ns + input.slack_ns;
            let mut seen: Vec<String> = Vec::new();
            for it in items.iter().filter(|it| it.at >= a && it.at <= b) {
                if !seen.contains(&it.what) {
                    seen.push(it.what.clone());
                }
            }
            if seen.is_empty() {
                unexplained += 1;
            }
            let _ = writeln!(
                out,
                "| {} | {}{} | {} |",
                g.end_ns,
                us(g.gap_ns),
                if g.gap_ns == max { " (the max)" } else { "" },
                if seen.is_empty() {
                    "nothing in the guest's trace: below the kernel (hypervisor or hardware)"
                        .to_string()
                } else {
                    seen.join("; ")
                },
            );
        }
    }
    let _ = writeln!(
        out,
        "\n{} gaps listed, {unexplained} with no kernel event on or sent to CPU {cpu}.",
        long.len()
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACE: &str = "# tracer: nop
#
# entries-in-buffer/entries-written: 12/12   #P:8
#
#           TASK-PID     CPU#  |||||  TIMESTAMP  FUNCTION
#              | |         |   |||||     |         |
     kubelet-901     [001] d..2.   100.000100: sched_waking: comm=kworker/3:1 pid=77 prio=120 target_cpu=003
     kubelet-901     [001] d..2.   100.000101: sched_wakeup: comm=kworker/3:1 pid=77 prio=120 target_cpu=003
     kubelet-901     [001] d..2.   100.000102: <stack trace>
 => __traceiter_sched_wakeup+0x40/0x60
 => ttwu_do_activate+0x100/0x200
 => try_to_wake_up+0x10/0x20
 => queue_work_on+0x30/0x40
 => vmstat_shepherd+0x50/0x60
     kubelet-901     [001] d..2.   100.000103: ipi_send_cpu: cpu=3 callsite=try_to_wake_up+0x1a8/0x5c0 callback=0x0
     kubelet-901     [001] d..2.   100.000104: <stack trace>
 => __traceiter_ipi_send_cpu+0x40/0x60
 => smp_send_reschedule+0x10/0x20
 => try_to_wake_up+0x1a8/0x5c0
          gp-hot-4242    [003] d.h1.   100.000110: ipi_entry: (Rescheduling interrupts)
          gp-hot-4242    [003] d..2.   100.000115: sched_switch: prev_comm=gp-hot prev_pid=4242 prev_prio=120 prev_state=R+ ==> next_comm=kworker/3:1 next_pid=77 next_prio=120
     kworker/3:1-77      [003] ...1.   100.000116: workqueue_execute_start: work struct 00000000deadbeef: function vmstat_update
     kworker/3:1-77      [003] d..2.   100.000120: sched_switch: prev_comm=kworker/3:1 prev_pid=77 prev_prio=120 prev_state=I ==> next_comm=gp-hot next_pid=4242 next_prio=120
          gp-hot-4242    [003] d.h1.   101.000000: irq_handler_entry: irq=10 name=arch_timer
          gp-hot-4242    [003] d.h1.   101.000001: hrtimer_expire_entry: hrtimer=00000000abcd function=tick_nohz_handler now=101000000000
          gp-hot-4242    [003] ..s1.   101.000002: softirq_entry: vec=1 [action=TIMER]
     systemd-1       [000] d..2.   102.000000: ipi_send_cpumask: cpumask=0-7 callsite=on_each_cpu_cond_mask+0x40/0x80 callback=do_sync_core+0x0/0x10
     systemd-1       [000] d..2.   102.000001: ipi_raise: target_mask=00000000,00000003 (Function call interrupts)
CPU:3 [LOST 5 EVENTS]
";

    fn input() -> Input {
        let trace = parse(TRACE).map_err(|e| e.0);
        Input {
            trace: trace.unwrap_or_default(),
            cpu: 3,
            hot_tid: None,
            // mono 100 s is realtime 1_000_000_100 s
            offset_ns: 1_000_000_000 * 1_000_000_000,
            offset_end_ns: Some(1_000_000_000 * 1_000_000_000 + 250),
            window: (0, 0),
            gaps: vec![
                Gap {
                    end_ns: 1_000_000_100_000_000_000 + 121_000,
                    gap_ns: 12_000,
                },
                Gap {
                    end_ns: 1_000_000_101_000_000_000 + 3_000,
                    gap_ns: 4_000,
                },
                Gap {
                    end_ns: 1_000_000_105_000_000_000,
                    gap_ns: 6_000,
                },
                Gap {
                    end_ns: 1_000_000_106_000_000_000,
                    gap_ns: 1_200,
                },
            ],
            nivcsw: Some(1),
            min_gap_ns: 3_000,
            slack_ns: 5_000,
            enabled: vec!["sched:sched_switch".into()],
            missing: vec!["irq_vectors:local_timer_entry".into()],
        }
    }

    #[test]
    fn parses_events_stacks_and_lost_notices() {
        let t = parse(TRACE).map_err(|e| e.0);
        let t = t.as_ref().map_err(Clone::clone);
        let Ok(t) = t else { return assert!(t.is_ok()) };
        assert_eq!(t.events.len(), 12);
        assert_eq!(t.lost, vec![(3, 5)]);
        let w = &t.events[1];
        assert_eq!(
            (w.comm.as_str(), w.pid, w.cpu, w.ts_ns, w.name.as_str()),
            ("kubelet", 901, 1, 100_000_101_000, "sched_wakeup")
        );
        assert_eq!(
            w.stack,
            vec![
                "__traceiter_sched_wakeup",
                "ttwu_do_activate",
                "try_to_wake_up",
                "queue_work_on",
                "vmstat_shepherd"
            ]
        );
        let k = &t.events[6];
        assert_eq!((k.comm.as_str(), k.pid), ("kworker/3:1", 77));
        assert!(parse("garbage line\n").is_err());
    }

    #[test]
    fn fields_including_a_comm_with_a_space() {
        let b = "prev_comm=gp hot prev_pid=4242 prev_prio=120 prev_state=R+ ==> next_comm=kworker/3:1 next_pid=77 next_prio=120";
        assert_eq!(field(b, "prev_comm"), Some("gp hot"));
        assert_eq!(field(b, "prev_pid"), Some("4242"));
        assert_eq!(field(b, "pid"), None);
        assert_eq!(field(b, "next_comm"), Some("kworker/3:1"));
        assert_eq!(field(b, "prev_state"), Some("R+"));
        assert_eq!(
            field("comm=a pid=7 prio=1 target_cpu=003", "pid"),
            Some("7")
        );
    }

    #[test]
    fn cpu_lists_and_masks() {
        assert_eq!(in_cpulist("0-7", 3), Some(true));
        assert_eq!(in_cpulist("1,4-6", 3), Some(false));
        assert_eq!(in_cpulist("x", 3), None);
        assert_eq!(in_hexmask("00000000,00000008", 3), Some(true));
        assert_eq!(in_hexmask("00000003", 3), Some(false));
        assert_eq!(in_hexmask("00000001,00000000", 32), Some(true));
    }

    #[test]
    fn gaps_csv() {
        assert_eq!(
            parse_gaps("end_realtime_ns,gap_ns\n10,2\n"),
            Ok(vec![Gap {
                end_ns: 10,
                gap_ns: 2
            }])
        );
        assert!(parse_gaps("10;2\n").is_err());
    }

    #[test]
    fn report_names_each_switch_its_waker_and_each_gap() {
        let md = render(&input());
        // the spin thread found by its comm; one involuntary switch, matching /proc
        assert!(
            md.contains("| spin thread | tid 4242 (gp-hot in the trace) |"),
            "{md}"
        );
        assert!(md.contains("| involuntary switches of the spin thread | 1 in the trace; 1 from /proc nonvoluntary_ctxt_switches |"), "{md}");
        assert!(md.contains("| events lost | **5**"), "{md}");
        assert!(
            md.contains("= CLOCK_REALTIME (250 ns of drift by the end)"),
            "{md}"
        );
        assert!(
            md.contains("| events this kernel lacks | irq_vectors:local_timer_entry |"),
            "{md}"
        );
        // the switch: what ran, for how long, who woke it and from where
        assert!(md.contains("| 1000000100000115000 | involuntary | R+ | kworker/3:1/77 | vmstat_update | 5.00 | kubelet/901 on CPU 1 (sched_wakeup) | ttwu_do_activate < try_to_wake_up < queue_work_on < vmstat_shepherd | 12.00 |"), "{md}");
        // the IPIs sent to CPU 3: the resched, and the cpumask one; not the raise to CPUs 0-1
        assert!(md.contains("| 1000000100000103000 | ipi_send_cpu | 1 | kubelet/901 | try_to_wake_up | 0x0 | smp_send_reschedule < try_to_wake_up |"), "{md}");
        assert!(
            md.contains(
                "| ipi_send_cpumask | 0 | systemd/1 | on_each_cpu_cond_mask | do_sync_core |"
            ),
            "{md}"
        );
        assert!(!md.contains("| ipi_raise |"), "{md}");
        // the work item and who queued it (not in this trace)
        assert!(
            md.contains(
                "| 1000000100000116000 | vmstat_update | kworker/3:1/77 | not in the trace | - |"
            ),
            "{md}"
        );
        // counted interrupts and timers
        assert!(md.contains("| irq arch_timer (10) | 1 |"), "{md}");
        assert!(md.contains("| hrtimer tick_nohz_handler | 1 |"), "{md}");
        assert!(md.contains("| softirq TIMER | 1 |"), "{md}");
        // gaps: the max with its cause; the tick gap; one with nothing seen
        assert!(md.contains("| 1000000100000121000 | 12.00 (the max) | ipi Rescheduling interrupts; switch gp-hot -> kworker/3:1 (woken by kubelet/901 on CPU 1); work vmstat_update; switch kworker/3:1 -> gp-hot |"), "{md}");
        assert!(
            md.contains(
                "| 1000000101000003000 | 4.00 | irq arch_timer (10); hrtimer tick_nohz_handler; softirq TIMER |"
            ),
            "{md}"
        );
        assert!(md.contains("| 1000000105000000000 | 6.00 | nothing in the guest's trace: below the kernel (hypervisor or hardware) |"), "{md}");
        assert!(!md.contains("| 1000000106000000000 |"), "{md}");
        assert!(
            md.contains("3 gaps listed, 1 with no kernel event on or sent to CPU 3."),
            "{md}"
        );
    }

    #[test]
    fn a_wakeup_from_a_timer_on_the_cpu_says_so() {
        let text = "\
          gp-hot-7       [003] d.s2.    50.000001: sched_wakeup: comm=kworker/3:1 pid=77 prio=120 target_cpu=003
          gp-hot-7       [003] d..2.    50.000002: sched_switch: prev_comm=gp-hot prev_pid=7 prev_prio=120 prev_state=R ==> next_comm=kworker/3:1 next_pid=77 next_prio=120
     kworker/3:1-77      [003] d..2.    50.000009: sched_switch: prev_comm=kworker/3:1 prev_pid=77 prev_prio=120 prev_state=I ==> next_comm=gp-hot next_pid=7 next_prio=120
";
        let i = Input {
            trace: parse(text).unwrap_or_default(),
            cpu: 3,
            ..Input::default()
        };
        let md = render(&i);
        assert!(md.contains("| kworker/3:1/77 | - | 7.00 | a timer or interrupt on CPU 3, in the spin thread's context (sched_wakeup) |"), "{md}");
    }

    #[test]
    fn a_given_tid_the_trace_lacks_yields_to_gp_hot() {
        let mut i = input();
        i.hot_tid = Some(9);
        let md = render(&i);
        assert!(
            md.contains(
                "| spin thread | tid 4242 (gp-hot in the trace; the tid given, 9, is not in it) |"
            ),
            "{md}"
        );
        i.hot_tid = Some(4242);
        assert!(render(&i).contains("| spin thread | tid 4242 (host) |"));
    }

    #[test]
    fn report_window_and_a_given_tid() {
        let mut i = input();
        i.hot_tid = Some(9);
        i.window = (1_000_000_100_500_000_000, 1_000_000_103_000_000_000);
        let md = render(&i);
        assert!(md.contains("| spin thread | tid 9 (host) |"), "{md}");
        assert!(
            md.contains("| involuntary switches of the spin thread | 0 in the trace"),
            "{md}"
        );
        // the tick at 101 s is inside the window, the switch at 100 s is not
        assert!(md.contains("| irq arch_timer (10) | 1 |"), "{md}");
        assert!(!md.contains("vmstat_update"), "{md}");
    }
}
