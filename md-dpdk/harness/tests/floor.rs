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
    let (ping, pong) = (
        format!("127.0.0.1:{}", free_port()),
        format!("127.0.0.1:{}", free_port()),
    );
    let ponger = start(bin, &["xpong", &ping, &pong, "-"], dir, false, extra);
    let pinged = start(bin, &["xping", &ping, &pong, messages, "200", "-"], dir, true, extra).finish(90);
    let ponged = ponger.terminate(30);
    pinged.assert_ok("xping");
    ponged.assert_ok("xpong");
    (pinged.result(), ponged.stderr)
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
