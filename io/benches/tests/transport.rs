//! `udp_lat` and `tcp_lat` end to end over loopback, both arms (mio and
//! ergon-io), 1 and 128 streams: a `pong` process and a `ping` process,
//! then both ends in one process (`local --round-trip`, and one way). Each
//! point's row must read back through the analysis's own readers
//! (md-dpdk/benches/cloud: `transport_points.read_points`,
//! `analyse.read_results`, `spikes.transport_max`), lose nothing at a low
//! rate, carry the manifest the fairness checks compare, and show that the
//! busy threads allocated nothing while measuring. Settings an arm cannot
//! take are refused.
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
}

impl Run {
    const fn udp(imp: &'static str, streams: u32, rate: u64, port: u16) -> Self {
        Self {
            bin: UDP,
            imp,
            streams,
            rate,
            port,
        }
    }

    const fn tcp(imp: &'static str, streams: u32, rate: u64, port: u16) -> Self {
        Self {
            bin: TCP,
            imp,
            streams,
            rate,
            port,
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
    every_role(&Run::udp("mio", 1, 1_000, 21_000))
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
        &with(&tcp, &["--impl", "ergon", "--busy-read-us", "50"]),
        "SO_BUSY_POLL 0",
    )?;
    refused(
        UDP,
        &with(&udp, &["--impl", "ergon", "--busy-read-us", "50"]),
        "SO_BUSY_POLL 0",
    )?;
    refused(
        UDP,
        &with(&udp, &["--impl", "mio", "--datagram-burst", "8"]),
        "--datagram-burst",
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
    refused(UDP, &dpdk, "20000-20999")?;
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
