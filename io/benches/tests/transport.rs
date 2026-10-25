//! `udp_lat` and `tcp_lat` end to end over loopback: a `pong` process and
//! a `ping` process per arm (mio and ergon-io, 1 and 128 streams), and
//! `local` (both ends in one process). Each run must print one row that
//! parses (34 columns once lab.sh prefixes the rep, read back by
//! analyse.py and transport_points.py), lose nothing at a low rate, and
//! write its manifest, histograms, spikes (with the `# max=` line) and
//! stalls. Settings an arm cannot take are refused.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use ergon_latency_core::manifest::sha256_hex;
use ergon_latency_core::message::template;

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

const UDP: &str = env!("CARGO_BIN_EXE_udp_lat");
const TCP: &str = env!("CARGO_BIN_EXE_tcp_lat");

/// The CPUs this process may use.
fn cpus() -> Vec<usize> {
    // SAFETY: a zeroed cpu_set_t is valid; the kernel fills at most its size.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set) != 0 {
            return vec![0];
        }
        (0..libc::CPU_SETSIZE as usize)
            .filter(|&c| libc::CPU_ISSET(c, &set))
            .collect()
    }
}

/// Two distinct CPUs (the last two this process may use, away from CPU 0).
fn two_cpus() -> (usize, usize) {
    let c = cpus();
    match c.len() {
        0 | 1 => (0, 0),
        n => (c[n - 2], c[n - 1]),
    }
}

