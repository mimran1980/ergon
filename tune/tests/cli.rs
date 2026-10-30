//! The `ergon-tune` command against fixture hosts (`--root`): what it prints,
//! and its exit status with and without `--require`.
#![allow(missing_docs)]

mod common;

use std::process::{Command, Output};

use common::{c9g_tuned, debian_default};

type R = Result<(), Box<dyn std::error::Error>>;

fn run(root: &std::path::Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_ergon-tune"))
        .arg("--root")
        .arg(root)
        .args(args)
        .env_remove("ERGON_HOST_CHECKS")
        .output()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// An untuned host: each warning is printed with its fix, then a summary,
/// and the command succeeds (it reports; `--require` refuses).
#[test]
fn an_untuned_host_prints_each_warning_and_a_summary() -> R {
    let f = debian_default("cli-untuned")?;
    let o = run(f.root(), &["--hot", "3"])?;
    let out = text(&o);
    assert_eq!(o.status.code(), Some(0), "{out}");
    assert!(out.contains("hot CPUs: 3 (--hot)"), "{out}");
    assert!(
        out.lines().any(|l| l.starts_with("WARN T1 ")),
        "no T1 warning: {out}"
    );
    assert!(out.lines().any(|l| l.starts_with("WARN T3 ")), "{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("summary: ") && l.contains(" warnings")),
        "{out}"
    );
    // A warning line is the finding itself, not the start-up switch's hint.
    assert!(!out.contains("ERGON_HOST_CHECKS=off"), "{out}");
    Ok(())
}

/// `--require` exits 1 naming the checks that refused, as a pod's start
/// would under `ERGON_HOST_CHECKS=require`.
#[test]
fn require_refuses_an_untuned_host() -> R {
    let f = debian_default("cli-require")?;
    let o = run(f.root(), &["--hot", "3", "--require"])?;
    let out = text(&o);
    assert_eq!(o.status.code(), Some(1), "{out}");
    assert!(out.contains("refused:") && out.contains("T1"), "{out}");
    Ok(())
}

/// Without `--hot` the hot CPUs are the node's isolated ones.
#[test]
fn the_hot_cpus_default_to_the_isolated_ones() -> R {
    let f = c9g_tuned("cli-isolated")?;
    let o = run(f.root(), &[])?;
    let out = text(&o);
    assert!(out.contains("hot CPUs: 1-7 (isolated)"), "{out}");
    assert!(!out.lines().any(|l| l.starts_with("WARN T1 ")), "{out}");
    Ok(())
}

/// A named NIC adds its interrupt checks (T2, T6, T20).
#[test]
fn a_nic_adds_its_interrupt_checks() -> R {
    let f = debian_default("cli-nic")?;
    let o = run(f.root(), &["--hot", "3", "--nic", "ens5", "--all"])?;
    let out = text(&o);
    for id in ["T2", "T6", "T20"] {
        assert!(
            out.lines().any(|l| l.split_whitespace().nth(1) == Some(id)),
            "{id} not run: {out}"
        );
    }
    // ens5's queue 3 interrupts on hot CPU 3.
    assert!(out.lines().any(|l| l.starts_with("WARN T2 ")), "{out}");
    Ok(())
}

/// A bad argument is a usage error (exit 2), never a silent default.
#[test]
fn a_bad_argument_is_a_usage_error() -> R {
    let f = debian_default("cli-usage")?;
    for args in [&["--hot", "x"][..], &["--frobnicate"], &["--nic"]] {
        let o = run(f.root(), args)?;
        let out = text(&o);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {out}");
        assert!(out.contains("usage: ergon-tune"), "{args:?}: {out}");
    }
    Ok(())
}
