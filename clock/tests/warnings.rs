//! Start-up warnings: the front `calibrate` prints one line per problem to
//! stderr, `ERGON_HOST_CHECKS=off` (or `0`) turns them off, and
//! `advanced::calibrate` never prints. Each check re-runs this test binary
//! for one child test and counts the lines on its stderr.
#![allow(missing_docs)]

use ergon_clock::advanced::{self, Policy, TimeSyncPolicy};
use ergon_clock::{Config, Domain, Warning, calibrate};
use std::process::Command;

type R = Result<(), Box<dyn std::error::Error>>;

/// A calibration with exactly one problem: no counter read may cost more
/// than 0 ns, so the clock falls back (or the host never offered the
/// counter), and nothing else is probed.
fn one_problem() -> Config {
    Config {
        domain: Domain::Monotonic,
        time_sync: TimeSyncPolicy::Off,
        max_read_ns: 0,
        ..Config::default()
    }
}

/// The child: calibrates through the front, which prints its warnings.
#[test]
fn child_prints_its_start_up_warnings() -> R {
    let clock = calibrate(&one_problem())?;
    let warnings = clock.report().warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(matches!(warnings[0], Warning::Fallback { .. }));
    Ok(())
}

/// The child: calibrates through `advanced`, which never prints.
#[test]
fn child_calibrates_through_advanced() -> R {
    let report = advanced::calibrate(&one_problem())?.report;
    assert_eq!(report.warnings().len(), 1, "{:?}", report.warnings());
    Ok(())
}

/// The child: the front, with printing turned off in its configuration.
#[test]
fn child_with_printing_off() -> R {
    let clock = calibrate(&Config {
        print_warnings: false,
        ..one_problem()
    })?;
    assert_eq!(clock.report().warnings().len(), 1);
    Ok(())
}

/// Runs child test `name` with `ERGON_HOST_CHECKS` set to `value` (`None`:
/// removed, not inherited) and counts `ergon-clock warning:` lines on its
/// stderr. The child must pass.
fn warning_lines(name: &str, value: Option<&str>) -> Result<usize, Box<dyn std::error::Error>> {
    let mut child = Command::new(std::env::current_exe()?);
    child.args(["--exact", name, "--nocapture", "--test-threads=1"]);
    match value {
        Some(v) => child.env("ERGON_HOST_CHECKS", v),
        None => child.env_remove("ERGON_HOST_CHECKS"),
    };
    let out = child.output()?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !stdout.contains("1 passed") {
        return Err(format!("{name} failed: {stdout}\n{stderr}").into());
    }
    Ok(stderr
        .lines()
        .filter(|l| l.starts_with("ergon-clock warning: "))
        .count())
}

/// One line by default; none with the switch `off` or `0`; a typo never
/// silences.
#[test]
fn the_switch_turns_the_lines_off() -> R {
    let child = "child_prints_its_start_up_warnings";
    assert_eq!(warning_lines(child, None)?, 1);
    assert_eq!(warning_lines(child, Some("0"))?, 0);
    assert_eq!(warning_lines(child, Some("off"))?, 0);
    assert_eq!(warning_lines(child, Some("of"))?, 1);
    Ok(())
}

/// `advanced::calibrate` prints nothing, whatever the switch, and the
/// configuration can silence the front.
#[test]
fn advanced_calibrate_and_the_config_switch_print_nothing() -> R {
    assert_eq!(warning_lines("child_calibrates_through_advanced", None)?, 0);
    assert_eq!(warning_lines("child_with_printing_off", None)?, 0);
    Ok(())
}

/// The line names the fallback, says it is slower, and gives the reason.
#[test]
fn a_fallback_line_says_slower_and_why() -> R {
    let report = advanced::calibrate(&Config {
        policy: Policy::Auto,
        ..one_problem()
    })?
    .report;
    let lines: Vec<String> = report.warnings().iter().map(Warning::line).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("ergon-clock warning: "),
        "{}",
        lines[0]
    );
    assert!(lines[0].contains("slower"), "{}", lines[0]);
    assert!(lines[0].contains("why: "), "{}", lines[0]);
    Ok(())
}
