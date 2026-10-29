//! `udp_lat` and `tcp_lat` end to end over loopback, both arms (mio and
//! ergon-io), 1 and 128 streams: a `pong` process and a `ping` process,
//! then both ends in one process (`local --round-trip`, and one way). Each
//! point's row must read back through the analysis's own readers
//! (md-dpdk/benches/cloud: `transport_points.read_points`,
//! `analyse.read_results`, `spikes.transport_max`), lose nothing at a low
//! rate, carry the manifest the fairness checks compare, and show that the
//! busy threads allocated nothing while measuring. Settings an arm cannot
//! take are refused. `tcp_lat --source` binds the connecting end's address
//! (127.0.0.2 on `lo` here; a data NIC's address in the lab).
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

use ergon_latency_core::manifest::{CORE_BUILD_HASH, sha256_hex};
use ergon_latency_core::message::template;

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

const UDP: &str = env!("CARGO_BIN_EXE_udp_lat");
const TCP: &str = env!("CARGO_BIN_EXE_tcp_lat");

/// The CPUs this process may use.
#[allow(unsafe_code)]
fn cpus() -> Vec<usize> {
    // SAFETY: a zeroed cpu_set_t is valid; the kernel fills at most its size.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set) != 0 {
            return Vec::new();
        }
        (0..libc::CPU_SETSIZE as usize)
            .filter(|&c| libc::CPU_ISSET(c, &set))
            .collect()
    }
}

/// Two distinct CPUs (the last two this process may use), or none
/// (unpinned) on a one-CPU box.
fn two_cpus() -> Option<(usize, usize)> {
    let c = cpus();
    (c.len() >= 2).then(|| (c[c.len() - 2], c[c.len() - 1]))
}

