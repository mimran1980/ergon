//! `ll-app`: the ping or the pong (`LL_ROLE`), as a container's command.
//!
//! Start-up order (each helper is created while its creator is still on the
//! shared CPUs; only `ll-hot` pins):
//!
//! 1. `main` reads `ERGON_*` (exit 2 when unset) and, if its own mask is not
//!    within the helper CPUs, pins itself there.
//! 2. `main` starts logging (spawns `ll-log`), the signal handler, the
//!    host checks, and `ll-stats`.
//! 3. `main` spawns `ll-hot`, which starts Aeron (spawning `aeron-client`
//!    on the shared mask), connects, pins itself and reports.
//! 4. `main` runs the self-check over `/proc/self/task`; a FAIL stops
//!    `ll-hot` before it measures (exit 2).
//! 5. `ll-hot` measures; `main` joins it, writes the report to
//!    `/dev/termination-log`, stops logging and prints the report.
//!
//! Exit: 0 after a report, 1 when the run failed, 2 when the placement or
//! the configuration is wrong.

use std::io::Write as _;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use ergon_tune::{Host, HostCheckPolicy, HotSet, Needs, Role as TuneRole, Thread, k8s_checks};
use low_latency_k8s::LlError;
use low_latency_k8s::cpus::{ErgonEnv, current_affinity, pin_current};
use low_latency_k8s::logging::{LogConfig, Logging, init_stdout};
use low_latency_k8s::pingpong::{AppConfig, Clock, HotOutcome, Pinned, Role, Shared, spawn_hot};
use low_latency_k8s::probe::expect::Plan;
use low_latency_k8s::report::{HotInfo, LatencyReport, LogInfo, RunInfo};
use low_latency_k8s::selfcheck::{REQUIRED, judge, observe};
use low_latency_k8s::stats::Counters;

/// A failure and its exit code.
struct Exit(u8, LlError);

impl From<LlError> for Exit {
    fn from(e: LlError) -> Self {
        let code = match e {
            LlError::MissingEnv(_)
            | LlError::BadValue { .. }
            | LlError::CpuList { .. }
            | LlError::SelfCheck(_) => 2,
            _ => 1,
        };
        Self(code, e)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(Exit(code, e)) => {
            // stderr, unbuffered: logging may be gone
            let _ = writeln!(std::io::stderr(), "ll-app: {e}");
            ExitCode::from(code)
        }
    }
}

/// Prints evidence lines straight to stdout (one write each, under the
/// lock): they must not be dropped as a full log queue would.
fn evidence(lines: &[String]) -> Result<(), LlError> {
    let mut out = std::io::stdout().lock();
    for line in lines {
        writeln!(out, "{line}").map_err(|e| LlError::io("stdout", e))?;
    }
    out.flush().map_err(|e| LlError::io("stdout", e))
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn run() -> Result<(), Exit> {
    let cfg = AppConfig::from_process()?;
    let ergon = ErgonEnv::from_process()?;
    let hot_cpu = cfg.hot_spec.resolve(&ergon)?;
    // 1. main on the helper CPUs
    let mine = current_affinity()?;
    let main_repinned = !mine.is_subset(&ergon.helpers);
    if main_repinned {
        pin_current(&ergon.helpers)?;
    }
    // 2. logging, signals, host checks, ll-stats
    let log_cfg = LogConfig {
        level: cfg.log_level,
        queue_lines: cfg.log_queue_lines,
        thread_name: "ll-log".to_owned(),
    };
    let mut logging = init_stdout(&log_cfg, Some(&ergon.helpers))?;
    tracing::info!(
        role = cfg.role.name(),
        isolated = %ergon.isolated,
        shared = %ergon.shared,
        helpers = %ergon.helpers,
        hot_cpu = %hot_cpu,
        main_inherited = %mine,
        main_repinned,
        log_worker = ?logging.placement,
        "ll-app starting"
    );
    let shared = Arc::new(Shared::default());
    let on_signal = Arc::clone(&shared);
    ctrlc::set_handler(move || on_signal.stop.store(true, Ordering::Relaxed))
        .map_err(|e| LlError::Thread(format!("signal handler: {e}")))?;
    host_checks(&hot_cpu)?;
    let stats = spawn_stats(Arc::clone(&shared), ergon.helpers)?;
    // 3. ll-hot
    let hot = spawn_hot(cfg.clone(), hot_cpu, Arc::clone(&shared))?;
    let wait = cfg.driver_wait + cfg.connect_wait + Duration::from_secs(30);
    let pinned: Pinned = match hot.pinned.recv_timeout(wait) {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            stop_all(&shared, stats);
            return Err(join_err(hot.join, e).into());
        }
        Err(_) => {
            stop_all(&shared, stats);
            return Err(LlError::Timeout(format!("ll-hot not pinned within {wait:?}")).into());
        }
    };
    // 4. self-check
    let plan = Plan::from_env(&std::env::vars().collect())?;
    let check = judge(&plan, &observe(5, Duration::from_millis(20))?, &REQUIRED);
    evidence(&check.lines)?;
    if check.failed > 0 {
        let _ = hot.go.send(false);
        let _ = hot.join.join();
        stop_all(&shared, stats);
        return Err(LlError::SelfCheck(check.failed).into());
    }
    let _ = hot.go.send(true);
    // 5. measure, report
    let outcome = hot
        .join
        .join()
        .map_err(|_| LlError::Thread("ll-hot panicked".to_owned()))??;
    stop_all(&shared, stats);
    tracing::info!(ended = %outcome.ended, messages = outcome.messages, lost = outcome.lost, "ll-hot done");
    let report = build_report(&cfg, &outcome, pinned, &logging);
    finish(&report, &mut logging)?;
    Ok(())
}

