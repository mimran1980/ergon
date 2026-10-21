//! Benchmark client.
//!
//! ```text
//! io-bench binance  --variants recv-busy,uring-spin,uring-sqpoll --cpus 2,4,6 --sqpoll-cpu 8 --duration 1200 --out DIR
//! io-bench lab      --server IP:PORT --ca cert.pem --variants ... --cpus ... --duration S --out DIR
//! io-bench pingpong --server IP:PORT --ca cert.pem --variant V --cpu N --rate MSGS_PER_S --duration S --out DIR [--feeds K]
//! io-bench capture  --cpu N --duration 600 --out capture.bin
//! io-bench clockcost
//! ```

use std::process::ExitCode;

fn main() -> ExitCode {
    match imp::main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("io-bench: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        Err("io-bench needs Linux (SO_TIMESTAMPING, epoll, io_uring)".into())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::error::Error as StdError;
    use std::fmt::Write as _;
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier};

    use ergon_io::{Backend as Variant, Driver, Target, WsConn, WsStats};
    use io_bench::app::{Capture, Mode, PingPong, Recorder};
    use io_bench::args::Args;
    use io_bench::clock::{
        counter_meta, counter_name, monotonic_ns, pin_current_thread, read_costs, realtime_ns,
    };
    use io_bench::record::{Record, write_records};
    use io_bench::runner::{self, App, LoopStats, RunOpts};
    use io_bench::stalls::StallLog;
    use io_bench::{Error, tls};
    use rustls::ClientConfig;
    use rustls::pki_types::CertificateDer;
    use rustls::pki_types::pem::PemObject;

    type BoxResult<T> = Result<T, Box<dyn StdError>>;

    const BUF: usize = 4 << 20;
    static MLOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    /// `TcpExt: BusyPollRxPackets` from `/proc/net/netstat` (system-wide).
    fn busy_poll_rx_packets() -> u64 {
        let Ok(t) = std::fs::read_to_string("/proc/net/netstat") else {
            return 0;
        };
        let mut lines = t.lines();
        while let (Some(h), Some(v)) = (lines.next(), lines.next()) {
            if h.starts_with("TcpExt:") {
                let idx = h.split_whitespace().position(|k| k == "BusyPollRxPackets");
                return idx
                    .and_then(|i| v.split_whitespace().nth(i))
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);
            }
        }
        0
    }

    fn socket_info(c: &WsConn) -> (u32, i32) {
        let fd = c.raw_fd();
        (
            ergon_io::sys::incoming_napi_id(fd).unwrap_or(0),
            ergon_io::sys::incoming_cpu(fd).unwrap_or(-1),
        )
    }

    /// Connects every target, hands the connections to a driver with `cfg`.
    fn connect_all(
        targets: &[Target],
        cfg: &Arc<ClientConfig>,
        dcfg: ergon_io::Config,
    ) -> Result<Driver<WsConn>, String> {
        let mut d = Driver::new(dcfg).map_err(|e| format!("driver: {e}"))?;
        for t in targets {
            let c = WsConn::connect(t, cfg.clone(), BUF)
                .map_err(|e| format!("connect {}: {e}", t.host))?;
            d.add(c).map_err(|e| format!("add {}: {e}", t.host))?;
        }
        Ok(d)
    }

    /// What one loop reports: its counters, per-connection socket info and stats.
    type Ran = (LoopStats, Vec<(u32, i32)>, Vec<WsStats>);

    /// Runs `app` on `driver` until the deadline and collects per-connection
    /// info; slow iterations go to `stalls`.
    fn run_app<A: App>(
        driver: &mut Driver<WsConn>,
        app: &mut A,
        duration_s: i64,
        stalls: &mut StallLog,
    ) -> Result<Ran, Error> {
        let opts = RunOpts {
            deadline_mono_ns: monotonic_ns() + duration_s * 1_000_000_000,
        };
        let st = runner::run(driver, app, opts, stalls)?;
        let sockets = driver.iter().map(|(_, c)| socket_info(c)).collect();
        let stats = driver.iter().map(|(_, c)| c.stats()).collect();
        Ok((st, sockets, stats))
    }

    /// The stall log for one loop: iterations over `--stall-us` (default 20).
    fn stall_log(a: &Args) -> BoxResult<StallLog> {
        let us: f64 = a.num_or("stall-us", 20.0)?;
        Ok(StallLog::new(us, 1 << 20))
    }

    pub fn main() -> BoxResult<()> {
        let a = Args::from_env();
        // Lock current and future memory so record vectors and receive buffers are
        // faulted in at allocation, not on the timed path. Needs root (the runs
        // use sudo); without it the run continues and the meta file says so.
        // SAFETY: plain syscall with valid flags.
        let locked = std::env::var_os("IO_BENCH_NO_MLOCK").is_none()
            && unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } == 0;
        MLOCKED.store(locked, std::sync::atomic::Ordering::Relaxed);
        // Calibrate the counter before any loop starts (the loops track their
        // iteration length in counter ticks).
        let _ = io_bench::clock::ticks_per_us();
        match a.command.as_str() {
            "binance" => run_multi(&a, Mode::Binance),
            "lab" => run_multi(&a, Mode::Lab),
            "pingpong" => run_pingpong(&a),
            "capture" => run_capture(&a),
            "clockcost" => {
                let (rt, tsc) = read_costs(10_000_000);
                println!(
                    "clock_gettime(CLOCK_REALTIME): {rt:.1} ns/read\n{}: {tsc:.1} ns/read\n{}",
                    counter_name(),
                    counter_meta().trim_end()
                );
                Ok(())
            }
            other => Err(format!("unknown command {other:?}; see the module docs").into()),
        }
    }

    fn resolve(host: &str, port: u16) -> BoxResult<SocketAddr> {
        (host, port)
            .to_socket_addrs()?
            .find(SocketAddr::is_ipv4)
            .ok_or_else(|| format!("{host} has no IPv4 address").into())
    }

    fn target(host: &str, port: u16, path: String) -> BoxResult<Target> {
        // Resolved once up front so every variant connects to the same address.
        let mut t = Target::new(host, port, path).with_addr(resolve(host, port)?);
        if let Ok(k) = std::env::var("BINANCE_API_KEY") {
            t = t.with_header("X-MBX-APIKEY", k);
        }
        Ok(t)
    }

    /// Binance connections (shared by every variant, resolved once) and the global
    /// stream list.
    fn binance_targets(a: &Args) -> BoxResult<(Vec<Target>, Vec<String>)> {
        let spot = a.list("spot-streams", "btcusdt@trade,ethusdt@trade,solusdt@trade");
        let fpub = a.list("fut-public", "btcusdt@bookTicker,ethusdt@bookTicker");
        let fmkt = a.list("fut-market", "btcusdt@aggTrade");
        let spot_host = a.str_or("spot-host", "stream.binance.com");
        let spot_port: u16 = a.num_or("spot-port", 9443)?;
        let fut_host = a.str_or("fut-host", "fstream.binance.com");
        let mut targets = Vec::new();
        let mut streams = Vec::new();
        if !spot.is_empty() {
            targets.push(target(
                &spot_host,
                spot_port,
                format!("/stream?streams={}&timeUnit=MICROSECOND", spot.join("/")),
            )?);
            streams.extend(spot);
        }
        if !fpub.is_empty() {
            targets.push(target(
                &fut_host,
                443,
                format!("/public/stream?streams={}", fpub.join("/")),
            )?);
            streams.extend(fpub);
        }
        if !fmkt.is_empty() {
            targets.push(target(
                &fut_host,
                443,
                format!("/market/stream?streams={}", fmkt.join("/")),
            )?);
            streams.extend(fmkt);
        }
        Ok((targets, streams))
    }

    fn lab_config(a: &Args) -> BoxResult<(Target, Arc<ClientConfig>)> {
        let server: SocketAddr = a.req("server")?.parse()?;
        let sni = a.str_or("sni", "lab.local");
        let mut roots = rustls::RootCertStore::empty();
        let ca = CertificateDer::from_pem_file(a.req("ca")?).map_err(|e| format!("ca: {e}"))?;
        roots.add(ca)?;
        let cfg = tls::client_config(roots)?;
        Ok((Target::new(sni, server.port(), "/").with_addr(server), cfg))
    }

    struct ThreadResult {
        variant: Variant,
        records: Vec<Record>,
        loop_stats: LoopStats,
        conn_stats: Vec<WsStats>,
        sockets: Vec<(u32, i32)>,
        dropped: u64,
        unparsed: u64,
    }

    fn run_multi(a: &Args, mode: Mode) -> BoxResult<()> {
        let variants: Vec<Variant> = a
            .list("variants", "epoll-spin,uring-spin")
            .iter()
            .map(|s| s.parse())
            .collect::<Result<_, Error>>()?;
        let cpus: Vec<usize> = a
            .list("cpus", "")
            .iter()
            .map(|s| s.parse())
            .collect::<Result<_, _>>()?;
        let sqpoll_cpu: Option<u32> = a
            .list("sqpoll-cpu", "")
            .first()
            .map(|s| s.parse())
            .transpose()?;
        let duration_s: i64 = a.num_or("duration", 60)?;
        let busy_poll_us: u32 = a.num_or("busy-poll", 50)?;
        let wait_us: u32 = a.num_or("wait-us", 100)?;
        let capacity: usize = a.num_or("capacity", 4_000_000)?;
        let conns_per_variant: usize = a.num_or("conns", 1)?;
        let out = PathBuf::from(a.str_or("out", "results"));
        std::fs::create_dir_all(&out)?;

        let (targets, streams, cfg) = match mode {
            Mode::Binance => {
                let (t, s) = binance_targets(a)?;
                (t, s, tls::client_config(tls::webpki_roots())?)
            }
            Mode::Lab => {
                let (t, cfg) = lab_config(a)?;
                // Each connection names its slot, so `replay_server --mode
                // mixed` sends connection j of every variant the same stream
                // (the other modes ignore the path).
                let targets = (0..conns_per_variant)
                    .map(|j| Target {
                        path: format!("/slot/{j}"),
                        ..t.clone()
                    })
                    .collect();
                (targets, vec!["lab".to_owned()], cfg)
            }
        };
        let bp0 = busy_poll_rx_packets();
        let barrier = Arc::new(Barrier::new(variants.len()));
        let stall_us: f64 = a.num_or("stall-us", 20.0)?;
        let flags = a.clone();
        let mut handles = Vec::new();
        for (vi, &variant) in variants.iter().enumerate() {
            let cpu = cpus.get(vi).copied();
            let targets = targets.clone();
            let streams = streams.clone();
            let cfg = cfg.clone();
            let barrier = barrier.clone();
            let out_dir = out.clone();
            let flags = flags.clone();
            let opts_sqpoll = if variant == Variant::UringSqpoll {
                sqpoll_cpu
            } else {
                None
            };
            handles.push(
                std::thread::Builder::new()
                    .name(format!("{}-{vi}", variant.name()))
                    .spawn(move || -> Result<ThreadResult, String> {
                        if let Some(c) = cpu {
                            pin_current_thread(c).map_err(|e| format!("pin {c}: {e}"))?;
                        }
                        let mut dcfg = runner::config(variant, busy_poll_us, wait_us, opts_sqpoll);
                        runner::busy_poll_flags(&mut dcfg, &flags)
                            .map_err(|e| format!("flags: {e}"))?;
                        let mut driver = connect_all(&targets, &cfg, dcfg)
                            .map_err(|e| format!("{}: {e}", variant.name()))?;
                        let mut rec = Recorder::new(vi as u8, mode, &streams, capacity);
                        let mut stalls = StallLog::new(stall_us, 1 << 20);
                        barrier.wait();
                        let (loop_stats, sockets, conn_stats) =
                            run_app(&mut driver, &mut rec, duration_s, &mut stalls)
                                .map_err(|e| format!("{}: {e}", variant.name()))?;
                        stalls
                            .write(&out_dir.join(format!("stalls-{vi}.txt")))
                            .map_err(|e| format!("stalls: {e}"))?;
                        Ok(ThreadResult {
                            variant,
                            sockets,
                            conn_stats,
                            dropped: rec.dropped,
                            unparsed: rec.unparsed,
                            records: rec.records,
                            loop_stats,
                        })
                    })?,
            );
        }
        let mut results = Vec::new();
        let mut errors = Vec::new();
        for h in handles {
            match h.join() {
                Ok(Ok(r)) => results.push(r),
                Ok(Err(e)) => errors.push(e),
                Err(_) => errors.push("variant thread panicked".to_owned()),
            }
        }
        if !errors.is_empty() {
            return Err(errors.join("; ").into());
        }
        let mut all = Vec::new();
        for r in &results {
            all.extend_from_slice(&r.records);
        }
        // The run's end, before any of its own file I/O.
        let end_real = realtime_ns();
        write_records(&out.join("records.bin"), &all)?;
        let mut meta = base_meta(
            if mode == Mode::Binance {
                "binance"
            } else {
                "lab"
            },
            duration_s,
            busy_poll_us,
            end_real,
        );
        let _ = writeln!(meta, "wait_us={wait_us}");
        let _ = writeln!(meta, "conns_per_variant={}", targets.len());
        let _ = writeln!(
            meta,
            "busy_poll_rx_packets={}",
            busy_poll_rx_packets() - bp0
        );
        let _ = writeln!(
            meta,
            "variants={}",
            variants
                .iter()
                .map(|v| v.name())
                .collect::<Vec<_>>()
                .join(",")
        );
        let _ = writeln!(meta, "streams={}", streams.join(","));
        let _ = writeln!(
            meta,
            "targets={}",
            targets
                .iter()
                .map(|t| format!(
                    "{}={}",
                    t.host,
                    t.addr.map_or_else(String::new, |a| a.to_string())
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        for (vi, r) in results.iter().enumerate() {
            write_thread_meta(&mut meta, vi, r);
        }
        // Beside the percentiles: what arrived and what produced no sample.
        // A receive-only run does not see the sender's schedule.
        let completed: u64 = results.iter().map(|r| r.records.len() as u64).sum();
        let censored: u64 = results.iter().map(|r| r.dropped + r.unparsed).sum();
        io_bench::stats::Work::received(completed, censored).write_meta(&mut meta);
        std::fs::write(out.join("meta.txt"), &meta)?;
        print!("{meta}");
        Ok(())
    }

    fn base_meta(mode: &str, duration_s: i64, busy_poll_us: u32, end_real_ns: i64) -> String {
        let read = |p: &str| {
            std::fs::read_to_string(p)
                .map(|s| s.trim().to_owned())
                .unwrap_or_default()
        };
        let (rt, tsc) = read_costs(2_000_000);
        let mut m = String::new();
        let _ = writeln!(m, "mode={mode}");
        let _ = writeln!(m, "duration_s={duration_s}");
        let _ = writeln!(m, "busy_poll_us={busy_poll_us}");
        let _ = writeln!(m, "end_realtime_ns={end_real_ns}");
        let _ = writeln!(m, "kernel={}", read("/proc/sys/kernel/osrelease"));
        let _ = writeln!(m, "arch={}", std::env::consts::ARCH);
        let _ = writeln!(
            m,
            "clocksource={}",
            read("/sys/devices/system/clocksource/clocksource0/current_clocksource")
        );
        let _ = writeln!(m, "clock_cost_realtime_ns={rt:.1}");
        // The raw counter's read cost, whichever counter it is (`counter=`).
        let _ = writeln!(m, "clock_cost_rdtsc_ns={tsc:.1}");
        m.push_str(&counter_meta());
        let _ = writeln!(
            m,
            "mlockall={}",
            MLOCKED.load(std::sync::atomic::Ordering::Relaxed)
        );
        m
    }

    fn write_thread_meta(m: &mut String, vi: usize, r: &ThreadResult) {
        let l = &r.loop_stats;
        let _ = writeln!(m, "loop.{vi}.variant={}", r.variant.name());
        let _ = writeln!(m, "loop.{vi}.spins={}", l.spins);
        let d = &l.driver;
        let _ = writeln!(m, "loop.{vi}.recvmsg_calls={}", d.recvmsg_calls);
        let _ = writeln!(m, "loop.{vi}.recvmsg_eagain={}", d.recvmsg_eagain);
        let _ = writeln!(m, "loop.{vi}.epoll_wait_calls={}", d.epoll_wait_calls);
        let _ = writeln!(m, "loop.{vi}.send_calls={}", d.send_calls);
        let _ = writeln!(m, "loop.{vi}.enter_calls={}", d.enter_calls);
        let _ = writeln!(m, "loop.{vi}.syscalls={}", d.syscalls());
        let _ = writeln!(m, "loop.{vi}.completions={}", d.completions);
        let _ = writeln!(m, "loop.{vi}.messages={}", d.messages);
        let _ = writeln!(m, "loop.{vi}.utime_ticks={}", l.utime_ticks);
        let _ = writeln!(m, "loop.{vi}.stime_ticks={}", l.stime_ticks);
        let _ = writeln!(m, "loop.{vi}.napi={}", l.napi);
        let _ = writeln!(m, "loop.{vi}.vol_ctx_switches={}", l.vol_ctx_switches);
        let _ = writeln!(m, "loop.{vi}.invol_ctx_switches={}", l.invol_ctx_switches);
        let _ = writeln!(
            m,
            "loop.{vi}.gap_max_us={:.1}",
            l.gap_max_ticks as f64 / l.ticks_per_us.max(1e-3)
        );
        let _ = writeln!(m, "loop.{vi}.gaps_over_5us={}", l.gaps_over[0]);
        let _ = writeln!(m, "loop.{vi}.gaps_over_20us={}", l.gaps_over[1]);
        let _ = writeln!(m, "loop.{vi}.gaps_over_100us={}", l.gaps_over[2]);
        let _ = writeln!(m, "loop.{vi}.dropped={}", r.dropped);
        let _ = writeln!(m, "loop.{vi}.unparsed={}", r.unparsed);
        for (ci, c) in r.conn_stats.iter().enumerate() {
            let (napi_id, cpu) = r.sockets.get(ci).copied().unwrap_or((0, -1));
            let _ = writeln!(
                m,
                "conn.{vi}.{ci}=bytes_in:{} records:{} messages:{} span_copy_bytes:{} compact_bytes:{} frag_copy_bytes:{} pings:{} napi_id:{napi_id} rx_cpu:{cpu}",
                c.bytes_in,
                c.records,
                c.messages,
                c.span_copy_bytes,
                c.compact_bytes,
                c.frag_copy_bytes,
                c.pings
            );
        }
    }

    fn run_pingpong(a: &Args) -> BoxResult<()> {
        let variant: Variant = a.req("variant")?.parse()?;
        let cpu: Option<usize> = a.list("cpu", "").first().map(|s| s.parse()).transpose()?;
        let sqpoll_cpu: Option<u32> = a
            .list("sqpoll-cpu", "")
            .first()
            .map(|s| s.parse())
            .transpose()?;
        let rate: i64 = a.num_or("rate", 1000)?;
        let duration_s: i64 = a.num_or("duration", 30)?;
        let payload: usize = a.num_or("payload", 64)?;
        let busy_poll_us: u32 = a.num_or("busy-poll", 50)?;
        let wait_us: u32 = a.num_or("wait-us", 100)?;
        // Feed connections on the same loop (`/slot/<j>` of a `replay_server
        // --mode mixed`, which echoes the order connection).
        let feeds: usize = a.num_or("feeds", 0)?;
        let out = PathBuf::from(a.str_or("out", "results"));
        std::fs::create_dir_all(&out)?;
        if let Some(c) = cpu {
            pin_current_thread(c)?;
        }
        let (t, cfg) = lab_config(a)?;
        let mut dcfg = runner::config(variant, busy_poll_us, wait_us, sqpoll_cpu);
        runner::busy_poll_flags(&mut dcfg, a)?;
        let mut driver = connect_all(std::slice::from_ref(&t), &cfg, dcfg)?;
        let order = driver
            .iter()
            .next()
            .map(|(id, _)| id)
            .ok_or("the order connection did not open")?;
        for j in 0..feeds {
            let ft = Target {
                path: format!("/slot/{j}"),
                ..t.clone()
            };
            let c = WsConn::connect(&ft, cfg.clone(), BUF).map_err(|e| format!("feed {j}: {e}"))?;
            driver.add(c).map_err(|e| format!("feed {j}: {e}"))?;
        }
        let interval = if rate <= 0 { 0 } else { 1_000_000_000 / rate };
        // Room for every round trip: the paced rate, or (back to back) well
        // above the 50k/s this path reaches. mlockall faults it all in.
        let per_s = if rate > 0 { rate } else { 100_000 };
        let mut app = PingPong::new(0, interval, payload, (per_s * duration_s + 1_000) as usize)
            .with_order(order);
        let mut stalls = stall_log(a)?;
        let (stats, sockets, conn_stats) = run_app(&mut driver, &mut app, duration_s, &mut stalls)?;
        // The run's end, before any of its own file I/O: `Work` and
        // `end_realtime_ns` must describe the same instant.
        let end_real = realtime_ns();
        let work = app.work(end_real);
        stalls.write(&out.join("stalls-0.txt"))?;
        write_records(&out.join("records.bin"), &app.records)?;
        let mut meta = base_meta("pingpong", duration_s, busy_poll_us, end_real);
        let _ = writeln!(meta, "variants={}", variant.name());
        let _ = writeln!(meta, "rate={rate}");
        let _ = writeln!(meta, "payload={payload}");
        let _ = writeln!(meta, "wait_us={wait_us}");
        let _ = writeln!(meta, "conns_per_variant={}", feeds + 1);
        let _ = writeln!(meta, "feeds={feeds}");
        let _ = writeln!(meta, "feed_messages={}", app.feed_messages);
        let r = ThreadResult {
            variant,
            sockets,
            records: Vec::new(),
            loop_stats: stats,
            conn_stats,
            dropped: app.dropped,
            unparsed: 0,
        };
        write_thread_meta(&mut meta, 0, &r);
        // Beside the percentiles: what the run planned and what became of it.
        work.write_meta(&mut meta);
        std::fs::write(out.join("meta.txt"), &meta)?;
        print!("{meta}");
        Ok(())
    }

    fn run_capture(a: &Args) -> BoxResult<()> {
        let duration_s: i64 = a.num_or("duration", 600)?;
        // The loop busy-spins: keep it off the housekeeping CPU.
        if let Some(c) = a.list("cpu", "").first() {
            pin_current_thread(c.parse()?)?;
        }
        let out = PathBuf::from(a.str_or("out", "capture.bin"));
        let (targets, _streams) = binance_targets(a)?;
        let cfg = tls::client_config(tls::webpki_roots())?;
        let mut driver = connect_all(
            &targets,
            &cfg,
            runner::config(Variant::EpollSpin, 0, 0, None),
        )?;
        let mut app = Capture {
            out: Vec::with_capacity(512 << 20),
            count: 0,
            closed: None,
        };
        run_app(
            &mut driver,
            &mut app,
            duration_s,
            &mut StallLog::new(1e9, 0),
        )?;
        write_capture(&out, &app)?;
        Ok(())
    }

    fn write_capture(path: &Path, app: &Capture) -> BoxResult<()> {
        std::fs::write(path, &app.out)?;
        println!(
            "captured {} messages ({} bytes) into {}",
            app.count,
            app.out.len(),
            path.display()
        );
        Ok(())
    }
}
