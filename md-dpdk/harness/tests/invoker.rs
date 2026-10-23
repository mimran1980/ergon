//! The lab clients with `HARNESS_DRIVER=invoker`: each embeds its own media
//! driver in Aeron's INVOKER mode, run on its one thread with its client's
//! conductor, over the kernel's UDP on 127.0.0.1 (each process its own
//! `AERON_DIR`). Round trips, offered load and throughput between two such
//! clients, and between one and a client of an external driver; every
//! publication carries `PUB_CHANNEL_PARAMS`; and an embedded driver's `CnC`
//! stays readable after its process exits, every counter in it, so the lab's
//! `counters` snapshots read it unchanged.

use std::ffi::CString;
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// A publication parameter the publishers' counter labels then carry (their
/// channels), and a small log for the test.
const PUB_PARAMS: &str = "term-length=65536";

/// A free UDP port on 127.0.0.1 (released for the client to bind).
fn free_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .expect("a free UDP port")
}

/// A fresh Aeron directory for one client's driver.
fn aeron_dir(tag: &str) -> String {
    let dir = format!(
        "{}/aeron-harness-invoker-{tag}-{}",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A client process, its output collected as it runs.
struct Running {
    what: String,
    child: Child,
    stdout: std::thread::JoinHandle<String>,
    stderr: std::thread::JoinHandle<String>,
}

/// What a client printed, and how it exited.
struct Finished {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

/// Start `bin` (a harness client) with `args`; `driver` is `HARNESS_DRIVER`
/// (`None`: unset), `dir` its `AERON_DIR`.
fn start(bin: &str, args: &[&str], driver: Option<&str>, dir: &str, extra: &[(&str, &str)]) -> Running {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .env("AERON_DIR", dir)
        .env("AERON_DIR_DELETE_ON_START", "true")
        .env("AERON_TERM_BUFFER_LENGTH", "65536")
        .env("PUB_CHANNEL_PARAMS", PUB_PARAMS)
        .env("LABEL", "invoker-test")
        .env("RTT_SECONDS", "10");
    for var in [
        "HARNESS_DRIVER",
        "SUB_CHANNEL_PARAMS",
        "AERON_THREADING_MODE",
        "RUSTERON_MEDIA_DRIVER_TRANSPORT",
        "RUSTERON_MEDIA_DRIVER_DIR",
        "LOOP_STALLS_DIR",
        "LOAD_SPIKES",
        "RTT_HISTOGRAM",
    ] {
        cmd.env_remove(var);
    }
    if let Some(driver) = driver {
        cmd.env("HARNESS_DRIVER", driver);
    }
    for (k, v) in extra {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawning {bin}: {e}"));
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    Running {
        what: format!("{} {}", bin.rsplit('/').next().unwrap(), args.join(" ")),
        child,
        stdout: std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            s
        }),
        stderr: std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        }),
    }
}

impl Running {
    /// Waits up to `secs` for it to exit, killing it (by its PID) past that.
    fn finish(mut self, secs: u64) -> Finished {
        let deadline = Instant::now() + Duration::from_secs(secs);
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let stderr = self.stderr.join().unwrap_or_default();
                panic!("{} did not exit within {secs} s; stderr:\n{stderr}", self.what);
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        Finished {
            status,
            stdout: self.stdout.join().unwrap_or_default(),
            stderr: self.stderr.join().unwrap_or_default(),
        }
    }

    /// SIGTERM, which `rtt xpong` takes as the end of its run.
    fn terminate(self, secs: u64) -> Finished {
        let pid = self.child.id().to_string();
        let killed = Command::new("kill").args(["-TERM", &pid]).status().expect("kill -TERM");
        assert!(killed.success(), "kill -TERM {pid}");
        self.finish(secs)
    }
}

impl Finished {
    /// It exited 0 and said which driver it ran on.
    fn assert_ok(&self, what: &str, driver_line: &str) {
        assert!(
            self.status.success(),
            "{what}: {:?}\nstdout:\n{}\nstderr:\n{}",
            self.status,
            self.stdout,
            self.stderr
        );
        assert!(
            self.stderr.lines().any(|l| l.starts_with(driver_line)),
            "{what}: no {driver_line:?} line on stderr:\n{}",
            self.stderr
        );
    }

