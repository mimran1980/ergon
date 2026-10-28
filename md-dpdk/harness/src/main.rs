//! DPDK interoperability harness for ergon-md-dpdk.
//!
//! `scripts/test-dpdk-vdev.sh` starts one `dpdk-harness` process per role
//! (primary/secondary). Each process configures its embedded media driver's
//! transport from the environment — `RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena`
//! selects the DPDK vdev path, absent selects the default kernel-UDP driver —
//! then runs the named scenario and writes a JSON report to `--report`.

mod report;
mod scenario;

use std::error::Error;
use std::time::Duration;

use ergon_md_dpdk::clock::{RateSource, TransportClock};
use ergon_md_dpdk::configure_media_transport_from_env;
use log::info;
use rusteron_client::{Aeron, AeronCnc, AeronContext, IntoCString};
use rusteron_media_driver::{AeronDriver, AeronDriverContext, AeronIdleStrategyKind};

use report::Report;
use scenario::{Endpoint, ScenarioCfg};

struct Args {
    role: String,
    scenario: String,
    report: String,
    /// Connect to a separately-started standalone media driver instead of
    /// launching an embedded one (plan §11.3 standalone row).
    standalone: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut role = None;
    let mut scenario = None;
    let mut report = None;
    let mut standalone = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--role" => role = Some(it.next().ok_or("--role needs a value")?),
            "--scenario" => scenario = Some(it.next().ok_or("--scenario needs a value")?),
            "--report" => report = Some(it.next().ok_or("--report needs a value")?),
            "--standalone" => standalone = true,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(Args {
        role: role.ok_or("missing --role (primary|secondary)")?,
        scenario: scenario.ok_or("missing --scenario")?,
        report: report.ok_or("missing --report")?,
        standalone,
    })
}

