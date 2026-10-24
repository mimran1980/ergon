//! `ll-probe`: proves the thread layout of the sample's pods from outside
//! them. Runs in a `hostPID` pod on the node (or beside the processes in
//! the Docker e2e), pinned to the node's IRQ CPU, and never execs into a
//! hot pod.
//!
//! ```text
//! ll-probe watch [--contract placement|full] [--window-s 60] [--interval-ms 100]
//!                [--node NAME] [--pin CPU] [--cpuset-check on|off] [--tick-limit 2]
//!                [--root /] (--pod LABEL=UID | --target LABEL=PID)...
//! ```
//!
//! Prints the `ll-verify` lines (`src/probe/format.rs`) and exits 0 on
//! `RESULT PASS`, 1 on `RESULT FAIL`, 2 on a usage error.

use std::io::Write as _;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use low_latency_k8s::LlError;
use low_latency_k8s::cpus::{CpuList, pin_current};
use low_latency_k8s::probe::format::{header, line, result_line};
use low_latency_k8s::probe::proc::Root;
use low_latency_k8s::probe::verdict::{Contract, Outcome, result};
use low_latency_k8s::probe::watch::{Options, Selector, Watch};

struct Args {
    opts: Options,
    window: Duration,
    interval: Duration,
    pin: Option<usize>,
    root: Root,
    selectors: Vec<Selector>,
}

fn usage(msg: &str) -> LlError {
    LlError::Usage(format!(
        "{msg}\nll-probe watch [--contract placement|full] [--window-s S] [--interval-ms MS] [--node NAME] \
         [--pin CPU] [--cpuset-check on|off] [--tick-limit N] [--root DIR] (--pod LABEL=UID | --target LABEL=PID)..."
    ))
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, LlError> {
    if args.next().as_deref() != Some("watch") {
        return Err(usage("the only command is watch"));
    }
    let mut out = Args {
        opts: Options {
            contract: Contract::Placement,
            node: std::fs::read_to_string("/proc/sys/kernel/hostname")
                .map_or_else(|_| "-".to_owned(), |h| h.trim().to_owned()),
            cpuset_check: true,
            tick_limit: 2.0,
        },
        window: Duration::from_secs(60),
        interval: Duration::from_millis(100),
        pin: None,
        root: Root::system(),
        selectors: Vec::new(),
    };
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| usage(&format!("{flag} needs a value")))
        };
        match flag.as_str() {
            "--contract" => {
                let v = value()?;
                out.opts.contract =
                    Contract::parse(&v).ok_or_else(|| usage(&format!("--contract {v}")))?;
            }
            "--window-s" => {
                let v = value()?;
                out.window = Duration::from_secs_f64(
                    v.parse().map_err(|_| usage(&format!("--window-s {v}")))?,
                );
            }
            "--interval-ms" => {
                let v = value()?;
                out.interval = Duration::from_millis(
                    v.parse()
                        .map_err(|_| usage(&format!("--interval-ms {v}")))?,
                );
            }
            "--node" => out.opts.node = value()?,
            "--pin" => {
                let v = value()?;
                out.pin = Some(v.parse().map_err(|_| usage(&format!("--pin {v}")))?);
            }
            "--cpuset-check" => out.opts.cpuset_check = value()? != "off",
            "--tick-limit" => {
                let v = value()?;
                out.opts.tick_limit = v.parse().map_err(|_| usage(&format!("--tick-limit {v}")))?;
            }
            "--root" => out.root = Root(value()?.into()),
            "--pod" => out.selectors.push(Selector::parse("pod", &value()?)?),
            "--target" => out.selectors.push(Selector::parse("target", &value()?)?),
            other => return Err(usage(&format!("unknown flag {other}"))),
        }
    }
    if out.selectors.is_empty() {
        return Err(usage("no --pod or --target"));
    }
    if out.interval.is_zero() || out.window < out.interval {
        return Err(usage("the window must hold at least one interval"));
    }
    Ok(out)
}

fn run() -> Result<bool, LlError> {
    let args = parse_args(std::env::args().skip(1))?;
    if let Some(cpu) = args.pin {
        // off the exclusive CPUs: the probe must not disturb what it measures
        pin_current(&CpuList::one(cpu))?;
    }
    let mut watch = Watch::start(args.root, args.opts.clone(), &args.selectors)?;
    let start = Instant::now();
    let mut next_scan = start;
    while start.elapsed() < args.window {
        watch.sample();
        if Instant::now() >= next_scan {
            watch.scan_node();
            next_scan += Duration::from_secs(1);
        }
        std::thread::sleep(args.interval);
    }
    let window_s = start.elapsed().as_secs_f64();
    let checks = watch.finish(window_s)?;
    let (outcome, counted, failed) = result(&checks);
    let mut out = std::io::stdout().lock();
    let w = |out: &mut std::io::StdoutLock<'_>, s: String| {
        writeln!(out, "{s}").map_err(|e| LlError::io("stdout", e))
    };
    w(
        &mut out,
        header(
            args.opts.contract,
            &args.opts.node,
            window_s,
            watch.rounds(),
        ),
    )?;
    for c in &checks {
        w(&mut out, line(c))?;
    }
    w(&mut out, result_line(outcome, counted, failed))?;
    Ok(outcome == Outcome::Pass)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "ll-probe: {e}");
            ExitCode::from(2)
        }
    }
}
