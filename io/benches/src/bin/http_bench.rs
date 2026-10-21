//! HTTP client round-trip benchmark: one request in flight, order-sized POSTs to
//! the lab server (`replay_server --mode http`), or GETs to a live endpoint.
//!
//! ```text
//! http_bench --client ergon --backend recv-busy --server 10.0.0.5:9445 --ca cert.pem --rate 1000 --duration 30 --cpu 2 --out DIR
//! http_bench --client curl  --server 10.0.0.5:9445 --ca cert.pem ...      (built with --features curl or curl-static)
//! http_bench --client reqwest --server 10.0.0.5:9445 --ca cert.pem ...    (built with --features reqwest)
//! http_bench --client ergon --backend epoll-spin --live api.binance.com --path /api/v3/time --rate 2 ...
//! http_bench ... --connect background   (connect on the driver's helper thread)
//! http_bench --connects 50 --connect background --backend recv-busy --server ... --ca ... --cpu 2
//!            (reconnect cost: the longest single poll of each connect)
//! ```
//!
//! Every client sends the same bytes: `POST /api/v3/order` with form-encoded
//! parameters, a 64-hex-digit signature, `X-MBX-APIKEY` and `Content-Type`.
//! Records: `src_ns` = just before the request is handed to the client,
//! `src2_ns` = when it was due on the paced schedule (0 when unpaced; the
//! schedule never slips, so the time from it includes any wait behind a
//! stall), `user_ns` = when the client returns the complete response;
//! analysed as ping-pong round trips.

use std::process::ExitCode;