fn scratch(name: &str) -> R<PathBuf> {
    let d = std::env::temp_dir().join(format!("transport-{name}-{}", std::process::id()));
    if d.exists() {
        std::fs::remove_dir_all(&d)?;
    }
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

fn s(v: impl ToString) -> String {
    v.to_string()
}

/// Waits for the child's `ready` line.
fn wait_ready(child: &mut Child) -> R {
    let out = child.stdout.take().ok_or("no stdout")?;
    for l in BufReader::new(out).lines() {
        if l? == "ready" {
            return Ok(());
        }
    }
    Err("the pong never said ready".into())
}

#[allow(unsafe_code)]
fn sigterm(child: &Child) -> R {
    let pid = libc::pid_t::try_from(child.id())?;
    // SAFETY: a signal to our own child.
    if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

fn finished(name: &str, o: &Output) -> R {
    if o.status.success() {
        return Ok(());
    }
    Err(format!(
        "{name} failed ({}):\nstdout: {}\nstderr: {}",
        o.status,
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
    .into())
}

/// One arm at one stream count.
struct Run {
    bin: &'static str,
    imp: &'static str,
    streams: u32,
    rate: u64,
    /// UDP: the ping end's base port (the echo end's is + 1000); TCP: the
    /// port the echo end listens on. Below the kernel's ephemeral range
    /// (32768-60999): an earlier test's connecting socket, still closing,
    /// can hold a port in it and fail the bind (seen once, `EADDRINUSE`).
    port: u16,
    /// The arm's own flags, given to every end (as the runner does).
    extra: &'static [&'static str],
}

impl Run {
    const fn udp(imp: &'static str, streams: u32, rate: u64, port: u16) -> Self {
        Self {
            bin: UDP,
            imp,
            streams,
            rate,
            port,
            extra: &[],
        }
    }

    const fn tcp(imp: &'static str, streams: u32, rate: u64, port: u16) -> Self {
        Self {
            bin: TCP,
            imp,
            streams,
            rate,
            port,
            extra: &[],
        }
    }

    fn transport(&self) -> &'static str {
        if self.bin == UDP { "udp" } else { "tcp" }
    }

    fn label(&self) -> String {
        let arm = format!("2a:{}-{}", self.imp, self.transport());
        if self.streams == 1 {
            arm
        } else {
            format!("{arm}@STREAMS={}", self.streams)
        }
    }

    fn common(&self, role: &str, cpus: Option<String>, out: &Path) -> Vec<String> {
        let mut v = vec![
            s("--impl"),
            s(self.imp),
            s("--role"),
            s(role),
            s("--streams"),
            s(self.streams),
            s("--rate"),
            s(self.rate),
            s("--warmup-s"),
            s(1),
            s("--duration-s"),
            s(2),
            s("--label"),
            self.label(),
            s("--out"),
            out.display().to_string(),
        ];
        if let Some(c) = cpus {
            v.extend([s("--cpus"), c]);
        }
        v
    }

    /// The arm's addresses for `role` (UDP: the echo end binds base + 1000;
    /// TCP: the echo end listens, the ping connects).
    fn addrs(&self, role: &str) -> Vec<String> {
        let (a, b) = (
            format!("127.0.0.1:{}", self.port),
            format!("127.0.0.1:{}", self.port + 1000),
        );
        if self.bin == UDP {
            let (l, p) = if role == "pong" { (b, a) } else { (a, b) };
            vec![s("--local"), l, s("--peer"), p]
        } else {
            let k = if role == "ping" {
                "--connect"
            } else {
                "--listen"
            };
            vec![s(k), a]
        }
    }

    /// A pong process, then a ping process; the ping's out dir.
    fn ping_pong(&self) -> R<PathBuf> {
        let cpus = two_cpus();
        let tag = format!("{}-{}-{}", self.transport(), self.imp, self.streams);
        let (po, pi) = (
            scratch(&format!("{tag}-pong"))?,
            scratch(&format!("{tag}-ping"))?,
        );
        let mut pong = Command::new(self.bin)
            .args(self.common("pong", cpus.map(|c| s(c.1)), &po))
            .args(self.addrs("pong"))
            .args(self.extra)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Err(e) = wait_ready(&mut pong) {
            let _ = pong.kill();
            let o = pong.wait_with_output()?;
            return Err(format!("{e}: {}", String::from_utf8_lossy(&o.stderr)).into());
        }
        let ping = Command::new(self.bin)
            .args(self.common("ping", cpus.map(|c| s(c.0)), &pi))
            .args(self.addrs("ping"))
            .args(self.extra)
            .output()?;
        // a TCP pong stops once every peer closed; a UDP one on SIGTERM
        if self.bin == UDP || !ping.status.success() {
            sigterm(&pong)?;
        }
        let pong = pong.wait_with_output()?;
        finished("ping", &ping)?;
        finished("pong", &pong)?;
        assert!(pong.stdout.is_empty(), "a pong prints no row");
        assert!(!po.join("results.csv").exists());
        let pm = std::fs::read_to_string(po.join("manifest.json"))?;
        for want in [
            "\"role\": \"pong\"",
            "\"hot_path_allocs\": \"0\"",
            "\"thread_order\": \"echo\"",
        ] {
            assert!(pm.contains(want), "pong manifest: no {want}: {pm}");
        }
        if self.bin == TCP {
            // loopback loses nothing: every retransmit is one the peer had
            for want in [
                "\"tcp.lost_end\": \"0\"",
                "\"tcp.retrans_not_dsack\": \"0\"",
                "\"netstat.window.TcpExtTCPLossProbes\"",
                "\"sched.run_delay_ns\"",
            ] {
                assert!(pm.contains(want), "pong manifest: no {want}: {pm}");
            }
        }
        let echoes = format!("\"echoes\": \"{}\"", self.rate * 3);
        assert!(pm.contains(&echoes), "pong manifest: no {echoes}: {pm}");
        assert_eq!(
            String::from_utf8(ping.stdout)?,
            std::fs::read_to_string(pi.join("results.csv"))?,
            "the printed row is the written one"
        );
        Ok(pi)
    }

    fn local(&self, round_trip: bool) -> R<PathBuf> {
        let cpus = two_cpus().map(|(a, b)| format!("{a},{b}"));
        let out = scratch(&format!(
            "{}-{}-{}-local-{round_trip}",
            self.transport(),
            self.imp,
            self.streams
        ))?;
        let o = Command::new(self.bin)
            .args(self.common("local", cpus, &out))
            .args(self.addrs("local"))
            .args(round_trip.then_some("--round-trip"))
            .output()?;
        finished("local", &o)?;
        Ok(out)
    }
}

/// The analysis's check of one point, as the runner lays it out: the row in
/// `<run>/results.csv` (rep prefixed, the channel column `<transport>-r<rate>`),
/// the manifest and spikes file under `<run>/<set>/` by the point's tag.
const CHECK: &str = r#"
import json, os, shutil, sys
cloud, out, run, kind, streams, sha, core, arm, transport, busy = sys.argv[1:11]
streams, busy = int(streams), int(busy)
sys.path.insert(0, cloud)
import analyse, spikes, transport_points as tp
line = open(os.path.join(out, "results.csv")).read().strip()
cols = line.split(",")
assert len(cols) == tp.ROW_LEN - 1, (len(cols), line)
label, rate = cols[1], int(cols[4])
assert cols[2] == transport, cols
cols[2] = f"{cols[2]}-r{rate}"
os.makedirs(os.path.join(run, "2a"), exist_ok=True)
open(os.path.join(run, "results.csv"), "w").write("1," + ",".join(cols) + "\n")
tag = tp.tag_of(label, 64, 1, rate, kind)
shutil.copy(os.path.join(out, "manifest.json"), os.path.join(run, "2a", f"manifest-{tag}.json"))
shutil.copy(os.path.join(out, f"spikes-{label}.csv"), os.path.join(run, "2a", f"spikes-{tag}.csv"))
[p] = tp.read_points(run)
r = p.row
assert (p.set, p.arm, p.kind, p.streams, p.rate, p.size) == ("2a", arm, kind, streams, rate, 64), p
assert r["scheduled"] == r["sent"] == r["received"], r
assert r["lost"] == 0 and r["reordered"] == 0 and r["dup"] == 0, r
assert r["samples"] == rate * 2 and r["censored"] == 0, r
if transport == "udp":
    assert r["retrans"] is None and r["wire_bytes"] == 106, r
else:
    assert r["retrans"] is not None and r["wire_bytes"] in (122, 134), r
assert r["busy_threads"] == busy, r
assert 0 < r["min"] <= r["p50"] <= r["p99"] <= r["p99.99"] <= r["max"], r
assert 0 < r["stream0_p50"] <= r["stream0_max"] <= r["max"], r
m = p.manifest
assert m["payload_sha256"] == sha and m["payload_size"] == 64 and m["wire_bytes"] == r["wire_bytes"], m
assert m["core_build_hash"] == core and m["arm"] == arm, m
s = m["settings"]
assert s["transport"] == transport and s["kind"] == kind and s["streams"] == str(streams), s
assert s["hot_path_allocs"] == "0", s
assert s["backend"] == "epoll-spin" and s["readback.so_busy_poll"] == "0", s
assert s["max_ns"] == str(r["max"]) and s["lost"] == "0", s
TCP_KEYS = ("tcp.retrans", "tcp.retrans_setup", "tcp.dsack_dups", "tcp.dsack_dups_setup",
            "tcp.retrans_not_dsack", "tcp.lost_end", "tcp.rtos", "tcp.rtos_setup",
            "netstat.setup.TcpRetransSegs", "netstat.setup.TcpExtTCPSynRetrans",
            "netstat.setup.TcpExtListenOverflows", "netstat.window.TcpRetransSegs",
            "netstat.window.TcpExtTCPLossProbes", "netstat.window.TcpExtTCPDSACKRecv",
            "netstat.window.TcpExtTCPTimeouts", "netstat.window.TcpExtTCPSpuriousRTOs",
            "netstat.window.TcpExtTCPLostRetransmit", "sched.run_delay_ns", "sched.involuntary_switches")
if transport == "tcp":
    assert s["retrans_window"] == "after set-up: warm-up, measured, drain", s
    ends = [""] + (["echo."] if kind == "load" and busy == 2 else [])
    for e in ends:
        for k in TCP_KEYS:
            assert e + k in s, (e + k, s)
        # loopback loses nothing: every retransmit is one the peer already had
        assert s[e + "tcp.lost_end"] == "0" and s[e + "tcp.retrans_not_dsack"] == "0", s
        assert int(s[e + "tcp.retrans"]) == int(s[e + "tcp.dsack_dups"]), s
    assert sum(int(s[e + "tcp.retrans"]) for e in ends) == r["retrans"], (r["retrans"], s)
else:
    assert not any(k.startswith("tcp.") for k in s), s
loc = spikes.transport_max(os.path.join(run, "2a"), tag, r["max"])
assert loc is not None and loc[2] == r["max"], (loc, r["max"])
if kind == "load":
    res = analyse.read_results(os.path.join(run, "results.csv"))
    [rep] = res.load[(label, 64, cols[2].removeprefix("udp-"))]
    assert rep[:6] == [float(r[c]) for c in ("p50", "p90", "p99", "p99.9", "p99.99", "max")], rep
print("ok")
"#;

fn cloud() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../md-dpdk/benches/cloud")
}