    /// Its one result line with `prefix`, split on commas; nothing else on
    /// stdout, which the lab appends to its results as it is.
    fn result(&self, prefix: &str) -> Vec<String> {
        let lines: Vec<&str> = self.stdout.lines().collect();
        assert!(
            lines.len() == 1 && lines[0].starts_with(prefix),
            "expected one {prefix:?} line on stdout, got:\n{}",
            self.stdout
        );
        lines[0].split(',').map(str::to_string).collect()
    }
}

/// `counters <filter>` against `dir` (`extra` its environment's additions).
fn counters(dir: &str, filter: &str, extra: &[(&str, &str)]) -> Finished {
    start(env!("CARGO_BIN_EXE_counters"), &[filter], None, dir, extra).finish(30)
}

/// The `CnC` an INVOKER client left: its publications' counters carry
/// `PUB_CHANNEL_PARAMS` in their channels, and `counters loss` reads Aeron's
/// loss counters from it, the process long gone: here past a 100 ms driver
/// timeout, which a client of the driver would no longer connect within.
fn assert_cnc_left_readable(dir: &str, what: &str) {
    std::thread::sleep(Duration::from_millis(300));
    let stale = [("AERON_DRIVER_TIMEOUT", "100")];
    let published = counters(dir, PUB_PARAMS, &stale);
    assert!(
        published.status.success(),
        "{what}: counters {PUB_PARAMS}: {}",
        published.stderr
    );
    assert!(
        published
            .stdout
            .lines()
            .any(|l| l.contains("pub-lmt") && l.contains(PUB_PARAMS)),
        "{what}: no publication counter with {PUB_PARAMS} in {dir}'s CnC:\n{}",
        published.stdout
    );
    let loss = counters(dir, "loss", &stale);
    assert!(loss.status.success(), "{what}: counters loss: {}", loss.stderr);
    for key in ["naks-sent=", "retransmits-sent=", "short-sends=", "errors="] {
        assert!(
            loss.stdout.contains(key),
            "{what}: counters loss without {key}:\n{}",
            loss.stdout
        );
    }
}

/// The first CPU this process may run on (Linux), else 0.
fn first_allowed_cpu() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            let list = status
                .lines()
                .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))?
                .trim()
                .to_string();
            list.split([',', '-']).next()?.parse().ok()
        })
        .unwrap_or(0)
}

const EMBEDDED: &str = "harness driver: embedded threading=INVOKER";
const EXTERNAL: &str = "harness driver: external";