fn out_dir(name: &str) -> R<PathBuf> {
    let d = std::env::temp_dir().join(format!("transport-test-{}-{name}", std::process::id()));
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
    let mut lines = BufReader::new(out).lines();
    let until = Instant::now() + Duration::from_secs(30);
    while Instant::now() < until {
        match lines.next() {
            Some(l) => {
                if l? == "ready" {
                    return Ok(());
                }
            }
            None => break,
        }
    }
    Err("the pong never said ready".into())
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

/// The point: one ping and one pong process, or one local one.
struct Run {
    bin: &'static str,
    imp: &'static str,
    streams: u32,
    rate: u64,
    /// `--local/--peer` (udp) or `--listen` (tcp) for each side
    addrs: Vec<String>,
}

impl Run {
    fn udp(imp: &'static str, streams: u32, rate: u64, base: u16) -> Self {
        Self {
            bin: UDP,
            imp,
            streams,
            rate,
            addrs: vec![
                format!("127.0.0.1:{base}"),
                format!("127.0.0.1:{}", base + 1000),
            ],
        }
    }

    fn tcp(imp: &'static str, streams: u32, rate: u64, port: u16) -> Self {
        Self {
            bin: TCP,
            imp,
            streams,
            rate,
            addrs: vec![format!("127.0.0.1:{port}")],
        }
    }

    fn transport(&self) -> &'static str {
        if self.bin == UDP { "udp" } else { "tcp" }
    }

    fn label(&self) -> String {
        format!(
            "t:{}-{}@STREAMS={}",
            self.imp,
            self.transport(),
            self.streams
        )
    }

    fn common(&self, role: &str, cpus: &str, out: &Path) -> Vec<String> {
        vec![
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
            s(1),
            s("--cpus"),
            s(cpus),
            s("--label"),
            self.label(),
            s("--out"),
            out.display().to_string(),
            s("--idle-exit-s"),
            s(2),
            s("--wait-s"),
            s(20),
        ]
    }

    /// The arm's addresses for `role` (udp: ping binds the first, pong the
    /// second; tcp: pong listens, ping connects).
    fn addr_args(&self, role: &str) -> Vec<String> {
        if self.bin == UDP {
            let (l, p) = if role == "pong" {
                (&self.addrs[1], &self.addrs[0])
            } else {
                (&self.addrs[0], &self.addrs[1])
            };
            vec![s("--local"), l.clone(), s("--peer"), p.clone()]
        } else {
            let k = if role == "ping" {
                "--connect"
            } else {
                "--listen"
            };
            vec![s(k), self.addrs[0].clone()]
        }
    }

    /// A pong process, then a ping process; the ping's out dir.
    fn ping_pong(&self) -> R<PathBuf> {
        let (cp, cq) = two_cpus();
        let tag = format!("{}-{}-{}", self.transport(), self.imp, self.streams);
        let (po, pi) = (
            out_dir(&format!("{tag}-pong"))?,
            out_dir(&format!("{tag}-ping"))?,
        );
        let mut pong = Command::new(self.bin)
            .args(self.common("pong", &s(cq), &po))
            .args(self.addr_args("pong"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Err(e) = wait_ready(&mut pong) {
            let _ = pong.kill();
            let o = pong.wait_with_output()?;
            return Err(format!("{e}: {}", String::from_utf8_lossy(&o.stderr)).into());
        }
        let ping = Command::new(self.bin)
            .args(self.common("ping", &s(cp), &pi))
            .args(self.addr_args("ping"))
            .output()?;
        let pong = pong.wait_with_output()?;
        finished("ping", &ping)?;
        finished("pong", &pong)?;
        // the pong wrote its manifest and stalls, no row
        assert!(po.join("manifest.json").exists());
        assert!(!po.join("results.csv").exists());
        let pm = std::fs::read_to_string(po.join("manifest.json"))?;
        assert!(pm.contains("\"role\": \"pong\""), "{pm}");
        Ok(pi)
    }

    fn local(&self) -> R<PathBuf> {
        let (cp, cq) = two_cpus();
        let out = out_dir(&format!(
            "{}-{}-{}-local",
            self.transport(),
            self.imp,
            self.streams
        ))?;
        let o = Command::new(self.bin)
            .args(self.common("local", &format!("{cp},{cq}"), &out))
            .args(self.addr_args("local"))
            .output()?;
        finished("local", &o)?;
        Ok(out)
    }

    /// Checks the row and files in `out`; returns the row's columns.
    fn check(&self, out: &Path, busy: u64) -> R<Vec<String>> {
        let text = std::fs::read_to_string(out.join("results.csv"))?;
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 1, "{text}");
        let row: Vec<String> = lines[0].split(',').map(str::to_owned).collect();
        assert_eq!(row.len(), 33, "{text}");
        let n = |i: usize| -> R<u64> { Ok(row[i].parse::<u64>()?) };
        let label = self.label();
        assert_eq!(
            (row[0].as_str(), row[1].as_str(), row[2].as_str()),
            ("load", label.as_str(), self.transport())
        );
        assert_eq!((n(3)?, n(4)?, n(5)?), (64, self.rate, 1));
        let total = self.rate * 2;
        let measured = self.rate;
        // samples, sent, received, backpressured
        assert_eq!(
            (n(13)?, n(14)?, n(15)?, n(16)?),
            (measured, total, total, 0),
            "{text}"
        );
        // scheduled, censored; min .. max ordered
        assert_eq!((n(19)?, n(21)?), (total, 0), "{text}");
        let (min, p50, p99, max) = (n(22)?, n(6)?, n(8)?, n(11)?);
        assert!(min > 0 && min <= p50 && p50 <= p99 && p99 <= max, "{text}");
        // lost, reordered, dup
        assert_eq!((n(23)?, n(24)?, n(25)?), (0, 0, 0), "{text}");
        if self.bin == UDP {
            assert_eq!(row[26], "", "UDP has no retransmit counter");
            assert_eq!(n(27)?, 106);
        } else {
            let _retrans = n(26)?;
            assert!(matches!(n(27)?, 122 | 134), "{text}");
        }
        assert_eq!(n(28)?, busy);
        // stream 0's max within the point's
        assert!(n(32)? <= max);

        let name = label.replacen(':', "-", 1);
        let manifest = std::fs::read_to_string(out.join("manifest.json"))?;
        let sha = sha256_hex(&template());
        assert!(
            manifest.contains(&format!("\"payload_sha256\": \"{sha}\"")),
            "{manifest}"
        );
        assert!(manifest.contains(&format!("\"arm\": \"{}-{}\"", self.imp, self.transport())));
        assert!(manifest.contains("\"payload_size\": 64"));
        for f in ["histogram-{}.csv", "histogram-{}-send.csv", "stalls-{}.csv"] {
            let p = out.join(f.replace("{}", &name));
            assert!(p.exists(), "{} missing", p.display());
        }
        let spikes = std::fs::read_to_string(out.join(format!("spikes-{name}.csv")))?;
        let max_line = spikes
            .lines()
            .find_map(|l| l.strip_prefix("# max="))
            .ok_or("no # max= line")?;
        let v: Vec<u64> = max_line
            .split(',')
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        assert_eq!(v.len(), 3, "{max_line}");
        assert_eq!(v[1], max, "the # max= line is the row's max");
        assert!(v[2] < u64::from(self.streams));
        Ok(row)
    }
}

fn cloud_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../md-dpdk/benches/cloud")
}