fn check(run: &Run, out: &Path, kind: &str, busy: u32) -> R {
    let o = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(CHECK)
        .arg(cloud())
        .arg(out)
        .arg(out.join("run"))
        .arg(kind)
        .arg(s(run.streams))
        .arg(sha256_hex(&template()))
        .arg(CORE_BUILD_HASH)
        .arg(format!("{}-{}", run.imp, run.transport()))
        .arg(run.transport())
        .arg(s(busy))
        .output()?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(
        o.status.success() && said == "ok\n",
        "{}: {said}",
        run.label()
    );
    for f in ["histogram", "histogram-send", "spikes", "stalls"] {
        let p = out.join(format!("{f}-{}.csv", run.label()));
        assert!(p.exists(), "{} missing", p.display());
    }
    Ok(())
}

fn every_role(run: &Run) -> R {
    let out = run.ping_pong()?;
    check(run, &out, "load", 1)?;
    std::fs::remove_dir_all(out)?;
    let out = run.local(true)?;
    check(run, &out, "load", 2)?;
    std::fs::remove_dir_all(out)?;
    let out = run.local(false)?;
    check(run, &out, "oneway", 2)?;
    std::fs::remove_dir_all(out)?;
    Ok(())
}

#[test]
fn udp_mio_one_stream() -> R {
    every_role(&Run::udp("mio", 1, 1_000, 35_000))
}