fn env_str(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// `host:port`-list env var, comma-separated.
fn env_endpoints(key: &str, default: &str) -> Result<Vec<Endpoint>, Box<dyn Error>> {
    env_str(key, default).split(',').map(Endpoint::parse).collect()
}

/// `key` as a `T` (`default` when unset or not a number), or an error where it
/// does not fit one.
fn env_num<T: TryFrom<u64>>(key: &str, default: u64) -> Result<T, Box<dyn Error>>
where
    T::Error: std::fmt::Display,
{
    let v = env_u64(key, default);
    T::try_from(v).map_err(|e| format!("{key}={v}: {e}").into())
}

fn fill_byte(role: &str) -> u8 {
    if role == "primary" { 0xAA } else { 0xBB }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("usage: dpdk-harness --role primary|secondary --scenario NAME --report PATH [--standalone]");
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match run(&args) {
        Ok(report) => {
            let written = report
                .to_json()
                .map_err(|e| e.to_string())
                .and_then(|json| std::fs::write(&args.report, json).map_err(|e| e.to_string()));
            if let Err(e) = written {
                eprintln!("failed to write report {}: {e}", args.report);
                std::process::exit(1);
            }
            println!(
                "[harness-done] role={} scenario={} transport={} ok={} sent={} received={} duration_ms={} backpressure_ops={}",
                report.role,
                report.scenario,
                report.transport,
                report.ok,
                report.sent,
                report.received,
                report.duration_ms,
                report.backpressure_ops
            );
            std::process::exit(i32::from(!report.ok));
        }
        Err(e) => {
            eprintln!("harness failed: {e}");
            std::process::exit(1);
        }
    }
}

/// The scenario's configuration, from `args` and the environment.
fn config(args: &Args) -> Result<ScenarioCfg, Box<dyn Error>> {
    let default_ctrl = env_str("RUSTERON_HARNESS_PUB_CTRL", "127.0.0.1:40101");
    let default_ctrl2 = env_str("RUSTERON_HARNESS_PUB_CTRL2", "");

    Ok(ScenarioCfg {
        role: args.role.clone(),
        name: args.scenario.clone(),
        pub_ctrl: Endpoint::parse(&default_ctrl)?,
        pub_ctrl2: if default_ctrl2.is_empty() {
            let mut ep = Endpoint::parse(&default_ctrl)?;
            ep.port = ep.port.saturating_add(1);
            ep
        } else {
            Endpoint::parse(&default_ctrl2)?
        },
        sub_endpoints: env_endpoints("RUSTERON_HARNESS_SUB_ENDPOINTS", "127.0.0.1:40102")?,
        destinations: env_endpoints("RUSTERON_HARNESS_DESTINATIONS", "127.0.0.1:40102")?,
        msgs: env_u64("RUSTERON_HARNESS_MSGS", 1000),
        payload: env_num("RUSTERON_HARNESS_PAYLOAD", 130)?,
        stream: env_num("RUSTERON_HARNESS_STREAM", 32931)?,
        timeout: Duration::from_secs(env_u64("RUSTERON_HARNESS_TIMEOUT_SECS", 30)),
        fill_byte: fill_byte(&args.role),
        expect_byte: fill_byte(if args.role == "primary" { "secondary" } else { "primary" }),
        mtu: env_num("RUSTERON_HARNESS_MTU", 1408)?,
        duration: {
            let d = env_u64("RUSTERON_HARNESS_DURATION_SECS", 0);
            if d > 0 { Some(Duration::from_secs(d)) } else { None }
        },
        load_rps: env_u64("RUSTERON_HARNESS_LOAD_RPS", 0),
        latency_samples: {
            let p = env_str("RUSTERON_HARNESS_LATENCY_SAMPLES", "");
            if p.is_empty() { None } else { Some(p) }
        },
        hold_dir: {
            let p = env_str("RUSTERON_HARNESS_HOLD_DIR", "");
            if p.is_empty() { None } else { Some(p.into()) }
        },
        pause_at: std::env::var("RUSTERON_HARNESS_PAUSE_AT")
            .ok()
            .and_then(|v| v.parse().ok()),
        hold_secs: Duration::from_secs(env_u64("RUSTERON_HARNESS_HOLD_SECS", 6)),
    })
}

/// The embedded DPDK transport's clock into `report`.
fn record_clock(report: &mut Report, clock: &TransportClock) {
    match clock {
        TransportClock::Counter {
            hz,
            rate,
            eal_hz,
            measured_hz,
        } => {
            report.clock = "counter".to_string();
            report.clock_hz = *hz;
            report.clock_rate = match rate {
                RateSource::Eal => "eal",
                RateSource::Measured => "measured",
            }
            .to_string();
            report.clock_eal_hz = *eal_hz;
            report.clock_measured_hz = *measured_hz;
        }
        TransportClock::Monotonic { reason } => {
            report.clock = "monotonic".to_string();
            report.clock_reason = reason.to_string();
        }
        // A clock this harness does not know yet: named, never taken
        // for the counter.
        other => {
            report.clock = "unknown".to_string();
            report.clock_reason = other.to_string();
        }
    }
}

fn run(args: &Args) -> Result<Report, Box<dyn Error>> {
    let cfg = config(args)?;

    let mut report = Report {
        role: cfg.role.clone(),
        scenario: cfg.name.clone(),
        ..Report::default()
    };

    // --- driver setup: standalone (plan §11.3) or embedded ---
    // The embedded driver's RAII guard must outlive the whole scenario, and the
    // DPDK transport must outlive the driver (its agents poll the transport's
    // ports until they join). Bindings from one pattern drop in reverse, which
    // would close the transport first, so the order is explicit at the end.
    let (driver, client_ctx, transport, driver_dir) = if args.standalone {
        // A separate `media_driver` process owns the DPDK transport
        // and the Aeron dir (pinned via RUSTERON_MEDIA_DRIVER_DIR); this
        // process is only a client.
        let dir = env_str("RUSTERON_HARNESS_DRIVER_DIR", "");
        if dir.is_empty() {
            return Err("--standalone requires RUSTERON_HARNESS_DRIVER_DIR".into());
        }
        report.transport = if env_str("RUSTERON_MEDIA_DRIVER_TRANSPORT", "default") == "dpdk-ena" {
            "standalone-dpdk".to_string()
        } else {
            "standalone-udp".to_string()
        };
        let ctx = AeronContext::new()?;
        ctx.set_dir(&dir.clone().into_c_string())?;
        (None, ctx, None, dir)
    } else {
        // --- media driver context (§6.4 required state) ---
        let driver_ctx = AeronDriverContext::new()?;
        driver_ctx.set_dir_delete_on_shutdown(true)?;
        driver_ctx.set_dir_delete_on_start(true)?;
        driver_ctx.set_dir(
            &format!(
                "{}{}-{}",
                driver_ctx.get_dir(),
                Aeron::epoch_clock(),
                std::process::id()
            )
            .into_c_string(),
        )?;
        driver_ctx.set_sender_cpu_affinity(env_num("RUSTERON_HARNESS_SENDER_CPU", 1)?)?;
        driver_ctx.set_receiver_cpu_affinity(env_num("RUSTERON_HARNESS_RECEIVER_CPU", 2)?)?;
        driver_ctx.set_sender_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)?;
        driver_ctx.set_receiver_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)?;
        driver_ctx.set_sender_wildcard_port_range(20000, 20999)?;
        driver_ctx.set_receiver_wildcard_port_range(21000, 21999)?;
        driver_ctx.set_mtu_length(cfg.mtu)?;

        // --- transport selection: DPDK (vdev/ENA) or default kernel UDP ---
        // The guard owns the native runtime the installed bindings point into,
        // so it is returned with the driver and outlives it.
        let transport = configure_media_transport_from_env(&driver_ctx)
            .map_err(|e| format!("transport configuration failed: {e}"))?;
        report.transport = if transport.is_some() { "dpdk" } else { "udp" }.to_string();
        info!("[transport] selected {}", report.transport);
        if let Some(transport) = &transport {
            // The clock the real EAL's rate gave the transport (the vdev
            // script asserts the counter, at CNTFRQ_EL0's rate on aarch64).
            info!("[transport] clock: {}", transport.clock());
            record_clock(&mut report, transport.clock());
            report.cntfrq_hz = cntfrq_hz();
        }

        let driver = AeronDriver::launch_embedded_guard(driver_ctx.clone(), false);
        let dir = driver_ctx.get_dir().to_string();
        let ctx = AeronContext::new()?;
        ctx.set_dir(&dir.clone().into_c_string())?;
        (Some(driver), ctx, transport, dir)
    };

    // --- client to the (embedded or standalone) driver ---
    let aeron = Aeron::new(&client_ctx)?;
    aeron.start()?;

    // --- the script's go: its taps are bridged (RUSTERON_HARNESS_START_FILE) ---
    let start_file = env_str("RUSTERON_HARNESS_START_FILE", "");
    if !start_file.is_empty() {
        let started = std::time::Instant::now();
        while !std::path::Path::new(&start_file).exists() {
            if started.elapsed() >= cfg.timeout {
                return Err(format!("{start_file} did not appear within {:?}", cfg.timeout).into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    // --- run the scenario and fold the result into the report ---
    let res = scenario::run(&cfg, &aeron);
    report.ok = res.ok;
    report.sent = res.sent;
    report.received = res.received;
    report.bytes = res.bytes;
    report.duration_ms = res.duration_ms;
    report.latency_p50_ns = res.latency_p50_ns;
    report.latency_p99_ns = res.latency_p99_ns;
    report.latency_max_ns = res.latency_max_ns;
    report.offered_per_sec = res.offered_per_sec;
    report.delivered_per_sec = res.delivered_per_sec;
    report.backpressure_ops = res.backpressure_ops;
    report.gaps = res.gaps;
    report.detail = res.detail;
    report.error = res.error;
    // The driver's counters and error log, while it still runs: the vdev
    // scenarios assert on them (ICMP echoes, ARP misses, MTU refusals).
    match snapshot_driver(&driver_dir) {
        Ok((counters, errors)) => {
            report.dpdk_counters = counters;
            report.driver_errors = errors;
        }
        Err(e) => report.detail.push(format!("driver counters unreadable: {e}")),
    }

    // Client, then the driver (stops and joins its agents), then the transport.
    drop(aeron);
    drop(driver);
    drop(client_ctx);
    drop(transport);
    Ok(report)
}

/// The DPDK transport's counters, summed by `<role>.<dir>.<name>`, and the
/// driver's distinct error log, read from the driver's `CnC` file.
fn snapshot_driver(dir: &str) -> Result<(String, String), Box<dyn Error>> {
    let cnc = AeronCnc::open(&dir.into_c_string())?;
    let mut values = Vec::new();
    cnc.counters_reader()
        .foreach_counter_fn(|value, _id, _type_id, _key, label| {
            if let Some(key) = report::dpdk_counter_key(label) {
                values.push((key, value));
            }
        });
    let mut errors = Vec::new();
    cnc.error_log_read_fn(
        |count: i32, _first_ns: i64, _last_ns: i64, error: &str| {
            errors.push(format!("{count}x {}", error.replace('\n', " ")));
        },
        0,
    );
    Ok((report::sum_counters(values), errors.join(" | ")))
}

/// The generic timer's rate, `CNTFRQ_EL0`, read here (0 off aarch64): what
/// the transport's counter rate must equal there.
#[cfg_attr(
    target_arch = "aarch64",
    allow(
        unsafe_code,
        reason = "CNTFRQ_EL0 is read with one instruction, which only inline assembly gives"
    )
)]
fn cntfrq_hz() -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        let hz: u64;
        // SAFETY: Linux lets EL0 read CNTFRQ_EL0; the read touches no memory.
        unsafe { std::arch::asm!("mrs {}, cntfrq_el0", out(reg) hz, options(nomem, nostack, preserves_flags)) };
        hz
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        0
    }
}
