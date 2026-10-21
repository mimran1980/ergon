//! Diagnosis runs (`MODE=diagnose`, `DIR/diag/…`): what the timed runs'
//! "no kernel event" stalls were.
//!
//! - `syscalls-pp-<backend>`: every syscall over 100 µs on the loop CPU
//!   (bpftrace: end time, number, duration), matched to the loop's stalls.
//! - `fgraph-pp-<backend>`: every kernel function over 50 µs on the loop CPU
//!   (function_graph with `tracing_thresh`).
//! - `hwlat`: the kernel's hardware-latency detector on every isolated CPU at
//!   once: with interrupts off, any gap is the hypervisor or firmware. Each
//!   CPU's gaps are shown with the second of the minute they bunch at.
//!
//! These runs are slowed by their tracing, so they are not timed comparisons.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::periodic::{self, Series};
use crate::stalls::{self, Stall};

/// One slow syscall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlowSyscall {
    /// `CLOCK_MONOTONIC` at its end, nanoseconds.
    pub end_ns: i64,
    /// Syscall number.
    pub nr: i64,
    /// Duration, nanoseconds.
    pub ns: i64,
}

/// Parses the bpftrace output (`end_ns nr duration_ns` per line; the banner
/// and anything else is skipped).
#[must_use]
pub fn parse_slowsys(text: &str) -> Vec<SlowSyscall> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace().map(str::parse::<i64>);
            match (f.next(), f.next(), f.next(), f.next()) {
                (Some(Ok(end_ns)), Some(Ok(nr)), Some(Ok(ns)), None) => {
                    Some(SlowSyscall { end_ns, nr, ns })
                }
                _ => None,
            }
        })
        .collect()
}

/// The name of syscall `nr` on `arch` (`x86_64` or `aarch64`), for the calls
/// these loops make.
#[must_use]
pub fn syscall_name(arch: &str, nr: i64) -> String {
    let name = if arch == "aarch64" {
        match nr {
            21 => "epoll_ctl",
            22 => "epoll_pwait",
            29 => "ioctl",
            57 => "close",
            63 => "read",
            64 => "write",
            98 => "futex",
            101 => "nanosleep",
            113 => "clock_gettime",
            115 => "clock_nanosleep",
            124 => "sched_yield",
            206 => "sendto",
            207 => "recvfrom",
            208 => "setsockopt",
            209 => "getsockopt",
            211 => "sendmsg",
            212 => "recvmsg",
            215 => "munmap",
            222 => "mmap",
            426 => "io_uring_enter",
            441 => "epoll_pwait2",
            _ => "",
        }
    } else {
        match nr {
            0 => "read",
            1 => "write",
            3 => "close",
            9 => "mmap",
            11 => "munmap",
            16 => "ioctl",
            24 => "sched_yield",
            35 => "nanosleep",
            44 => "sendto",
            45 => "recvfrom",
            46 => "sendmsg",
            47 => "recvmsg",
            54 => "setsockopt",
            55 => "getsockopt",
            202 => "futex",
            228 => "clock_gettime",
            230 => "clock_nanosleep",
            232 => "epoll_wait",
            233 => "epoll_ctl",
            281 => "epoll_pwait",
            426 => "io_uring_enter",
            441 => "epoll_pwait2",
            _ => "",
        }
    };
    if name.is_empty() {
        format!("syscall {nr}")
    } else {
        name.to_owned()
    }
}