fn join_err(
    join: std::thread::JoinHandle<Result<HotOutcome, LlError>>,
    fallback: LlError,
) -> LlError {
    match join.join() {
        Ok(Err(e)) => e,
        _ => fallback,
    }
}

fn stop_all(shared: &Shared, stats: std::thread::JoinHandle<()>) {
    shared.stop.store(true, Ordering::Relaxed);
    let _ = stats.join();
}

/// The `ergon-tune` Kubernetes checks (K1–K7) for the hot CPU, printed under
/// `ERGON_HOST_CHECKS` (`on` by default; `require` refuses a warning).
fn host_checks(hot_cpu: &low_latency_k8s::cpus::CpuList) -> Result<(), LlError> {
    let policy = HostCheckPolicy::from_env()
        .map_err(|e| LlError::bad("ERGON_HOST_CHECKS", "", e.to_string()))?;
    let mut set = HotSet::new();
    for cpu in hot_cpu.iter() {
        set.push(u32::try_from(cpu).unwrap_or(u32::MAX), TuneRole::Loop);
    }
    let findings = k8s_checks(
        &Host::system(),
        &set,
        &Needs::new(Thread::Current, TuneRole::Loop),
    );
    for line in policy.lines("ll-app host-check", &findings) {
        tracing::info!("{line}");
    }
    policy.refusal(&findings).map_or(Ok(()), |e| {
        Err(LlError::bad("ERGON_HOST_CHECKS", "require", e.to_string()))
    })
}

/// `ll-stats`: once a second, the hot thread's counters, and whether this
/// thread and every helper are still within the helper CPUs (the plugin's
/// rule: a sleepy helper checks where it runs each time it wakes).
fn spawn_stats(
    shared: Arc<Shared>,
    helpers: low_latency_k8s::cpus::CpuList,
) -> Result<std::thread::JoinHandle<()>, LlError> {
    std::thread::Builder::new()
        .name("ll-stats".to_owned())
        .spawn(move || {
            let c = &shared.counters;
            let mut tick = 0_u64;
            while !shared.stopping() {
                std::thread::sleep(Duration::from_millis(100));
                tick += 1;
                if !tick.is_multiple_of(10) {
                    continue;
                }
                if let Ok(cpu) = low_latency_k8s::cpus::current_cpu()
                    && !helpers.contains(cpu)
                {
                    tracing::warn!(cpu, helpers = %helpers, "ll-stats woke off the helper CPUs");
                }
                if let Ok(threads) = observe(1, Duration::ZERO) {
                    for t in threads.iter().filter(|t| t.comm != "ll-hot" && !t.mask.is_subset(&helpers)) {
                        tracing::warn!(thread = %t.comm, tid = t.tid, allowed = %t.mask, helpers = %helpers, "a helper drifted off the helper CPUs");
                    }
                }
                tracing::info!(
                    messages = Counters::get(&c.messages),
                    lost = Counters::get(&c.lost),
                    max_gap_ns = Counters::get(&c.max_gap_ns),
                    hot_warn_events = Counters::get(&c.warn_events),
                    "ll-stats"
                );
            }
        })
        .map_err(|e| LlError::Thread(format!("spawn ll-stats: {e}")))
}

fn build_report(
    cfg: &AppConfig,
    out: &HotOutcome,
    pinned: Pinned,
    logging: &Logging,
) -> LatencyReport {
    LatencyReport {
        run: RunInfo {
            role: cfg.role.name().to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            cpu_build: env_or("LL_CPU_BUILD", "-"),
            counter_hz: Clock::new().counter_hz(Duration::from_millis(50)),
            transport: env_or("RUSTERON_MEDIA_DRIVER_TRANSPORT", "-"),
            overlay: env_or("LL_OVERLAY", "-"),
            msg_bytes: cfg.msg_bytes,
            rate: cfg.rate,
            warmup: out.warmed,
            samples: out.messages,
            lost: out.lost,
        },
        rtt: if cfg.role == Role::Ping {
            out.rtt
        } else {
            None
        },
        top: out.top.clone(),
        gap: out.gap.clone(),
        hot: HotInfo {
            cpu: pinned.cpu,
            ran_on: out.ran_on,
            voluntary: out.ctxt.0,
            nonvoluntary: out.ctxt.1,
        },
        log: LogInfo {
            dropped_lines: logging.dropped_lines(),
            hot_warn_events: out.warn_events,
            suppressed: out.suppressed,
        },
    }
}

/// Writes the report to the termination log (when the container has one),
/// stops logging, then prints the report.
fn finish(report: &LatencyReport, logging: &mut Logging) -> Result<(), LlError> {
    let text = report.text();
    let path = env_or("LL_TERMINATION_LOG", "/dev/termination-log");
    if std::path::Path::new(&path).exists() {
        // kubectl shows at most 4 KiB of it
        let cut = text.len().min(4096);
        std::fs::write(&path, &text.as_bytes()[..cut])
            .map_err(|e| LlError::io(format!("write {path}"), e))?;
    }
    logging.shutdown();
    let dropped = logging.dropped_lines();
    let mut lines = report.lines();
    if let Some(log) = lines.last_mut()
        && dropped != report.log.dropped_lines
    {
        // lines dropped after the report was built, during the shutdown
        *log = log.replace(
            &format!("dropped_lines={}", report.log.dropped_lines),
            &format!("dropped_lines={dropped}"),
        );
    }
    evidence(&lines)
}
