//! `hostcheck` against fixture trees standing for a host's `/` (ergon-tune's
//! own, `tune/tests/common`): a tuned host is clean (exit 0), an untuned one
//! and an unreadable one are not (exit 2, the lines saying why), and a
//! command line it cannot run exits 1. Every line is ergon-tune's
//! `<id> <ok|info|warn|unknown>: ...`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "an integration test fails by panicking: unwrap, expect and panic! are its assertions (as md-dpdk/tests)"
)]
#![allow(clippy::similar_names, reason = "ping and pong name the two ends of each round trip")]

// tune formats it (rustfmt's defaults), not this crate (max_width 120).
#[path = "../../../tune/tests/common/mod.rs"]
#[allow(clippy::all, clippy::pedantic, clippy::nursery, unreachable_pub)]
#[rustfmt::skip]
mod common;

use std::path::Path;
use std::process::{Command, Output};

type R = Result<(), Box<dyn std::error::Error>>;

fn hostcheck(root: &Path, out: &Path, extra: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_hostcheck"))
        .arg("--root")
        .arg(root)
        .arg("--out")
        .arg(out)
        .args(extra)
        .output()?)
}

fn lines(out: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(out)?.lines().map(str::to_owned).collect())
}

/// Each line is `<id> <status>: ...` with a status the analysis reads.
fn well_formed(ls: &[String]) {
    assert!(!ls.is_empty(), "no line");
    for l in ls {
        let (id, rest) = l.split_once(' ').unwrap_or_default();
        assert!(!id.is_empty(), "{l}");
        assert!(
            ["ok:", "info:", "warn:", "unknown:"]
                .iter()
                .any(|s| rest.starts_with(s)),
            "{l}"
        );
    }
}

/// A tuned c9g (ergon-tune's fixture: CPUs 1-7 isolated and `nohz_full`,
/// THP never, no SMT, the data NIC's IRQs on CPU 0), with what the host-wide
/// checks read besides: no CPU quota on the process's cgroup, a TSC clock
/// source, no unbound workqueue on the hot CPUs, compaction and KSM off.
fn tuned(name: &str) -> Result<common::Fixture, Box<dyn std::error::Error>> {
    let f = common::c9g_tuned(name)?;
    f.cgroup_v2("/bench")?
        .file("/sys/fs/cgroup/bench/cpu.max", "max 100000\n")?
        .cpu_stat("/sys/fs/cgroup/bench", 0, 0, 0)?
        .file(
            "/sys/devices/system/clocksource/clocksource0/current_clocksource",
            "arch_sys_counter\n",
        )?
        .file(
            "/sys/devices/system/clocksource/clocksource0/available_clocksource",
            "arch_sys_counter\n",
        )?
        .file("/sys/devices/virtual/workqueue/cpumask", "01\n")?
        .file("/proc/sys/vm/compaction_proactiveness", "0\n")?
        .file("/sys/kernel/mm/ksm/run", "0\n")?;
    Ok(f)
}

#[test]
fn a_tuned_host_is_clean() -> R {
    let f = tuned("hostcheck-tuned")?;
    let out = f.root().join("report.txt");
    let o = hostcheck(f.root(), &out, &["--hot-cpus", "3,5", "--nic", "ens6"])?;
    let ls = lines(&out)?;
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}\n{}",
        ls.join("\n"),
        String::from_utf8_lossy(&o.stderr)
    );
    well_formed(&ls);
    for id in ["T1", "T3", "T5", "T8", "T9", "T10", "T12", "T2", "C1"] {
        assert!(ls.iter().any(|l| l.starts_with(&format!("{id} "))), "{id}: {ls:#?}");
    }
    assert!(ls.iter().any(|l| l.starts_with("T1 ok:")), "{ls:#?}");
    assert!(!ls.iter().any(|l| l.contains(" warn:")), "{ls:#?}");
    Ok(())
}

#[test]
fn an_untuned_host_exits_2_naming_each_warning() -> R {
    let f = common::debian_default("hostcheck-debian")?;
    let out = f.root().join("report.txt");
    let o = hostcheck(f.root(), &out, &["--hot-cpus", "2,3"])?;
    assert_eq!(o.status.code(), Some(2));
    let ls = lines(&out)?;
    well_formed(&ls);
    assert!(ls.iter().any(|l| l.starts_with("T1 warn:")), "{ls:#?}");
    assert!(ls.iter().any(|l| l.starts_with("T3 warn:")), "{ls:#?}");
    let said = String::from_utf8_lossy(&o.stderr);
    assert!(said.contains("T1 warn:"), "{said}");
    Ok(())
}

/// A host whose isolation cannot be read is not proven clean: T1 must prove
/// it, so its unknown blocks.
#[test]
fn an_unproven_isolation_exits_2() -> R {
    let f = common::Fixture::empty("hostcheck-empty")?;
    f.dir("/sys")?.dir("/proc")?;
    let out = f.root().join("report.txt");
    let o = hostcheck(f.root(), &out, &["--hot-cpus", "2"])?;
    assert_eq!(o.status.code(), Some(2));
    let ls = lines(&out)?;
    well_formed(&ls);
    assert!(ls.iter().any(|l| l.starts_with("T1 unknown:")), "{ls:#?}");
    Ok(())
}

#[test]
fn a_command_line_it_cannot_run_exits_1() -> R {
    let f = common::Fixture::empty("hostcheck-usage")?;
    let out = f.root().join("report.txt");
    for args in [&[][..], &["--hot-cpus", "x"], &["--hot-cpus", "1", "--bogus", "1"]] {
        let o = hostcheck(f.root(), &out, args)?;
        assert_eq!(o.status.code(), Some(1), "{args:?}");
    }
    let o = Command::new(env!("CARGO_BIN_EXE_hostcheck"))
        .args(["--hot-cpus", "1"])
        .output()?;
    assert_eq!(o.status.code(), Some(1), "no --out");
    Ok(())
}