#[test]
fn udp_ergon_one_stream() -> R {
    every_role(&Run::udp("ergon", 1, 1_000, 23_000))
}

#[test]
fn udp_mio_128_streams() -> R {
    every_role(&Run::udp("mio", 128, 2_560, 25_000))
}

#[test]
fn udp_ergon_128_streams() -> R {
    every_role(&Run::udp("ergon", 128, 2_560, 27_000))
}

/// The transport runner sends a pong's stdout and stderr to one file and
/// waits for the line `ready`. Every write the pong makes there must be
/// whole lines: a warning written as its text, then its newline, let the
/// pinned thread's `ready` land between the two (`...the polling CPU is
/// 2.ready`), and the runner timed out on a pong that was echoing. Here
/// both go to one end of a Unix datagram socket pair, so each `write(2)`
/// arrives as one datagram.
#[test]
fn a_pongs_writes_are_whole_lines_with_stderr_on_stdout() -> R {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixDatagram;
    use std::time::{Duration, Instant};

    let run = Run::udp("ergon", 1, 1_000, 39_300);
    let out = scratch("pong-lines")?;
    let (ours, theirs) = UnixDatagram::pair()?;
    let mut pong = Command::new(run.bin)
        .args(run.common("pong", two_cpus().map(|c| s(c.1)), &out))
        .args(run.addrs("pong"))
        .stdout(Stdio::from(OwnedFd::from(theirs.try_clone()?)))
        .stderr(Stdio::from(OwnedFd::from(theirs)))
        .spawn()?;
    ours.set_read_timeout(Some(Duration::from_millis(100)))?;
    let (mut writes, mut buf) = (Vec::<String>::new(), vec![0u8; 1 << 16]);
    let until = Instant::now() + Duration::from_secs(30);
    let (mut stopped, mut exited) = (false, None);
    loop {
        match ours.recv(&mut buf) {
            Ok(n) => writes.push(String::from_utf8_lossy(&buf[..n]).into_owned()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if exited.is_some() {
                    break;
                }
                exited = pong.try_wait()?;
                // stopped once it said ready and ergon-io wrote its lines
                let ready = writes.iter().any(|w| w == "ready\n");
                let warned = writes.iter().any(|w| w.starts_with("ergon-io "));
                if !stopped && exited.is_none() && ready && warned {
                    sigterm(&pong)?;
                    stopped = true;
                }
                if Instant::now() > until {
                    let _ = pong.kill();
                    return Err(format!("the pong did not say ready and stop: {writes:?}").into());
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    let status = exited.ok_or("no exit status")?;
    assert!(status.success(), "the pong failed ({status}): {writes:?}");
    for w in &writes {
        assert!(
            w.ends_with('\n') && w.matches('\n').count() == 1,
            "a write that is not one whole line: {w:?} (all: {writes:?})"
        );
    }
    assert_eq!(
        writes.iter().filter(|w| *w == "ready\n").count(),
        1,
        "{writes:?}"
    );
    std::fs::remove_dir_all(out)?;
    Ok(())
}

#[test]
fn tcp_mio_one_stream() -> R {
    every_role(&Run::tcp("mio", 1, 1_000, 30_001))
}

#[test]
fn tcp_ergon_one_stream() -> R {
    every_role(&Run::tcp("ergon", 1, 1_000, 30_002))
}

#[test]
fn tcp_mio_128_streams() -> R {
    every_role(&Run::tcp("mio", 128, 2_560, 30_003))
}

#[test]
fn tcp_ergon_128_streams() -> R {
    every_role(&Run::tcp("ergon", 128, 2_560, 30_004))
}

/// `--source-port` pins the connecting end's ports, so arms run one after
/// another use the same 4-tuples (the same NIC queues and flow hash): each
/// run closes with a reset, leaving no `TIME_WAIT`, and the next one binds
/// and connects at once. The listening end takes the flag and ignores it.
#[test]
fn tcp_source_port_is_reused_by_the_next_arm_at_once() -> R {
    for imp in ["mio", "ergon", "mio", "ergon"] {
        let mut run = Run::tcp(imp, 2, 1_000, 30_020);
        run.extra = &["--source-port", "30120"];
        let out = run.ping_pong()?;
        check(&run, &out, "load", 1)?;
        let m = std::fs::read_to_string(out.join("manifest.json"))?;
        assert!(m.contains("\"source_port\": \"30120\""), "{imp}: {m}");
        std::fs::remove_dir_all(out)?;
    }
    let out = scratch("tcp-source-port-local")?;
    let run = Run::tcp("mio", 1, 1_000, 30_021);
    let o = Command::new(TCP)
        .args(run.common("local", None, &out))
        .args(run.addrs("local"))
        .args(["--source-port", "30121"])
        .output()?;
    assert!(!o.status.success(), "--role local took --source-port");
    Ok(())
}

/// TCP's settings reach the socket the same way in both arms: per-read
/// quick-ACK re-arms are counted, and the two timers are read back as set.
#[test]
fn tcp_settings_reach_both_arms_sockets() -> R {
    for imp in ["mio", "ergon"] {
        let run = Run::tcp(imp, 2, 1_000, 30_010);
        let out = scratch(&format!("tcp-settings-{imp}"))?;
        let o = Command::new(TCP)
            .args(run.common("local", two_cpus().map(|(a, b)| format!("{a},{b}")), &out))
            .args(run.addrs("local"))
            .args([
                "--round-trip",
                "--quickack-per-read",
                "--rto-min-us",
                "5000",
                "--delack-max-us",
                "4000",
                "--nodelay",
            ])
            .output()?;
        finished(imp, &o)?;
        check(&run, &out, "load", 2)?;
        let m = std::fs::read_to_string(out.join("manifest.json"))?;
        for want in [
            "\"readback.tcp_nodelay\": \"1\"",
            "\"readback.tcp_rto_min_us\": \"5000\"",
            "\"readback.tcp_delack_max_us\": \"4000\"",
            "\"echo.readback.tcp_rto_min_us\": \"5000\"",
            "\"quickack_per_read\": \"true\"",
        ] {
            assert!(m.contains(want), "{imp}: no {want} in {m}");
        }
        let quickacks: u64 = m
            .lines()
            .find_map(|l| l.trim().strip_prefix("\"stat.quickacks\": \""))
            .and_then(|v| v.trim_end_matches(['"', ',']).parse().ok())
            .ok_or("no stat.quickacks")?;
        assert!(quickacks > 0, "{imp}: no quick-ACK re-arm counted");
        std::fs::remove_dir_all(out)?;
    }
    Ok(())
}

/// `--busy-read-us` reaches both arms' sockets, TCP and UDP, at both ends:
/// mio by `setsockopt`, ergon-io by `Config::busy_poll_us`. Each manifest
/// records the setting and `SO_BUSY_POLL` as read back.
#[test]
fn busy_read_reaches_both_arms_sockets() -> R {
    for (k, run) in [
        Run::tcp("mio", 1, 1_000, 30_110),
        Run::tcp("ergon", 1, 1_000, 30_120),
        Run::udp("mio", 1, 1_000, 30_130),
        Run::udp("ergon", 1, 1_000, 30_140),
    ]
    .iter()
    .enumerate()
    {
        let out = scratch(&format!("busy-read-{k}"))?;
        let o = Command::new(run.bin)
            .args(run.common("local", two_cpus().map(|(a, b)| format!("{a},{b}")), &out))
            .args(run.addrs("local"))
            .args(["--round-trip", "--busy-read-us", "50"])
            .output()?;
        finished(run.imp, &o)?;
        let m = std::fs::read_to_string(out.join("manifest.json"))?;
        for want in [
            "\"busy_read_us\": \"50\"",
            "\"readback.so_busy_poll\": \"50\"",
            "\"echo.readback.so_busy_poll\": \"50\"",
        ] {
            assert!(m.contains(want), "{}: no {want} in {m}", run.label());
        }
        std::fs::remove_dir_all(out)?;
    }
    Ok(())
}

/// `tcp_lat --impl ergon --backend uring-sqpoll`: both ends' drivers on
/// io_uring, each `SQPOLL` thread on `--sqpoll-cpu` (a CPU off `--cpus`),
/// `--busy-read-us 0`; the row reads back as every other arm's and the
/// manifest records the backend and the `SQPOLL` CPU. Needs three CPUs:
/// the two ends' and the `SQPOLL` threads'.
#[test]
fn tcp_ergon_uring_sqpoll_round_trip() -> R {
    let c = cpus();
    assert!(
        c.len() >= 3,
        "uring-sqpoll needs a CPU off the two ends' ({c:?}): run on 3 or more CPUs"
    );
    let (a, b, sq) = (c[c.len() - 2], c[c.len() - 1], c[0]);
    let run = Run::tcp("ergon", 1, 1_000, 30_210);
    let out = scratch("tcp-uring-sqpoll")?;
    let o = Command::new(TCP)
        .args(run.common("local", Some(format!("{a},{b}")), &out))
        .args(run.addrs("local"))
        .args([
            "--round-trip",
            "--busy-read-us",
            "0",
            "--backend",
            "uring-sqpoll",
            "--sqpoll-cpu",
            &sq.to_string(),
        ])
        .output()?;
    finished("uring-sqpoll", &o)?;
    check(&run, &out, "load", 2)?;
    let m = std::fs::read_to_string(out.join("manifest.json"))?;
    for want in [
        "\"ergon.backend\": \"uring-sqpoll\"".to_owned(),
        format!("\"ergon.sqpoll_cpu\": \"{sq}\""),
        "\"busy_read_us\": \"0\"".to_owned(),
    ] {
        assert!(m.contains(&want), "no {want} in {m}");
    }
    std::fs::remove_dir_all(out)?;
    Ok(())
}

fn refused(bin: &str, args: &[&str], why: &str) -> R {
    let o = Command::new(bin).args(args).output()?;
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(!o.status.success(), "{args:?} ran");
    assert!(err.contains(why), "{args:?}: {err}");
    assert!(
        o.stdout.is_empty(),
        "{args:?} printed {}",
        String::from_utf8_lossy(&o.stdout)
    );
    Ok(())
}

fn with<'a>(b: &[&'a str], extra: &[&'a str]) -> Vec<&'a str> {
    [b, extra].concat()
}

/// A timer the kernel would round fails closed in both arms; flags an arm
/// cannot take, unknown flags, other sizes, a CPU short, the DPDK port
/// range and the cross-process one-way roles are refused.
#[test]
fn settings_an_arm_cannot_take_are_refused() -> R {
    let out = scratch("refused")?;
    let o = out.display().to_string();
    let base = [
        "--role",
        "local",
        "--round-trip",
        "--rate",
        "100",
        "--warmup-s",
        "0",
        "--duration-s",
        "1",
        "--label",
        "t:x",
        "--out",
        &o,
    ];
    let tcp = with(
        &base,
        &["--listen", "127.0.0.1:39020", "--connect-timeout-s", "5"],
    );
    let udp = with(
        &base,
        &["--local", "127.0.0.1:39030", "--peer", "127.0.0.1:39040"],
    );
    // 2.5 ms is not whole ticks at any CONFIG_HZ
    refused(
        TCP,
        &with(&tcp, &["--impl", "mio", "--rto-min-us", "2500"]),
        "TCP_RTO_MIN_US",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "ergon", "--rto-min-us", "2500"]),
        "tcp_rto_min_us",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "mio", "--delack-max-us", "2500"]),
        "TCP_DELACK_MAX_US",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "ergon", "--delack-max-us", "2500"]),
        "tcp_delack_max_us",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "ergon", "--busy-read-us", "2147483648"]),
        "busy_poll_us",
    )?;
    refused(
        UDP,
        &with(&udp, &["--impl", "ergon", "--busy-read-us", "2147483648"]),
        "busy_poll_us",
    )?;
    refused(
        UDP,
        &with(&udp, &["--impl", "mio", "--datagram-burst", "8"]),
        "--datagram-burst",
    )?;
    // the backend flags: ergon-io's, uring-sqpoll with its SQPOLL CPU and
    // no SO_BUSY_POLL, and nothing else
    refused(
        TCP,
        &with(&tcp, &["--impl", "mio", "--backend", "uring-sqpoll"]),
        "--impl ergon",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "ergon", "--backend", "recv-busy"]),
        "--backend recv-busy",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "ergon", "--backend", "uring-sqpoll"]),
        "needs --sqpoll-cpu",
    )?;
    refused(
        TCP,
        &with(
            &tcp,
            &[
                "--impl",
                "ergon",
                "--backend",
                "uring-sqpoll",
                "--sqpoll-cpu",
                "0",
                "--busy-read-us",
                "50",
            ],
        ),
        "--busy-read-us 50",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "ergon", "--sqpoll-cpu", "0"]),
        "--sqpoll-cpu is uring-sqpoll's",
    )?;
    refused(
        UDP,
        &with(&udp, &["--impl", "mio", "--size", "128"]),
        "--size 128",
    )?;
    refused(
        UDP,
        &with(&udp, &["--impl", "mio", "--bogus", "1"]),
        "--bogus",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "mio", "--nodelay", "false"]),
        "always on",
    )?;
    refused(
        UDP,
        &with(&udp, &["--impl", "mio", "--cpus", "1"]),
        "2 CPU(s)",
    )?;
    let dpdk = with(
        &base,
        &[
            "--impl",
            "mio",
            "--local",
            "127.0.0.1:19999",
            "--peer",
            "127.0.0.1:39050",
            "--streams",
            "2",
        ],
    );
    refused(UDP, &dpdk, "20000-21999")?;
    // the receiver's wildcard half too (the media driver's 21000-21999)
    let wildcard = with(
        &base,
        &[
            "--impl",
            "mio",
            "--local",
            "127.0.0.1:21500",
            "--peer",
            "127.0.0.1:39050",
        ],
    );
    refused(UDP, &wildcard, "20000-21999")?;
    let oneway = [
        "--impl",
        "mio",
        "--role",
        "oneway-send",
        "--rate",
        "100",
        "--label",
        "t:x",
        "--out",
        &o,
    ];
    refused(UDP, &oneway, "--role oneway-send")?;
    assert!(!out.join("results.csv").exists());
    std::fs::remove_dir_all(out)?;
    Ok(())
}