/// Slow syscalls by name, and how many of the loop's stalls over 100 µs
/// overlap one.
#[must_use]
pub fn slowsys_report(
    title: &str,
    arch: &str,
    calls: &[SlowSyscall],
    loop_stalls: &[Stall],
) -> String {
    let mut out = String::new();
    let mut by: HashMap<String, (u64, i64, i64)> = HashMap::new();
    for c in calls {
        let e = by.entry(syscall_name(arch, c.nr)).or_default();
        e.0 += 1;
        e.1 = e.1.max(c.ns);
        e.2 += c.ns;
    }
    let mut rows: Vec<_> = by.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.2));
    let _ = writeln!(
        out,
        "#### Slow syscalls (over 100 µs) on the loop CPU: {title}\n\n| syscall | count | max µs | total ms |\n|---|---|---|---|"
    );
    #[allow(clippy::cast_precision_loss)]
    for (name, (n, max, total)) in &rows {
        let _ = writeln!(
            out,
            "| {name} | {n} | {:.1} | {:.2} |",
            *max as f64 / 1e3,
            *total as f64 / 1e6
        );
    }
    let big: Vec<&Stall> = loop_stalls.iter().filter(|s| s.ns > 100_000).collect();
    let inside = big
        .iter()
        .filter(|s| {
            let (a, b) = (s.start_mono_ns(), s.end_mono_ns);
            calls.iter().any(|c| c.end_ns > a && c.end_ns - c.ns < b)
        })
        .count();
    let _ = writeln!(
        out,
        "\nLoop stalls over 100 µs: {}; inside a slow syscall: {inside}.\n",
        big.len()
    );
    out
}

/// Parses function_graph output with `tracing_thresh`: each slow function's
/// name and duration in microseconds.
#[must_use]
pub fn parse_fgraph(text: &str) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = l.split('|').collect();
        let Some(us) = fields.iter().find_map(|f| {
            let f = f.trim();
            let num = f.strip_suffix("us")?.trim();
            num.trim_start_matches(|c: char| !c.is_ascii_digit())
                .parse::<f64>()
                .ok()
        }) else {
            continue;
        };
        let Some(last) = fields.last() else { continue };
        let name = last.split("/* ").nth(1).map_or_else(
            || {
                last.trim()
                    .trim_end_matches("();")
                    .trim_end_matches(['{', '}', ';', ' '])
                    .to_owned()
            },
            |r| r.trim_end_matches(" */").trim().to_owned(),
        );
        if !name.is_empty() && name != "}" {
            out.push((name, us));
        }
    }
    out
}

/// The slowest kernel functions: count and maximum.
#[must_use]
pub fn fgraph_report(title: &str, funcs: &[(String, f64)]) -> String {
    let mut by: HashMap<&str, (u64, f64)> = HashMap::new();
    for (f, us) in funcs {
        let e = by.entry(f.as_str()).or_default();
        e.0 += 1;
        e.1 = e.1.max(*us);
    }
    let mut rows: Vec<_> = by.into_iter().collect();
    rows.sort_by(|a, b| b.1.1.total_cmp(&a.1.1));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "#### Kernel functions over 50 µs on the loop CPU: {title}\n\n| function | count | max µs |\n|---|---|---|"
    );
    for (f, (n, max)) in rows.iter().take(20) {
        let _ = writeln!(out, "| `{f}` | {n} | {max:.1} |");
    }
    out.push('\n');
    out
}

/// One hwlat sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HwlatGap {
    /// The CPU it was measured on.
    pub cpu: u32,
    /// The longer of its inner and outer gaps, microseconds.
    pub us: u64,
    /// When (`ts:`, wall clock), nanoseconds; 0 if the line has none.
    pub ts_ns: i64,
}

/// Parses hwlat output: each sample's CPU, longest gap and time.
#[must_use]
pub fn parse_hwlat(text: &str) -> Vec<HwlatGap> {
    text.lines()
        .filter_map(|l| {
            let (head, rest) = l.split_once("inner/outer(us):")?;
            let cpu = head
                .split_once('[')?
                .1
                .split_once(']')?
                .0
                .trim()
                .parse()
                .ok()?;
            let pair = rest.split_whitespace().next()?;
            let (i, o) = pair.split_once('/')?;
            let us = i
                .trim()
                .parse::<u64>()
                .ok()?
                .max(o.trim().parse::<u64>().ok()?);
            let ts_ns = rest
                .split_whitespace()
                .find_map(|f| f.strip_prefix("ts:"))
                .and_then(|t| {
                    let (s, n) = t.split_once('.')?;
                    Some(s.parse::<i64>().ok()? * 1_000_000_000 + n.parse::<i64>().ok()?)
                })
                .unwrap_or(0);
            Some(HwlatGap { cpu, us, ts_ns })
        })
        .collect()
}