/// analyse.py's reader and transport_points.py's take the row as lab.sh
/// records it (rep first, the channel `<transport>-r<rate>`).
fn python_reads(run: &Run, row: &[String]) -> R {
    let dir = out_dir(&format!(
        "py-{}-{}-{}",
        run.transport(),
        run.imp,
        run.streams
    ))?;
    let mut cols = row.to_vec();
    cols[2] = format!("{}-r{}", run.transport(), run.rate);
    std::fs::write(dir.join("results.csv"), format!("1,{}\n", cols.join(",")))?;
    let script = r#"
import sys
sys.path.insert(0, sys.argv[1])
import analyse, transport_points
run_dir, label, point, rate = sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
r = analyse.read_results(run_dir + "/results.csv")
key = (label, 64, point.split("-", 1)[1] if point.startswith("udp-") else point)
assert key in r.load, (key, dict(r.load))
[p] = transport_points.read_points(run_dir)
assert p.label == label and p.rate == rate and p.size == 64 and p.kind == "load", p
assert p.row["lost"] == 0 and p.row["dup"] == 0 and p.row["samples"] == rate, p.row
print("ok")
"#;
    let o = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(cloud_dir())
        .arg(&dir)
        .arg(run.label())
        .arg(&cols[2])
        .arg(s(run.rate))
        .output()?;
    if !o.status.success() || o.stdout != b"ok\n" {
        return Err(format!(
            "python3: {}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
        .into());
    }
    Ok(())
}

fn ping_pong_and_local(run: &Run) -> R {
    let out = run.ping_pong()?;
    let row = run.check(&out, 1)?;
    python_reads(run, &row)?;
    let out = run.local()?;
    run.check(&out, 2)?;
    Ok(())
}

#[test]
fn udp_mio_one_stream() -> R {
    ping_pong_and_local(&Run::udp("mio", 1, 1_000, 21_000))
}

#[test]
fn udp_ergon_one_stream() -> R {
    ping_pong_and_local(&Run::udp("ergon", 1, 1_000, 23_000))
}

#[test]
fn udp_mio_128_streams() -> R {
    ping_pong_and_local(&Run::udp("mio", 128, 2_560, 25_000))
}

#[test]
fn udp_ergon_128_streams() -> R {
    ping_pong_and_local(&Run::udp("ergon", 128, 2_560, 27_000))
}

#[test]
fn tcp_mio_one_stream() -> R {
    ping_pong_and_local(&Run::tcp("mio", 1, 1_000, 29_001))
}

#[test]
fn tcp_ergon_one_stream() -> R {
    ping_pong_and_local(&Run::tcp("ergon", 1, 1_000, 29_002))
}

#[test]
fn tcp_mio_128_streams() -> R {
    ping_pong_and_local(&Run::tcp("mio", 128, 2_560, 29_003))
}

#[test]
fn tcp_ergon_128_streams() -> R {
    ping_pong_and_local(&Run::tcp("ergon", 128, 2_560, 29_004))
}

/// TCP's settings reach the socket the same way in both arms: per-read
/// quick-ACK re-arms are counted, and the two timers are read back as set.
#[test]
fn tcp_settings_reach_both_arms_sockets() -> R {
    for imp in ["mio", "ergon"] {
        let run = Run::tcp(imp, 2, 1_000, 29_010);
        let (cp, cq) = two_cpus();
        let out = out_dir(&format!("tcp-settings-{imp}"))?;
        let o = Command::new(TCP)
            .args(run.common("local", &format!("{cp},{cq}"), &out))
            .args(run.addr_args("local"))
            .args([
                "--quickack-per-read",
                "--rto-min-us",
                "5000",
                "--delack-max-us",
                "4000",
                "--nodelay",
            ])
            .output()?;
        finished(imp, &o)?;
        run.check(&out, 2)?;
        let m = std::fs::read_to_string(out.join("manifest.json"))?;
        for want in [
            "\"readback.tcp_nodelay\": \"1\"",
            "\"readback.tcp_rto_min_us\": \"5000\"",
            "\"readback.tcp_delack_max_us\": \"4000\"",
            "\"readback.so_busy_poll\": \"0\"",
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
    }
    Ok(())
}

fn with<'a>(b: &[&'a str], extra: &[&'a str]) -> Vec<&'a str> {
    [b, extra].concat()
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

/// A timer the kernel would round fails closed in both arms; flags an arm
/// cannot take, unknown flags, other sizes and a CPU short are refused.
#[test]
fn settings_an_arm_cannot_take_are_refused() -> R {
    let out = out_dir("refused")?;
    let o = out.display().to_string();
    let (cp, cq) = two_cpus();
    let cpus = format!("{cp},{cq}");
    let base = [
        "--role",
        "local",
        "--rate",
        "100",
        "--warmup-s",
        "0",
        "--duration-s",
        "1",
        "--label",
        "t:x",
        "--out",
        o.as_str(),
        "--cpus",
        cpus.as_str(),
    ];
    let tcp = [&base[..], &["--listen", "127.0.0.1:29020", "--wait-s", "5"]].concat();
    let udp = [
        &base[..],
        &["--local", "127.0.0.1:29030", "--peer", "127.0.0.1:29040"],
    ]
    .concat();
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
        "unknown flag --bogus",
    )?;
    refused(
        TCP,
        &with(&tcp, &["--impl", "mio", "--nodelay", "false"]),
        "always on",
    )?;
    let one_cpu = [&base[..12], &["--cpus", "1", "--impl", "mio"], &udp[14..]].concat();
    refused(UDP, &one_cpu, "--cpus: 2 CPU(s)")?;
    let oneway = with(&udp, &["--impl", "mio", "--role", "oneway-send"]);
    refused(UDP, &oneway, "--role oneway-send")?;
    assert!(!out.join("results.csv").exists());
    Ok(())
}