/// The connecting end binds `--source` before it connects: each
/// connection's own address is the source asked for, as the accepting end
/// sees it too; an address the host lacks fails the connect; tcp_lat takes
/// `--source` on the connecting end alone and records it.
#[test]
fn tcp_connections_leave_from_the_source_address() -> R {
    use io_bench::transport::sock::{accept_all, connect_all};
    use std::net::IpAddr;
    use std::time::Duration;

    let l = std::net::TcpListener::bind("127.0.0.3:0")?;
    let to = l.local_addr()?;
    let src: IpAddr = "127.0.0.2".parse()?;
    let wait = Duration::from_secs(5);
    let dial = std::thread::spawn(move || connect_all(to, Some(src), 2, wait));
    let accepted = accept_all(&l, 2, wait)?;
    let dialled = dial.join().map_err(|_| "dial panicked")??;
    for (d, a) in dialled.iter().zip(&accepted) {
        assert_eq!(d.local_addr()?.ip(), src);
        assert_eq!(a.peer_addr()?.ip(), src);
        assert_eq!(d.peer_addr()?, to);
    }
    // TEST-NET-1: no host has it
    let none: IpAddr = "192.0.2.1".parse()?;
    let e = connect_all(to, Some(none), 1, Duration::from_millis(200))
        .err()
        .ok_or("a connect from an address the host lacks ran")?;
    assert!(e.to_string().contains("from 192.0.2.1"), "{e}");

    let out = scratch("source")?;
    let o = out.display().to_string();
    let run = Command::new(TCP)
        .args([
            "--impl",
            "mio",
            "--role",
            "local",
            "--round-trip",
            "--rate",
            "100",
            "--warmup-s",
            "0",
            "--duration-s",
            "1",
            "--label",
            "t:src",
            "--out",
            &o,
            "--listen",
            "127.0.0.3:39060",
            "--source",
            "127.0.0.2",
        ])
        .output()?;
    finished("tcp_lat --source", &run)?;
    let m = std::fs::read_to_string(out.join("manifest.json"))?;
    assert!(m.contains("\"source\": \"127.0.0.2\""), "{m}");
    std::fs::remove_dir_all(&out)?;
    let base = [
        "--impl", "mio", "--rate", "100", "--label", "t:x", "--out", &o, "--source",
    ];
    refused(
        TCP,
        &with(
            &base,
            &["127.0.0.2", "--role", "pong", "--listen", "127.0.0.3:39070"],
        ),
        "--source is the connecting end's",
    )?;
    refused(
        TCP,
        &with(
            &base,
            &["nic0", "--role", "ping", "--connect", "127.0.0.3:39070"],
        ),
        "--source nic0: an IP address",
    )?;
    Ok(())
}

