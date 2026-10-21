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

use ergon_md_dpdk::configure_media_transport_from_env;
use log::info;
use rusteron_client::{Aeron, AeronContext, IntoCString};
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
fn env_endpoints(key: &str, default: &str) -> Vec<Endpoint> {
    env_str(key, default).split(',').map(Endpoint::parse).collect()
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
            if let Err(e) = std::fs::write(&args.report, report.to_json()) {
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
            std::process::exit(if report.ok { 0 } else { 1 });
        }
        Err(e) => {
            eprintln!("harness failed: {e}");
            std::process::exit(1);
        }
    }
}

fn run(args: &Args) -> Result<Report, Box<dyn Error>> {
    let default_ctrl = env_str("RUSTERON_HARNESS_PUB_CTRL", "127.0.0.1:40101");
    let default_ctrl2 = env_str("RUSTERON_HARNESS_PUB_CTRL2", "");

    let cfg = ScenarioCfg {
        role: args.role.clone(),
        name: args.scenario.clone(),
        pub_ctrl: Endpoint::parse(&default_ctrl),
        pub_ctrl2: if default_ctrl2.is_empty() {
            let mut ep = Endpoint::parse(&default_ctrl);
            ep.port = ep.port.saturating_add(1);
            ep
        } else {
            Endpoint::parse(&default_ctrl2)
        },
        sub_endpoints: env_endpoints("RUSTERON_HARNESS_SUB_ENDPOINTS", "127.0.0.1:40102"),
        destinations: env_endpoints("RUSTERON_HARNESS_DESTINATIONS", "127.0.0.1:40102"),
        msgs: env_u64("RUSTERON_HARNESS_MSGS", 1000),
        payload: env_u64("RUSTERON_HARNESS_PAYLOAD", 130) as usize,
        stream: env_u64("RUSTERON_HARNESS_STREAM", 32931) as i32,
        timeout: Duration::from_secs(env_u64("RUSTERON_HARNESS_TIMEOUT_SECS", 30)),
        fill_byte: fill_byte(&args.role),
        expect_byte: fill_byte(if args.role == "primary" { "secondary" } else { "primary" }),
        mtu: env_u64("RUSTERON_HARNESS_MTU", 1408) as usize,
        duration: {
            let d = env_u64("RUSTERON_HARNESS_DURATION_SECS", 0);
            if d > 0 { Some(Duration::from_secs(d)) } else { None }
        },
        load_rps: env_u64("RUSTERON_HARNESS_LOAD_RPS", 0),
        latency_samples: {
            let p = env_str("RUSTERON_HARNESS_LATENCY_SAMPLES", "");
            if p.is_empty() { None } else { Some(p) }
        },
    };

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
    let (driver, client_ctx, transport) = if args.standalone {
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
        ctx.set_dir(&dir.into_c_string())?;
        (None, ctx, None)
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
        driver_ctx.set_sender_cpu_affinity(env_u64("RUSTERON_HARNESS_SENDER_CPU", 1) as i32)?;
        driver_ctx.set_receiver_cpu_affinity(env_u64("RUSTERON_HARNESS_RECEIVER_CPU", 2) as i32)?;
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

        let driver = AeronDriver::launch_embedded_guard(driver_ctx.clone(), false);
        let dir = driver_ctx.get_dir().to_string();
        let ctx = AeronContext::new()?;
        ctx.set_dir(&dir.into_c_string())?;
        (Some(driver), ctx, transport)
    };

    // --- client to the (embedded or standalone) driver ---
    let aeron = Aeron::new(&client_ctx)?;
    aeron.start()?;

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

    // Client, then the driver (stops and joins its agents), then the transport.
    drop(aeron);
    drop(driver);
    drop(client_ctx);
    drop(transport);
    Ok(report)
}