/// Ping and pong, each on a driver of its own embedded in INVOKER mode.
#[test]
fn rtt_round_trips_between_invoker_clients_whose_cnc_outlives_them() {
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let (ping_dir, pong_dir) = (aeron_dir("ping"), aeron_dir("pong"));
    let rtt = env!("CARGO_BIN_EXE_rtt");
    let ponger = start(rtt, &["xpong", &ping, &pong, "-"], Some("invoker"), &pong_dir, &[]);
    let pinged = start(
        rtt,
        &["xping", &ping, &pong, "2000", "200", "-"],
        Some("invoker"),
        &ping_dir,
        &[],
    )
    .finish(90);
    let ponged = ponger.terminate(30);
    pinged.assert_ok("xping", EMBEDDED);
    ponged.assert_ok("xpong", EMBEDDED);
    let fields = pinged.result("rtt,invoker-test,udp,");
    let samples: u64 = fields[10].parse().expect("samples");
    assert!(samples > 0, "no round trips measured: {fields:?}");
    assert_cnc_left_readable(&ping_dir, "xping");
    assert_cnc_left_readable(&pong_dir, "xpong");
    for dir in [ping_dir, pong_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// An INVOKER client and a client of an external driver (`HARNESS_DRIVER`
/// unset, as before) round trip, the external one's publication carrying
/// `PUB_CHANNEL_PARAMS` too.
#[test]
fn an_invoker_client_round_trips_with_one_on_an_external_driver() {
    use rusteron_media_driver::{AeronDriver, AeronDriverContext};
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let (ping_dir, external_dir) = (aeron_dir("ping-x"), aeron_dir("external"));
    let context = AeronDriverContext::new().unwrap();
    context.set_dir(&CString::new(external_dir.clone()).unwrap()).unwrap();
    context.set_dir_delete_on_start(true).unwrap();
    context.set_dir_delete_on_shutdown(true).unwrap();
    context.set_term_buffer_length(65536).unwrap();
    let external = AeronDriver::launch_embedded_guard(context.clone(), false);

    let rtt = env!("CARGO_BIN_EXE_rtt");
    let ponger = start(rtt, &["xpong", &ping, &pong, "-"], None, &external_dir, &[]);
    let pinged = start(
        rtt,
        &["xping", &ping, &pong, "2000", "200", "-"],
        Some("invoker"),
        &ping_dir,
        &[],
    )
    .finish(90);
    let published = counters(&external_dir, PUB_PARAMS, &[]);
    let ponged = ponger.terminate(30);
    drop(external);
    pinged.assert_ok("xping", EMBEDDED);
    ponged.assert_ok("xpong", EXTERNAL);
    let fields = pinged.result("rtt,invoker-test,udp,");
    assert!(fields[10].parse::<u64>().expect("samples") > 0, "{fields:?}");
    assert!(
        published
            .stdout
            .lines()
            .any(|l| l.contains("pub-lmt") && l.contains(PUB_PARAMS)),
        "the external driver's publication lacks {PUB_PARAMS}:\n{}",
        published.stdout
    );
    assert_cnc_left_readable(&ping_dir, "xping");
    let _ = std::fs::remove_dir_all(ping_dir);
}

/// `load` (one stream, echoed by `rtt xpong`; and a mix, echoed by `load
/// xecho`) and `tput` on INVOKER drivers.
#[test]
fn load_and_tput_run_on_invoker_drivers() {
    let (load, rtt, tput) = (
        env!("CARGO_BIN_EXE_load"),
        env!("CARGO_BIN_EXE_rtt"),
        env!("CARGO_BIN_EXE_tput"),
    );

    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let (near, far) = (aeron_dir("load-near"), aeron_dir("load-far"));
    let echo = start(rtt, &["xpong", &ping, &pong, "-"], Some("invoker"), &far, &[]);
    let offered = start(
        load,
        &["xping", &ping, &pong, "1000", "2", "32", "-"],
        Some("invoker"),
        &near,
        &[],
    )
    .finish(90);
    let echoed = echo.terminate(30);
    offered.assert_ok("load xping", EMBEDDED);
    echoed.assert_ok("rtt xpong", EMBEDDED);
    // every message of the schedule (a warm-up second and two) sent on time
    // and echoed: a driver that ran only when back-pressured would fall behind
    let fields = offered.result("load,invoker-test,udp,32,1000,1,");
    let (sent, received): (u64, u64) = (fields[14].parse().unwrap(), fields[15].parse().unwrap());
    assert!(sent == 3000 && received == sent, "load xping: {fields:?}");
    assert_cnc_left_readable(&near, "load xping");

    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let (near, far) = (aeron_dir("mix-near"), aeron_dir("mix-far"));
    // xecho ends once the ping image goes: soon after xmix's driver stops
    let liveness = [("AERON_IMAGE_LIVENESS_TIMEOUT", "2s")];
    let echo = start(
        load,
        &["xecho", &ping, &pong, "-", "2001"],
        Some("invoker"),
        &far,
        &liveness,
    );
    let offered = start(
        load,
        &["xmix", &ping, &pong, "2", "-", "a:2001:1000:64"],
        Some("invoker"),
        &near,
        &[],
    )
    .finish(90);
    let echoed = echo.finish(60);
    offered.assert_ok("load xmix", EMBEDDED);
    echoed.assert_ok("load xecho", EMBEDDED);
    let fields = offered.result("mix,invoker-test,udp-a,64,1000,1,");
    let (sent, received): (u64, u64) = (fields[14].parse().unwrap(), fields[15].parse().unwrap());
    assert!(sent == 3000 && received == sent, "load xmix: {fields:?}");
    assert_cnc_left_readable(&near, "load xmix");
    assert_cnc_left_readable(&far, "load xecho");

    let channel = format!("127.0.0.1:{}", free_port());
    let (publisher, subscriber) = (aeron_dir("tput-pub"), aeron_dir("tput-sub"));
    // the subscriber on a CPU of its own: its embedded driver's conductor CPU,
    // where Aeron pins the thread as the driver starts
    let cpu = first_allowed_cpu().to_string();
    let sub = start(tput, &["xsub", &channel, "2", &cpu], Some("invoker"), &subscriber, &[]);
    let published = start(tput, &["xpub", &channel, "5", "-"], Some("invoker"), &publisher, &[]).finish(60);
    let measured = sub.finish(60);
    published.assert_ok("tput xpub", EMBEDDED);
    measured.assert_ok("tput xsub", EMBEDDED);
    assert!(
        measured
            .stderr
            .lines()
            .any(|l| l.starts_with(EMBEDDED) && l.contains(&format!(" cpu={cpu} "))),
        "tput xsub's driver not on its CPU {cpu}:\n{}",
        measured.stderr
    );
    let fields = measured.result("tput,invoker-test,udp,");
    assert!(
        fields[6].parse::<u64>().unwrap() > 0,
        "tput xsub received nothing: {fields:?}"
    );
    assert_cnc_left_readable(&publisher, "tput xpub");

    for dir in [near, far, publisher, subscriber] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// A round trip between two INVOKER clients with `SUB_CHANNEL_PARAMS` on
/// both ends' subscriptions: the ping's result line, split.
fn round_trip_with(tag: &str, sub_params: &str) -> Vec<String> {
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let (ping_dir, pong_dir) = (aeron_dir(&format!("{tag}-ping")), aeron_dir(&format!("{tag}-pong")));
    let rtt = env!("CARGO_BIN_EXE_rtt");
    let params = [("SUB_CHANNEL_PARAMS", sub_params)];
    let ponger = start(rtt, &["xpong", &ping, &pong, "-"], Some("invoker"), &pong_dir, &params);
    let pinged = start(
        rtt,
        &["xping", &ping, &pong, "2000", "200", "-"],
        Some("invoker"),
        &ping_dir,
        &params,
    )
    .finish(90);
    let ponged = ponger.terminate(30);
    pinged.assert_ok(&format!("xping {sub_params}"), EMBEDDED);
    ponged.assert_ok(&format!("xpong {sub_params}"), EMBEDDED);
    for dir in [ping_dir, pong_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
    pinged.result("rtt,invoker-test,udp,")
}

/// Aeron writing a receive timestamp into every data frame a subscription
/// receives leaves the clients' measurements intact where the lab's
/// media-receive-timestamp mode has it written: the frame header's reserved
/// value (`SUB_PARAMS=media-rcv-ts-offset=reserved`, md-dpdk/benches/cloud/
/// modes.sh), which no client reads (they read payload bytes 0 to 15: rtt's
/// and load's send stamp and sequence, tput's sequence). A media stamp
/// needs the transport's clock (`SO_TIMESTAMPNS` on Linux, the DPDK
/// transport's per burst); `channel-rcv-ts-offset` is Aeron's own stamp of
/// the same frames, written at the same place by the same code
/// (`aeron_timestamps_set_timestamp`) on any OS, so it stands in off Linux,
/// beside the media stamp itself on Linux. Both ends of a round trip, and
/// an offered load's ping, then measure every message, well under a second;
/// and the control: at offset 0 the same write lands on the send stamp, which
/// the round trip then reads as `CLOCK_REALTIME`, not its clock, so this
/// test would see an offset that overwrote what a client reads.
#[test]
fn a_receive_timestamp_in_the_reserved_value_leaves_the_measurements_intact() {
    let mut stamps = vec!["channel-rcv-ts-offset=reserved"];
    if cfg!(target_os = "linux") {
        stamps.push("media-rcv-ts-offset=reserved");
    }
    for params in stamps {
        let fields = round_trip_with("stamp", params);
        let (max, samples, lost): (u64, u64, u64) = (
            fields[8].parse().unwrap(),
            fields[10].parse().unwrap(),
            fields[11].parse().unwrap(),
        );
        assert!(
            samples == 2000 && lost == 0 && max < 1_000_000_000,
            "rtt with {params}: every round trip measured, sub-second: {fields:?}"
        );

        let (ping, pong) = (
            format!("127.0.0.1:{}", free_port()),
            format!("127.0.0.1:{}", free_port()),
        );
        let (near, far) = (aeron_dir("stamp-load-near"), aeron_dir("stamp-load-far"));
        let sub = [("SUB_CHANNEL_PARAMS", params)];
        let echo = start(
            env!("CARGO_BIN_EXE_rtt"),
            &["xpong", &ping, &pong, "-"],
            Some("invoker"),
            &far,
            &sub,
        );
        let offered = start(
            env!("CARGO_BIN_EXE_load"),
            &["xping", &ping, &pong, "1000", "1", "32", "-"],
            Some("invoker"),
            &near,
            &sub,
        )
        .finish(90);
        let echoed = echo.terminate(30);
        offered.assert_ok(&format!("load xping {params}"), EMBEDDED);
        echoed.assert_ok(&format!("rtt xpong {params}"), EMBEDDED);
        let fields = offered.result("load,invoker-test,udp,32,1000,1,");
        let (max, samples, sent, received): (u64, u64, u64, u64) = (
            fields[11].parse().unwrap(),
            fields[13].parse().unwrap(),
            fields[14].parse().unwrap(),
            fields[15].parse().unwrap(),
        );
        assert!(
            sent == 2000 && received == sent && samples == 1000 && max < 1_000_000_000,
            "load with {params}: every message measured, sub-second: {fields:?}"
        );
        for dir in [near, far] {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
    let fields = round_trip_with("stamp-0", "channel-rcv-ts-offset=0");
    let max: u64 = fields[8].parse().unwrap();
    assert!(
        max >= 1_000_000_000,
        "the control: a stamp at offset 0 overwrites the send stamp, yet the round trips read sub-second: {fields:?}"
    );
}

/// `HARNESS_DRIVER` takes `invoker` or `external` (or nothing); anything else
/// fails the client before it measures, naming the variable.
#[test]
fn an_unknown_harness_driver_is_refused() {
    let dir = aeron_dir("unknown");
    let refused = start(
        env!("CARGO_BIN_EXE_rtt"),
        &["xping", "127.0.0.1:1", "127.0.0.1:2", "10", "1", "-"],
        Some("shared"),
        &dir,
        &[],
    )
    .finish(30);
    assert!(!refused.status.success(), "HARNESS_DRIVER=shared accepted");
    assert!(refused.stderr.contains("HARNESS_DRIVER"), "{}", refused.stderr);
    assert!(refused.stdout.is_empty(), "{}", refused.stdout);
}

/// Every spike row `rtt xping` writes (`LOAD_SPIKES`) carries the ping's index
/// in the measured run, so the lab can place a spike in the run (a term
/// rotation, the n-th ping after warm-up): at `LOAD_SPIKE_NS=1` every
/// measured round trip is a spike, so the rows are pings 0, 1, 2, ... in
/// order, one each, as many as `# spikes_total=` counts.
#[test]
fn rtt_spike_rows_carry_their_ping_index() {
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let (ping_dir, pong_dir) = (aeron_dir("index-ping"), aeron_dir("index-pong"));
    let spikes = format!("{ping_dir}-spikes.csv");
    let _ = std::fs::remove_file(&spikes);
    let rtt = env!("CARGO_BIN_EXE_rtt");
    let ponger = start(rtt, &["xpong", &ping, &pong, "-"], Some("invoker"), &pong_dir, &[]);
    let pinged = start(
        rtt,
        &["xping", &ping, &pong, "2000", "200", "-"],
        Some("invoker"),
        &ping_dir,
        &[("LOAD_SPIKES", spikes.as_str()), ("LOAD_SPIKE_NS", "1")],
    )
    .finish(90);
    let ponged = ponger.terminate(30);
    pinged.assert_ok("xping", EMBEDDED);
    ponged.assert_ok("xpong", EMBEDDED);
    let samples: usize = pinged.result("rtt,invoker-test,udp,")[10].parse().expect("samples");
    let text = std::fs::read_to_string(&spikes).unwrap_or_else(|e| panic!("{spikes}: {e}"));
    let mut lines = text.lines();
    assert!(
        lines.next().is_some_and(|l| l.starts_with("# start_realtime_ns=")),
        "{text}"
    );
    assert_eq!(
        lines.next(),
        Some(format!("# spikes_total={samples}").as_str()),
        "{text}"
    );
    assert_eq!(lines.next(), Some("stream,due_ns,latency_ns,ping"), "{text}");
    let rows: Vec<Vec<&str>> = lines.map(|l| l.split(',').collect()).collect();
    assert_eq!(rows.len(), samples, "one row a measured round trip");
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), 4, "row {i}: {row:?}");
        assert_eq!(row[3], i.to_string(), "row {i}: {row:?}");
    }
    let _ = std::fs::remove_file(&spikes);
    for dir in [ping_dir, pong_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}