/// The mio arms' socket code on a caller's own poll: two UDP stream sets and
/// two TCP ones on one `Registry`, each from its own token base, each token
/// mapped back to its set's stream and read there, frames and datagrams
/// whole and in order.
#[test]
fn mio_stream_sets_share_a_callers_registry() -> R {
    use io_bench::transport::mio_tcp::{MioTcpOptions, MioTcpReader, MioTcpStreams};
    use io_bench::transport::mio_udp::{MioUdpOptions, MioUdpReader, MioUdpStreams};
    use io_bench::transport::point::Sent;
    use io_bench::transport::sock::{accept_all, connect_all};
    use std::time::{Duration, Instant};

    let mut poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(64);
    let udp = MioUdpOptions {
        rcvbuf: 1 << 20,
        sndbuf: None,
        busy_read_us: 0,
        epoll_events: 64,
    };
    let (pa, pb) = ("127.0.0.1:39200".parse()?, "127.0.0.1:39210".parse()?);
    let a = MioUdpStreams::new(poll.registry(), 0, pa, pb, 2, udp.clone())?;
    let b = MioUdpStreams::new(poll.registry(), 100, pb, pa, 2, udp)?;
    assert_eq!(
        (b.stream_of(mio::Token(101)), b.stream_of(mio::Token(1))),
        (Some(1), None)
    );
    let l = std::net::TcpListener::bind("127.0.0.1:0")?;
    let to = l.local_addr()?;
    let wait = Duration::from_secs(5);
    let dial = std::thread::spawn(move || connect_all(to, None, 2, wait));
    let accepted = accept_all(&l, 2, wait)?;
    let dialled = dial.join().map_err(|_| "dial panicked")??;
    let tcp = MioTcpOptions {
        rcvbuf: 1 << 20,
        sndbuf: None,
        busy_read_us: 0,
        epoll_events: 64,
        quickack_per_read: true,
        rto_min_us: None,
        delack_max_us: None,
    };
    let c = MioTcpStreams::new(poll.registry(), 200, dialled, &tcp)?;
    let d = MioTcpStreams::new(poll.registry(), 300, accepted, &tcp)?;
    for k in 0u8..4 {
        assert_eq!(a.send(u32::from(k % 2), &[k; 64])?, Sent::Sent);
        assert_eq!(c.send(u32::from(k % 2), &[k + 10; 64])?, Sent::Sent);
    }
    let (mut ur, mut tr) = (MioUdpReader::new(), MioTcpReader::new(&d)?);
    let (mut got_udp, mut got_tcp) = (Vec::new(), Vec::new());
    let until = Instant::now() + wait;
    while (got_udp.len() < 4 || got_tcp.len() < 4) && Instant::now() < until {
        a.flush()?;
        c.flush()?;
        poll.poll(&mut events, Some(Duration::from_millis(10)))?;
        for ev in &events {
            if let Some(s) = b.stream_of(ev.token()) {
                ur.read(&b, s, &mut |p: &[u8]| got_udp.push((s, p[0], p.len())))?;
            } else if let Some(s) = d.stream_of(ev.token()) {
                loop {
                    let r = tr.read(&d, s, &mut |p: &[u8]| got_tcp.push((s, p[0], p.len())))?;
                    if r.done {
                        break;
                    }
                }
            } else {
                assert!(
                    a.stream_of(ev.token()).is_some() || c.stream_of(ev.token()).is_some(),
                    "{ev:?}"
                );
            }
        }
    }
    got_udp.sort_unstable();
    got_tcp.sort_unstable();
    assert_eq!(got_udp, [(0, 0, 64), (0, 2, 64), (1, 1, 64), (1, 3, 64)]);
    assert_eq!(
        got_tcp,
        [(0, 10, 64), (0, 12, 64), (1, 11, 64), (1, 13, 64)]
    );
    assert!(!tr.all_closed() && tr.stats.quickacks > 0);
    Ok(())
}