fn main() -> ExitCode {
    match imp::main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("http-bench: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        Err("http-bench needs Linux".into())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::error::Error as StdError;
    use std::fmt::Write as _;
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::path::PathBuf;
    use std::sync::Arc;

    use ergon_io::{
        Backend, ConnId, Driver, Error, Handler, HttpConn, HttpDial, HttpHandler, HttpOptions,
        Response, RxMeta, Target,
    };
    use io_bench::args::Args;
    use io_bench::clock::{monotonic_ns, pin_current_thread, realtime_ns, thread_ctx_switches};
    use io_bench::record::{Record, write_records};
    use io_bench::stalls::StallLog;
    use io_bench::tls;
    use rustls::pki_types::CertificateDer;
    use rustls::pki_types::pem::PemObject;

    type BoxResult<T> = Result<T, Box<dyn StdError>>;

    // The same request bytes as every other lab client (`mio_bench http`).
    pub use io_bench::order::{API_KEY, order_body};

    pub struct Run {
        pub label: String,
        pub records: Vec<Record>,
        pub connect_ns: i64,
        pub extra: String,
        /// The timed loop's slow iterations.
        pub stalls: StallLog,
        /// The work of the run, shown beside its percentiles.
        pub work: io_bench::stats::Work,
    }

    struct Plan {
        server: SocketAddr,
        host: String,
        path: String,
        live: bool,
        rate: i64,
        duration_s: i64,
        capacity: usize,
        stall_us: f64,
    }

    impl Plan {
        fn interval_ns(&self) -> i64 {
            if self.rate <= 0 {
                0
            } else {
                1_000_000_000 / self.rate
            }
        }
        #[cfg_attr(not(any(feature = "curl", feature = "reqwest")), allow(dead_code))]
        fn url(&self) -> String {
            if self.server.port() == 443 {
                format!("https://{}{}", self.host, self.path)
            } else {
                format!("https://{}:{}{}", self.host, self.server.port(), self.path)
            }
        }
    }

    pub fn main() -> BoxResult<()> {
        let a = Args::from_env();
        // As io_bench: fault every page in at allocation, not on the timed path
        // (needs root; without it the run continues and the meta file says so).
        // SAFETY: plain syscall with valid flags.
        let locked = std::env::var_os("IO_BENCH_NO_MLOCK").is_none()
            && unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } == 0;
        let _ = io_bench::clock::ticks_per_us();
        if let Some(c) = a.list("cpu", "").first() {
            pin_current_thread(c.parse()?)?;
        }
        let live = a.list("live", "").first().cloned();
        let (server, host) = match &live {
            Some(h) => (resolve(h, 443)?, h.clone()),
            None => (a.req("server")?.parse()?, a.str_or("sni", "lab.local")),
        };
        let plan = Plan {
            server,
            host,
            path: a.str_or(
                "path",
                if live.is_some() {
                    "/api/v3/time"
                } else {
                    "/api/v3/order"
                },
            ),
            live: live.is_some(),
            rate: a.num_or("rate", 1000)?,
            duration_s: a.num_or("duration", 30)?,
            capacity: a.num_or("capacity", 2_000_000)?,
            stall_us: a.num_or("stall-us", 20.0)?,
        };
        let out = PathBuf::from(a.str_or("out", "results"));
        std::fs::create_dir_all(&out)?;
        if let Some(n) = a.list("connects", "").first() {
            return connect_test(&a, &plan, n.parse()?, &out);
        }
        let client = a.str_or("client", "ergon");
        let (v0, _) = thread_ctx_switches();
        let run = match client.as_str() {
            "ergon" => ergon(&a, &plan)?,
            "client" => pooled_client(&a, &plan)?,
            #[cfg(feature = "curl")]
            "curl" => curl_client::run(&a, &plan)?,
            #[cfg(feature = "reqwest")]
            "reqwest" => reqwest_client::run(&a, &plan)?,
            other => return Err(format!("client {other:?} not built in").into()),
        };
        let (v1, _) = thread_ctx_switches();
        write_records(&out.join("records.bin"), &run.records)?;
        run.stalls.write(&out.join("stalls-0.txt"))?;
        let mut m = String::new();
        let _ = writeln!(m, "mode=pingpong");
        let _ = writeln!(m, "mlockall={locked}");
        let _ = writeln!(m, "loop.0.gap_max_us={:.1}", run.stalls.max_us());
        let _ = writeln!(m, "loop.0.gaps_over_5us={}", run.stalls.over[0]);
        let _ = writeln!(m, "loop.0.gaps_over_20us={}", run.stalls.over[1]);
        let _ = writeln!(m, "loop.0.gaps_over_100us={}", run.stalls.over[2]);
        let _ = writeln!(m, "variants={}", run.label);
        let _ = writeln!(m, "rate={}", plan.rate);
        let _ = writeln!(m, "duration_s={}", plan.duration_s);
        let _ = writeln!(m, "target={} {}{}", plan.server, plan.host, plan.path);
        let _ = writeln!(m, "live={}", plan.live);
        let _ = writeln!(m, "requests={}", run.records.len());
        let _ = writeln!(m, "connect_us={:.1}", run.connect_ns as f64 / 1000.0);
        let _ = writeln!(m, "loop.0.vol_ctx_switches={}", v1 - v0);
        m.push_str(&run.extra);
        // Beside the percentiles: what the run planned and what became of it.
        run.work.write_meta(&mut m);
        let _ = writeln!(
            m,
            "kernel={}",
            std::fs::read_to_string("/proc/sys/kernel/osrelease")
                .unwrap_or_default()
                .trim()
        );
        std::fs::write(out.join("meta.txt"), &m)?;
        print!("{m}");
        Ok(())
    }

    fn resolve(host: &str, port: u16) -> BoxResult<SocketAddr> {
        (host, port)
            .to_socket_addrs()?
            .find(SocketAddr::is_ipv4)
            .ok_or_else(|| format!("{host} has no IPv4 address").into())
    }

    fn rustls_config(a: &Args, live: bool) -> BoxResult<Arc<rustls::ClientConfig>> {
        if live {
            return Ok(tls::client_config(tls::webpki_roots())?);
        }
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from_pem_file(a.req("ca")?).map_err(|e| format!("ca: {e}"))?)?;
        Ok(tls::client_config(roots)?)
    }

    /// Paces sends: busy-waits to the next slot (never sleeps); every spin is
    /// a loop iteration for the stall log. Returns the slot (monotonic), or 0
    /// when unpaced.
    fn wait_slot(next: &mut i64, interval: i64, stalls: &mut StallLog, done: u64) -> i64 {
        if interval == 0 {
            return 0;
        }
        while monotonic_ns() < *next {
            stalls.tick(done);
        }
        *next += interval;
        *next - interval
    }

    /// When a request sent at `sent` (wall clock, read just after `mono`) was
    /// due: `sent` less its lateness against `slot`; 0 when unpaced.
    const fn due(slot: i64, mono: i64, sent: i64) -> i64 {
        if slot == 0 { 0 } else { sent - (mono - slot) }
    }

    /// Pushes a sample, counting it as dropped when the vector is full: the
    /// percentiles are over the recorded samples alone, and this is part of the
    /// work they leave out.
    fn push_sample(v: &mut Vec<Record>, r: Record, dropped: &mut u64) {
        if v.len() < v.capacity() {
            v.push(r);
        } else {
            *dropped += 1;
        }
    }

    /// The schedule bookkeeping every paced client loop keeps for
    /// [`io_bench::stats::Work`]: when the first request was due, and how many
    /// went out after their slot.
    struct Pace {
        first_due_ns: i64,
        late: u64,
    }

    impl Pace {
        fn new() -> Self {
            Self {
                first_due_ns: 0,
                late: 0,
            }
        }

        /// Records one send: `slot` is [`wait_slot`]'s return (0 when unpaced),
        /// `mono` when the send was attempted, `sent` its wall-clock time.
        fn sent(&mut self, slot: i64, mono: i64, sent: i64, interval_ns: i64) {
            if slot == 0 {
                return;
            }
            if io_bench::stats::Work::is_late(mono, slot, interval_ns) {
                self.late += 1;
            }
            if self.first_due_ns == 0 {
                self.first_due_ns = due(slot, mono, sent);
            }
        }

        /// The work of the run: `issued` sends, `completed` of them with a
        /// recorded sample, out of `interval_ns`'s schedule (0 when unpaced).
        fn work(&self, interval_ns: i64, issued: u64, completed: u64) -> io_bench::stats::Work {
            io_bench::stats::Work::paced(
                realtime_ns(),
                self.first_due_ns,
                interval_ns,
                issued,
                completed,
                self.late,
            )
        }
    }

    struct Collect {
        records: Vec<Record>,
        sent_ns: i64,
        due_ns: i64,
        seq: u64,
        done: bool,
        status: u16,
        opened: bool,
        /// Responses dropped because the record vector was full.
        dropped: u64,
        closed: Option<Error>,
    }

    impl Handler for Collect {
        fn on_open(&mut self, _: ConnId) {
            self.opened = true;
        }
        fn on_closed(&mut self, _: ConnId, e: Error) {
            self.closed.get_or_insert(e);
        }
    }

    impl HttpHandler for Collect {
        fn on_response(&mut self, _: ConnId, _: u64, r: &Response<'_>, meta: &RxMeta) {
            let now = realtime_ns();
            self.status = r.status;
            push_sample(
                &mut self.records,
                Record {
                    id: i64::try_from(self.seq).unwrap_or(i64::MAX),
                    src_ns: self.sent_ns,
                    src2_ns: self.due_ns,
                    rx_sw_ns: meta.rx_sw_ns,
                    user_ns: now,
                    read_seq: meta.read_seq,
                    ..Record::default()
                },
                &mut self.dropped,
            );
            self.done = true;
        }
    }

    fn ergon(a: &Args, plan: &Plan) -> BoxResult<Run> {
        let backend: Backend = a.str_or("backend", "epoll-spin").parse()?;
        let mut cfg = ergon_io::Config::new(backend);
        io_bench::runner::busy_poll_flags(&mut cfg, a)?;
        cfg.sqpoll_cpu = a
            .list("sqpoll-cpu", "")
            .first()
            .map(|s| s.parse())
            .transpose()?;
        let mut d: Driver<HttpConn> = Driver::new(cfg)?;
        let mut target = Target::new(plan.host.clone(), plan.server.port(), "")
            .with_addr(plan.server)
            .with_header("User-Agent", "ergon-io-bench")
            .with_header("X-MBX-APIKEY", API_KEY);
        if !plan.live {
            target = target.with_header("Content-Type", "application/x-www-form-urlencoded");
        }
        let dial = HttpDial::new(target, rustls_config(a, plan.live)?, HttpOptions::default())?;
        let mut h = Collect {
            records: Vec::with_capacity(plan.capacity),
            sent_ns: 0,
            due_ns: 0,
            seq: 0,
            done: false,
            status: 0,
            opened: false,
            dropped: 0,
            closed: None,
        };
        let background = a.str_or("connect", "inline") == "background";
        let t0 = monotonic_ns();
        let id = if background {
            d.connect_background(dial)?
        } else {
            d.connect(dial)?
        };
        // The longest single poll while connecting: what the loop pays.
        let mut worst_poll_ns = 0i64;
        while !h.opened {
            let p0 = monotonic_ns();
            d.poll(&mut h)?;
            worst_poll_ns = worst_poll_ns.max(monotonic_ns() - p0);
            if let Some(e) = h.closed.take() {
                return Err(format!("connect: {e}").into());
            }
        }
        let connect_ns = monotonic_ns() - t0;
        let interval = plan.interval_ns();
        let end = monotonic_ns() + plan.duration_s * 1_000_000_000;
        let mut next = monotonic_ns();
        let method = if plan.live { "GET" } else { "POST" };
        let mut stalls = StallLog::new(plan.stall_us, 1 << 20);
        let mut pace = Pace::new();
        stalls.start(0);
        while monotonic_ns() < end {
            let slot = wait_slot(&mut next, interval, &mut stalls, h.seq);
            h.seq += 1;
            let body = if plan.live {
                String::new()
            } else {
                order_body(h.seq)
            };
            h.done = false;
            stalls.tick(h.seq);
            let mono = monotonic_ns();
            h.sent_ns = realtime_ns();
            h.due_ns = due(slot, mono, h.sent_ns);
            pace.sent(slot, mono, h.sent_ns, interval);
            let mut c = d.get_mut(id).ok_or("connection closed")?;
            c.request(method, &plan.path, &[], body.as_bytes())?;
            d.flush()?;
            while !h.done {
                stalls.tick(h.seq - 1);
                d.poll(&mut h)?;
                if let Some(e) = h.closed.take() {
                    return Err(e.into());
                }
            }
            if h.status != 200 {
                return Err(format!("status {}", h.status).into());
            }
        }
        let s = *d.stats();
        let mut extra = String::new();
        let _ = writeln!(extra, "syscalls={}", s.syscalls());
        let _ = writeln!(extra, "connect_step_max_ns={}", s.connect_step_max_ns);
        let _ = writeln!(
            extra,
            "connect_mode={}",
            if background { "background" } else { "inline" }
        );
        let _ = writeln!(extra, "connect_worst_poll_ns={worst_poll_ns}");
        let st = d.get(id).ok_or("connection closed")?.stats();
        let _ = writeln!(
            extra,
            "span_copy_bytes={} chunk_copy_bytes={}",
            st.span_copy_bytes, st.chunk_copy_bytes
        );
        let _ = writeln!(extra, "loop.0.dropped={}", h.dropped);
        let work = pace.work(interval, h.seq, h.records.len() as u64);
        Ok(Run {
            label: format!("ergon/{backend}"),
            records: h.records,
            connect_ns,
            extra,
            stalls,
            work,
        })
    }

    /// The pooled `ergon_io::HttpClient`, native or libcurl (`--impl`), through
    /// the same API an application uses.
    fn pooled_client(a: &Args, plan: &Plan) -> BoxResult<Run> {
        use ergon_io::{ClientHandler, CurlDrive, HttpClient, HttpClientConfig, HttpImpl};
        struct Done {
            sent_ns: i64,
            due_ns: i64,
            seq: u64,
            done: bool,
            status: u16,
            error: Option<String>,
            records: Vec<Record>,
            /// Responses dropped because the record vector was full.
            dropped: u64,
        }
        impl ClientHandler for Done {
            fn on_response(&mut self, _: u64, r: &Response<'_>) {
                let now = realtime_ns();
                self.status = r.status;
                push_sample(
                    &mut self.records,
                    Record {
                        id: i64::try_from(self.seq).unwrap_or(i64::MAX),
                        src_ns: self.sent_ns,
                        src2_ns: self.due_ns,
                        user_ns: now,
                        ..Record::default()
                    },
                    &mut self.dropped,
                );
                self.done = true;
            }
            fn on_error(&mut self, _: u64, _: ergon_io::Sent, e: Error) {
                self.error = Some(e.to_string());
                self.done = true;
            }
        }
        let imp: HttpImpl = a.str_or("impl", "native").parse()?;
        let backend: Backend = a.str_or("backend", "recv-busy").parse()?;
        let mut cfg = HttpClientConfig::new(imp);
        cfg.driver = ergon_io::Config::new(backend);
        io_bench::runner::busy_poll_flags(&mut cfg.driver, a)?;
        cfg.driver.sqpoll_cpu = a
            .list("sqpoll-cpu", "")
            .first()
            .map(|s| s.parse())
            .transpose()?;
        cfg.pool = a.num_or("pool", 1)?;
        cfg.curl_drive = if a.str_or("curl-drive", "perform") == "socket-action" {
            CurlDrive::SocketAction
        } else {
            CurlDrive::Perform
        };
        if !plan.live {
            cfg.curl_ca_file = Some(a.req("ca")?);
            // The lab server's POST is harmless to receive twice; the
            // measurement needs libcurl on the same request as native.
            cfg.curl_allow_non_idempotent_methods = true;
        }
        let mut target = Target::new(plan.host.clone(), plan.server.port(), "")
            .with_addr(plan.server)
            .with_header("User-Agent", "ergon-io-bench")
            .with_header("X-MBX-APIKEY", API_KEY);
        if !plan.live {
            target = target.with_header("Content-Type", "application/x-www-form-urlencoded");
        }
        let t0 = monotonic_ns();
        let mut c = HttpClient::new(&cfg, target, rustls_config(a, plan.live)?)?;
        let mut h = Done {
            sent_ns: 0,
            due_ns: 0,
            seq: 0,
            done: false,
            status: 0,
            error: None,
            records: Vec::with_capacity(plan.capacity),
            dropped: 0,
        };
        // Every connection of the pool open before timing starts: a pool
        // still connecting would put its handshakes in the measured loop.
        let deadline = t0 + 30_000_000_000;
        while c.idle() < cfg.pool {
            c.poll(&mut h)?;
            if monotonic_ns() > deadline {
                return Err(format!(
                    "only {} of the pool's {} connections opened in 30 s",
                    c.idle(),
                    cfg.pool
                )
                .into());
            }
        }
        let connect_ns = monotonic_ns() - t0;
        let interval = plan.interval_ns();
        let end = monotonic_ns() + plan.duration_s * 1_000_000_000;
        let mut next = monotonic_ns();
        let method = if plan.live { "GET" } else { "POST" };
        let mut stalls = StallLog::new(plan.stall_us, 1 << 20);
        let mut pace = Pace::new();
        stalls.start(0);
        while monotonic_ns() < end {
            let slot = wait_slot(&mut next, interval, &mut stalls, h.seq);
            h.seq += 1;
            let body = if plan.live {
                String::new()
            } else {
                order_body(h.seq)
            };
            h.done = false;
            stalls.tick(h.seq);
            let mono = monotonic_ns();
            h.sent_ns = realtime_ns();
            h.due_ns = due(slot, mono, h.sent_ns);
            pace.sent(slot, mono, h.sent_ns, interval);
            c.request(method, &plan.path, &[], body.as_bytes())?;
            while !h.done {
                stalls.tick(h.seq - 1);
                c.poll(&mut h)?;
            }
            if let Some(e) = h.error.take() {
                return Err(e.into());
            }
            if h.status != 200 {
                return Err(format!("status {}", h.status).into());
            }
        }
        let label = match imp {
            HttpImpl::Native => format!("client-native/{backend}"),
            HttpImpl::Curl => format!(
                "client-curl-{}/{}",
                a.str_or("curl-drive", "perform"),
                a.str_or("curl-label", "distro")
            ),
        };
        // The first request with libcurl includes its connect; drop it. It is
        // issued work with no sample, so it counts as censored either way.
        let mut records = h.records;
        if imp == HttpImpl::Curl && !records.is_empty() {
            records.remove(0);
        }
        let work = pace.work(interval, h.seq, records.len() as u64);
        Ok(Run {
            label,
            records,
            connect_ns,
            extra: format!(
                "pool={}\nconns_per_variant={}\nloop.0.dropped={}\n",
                cfg.pool, cfg.pool, h.dropped
            ),
            stalls,
            work,
        })
    }

    /// Connects and closes `n` times, polling throughout, and records the
    /// longest single poll of each connect: what a busy-spin loop pays for a
    /// reconnect. Writes `connect.txt` (one line per connect).
    fn connect_test(a: &Args, plan: &Plan, n: usize, out: &std::path::Path) -> BoxResult<()> {
        let backend: Backend = a.str_or("backend", "recv-busy").parse()?;
        let background = a.str_or("connect", "inline") == "background";
        let mut d: Driver<HttpConn> = Driver::new(ergon_io::Config::new(backend))?;
        let cfg = rustls_config(a, plan.live)?;
        let mut h = Collect {
            records: Vec::new(),
            sent_ns: 0,
            due_ns: 0,
            seq: 0,
            done: false,
            status: 0,
            opened: false,
            dropped: 0,
            closed: None,
        };
        let mut worst = Vec::with_capacity(n);
        let mut total = Vec::with_capacity(n);
        for _ in 0..n {
            let target =
                Target::new(plan.host.clone(), plan.server.port(), "").with_addr(plan.server);
            let dial = HttpDial::new(target, cfg.clone(), HttpOptions::default())?;
            h.opened = false;
            let t0 = monotonic_ns();
            let id = if background {
                d.connect_background(dial)?
            } else {
                d.connect(dial)?
            };
            let mut w = 0i64;
            while !h.opened {
                let p0 = monotonic_ns();
                d.poll(&mut h)?;
                w = w.max(monotonic_ns() - p0);
                if let Some(e) = h.closed.take() {
                    return Err(format!("connect: {e}").into());
                }
            }
            total.push(monotonic_ns() - t0);
            // Keep polling a little after open: the first delivery happens here.
            let settle = monotonic_ns() + 5_000_000;
            while monotonic_ns() < settle {
                let p0 = monotonic_ns();
                d.poll(&mut h)?;
                w = w.max(monotonic_ns() - p0);
            }
            worst.push(w);
            d.close(id);
            for _ in 0..1000 {
                d.poll(&mut h)?;
            }
        }
        let mut text = String::new();
        let _ = writeln!(
            text,
            "# backend={backend} mode={} connects={n}",
            if background { "background" } else { "inline" }
        );
        let _ = writeln!(text, "# worst_poll_ns total_connect_ns");
        for (w, t) in worst.iter().zip(&total) {
            let _ = writeln!(text, "{w} {t}");
        }
        let mut sorted = worst.clone();
        sorted.sort_unstable();
        let pick = |q: f64| sorted[((sorted.len() - 1) as f64 * q) as usize];
        let _ = writeln!(
            text,
            "# worst poll per connect: p50 {} ns, p90 {} ns, max {} ns; step_max_ns {}",
            pick(0.5),
            pick(0.9),
            pick(1.0),
            d.stats().connect_step_max_ns
        );
        std::fs::write(out.join("connect.txt"), &text)?;
        print!(
            "{}",
            text.lines()
                .filter(|l| l.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
        );
        println!();
        Ok(())
    }

    #[cfg(feature = "curl")]
    mod curl_client {
        use std::fmt::Write as _;

        use curl::easy::{Easy2, Handler as CurlHandler, HttpVersion, List, WriteError};
        use curl::multi::Multi;

        use super::{
            API_KEY, Args, BoxResult, Pace, Plan, Record, Run, StallLog, due, monotonic_ns,
            order_body, push_sample, realtime_ns, wait_slot,
        };

        struct Sink(Vec<u8>);

        impl CurlHandler for Sink {
            fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
                self.0.extend_from_slice(data);
                Ok(data.len())
            }
        }

        pub fn run(a: &Args, plan: &Plan) -> BoxResult<Run> {
            let v = curl::Version::get();
            let label = format!("curl-{}/{}", a.str_or("curl-label", "distro"), v.version());
            let mut easy = Easy2::new(Sink(Vec::with_capacity(4096)));
            easy.url(&plan.url())?;
            let mut resolve = List::new();
            resolve.append(&format!(
                "{}:{}:{}",
                plan.host,
                plan.server.port(),
                plan.server.ip()
            ))?;
            easy.resolve(resolve)?;
            if !plan.live {
                easy.cainfo(a.req("ca")?)?;
            }
            easy.http_version(HttpVersion::V11)?;
            easy.signal(false)?;
            easy.tcp_nodelay(true)?;
            let mut headers = List::new();
            headers.append("User-Agent: ergon-io-bench")?;
            headers.append(&format!("X-MBX-APIKEY: {API_KEY}"))?;
            if !plan.live {
                headers.append("Content-Type: application/x-www-form-urlencoded")?;
            }
            // Never wait for 100-continue.
            headers.append("Expect:")?;
            easy.http_headers(headers)?;
            if !plan.live {
                easy.post(true)?;
            }
            let multi = Multi::new();
            let mut records = Vec::with_capacity(plan.capacity);
            let interval = plan.interval_ns();
            let end = monotonic_ns() + plan.duration_s * 1_000_000_000;
            let mut next = monotonic_ns();
            let mut connect_ns = 0;
            let mut seq = 0u64;
            let mut slot = Some(easy);
            let mut dropped = 0u64;
            let mut pace = Pace::new();
            let mut stalls = StallLog::new(plan.stall_us, 1 << 20);
            stalls.start(0);
            while monotonic_ns() < end {
                let due_slot = wait_slot(&mut next, interval, &mut stalls, seq);
                seq += 1;
                let mut e = slot.take().ok_or("easy handle lost")?;
                e.get_mut().0.clear();
                if !plan.live {
                    e.post_fields_copy(order_body(seq).as_bytes())?;
                }
                let t_mono = monotonic_ns();
                let sent = realtime_ns();
                pace.sent(due_slot, t_mono, sent, interval);
                let handle = multi.add2(e)?;
                let result = loop {
                    stalls.tick(seq - 1);
                    multi.perform()?;
                    let mut r = None;
                    multi.messages(|m| {
                        if let Some(res) = m.result_for2(&handle) {
                            r = Some(res);
                        }
                    });
                    if let Some(r) = r {
                        break r;
                    }
                };
                let now = realtime_ns();
                let mut e = multi.remove2(handle)?;
                result?;
                let code = e.response_code()?;
                if code != 200 {
                    return Err(format!("status {code}").into());
                }
                if seq == 1 {
                    // The first request includes connect and TLS: issued work
                    // with no sample, so it counts as censored.
                    connect_ns = monotonic_ns() - t_mono;
                } else {
                    push_sample(
                        &mut records,
                        Record {
                            id: i64::try_from(seq).unwrap_or(i64::MAX),
                            src_ns: sent,
                            src2_ns: due(due_slot, t_mono, sent),
                            user_ns: now,
                            ..Record::default()
                        },
                        &mut dropped,
                    );
                }
                e.get_mut().0.clear();
                slot = Some(e);
            }
            let mut extra = String::new();
            let _ = writeln!(
                extra,
                "curl_version={} ssl={} libz={} async_dns={} http2={} host={}",
                v.version(),
                v.ssl_version().unwrap_or("-"),
                v.libz_version().unwrap_or("-"),
                v.feature_async_dns(),
                v.feature_http2(),
                v.host()
            );
            let _ = writeln!(extra, "first_request_includes_connect=1");
            let _ = writeln!(extra, "loop.0.dropped={dropped}");
            let work = pace.work(interval, seq, records.len() as u64);
            Ok(Run {
                label,
                records,
                connect_ns,
                extra,
                stalls,
                work,
            })
        }
    }

    #[cfg(feature = "reqwest")]
    mod reqwest_client {
        use super::{
            API_KEY, Args, BoxResult, Pace, Plan, Record, Run, StallLog, due, monotonic_ns,
            order_body, push_sample, realtime_ns, wait_slot,
        };

        pub fn run(a: &Args, plan: &Plan) -> BoxResult<Run> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                let mut b = reqwest::Client::builder()
                    .http1_only()
                    .tcp_nodelay(true)
                    .resolve(&plan.host, plan.server)
                    .user_agent("ergon-io-bench");
                if !plan.live {
                    let pem = std::fs::read(a.req("ca")?)?;
                    b = b.add_root_certificate(reqwest::Certificate::from_pem(&pem)?);
                }
                let client = b.build()?;
                let url = plan.url();
                let mut records = Vec::with_capacity(plan.capacity);
                let interval = plan.interval_ns();
                let end = monotonic_ns() + plan.duration_s * 1_000_000_000;
                let mut next = monotonic_ns();
                let mut connect_ns = 0;
                let mut seq = 0u64;
                let mut dropped = 0u64;
                let mut pace = Pace::new();
                // tokio owns the loop: only the pacing spin and one tick per
                // request are timed.
                let mut stalls = StallLog::new(plan.stall_us, 1 << 20);
                stalls.start(0);
                while monotonic_ns() < end {
                    let due_slot = wait_slot(&mut next, interval, &mut stalls, seq);
                    stalls.tick(seq);
                    seq += 1;
                    let t_mono = monotonic_ns();
                    let sent = realtime_ns();
                    pace.sent(due_slot, t_mono, sent, interval);
                    let req = if plan.live {
                        client.get(&url)
                    } else {
                        client
                            .post(&url)
                            .header("Content-Type", "application/x-www-form-urlencoded")
                            .body(order_body(seq))
                    };
                    let resp = req.header("X-MBX-APIKEY", API_KEY).send().await?;
                    let status = resp.status().as_u16();
                    let _ = resp.bytes().await?;
                    let now = realtime_ns();
                    if status != 200 {
                        return Err(format!("status {status}").into());
                    }
                    if seq == 1 {
                        // The first request includes connect and TLS: issued
                        // work with no sample, so it counts as censored.
                        connect_ns = monotonic_ns() - t_mono;
                    } else {
                        push_sample(
                            &mut records,
                            Record {
                                id: i64::try_from(seq).unwrap_or(i64::MAX),
                                src_ns: sent,
                                src2_ns: due(due_slot, t_mono, sent),
                                user_ns: now,
                                ..Record::default()
                            },
                            &mut dropped,
                        );
                    }
                }
                let work = pace.work(interval, seq, records.len() as u64);
                Ok(Run {
                    label: "reqwest/tokio-current-thread".to_owned(),
                    records,
                    connect_ns,
                    extra: format!("first_request_includes_connect=1\nloop.0.dropped={dropped}\n"),
                    stalls,
                    work,
                })
            })
        }
    }
}
