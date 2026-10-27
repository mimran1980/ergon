//! The few procfs and cgroup files the probe and its reports read, parsed
//! from text so the parsers run on fixtures: a thread's fault counters
//! (`/proc/thread-self/stat`), the per-CPU times of `/proc/stat`, a cgroup's
//! `cpu.stat` and a thread's `schedstat`.

use std::fmt;

/// A file did not have the shape the kernel writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// (minor, major) page faults from a `/proc/<pid>/task/<tid>/stat` line:
/// fields 10 and 12, counted after the `)` that ends the comm (a comm may
/// hold spaces and parentheses).
///
/// # Errors
/// No `)`, or either field missing or not a number.
pub fn faults(stat: &str) -> Result<(u64, u64), ParseError> {
    let rest = stat
        .rfind(')')
        .and_then(|i| stat.get(i + 1..))
        .ok_or_else(|| ParseError(format!("stat: no ')' in {stat:?}")))?;
    let f: Vec<&str> = rest.split_whitespace().collect();
    // after the comm: state(3) ppid pgrp session tty_nr tpgid flags minflt(10) cminflt majflt(12)
    let n = |i: usize| -> Result<u64, ParseError> {
        f.get(i)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| ParseError(format!("stat: field {} missing or not a number", i + 3)))
    };
    Ok((n(7)?, n(9)?))
}

/// One CPU's line of `/proc/stat`, in its clock ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTimes {
    /// user + nice + system + irq + softirq + steal
    pub busy: u64,
    /// every field up to steal (guest time is inside user already)
    pub total: u64,
}

/// `(cpu, times)` for every `cpuN` line of `/proc/stat`, ascending.
///
/// # Errors
/// A `cpuN` line with fewer than 8 numbers.
pub fn proc_stat(text: &str) -> Result<Vec<(usize, CpuTimes)>, ParseError> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let Some(name) = it.next() else { continue };
        let Some(cpu) = name
            .strip_prefix("cpu")
            .and_then(|n| n.parse::<usize>().ok())
        else {
            continue;
        };
        let v: Vec<u64> = it.take(8).filter_map(|x| x.parse().ok()).collect();
        let [user, nice, system, idle, iowait, irq, softirq, steal] = v[..] else {
            return Err(ParseError(format!("/proc/stat: short line {line:?}")));
        };
        let busy = user + nice + system + irq + softirq + steal;
        out.push((
            cpu,
            CpuTimes {
                busy,
                total: busy + idle + iowait,
            },
        ));
    }
    out.sort_by_key(|(c, _)| *c);
    Ok(out)
}

/// The value of `key` in a `key value` file such as `cpu.stat`.
#[must_use]
pub fn keyed(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        (it.next() == Some(key))
            .then(|| it.next().and_then(|v| v.parse().ok()))
            .flatten()
    })
}

/// Nanoseconds on CPU from a `schedstat` file (its first field).
///
/// # Errors
/// An empty file or a first field that is not a number.
pub fn schedstat_ns(text: &str) -> Result<u64, ParseError> {
    text.split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| ParseError(format!("schedstat: {text:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faults_after_a_comm_with_spaces_and_parens() {
        let line = "4321 (gp hot (x)) R 1 1 1 0 -1 4194560 17 0 3 0 100 5 0 0 20 0 1 0 1 0 0";
        assert_eq!(faults(line), Ok((17, 3)));
        assert!(faults("4321 gp-hot R").is_err());
        assert!(faults("4321 (gp-hot) R 1 1").is_err());
    }

    #[test]
    fn proc_stat_busy_and_total_per_cpu() {
        let text = "cpu  10 0 10 100 0 0 0 0 0 0\n\
                    cpu1 5 1 2 50 3 1 1 2 9 0\n\
                    cpu0 1 0 1 10 0 0 0 0 0 0\n\
                    intr 1 2 3\n";
        assert_eq!(
            proc_stat(text),
            Ok(vec![
                (0, CpuTimes { busy: 2, total: 12 }),
                (
                    1,
                    CpuTimes {
                        busy: 12,
                        total: 65
                    }
                ),
            ])
        );
        assert!(proc_stat("cpu3 1 2 3").is_err());
    }

    #[test]
    fn keyed_and_schedstat() {
        let stat = "usage_usec 2500\nnr_periods 0\nnr_throttled 0\nthrottled_usec 0\n";
        assert_eq!(keyed(stat, "usage_usec"), Some(2500));
        assert_eq!(keyed(stat, "nr_throttled"), Some(0));
        assert_eq!(keyed(stat, "usage"), None);
        assert_eq!(schedstat_ns("123456 789 10\n"), Ok(123_456));
        assert!(schedstat_ns("").is_err());
    }
}