/// `Cpus_allowed_list` for `pid` (ranges expanded).
fn allowed_list(pid: u32) -> R<Vec<usize>> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status"))?;
    let line = status
        .lines()
        .find(|l| l.starts_with("Cpus_allowed_list:"))
        .ok_or("no Cpus_allowed_list")?;
    let spec = line
        .split_whitespace()
        .nth(1)
        .ok_or("Cpus_allowed_list has no value")?;
    let mut out = Vec::new();
    for part in spec.split(',') {
        if let Some((start, end)) = part.split_once('-') {
            let start: usize = start.parse()?;
            let end: usize = end.parse()?;
            out.extend(start..=end);
        } else {
            out.push(part.parse()?);
        }
    }
    Ok(out)
}

/// The kernel pong (`wstls_lat --role pong`) pins its echo thread before it
/// says ready. smoltcp and F-Stack pongs call the same `kernel_serve`. The
/// starting mask has to be wider than `--cpus`, or the pin is invisible.
#[test]
fn the_kernel_pong_pins_its_echo_thread() -> R {
    let started = cpus();
    assert!(
        started.len() >= 2,
        "this process can use only {started:?}: the pin cannot be told from the starting mask"
    );
    let cpu = started[started.len() - 1];
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let out = scratch("wstls-pin")?;
    let out_s = out.display().to_string();
    let local = format!("127.0.0.1:{port}");
    let cpu_s = cpu.to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_wstls_lat"))
        .args([
            "--impl",
            "ergon",
            "--backend",
            "epoll-spin",
            "--echo",
            "tcp",
            "--role",
            "pong",
            "--local",
            &local,
            "--peer",
            "127.0.0.1:9",
            "--cpus",
            &cpu_s,
            "--streams",
            "1",
            "--rate",
            "1",
            "--warmup-s",
            "0",
            "--duration-s",
            "1",
            "--label",
            "pin",
            "--out",
            &out_s,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let ready = wait_ready(&mut child);
    let mask = allowed_list(child.id());
    let _ = child.kill();
    let _ = child.wait();
    ready.map_err(|e| format!("{e}"))?;
    let mask = mask?;
    assert_eq!(
        mask,
        vec![cpu],
        "echo affinity {mask:?}, --cpus {cpu}, started on {started:?}"
    );
    std::fs::remove_dir_all(&out)?;
    Ok(())
}