/// The hwlat summary: all CPUs, then each CPU with the second of the minute
/// its gaps over 100 µs bunch at.
#[must_use]
pub fn hwlat_report(cpus: &str, seconds: f64, gaps: &[HwlatGap]) -> String {
    let line = |g: &[&HwlatGap]| {
        let over = |t: u64| g.iter().filter(|x| x.us > t).count();
        (
            g.len(),
            over(100),
            over(1000),
            g.iter().map(|x| x.us).max().unwrap_or(0),
        )
    };
    let all: Vec<&HwlatGap> = gaps.iter().collect();
    let (n, o100, o1000, max) = line(&all);
    let mut out = format!(
        "#### Hardware latency (hwlat) on CPUs {cpus}, {seconds:.0} s\n\nGaps with interrupts off (the hypervisor or firmware): \
         {n} over 10 µs, {o100} over 100 µs, {o1000} over 1 ms; longest {max} µs.\n\n\
         | CPU | gaps | over 100 µs | over 1 ms | longest µs | gaps over 100 µs bunch at |\n|---|---|---|---|---|---|\n"
    );
    let mut cpu_ids: Vec<u32> = gaps.iter().map(|g| g.cpu).collect();
    cpu_ids.sort_unstable();
    cpu_ids.dedup();
    for cpu in cpu_ids {
        let mine: Vec<&HwlatGap> = gaps.iter().filter(|g| g.cpu == cpu).collect();
        let (n, o100, o1000, max) = line(&mine);
        let big: Vec<(i64, i64)> = mine
            .iter()
            .filter(|g| g.us > 100 && g.ts_ns > 0)
            .map(|g| (g.ts_ns, i64::try_from(g.us * 1000).unwrap_or(i64::MAX)))
            .collect();
        let span = mine
            .iter()
            .map(|g| g.ts_ns)
            .filter(|&t| t > 0)
            .min()
            .zip(mine.iter().map(|g| g.ts_ns).max());
        let at = periodic::peak(&Series {
            events: big,
            spans: span.into_iter().collect(),
        })
        .map_or_else(String::new, |p| {
            format!(
                ":{:02} ({} of {}, {} minutes{})",
                p.second,
                p.count,
                o100,
                p.minutes_hit,
                if p.is_periodic() { ", periodic" } else { "" }
            )
        });
        let _ = writeln!(out, "| {cpu} | {n} | {o100} | {o1000} | {max} | {at} |");
    }
    out.push('\n');
    out
}

/// The diagnosis section for `DIR/diag`, if it exists.
#[must_use]
pub fn report(diag: &Path) -> String {
    let Ok(rd) = std::fs::read_dir(diag) else {
        return String::new();
    };
    let mut dirs: Vec<_> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut out = String::from(
        "## Diagnosis\n\nSeparate runs, slowed by their tracing (not timed comparisons): what the stalls the timed \
         runs could not attribute were.\n\n",
    );
    for d in dirs {
        let name = d
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let meta: HashMap<String, String> = std::fs::read_to_string(d.join("meta.txt"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let arch = meta.get("arch").map_or("x86_64", String::as_str);
        if let Ok(t) = std::fs::read_to_string(d.join("hwlat.txt")) {
            let secs = meta
                .get("duration_s")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            out.push_str(&hwlat_report(
                meta.get("cpu").map_or("?", String::as_str),
                secs,
                &parse_hwlat(&t),
            ));
        }
        if let Ok(t) = std::fs::read_to_string(d.join("slowsys.txt")) {
            let loop_stalls =
                stalls::load_attributed(&d.join("stalls-0.txt"), &d.join("trace.txt"))
                    .map(|(_, a)| a.into_iter().map(|x| x.stall).collect::<Vec<_>>())
                    .unwrap_or_default();
            out.push_str(&slowsys_report(
                &name,
                arch,
                &parse_slowsys(&t),
                &loop_stalls,
            ));
        }
        if let Ok(t) = std::fs::read_to_string(d.join("fgraph.txt")) {
            out.push_str(&fgraph_report(&name, &parse_fgraph(&t)));
        }
    }
    out
}
