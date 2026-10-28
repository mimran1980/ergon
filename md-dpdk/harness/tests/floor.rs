//! The raw floor client, `floor`, on its kernel wire (UDP sockets, no Aeron)
//! over 127.0.0.1: it round trips, and writes exactly what `rtt` writes (the
//! result line, the histogram, the spikes and both loop-stall files), so the
//! lab reads a floor mode's rep as it reads an Aeron one's. `rtt` runs beside
//! it here, its driver embedded (`HARNESS_DRIVER=invoker`), for the
//! comparison. The DPDK wire is the vdev runner's (`raw_floor`).

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const FLOOR: &str = env!("CARGO_BIN_EXE_floor");
const RTT: &str = env!("CARGO_BIN_EXE_rtt");

/// A free UDP port on 127.0.0.1 (released for the client to bind).
fn free_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .expect("a free UDP port")
}

/// A fresh directory for one run's files.
fn scratch(tag: &str) -> String {
    let dir = format!(
        "{}/harness-floor-{tag}-{}",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

struct Running {
    child: Child,
    stdout: std::thread::JoinHandle<String>,
    stderr: std::thread::JoinHandle<String>,
}

struct Finished {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

/// `bin` with `args`, writing its files into `dir` (`files`: whether the
/// histogram and the spikes are asked for), every round trip a spike and
/// every pass a stall, so each file has rows.
fn start(bin: &str, args: &[&str], dir: &str, files: bool, extra: &[(&str, &str)]) -> Running {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .env("LABEL", "floor-test")
        .env("RTT_SECONDS", "10")
        .env("LOOP_STALLS_DIR", dir)
        .env("LOOP_STALL_NS", "1")
        .env("LOAD_SPIKE_NS", "1");
    for var in [
        "HARNESS_DRIVER",
        "RUSTERON_MEDIA_DRIVER_TRANSPORT",
        "RTT_LOST_TIMEOUT_US",
        "PUB_CHANNEL_PARAMS",
        "SUB_CHANNEL_PARAMS",
        "LOAD_SPIKES",
        "RTT_HISTOGRAM",
    ] {
        cmd.env_remove(var);
    }
    if files {
        cmd.env("LOAD_SPIKES", format!("{dir}/spikes.csv"))
            .env("RTT_HISTOGRAM", format!("{dir}/histogram.csv"));
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
    fn finish(mut self, secs: u64) -> Finished {
        let deadline = Instant::now() + Duration::from_secs(secs);
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                panic!(
                    "did not exit within {secs} s; stderr:\n{}",
                    self.stderr.join().unwrap_or_default()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        Finished {
            status,
            stdout: self.stdout.join().unwrap_or_default(),
            stderr: self.stderr.join().unwrap_or_default(),
        }
    }

    /// SIGTERM, which a pong takes as the end of its run.
    fn terminate(self, secs: u64) -> Finished {
        let pid = self.child.id().to_string();
        assert!(Command::new("kill").args(["-TERM", &pid]).status().unwrap().success());
        self.finish(secs)
    }
}

impl Finished {
    fn assert_ok(&self, what: &str) {
        assert!(
            self.status.success(),
            "{what}: {:?}\nstdout:\n{}\nstderr:\n{}",
            self.status,
            self.stdout,
            self.stderr
        );
    }

    /// The one result line, split on commas: nothing else on stdout.
    fn result(&self) -> Vec<String> {
        let lines: Vec<&str> = self.stdout.lines().collect();
        assert!(
            lines.len() == 1 && lines[0].starts_with("rtt,floor-test,udp,"),
            "expected one rtt line on stdout, got:\n{}",
            self.stdout
        );
        lines[0].split(',').map(str::to_string).collect()
    }
}

/// A file's lines, its `# ` comments' keys (`# key=value`) and its column
/// header: what the lab's readers key on.
fn shape(path: &str) -> (Vec<String>, String, Vec<String>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let comments = text
        .lines()
        .filter_map(|l| l.strip_prefix("# "))
        .map(|l| l.split('=').next().unwrap_or("").to_string())
        .collect();
    let mut rows = text.lines().filter(|l| !l.starts_with('#'));
    let header = rows.next().unwrap_or_else(|| panic!("{path}: no header")).to_string();
    (comments, header, rows.map(str::to_string).collect())
}

/// One round trip run of `bin` over 127.0.0.1 into `dir`: the ping's
/// result line (and the pong's stderr).
fn run(bin: &str, dir: &str, messages: &str, extra: &[(&str, &str)]) -> (Vec<String>, String) {
    let (fields, pong_stderr, _) = run_timed(bin, dir, messages, "200", extra);
    (fields, pong_stderr)
}

/// [`run`] with `warmup` pings first: also how long the ping ran.
fn run_timed(
    bin: &str,
    dir: &str,
    messages: &str,
    warmup: &str,
    extra: &[(&str, &str)],
) -> (Vec<String>, String, Duration) {
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let ponger = start(bin, &["xpong", &ping, &pong, "-"], dir, false, extra);
    let began = Instant::now();
    let pinged = start(bin, &["xping", &ping, &pong, messages, warmup, "-"], dir, true, extra).finish(90);
    let took = began.elapsed();
    let ponged = ponger.terminate(30);
    pinged.assert_ok("xping");
    ponged.assert_ok("xpong");
    (pinged.result(), ponged.stderr, took)
}

/// The numbers of a result line: p50, p90, p99, p99.9, p99.99, max, mean,
/// samples, lost.
fn numbers(fields: &[String]) -> Vec<u64> {
    fields[3..].iter().map(|f| f.parse().expect("a number")).collect()
}

/// A loop-stall file's rows (`end_ns,gap_ns`).
fn stall_rows(path: &str) -> Vec<String> {
    shape(path).2
}

#[test]
fn floor_round_trips_over_kernel_udp_and_writes_what_rtt_writes() {
    let floor_dir = scratch("floor");
    // loopback loses nothing: a long timeout keeps a loaded host's stall
    // from passing for a loss
    let (fields, pong_stderr) = run(FLOOR, &floor_dir, "2000", &[("RTT_LOST_TIMEOUT_US", "10000000")]);
    // rtt,<label>,udp,p50,p90,p99,p99.9,p99.99,max,mean,samples,lost
    assert_eq!(fields.len(), 12, "{fields:?}");
    let numbers: Vec<u64> = fields[3..].iter().map(|f| f.parse().expect("a number")).collect();
    let (p50, max, samples, lost) = (numbers[0], numbers[5], numbers[7], numbers[8]);
    assert!(p50 > 0 && p50 <= max, "{fields:?}");
    assert!(
        numbers[..6].windows(2).all(|w| w[0] <= w[1]),
        "quantiles out of order: {fields:?}"
    );
    assert_eq!((samples, lost), (2000, 0), "every ping measured and echoed: {fields:?}");
    assert!(
        pong_stderr.contains("floor: pong ended: SIGTERM"),
        "the pong says why it ended:\n{pong_stderr}"
    );

    let (_, header, rows) = shape(&format!("{floor_dir}/histogram.csv"));
    assert_eq!(header, "value_ns,count");
    let counted: u64 = rows
        .iter()
        .map(|r| r.split(',').nth(1).unwrap().parse::<u64>().unwrap())
        .sum();
    assert_eq!(counted, 2000, "the histogram holds every round trip");

    let (comments, header, rows) = shape(&format!("{floor_dir}/spikes.csv"));
    assert_eq!(comments, ["start_realtime_ns", "spikes_total"]);
    assert_eq!(header, "stream,due_ns,latency_ns,ping");
    assert_eq!(rows.len(), 2000, "LOAD_SPIKE_NS=1: every round trip a spike");
    for (i, row) in rows.iter().enumerate() {
        let f: Vec<u64> = row.split(',').map(|v| v.parse().unwrap()).collect();
        assert_eq!((f[0], f[3]), (1002, i as u64), "row {i}: stream 1002, ping {i}: {row}");
        assert!(f[2] > 0, "row {i}: a latency: {row}");
    }
    let text = std::fs::read_to_string(format!("{floor_dir}/spikes.csv")).unwrap();
    assert!(
        text.contains("# spikes_total=2000\n"),
        "{}",
        &text[..text.len().min(200)]
    );
    for name in ["stalls-ping.csv", "stalls-pong.csv"] {
        let (comments, header, rows) = shape(&format!("{floor_dir}/{name}"));
        assert_eq!(comments, ["start_realtime_ns"], "{name}");
        assert_eq!(header, "end_ns,gap_ns", "{name}");
        assert!(!rows.is_empty(), "{name}: LOOP_STALL_NS=1, so every pass a stall");
    }

    // rtt's files, its driver embedded: the same shapes, row for row
    let rtt_dir = scratch("rtt");
    let rtt_extra = [
        ("HARNESS_DRIVER", "invoker"),
        ("AERON_DIR_DELETE_ON_START", "true"),
        ("AERON_TERM_BUFFER_LENGTH", "65536"),
    ];
    let ping_aeron = format!("{rtt_dir}/aeron-ping");
    let pong_aeron = format!("{rtt_dir}/aeron-pong");
    let (rtt_ping, rtt_pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let ponger = start(
        RTT,
        &["xpong", &rtt_ping, &rtt_pong, "-"],
        &rtt_dir,
        false,
        &[rtt_extra.as_slice(), &[("AERON_DIR", pong_aeron.as_str())]].concat(),
    );
    let pinged = start(
        RTT,
        &["xping", &rtt_ping, &rtt_pong, "500", "100", "-"],
        &rtt_dir,
        true,
        &[rtt_extra.as_slice(), &[("AERON_DIR", ping_aeron.as_str())]].concat(),
    )
    .finish(90);
    ponger.terminate(30).assert_ok("rtt xpong");
    pinged.assert_ok("rtt xping");
    assert_eq!(
        pinged.result().len(),
        fields.len(),
        "rtt's line and floor's have the same fields"
    );
    for name in ["histogram.csv", "spikes.csv", "stalls-ping.csv", "stalls-pong.csv"] {
        let (rtt_comments, rtt_header, _) = shape(&format!("{rtt_dir}/{name}"));
        let (floor_comments, floor_header, _) = shape(&format!("{floor_dir}/{name}"));
        assert_eq!(
            (rtt_comments, rtt_header),
            (floor_comments, floor_header),
            "{name}: floor writes it as rtt does"
        );
    }
    for dir in [floor_dir, rtt_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// No pong at all: the ping's connect wait (`FLOOR_CONNECT_SECS`) runs out,
/// and it fails saying so, with no result line.
#[test]
fn floor_without_a_pong_fails_saying_so() {
    let dir = scratch("nopong");
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let pinged = start(
        FLOOR,
        &["xping", &ping, &pong, "100", "10", "-"],
        &dir,
        true,
        &[("FLOOR_CONNECT_SECS", "1")],
    )
    .finish(30);
    assert!(!pinged.status.success(), "a ping with no pong must fail");
    assert!(
        pinged.stderr.contains("no echo from the pong"),
        "it says why:\n{}",
        pinged.stderr
    );
    assert!(
        pinged.stdout.is_empty(),
        "no result line without a run:\n{}",
        pinged.stdout
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// `RTT_GAP_US`: one ping every that many microseconds on a fixed schedule, the
/// warm-up's too, the client spinning (its wire polled, its loop's stall
/// tracker passed) in between: the run takes the pings times the gap, its
/// round trips are each ping's alone (not the wait before it), no wait is a
/// loop stall, and the run still ends at `RTT_SECONDS` with fewer pings.
#[test]
fn floor_paces_its_pings_at_rtt_gap_us() {
    let dir = scratch("gap");
    let paced = [
        ("RTT_GAP_US", "25000"),
        ("RTT_LOST_TIMEOUT_US", "10000000"),
        // a wait between pings unpassed would be a stall of 25 ms each
        ("LOOP_STALL_NS", "15000000"),
    ];
    let (fields, _, took) = run_timed(FLOOR, &dir, "40", "4", &paced);
    let n = numbers(&fields);
    assert_eq!((n[7], n[8]), (40, 0), "every ping measured and echoed: {fields:?}");
    assert!(
        took >= Duration::from_millis(43 * 25),
        "43 gaps of 25 ms (the warm-up's 4 too) took {took:?}"
    );
    assert!(
        n[0] < 5_000_000,
        "a round trip is the ping's alone, not the gap before it: p50 {} ns",
        n[0]
    );
    let stalls = stall_rows(&format!("{dir}/stalls-ping.csv"));
    assert!(
        stalls.len() < 5,
        "the waits between pings counted as loop stalls: {stalls:?}"
    );

    // a budget of 1 s at one ping every 100 ms: about 10 measured, not 1000
    let budget = [
        ("RTT_GAP_US", "100000"),
        ("RTT_SECONDS", "1"),
        ("RTT_LOST_TIMEOUT_US", "10000000"),
    ];
    let (fields, _, took) = run_timed(FLOOR, &dir, "1000", "1", &budget);
    let n = numbers(&fields);
    assert!(
        (5..=12).contains(&n[7]),
        "RTT_SECONDS ends a paced run: {} pings in {took:?}",
        n[7]
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// `rtt` paces its pings at `RTT_GAP_US` as `floor` does (its embedded
/// driver run in between, as every pass).
#[test]
fn rtt_paces_its_pings_at_rtt_gap_us() {
    let dir = scratch("rtt-gap");
    let ping_aeron = format!("{dir}/aeron-ping");
    let pong_aeron = format!("{dir}/aeron-pong");
    let common = [
        ("HARNESS_DRIVER", "invoker"),
        ("AERON_DIR_DELETE_ON_START", "true"),
        ("AERON_TERM_BUFFER_LENGTH", "65536"),
        ("RTT_GAP_US", "25000"),
        ("LOOP_STALL_NS", "15000000"),
    ];
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let ponger = start(
        RTT,
        &["xpong", &ping, &pong, "-"],
        &dir,
        false,
        &[common.as_slice(), &[("AERON_DIR", pong_aeron.as_str())]].concat(),
    );
    let began = Instant::now();
    let pinged = start(
        RTT,
        &["xping", &ping, &pong, "40", "4", "-"],
        &dir,
        true,
        &[common.as_slice(), &[("AERON_DIR", ping_aeron.as_str())]].concat(),
    )
    .finish(90);
    let took = began.elapsed();
    ponger.terminate(30).assert_ok("rtt xpong");
    pinged.assert_ok("rtt xping");
    let n = numbers(&pinged.result());
    assert_eq!((n[7], n[8]), (40, 0), "every ping measured: {:?}", pinged.stdout);
    assert!(took >= Duration::from_millis(43 * 25), "43 gaps of 25 ms took {took:?}");
    assert!(n[0] < 5_000_000, "p50 {} ns: the gap counted in the round trip", n[0]);
    let stalls = stall_rows(&format!("{dir}/stalls-ping.csv"));
    assert!(
        stalls.len() < 5,
        "the waits between pings counted as loop stalls: {stalls:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// `floor xhairpin`: ping and pong in one process, the pong on a thread of
/// its own (its own CPU), each on its own endpoint (on a host, one data
/// NIC each): one result line as `xping`'s, both loop-stall files, the pong
/// ending with the ping's run. Over 127.0.0.1 here; one endpoint for both
/// is refused (nothing would cross between two NICs).
#[test]
fn floor_hairpin_runs_ping_and_pong_in_one_process() {
    let dir = scratch("hairpin");
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let done = start(
        FLOOR,
        &["xhairpin", &ping, &pong, "2000", "200", "-", "-"],
        &dir,
        true,
        &[("RTT_LOST_TIMEOUT_US", "10000000")],
    )
    .finish(90);
    done.assert_ok("xhairpin");
    let n = numbers(&done.result());
    assert_eq!(
        (n[7], n[8]),
        (2000, 0),
        "every ping measured and echoed: {}",
        done.stdout
    );
    assert!(n[0] > 0 && n[0] <= n[5], "{}", done.stdout);
    assert!(
        done.stderr.contains("floor: hairpin: ping and pong in one process"),
        "it says what it runs:\n{}",
        done.stderr
    );
    assert!(
        done.stderr.contains("floor: pong ended: the ping's run done"),
        "the pong says why it ended:\n{}",
        done.stderr
    );
    for name in ["stalls-ping.csv", "stalls-pong.csv", "histogram.csv", "spikes.csv"] {
        assert!(
            std::path::Path::new(&format!("{dir}/{name}")).exists(),
            "{name} not written"
        );
    }
    let same = start(
        FLOOR,
        &["xhairpin", &ping, &ping, "10", "1", "-", "-"],
        &dir,
        false,
        &[],
    )
    .finish(30);
    assert!(!same.status.success(), "one endpoint for both ends must be refused");
    assert!(same.stderr.contains("two endpoints"), "it says why:\n{}", same.stderr);
    let _ = std::fs::remove_dir_all(dir);
}

/// The transport suite's raw floor (`floor --role ping|pong`, its arm
/// `dpdk-raw` on DPDK; the kernel's wire here, over 127.0.0.1): one stream
/// at a rate, the pong ready before the ping probes it, every ping's echo
/// back, the row the analysis reads (34 columns with the rep, 138 wire
/// bytes, one busy thread), its manifest the suite's (`raw-udp`, nothing
/// allocated while measuring), the pong's echoes all the pings.
#[test]
fn floor_runs_the_transport_suites_point() {
    let dir = scratch("transport");
    let (ping_port, pong_port) = (free_port(), free_port());
    let (endpoint, reply) = (format!("127.0.0.1:{pong_port}"), format!("127.0.0.1:{ping_port}"));
    let common = [
        "--endpoint",
        &endpoint,
        "--reply",
        &reply,
        "--label",
        "f:dpdk-raw",
        "--warmup-s",
        "1",
        "--duration-s",
        "1",
    ];
    let pong_out = format!("{dir}/pong");
    let pong = start(
        FLOOR,
        &[&["--role", "pong", "--out", &pong_out][..], &common].concat(),
        &dir,
        false,
        &[],
    );
    let ping_out = format!("{dir}/ping");
    let ping = start(
        FLOOR,
        &[&["--role", "ping", "--rate", "2000", "--out", &ping_out][..], &common].concat(),
        &dir,
        false,
        &[],
    )
    .finish(60);
    ping.assert_ok("floor --role ping");
    let pong = pong.terminate(10);
    pong.assert_ok("floor --role pong");
    assert_eq!(pong.stdout, "ready\n", "one ready line, no row");
    let row = ping.stdout.trim();
    let cols: Vec<&str> = row.split(',').collect();
    assert_eq!(cols.len(), 33, "{row}");
    assert_eq!(
        (cols[0], cols[1], cols[2], cols[3], cols[4]),
        ("load", "f:dpdk-raw", "udp", "64", "2000"),
        "{row}"
    );
    // sent, received, lost, wire bytes, busy threads
    assert_eq!((cols[14], cols[15]), ("4000", "4000"), "{row}");
    assert_eq!((cols[23], cols[27], cols[28]), ("0", "138", "1"), "{row}");
    let m = std::fs::read_to_string(format!("{ping_out}/manifest.json")).unwrap();
    for want in [
        "\"transport\": \"raw-udp\"",
        "\"wire\": \"kernel\"",
        "\"hot_path_allocs\": \"0\"",
        "\"kind\": \"load\"",
    ] {
        assert!(m.contains(want), "{want}: {m}");
    }
    let p = std::fs::read_to_string(format!("{pong_out}/manifest.json")).unwrap();
    assert!(
        p.contains("\"echoes_sent\": \"4000\"") && p.contains("\"hot_path_allocs\": \"0\""),
        "{p} {}",
        pong.stderr
    );
    for f in ["histogram", "spikes", "stalls"] {
        assert!(
            std::path::Path::new(&format!("{ping_out}/{f}-f:dpdk-raw.csv")).exists(),
            "{f}"
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}
