//! `aeron_lat` end to end at a low rate, each run on a media driver it
//! embeds (`--driver embedded`, the kernel's UDP on 127.0.0.1): IPC, spy and
//! UDP (reliable and not), one way and round trip, 8 streams, and the
//! cross-host shape (a `ping` and a `pong` process, each with its own
//! driver). Each point's row must read back through the analysis's own
//! readers (md-dpdk/benches/cloud: `transport_points.read_points`,
//! `analyse.read_results`, `spikes.transport_max`), lose nothing, carry the
//! manifest the fairness checks compare, and show that the busy threads
//! allocated nothing while measuring.

use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

type R = Result<(), Box<dyn std::error::Error>>;
type Res<T> = Result<T, Box<dyn std::error::Error>>;

const RATE: u64 = 1000;
const TERM: &str = "65536";

/// The core's payload hash and build, as the binary's manifest must carry
/// them.
fn core() -> (String, &'static str) {
    (
        ergon_latency_core::manifest::sha256_hex(&ergon_latency_core::message::template()),
        ergon_latency_core::manifest::CORE_BUILD_HASH,
    )
}

fn scratch(name: &str) -> Res<PathBuf> {
    let dir = std::env::temp_dir().join(format!("aeron-lat-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn cloud() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benches/cloud")
}

/// A base port with `n` free UDP ports from it on 127.0.0.1.
fn free_ports(n: u16) -> Res<u16> {
    let seed = u16::try_from(std::process::id() % 1000)?;
    for k in 0..200u16 {
        let base = 30_000 + ((seed + k * 37) % 1000) * 20;
        let held: Result<Vec<UdpSocket>, _> = (0..n).map(|i| UdpSocket::bind(("127.0.0.1", base + i))).collect();
        if held.is_ok() {
            return Ok(base);
        }
    }
    Err("no free run of UDP ports".into())
}

fn command(out: &Path, dir: &Path, args: &[&str]) -> Res<Command> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aeron_lat"));
    cmd.args(args)
        .args([
            "--driver",
            "embedded",
            "--term-length",
            TERM,
            "--connect-timeout-s",
            "20",
        ])
        .arg("--out")
        .arg(out)
        .arg("--aeron-dir")
        .arg(dir)
        .env("ERGON_HOST_CHECKS", "off")
        .env_remove("AERON_THREADING_MODE")
        .env_remove("RUSTERON_MEDIA_DRIVER_TRANSPORT")
        .env_remove("RUSTERON_MEDIA_DRIVER_DIR");
    Ok(cmd)
}

fn ok(what: &str, o: &Output) -> Res<String> {
    if !o.status.success() {
        return Err(format!(
            "{what}: {}\n{}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(o.stdout.clone())?)
}

/// What the analysis must make of a point.
struct Expect<'a> {
    kind: &'a str,
    transport: &'a str,
    channel: &'a str,
    wire_bytes: u64,
    busy_threads: u32,
    streams: u32,
}

/// The analysis's check of one point, as the runner lays it out: the row in
/// `<run>/results.csv` (rep prefixed, the channel column `<transport>-r<rate>`),
/// the manifest and spikes file under `<run>/<set>/` by the point's tag.
const CHECK: &str = r#"
import json, os, shutil, sys
cloud, out, run, kind, streams, sha, core, transport, channel, wire, busy = sys.argv[1:12]
streams, wire, busy = int(streams), int(wire), int(busy)
sys.path.insert(0, cloud)
import analyse, spikes, transport_points as tp
line = open(os.path.join(out, "results.csv")).read().strip()
cols = line.split(",")
assert len(cols) == tp.ROW_LEN - 1, (len(cols), line)
label, rate = cols[1], int(cols[4])
assert cols[2] == channel, cols
cols[2] = f"{channel}-r{rate}"
set_ = label.partition(":")[0]
os.makedirs(os.path.join(run, set_), exist_ok=True)
open(os.path.join(run, "results.csv"), "w").write("1," + ",".join(cols) + "\n")
tag = tp.tag_of(label, 64, 1, rate, kind)
shutil.copy(os.path.join(out, "manifest.json"), os.path.join(run, set_, f"manifest-{tag}.json"))
shutil.copy(os.path.join(out, f"spikes-{label}.csv"), os.path.join(run, set_, f"spikes-{tag}.csv"))
[p] = tp.read_points(run)
r = p.row
assert (p.kind, p.streams, p.rate, p.size) == (kind, streams, rate, 64), p
assert r["scheduled"] == r["sent"] == r["received"], r
assert r["lost"] == 0 and r["reordered"] == 0 and r["dup"] == 0, r
assert r["samples"] == rate and r["censored"] == 0, r
assert r["retrans"] == 0 and r["wire_bytes"] == wire and r["busy_threads"] == busy, r
assert 0 < r["min"] <= r["p50"] <= r["p99"] <= r["p99.99"] <= r["max"], r
assert 0 < r["stream0_p50"] <= r["stream0_max"] <= r["max"], r
m = p.manifest
assert m["payload_sha256"] == sha and m["payload_size"] == 64 and m["wire_bytes"] == wire, m
assert m["core_build_hash"] == core and m["arm"] == transport, m
s = m["settings"]
assert s["transport"] == transport and s["kind"] == kind and s["streams"] == str(streams), s
assert s["hot_path_allocs"] == "0", s
assert s["max_ns"] == str(r["max"]) and s["lost"] == "0", s
assert s["aeron.retransmits-sent"] == "0" and "aeron.naks-sent" in s, s
loc = spikes.transport_max(os.path.join(run, set_), tag, r["max"])
assert loc is not None and loc[2] == r["max"], (loc, r["max"])
if kind == "load":
    res = analyse.read_results(os.path.join(run, "results.csv"))
    # analyse.py keys a load point by its channel column less a `udp-` prefix
    [rep] = res.load[(label, 64, f"{channel}-r{rate}".removeprefix("udp-"))]
    assert rep[:6] == [float(r[c]) for c in ("p50", "p90", "p99", "p99.9", "p99.99", "max")], rep
print("ok")
"#;

fn check(out: &Path, e: &Expect<'_>) -> R {
    let (sha, core) = core();
    let o = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(CHECK)
        .arg(cloud())
        .arg(out)
        .arg(out.join("run"))
        .args([e.kind, &e.streams.to_string(), &sha, core, e.transport, e.channel])
        .args([e.wire_bytes.to_string(), e.busy_threads.to_string()])
        .output()?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.status.success() && said == "ok\n", "{said}");
    Ok(())
}

fn label(arm: &str, set: &str, streams: u32) -> String {
    if streams == 1 {
        format!("{set}:{arm}")
    } else {
        format!("{set}:{arm}@STREAMS={streams}")
    }
}

/// One `--role local` point: `extra` after the common flags.
fn local(name: &str, set: &str, streams: u32, extra: &[&str], e: &Expect<'_>) -> R {
    let base = scratch(name)?;
    let (out, dir) = (base.join("out"), base.join("aeron"));
    let label = label(e.transport, set, streams);
    let (rate, n) = (RATE.to_string(), streams.to_string());
    let mut args = vec!["--role", "local", "--rate", &rate, "--streams", &n];
    args.extend(["--warmup-s", "1", "--duration-s", "1", "--label", &label]);
    args.extend(extra);
    let o = command(&out, &dir, &args)?.output()?;
    let row = ok(name, &o)?;
    assert_eq!(
        row,
        std::fs::read_to_string(out.join("results.csv"))?,
        "the printed row is the written one"
    );
    for f in ["histogram", "histogram-send", "spikes", "stalls"] {
        assert!(out.join(format!("{f}-{label}.csv")).exists(), "{f}");
    }
    check(&out, e)?;
    std::fs::remove_dir_all(base)?;
    Ok(())
}

const fn expect<'a>(kind: &'a str, transport: &'a str, channel: &'a str, wire_bytes: u64, streams: u32) -> Expect<'a> {
    // local: two client threads and the driver's three
    Expect {
        kind,
        transport,
        channel,
        wire_bytes,
        busy_threads: 5,
        streams,
    }
}

#[test]
fn ipc_one_way_one_stream() -> R {
    local(
        "ipc-1",
        "1",
        1,
        &["--mode", "ipc"],
        &expect("oneway", "aeron-ipc", "ipc", 96, 1),
    )
}

#[test]
fn ipc_one_way_eight_streams() -> R {
    local(
        "ipc-8",
        "1",
        8,
        &["--mode", "ipc"],
        &expect("oneway", "aeron-ipc", "ipc", 96, 8),
    )
}

#[test]
fn ipc_round_trip_eight_streams() -> R {
    local(
        "ipc-rtt-8",
        "1",
        8,
        &["--mode", "ipc", "--round-trip"],
        &expect("load", "aeron-ipc", "ipc", 96, 8),
    )
}

#[test]
fn spy_one_way_eight_streams() -> R {
    local(
        "spy-8",
        "2b",
        8,
        &["--mode", "spy"],
        &expect("oneway", "aeron-spy", "udp", 96, 8),
    )
}

#[test]
fn spy_round_trip_eight_streams() -> R {
    local(
        "spy-rtt-8",
        "2b",
        8,
        &["--mode", "spy", "--round-trip"],
        &expect("load", "aeron-spy", "udp", 96, 8),
    )
}

fn udp_local(name: &str, reliable: &str, round_trip: bool) -> R {
    let base = free_ports(16)?;
    let (ping, reply) = (format!("127.0.0.1:{base}"), format!("127.0.0.1:{}", base + 8));
    let mut extra = vec!["--mode", "udp", "--reliable", reliable, "--endpoint", &ping];
    if round_trip {
        extra.extend(["--round-trip", "--reply", &reply]);
    }
    let kind = if round_trip { "load" } else { "oneway" };
    local(name, "2b", 8, &extra, &expect(kind, "aeron-udp", "udp", 138, 8))
}

#[test]
fn udp_reliable_one_way_eight_streams() -> R {
    udp_local("udp-rel-8", "true", false)
}

#[test]
fn udp_reliable_round_trip_eight_streams() -> R {
    udp_local("udp-rel-rtt-8", "true", true)
}

#[test]
fn udp_unreliable_one_way_eight_streams() -> R {
    udp_local("udp-unrel-8", "false", false)
}

#[test]
fn udp_unreliable_round_trip_eight_streams() -> R {
    udp_local("udp-unrel-rtt-8", "false", true)
}

/// Kills `child` if it outlives `limit`; its output either way.
fn wait(mut child: Child, limit: Duration) -> Res<Output> {
    let deadline = Instant::now() + limit;
    while child.try_wait()?.is_none() {
        if Instant::now() > deadline {
            child.kill()?;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(child.wait_with_output()?)
}

/// The cross-host shape on one host: a pong process and a ping process,
/// each on a driver of its own, over UDP (reliable and not). The pong ends
/// by itself once the ping's images go, and writes its manifest alone.
fn ping_pong(name: &str, reliable: &str) -> R {
    let root = scratch(name)?;
    let base = free_ports(16)?;
    let (ping_ep, reply_ep) = (format!("127.0.0.1:{base}"), format!("127.0.0.1:{}", base + 8));
    let label = label("aeron-udp", "2a", 8);
    let common = [
        "--mode",
        "udp",
        "--reliable",
        reliable,
        "--endpoint",
        &ping_ep,
        "--reply",
        &reply_ep,
        "--streams",
        "8",
        "--label",
        &label,
    ];
    let pong_out = root.join("pong");
    let mut pong = command(
        &pong_out,
        &root.join("aeron-pong"),
        &[&["--role", "pong"][..], &common].concat(),
    )?;
    let pong = pong.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let rate = RATE.to_string();
    let ping_out = root.join("ping");
    let ping = command(
        &ping_out,
        &root.join("aeron-ping"),
        &[
            &[
                "--role",
                "ping",
                "--rate",
                &rate,
                "--warmup-s",
                "1",
                "--duration-s",
                "1",
            ][..],
            &common,
        ]
        .concat(),
    )?
    .output();
    let pong = wait(pong, Duration::from_secs(60))?;
    let row = ok("ping", &ping?)?;
    assert_eq!(ok("pong", &pong)?, "", "a pong prints no row");
    assert!(!pong_out.join("results.csv").exists(), "a pong writes no row");
    let m = std::fs::read_to_string(pong_out.join("manifest.json"))?;
    for want in [
        "\"role\": \"pong\"",
        "\"hot_path_allocs\": \"0\"",
        "\"transport\": \"aeron-udp\"",
    ] {
        assert!(m.contains(want), "{want}: {m}");
    }
    assert!(row.starts_with("load,"), "{row}");
    check(
        &ping_out,
        &Expect {
            kind: "load",
            transport: "aeron-udp",
            channel: "udp",
            wire_bytes: 138,
            busy_threads: 4,
            streams: 8,
        },
    )?;
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn udp_reliable_ping_pong_eight_streams() -> R {
    ping_pong("pp-rel-8", "true")
}

#[test]
fn udp_unreliable_ping_pong_eight_streams() -> R {
    ping_pong("pp-unrel-8", "false")
}

#[test]
fn flags_that_cannot_make_a_fair_point_are_refused() -> R {
    let root = scratch("refused")?;
    let (out, dir) = (root.join("out"), root.join("aeron"));
    let base = ["--rate", "10", "--label", "x"];
    for args in [
        &["--role", "local", "--mode", "ipc", "--size", "32"][..],
        &["--role", "local", "--mode", "ipc", "--reliable", "false"],
        &["--role", "local", "--mode", "udp"],
        &[
            "--role",
            "local",
            "--mode",
            "udp",
            "--round-trip",
            "--endpoint",
            "127.0.0.1:20000",
        ],
        &["--role", "oneway-recv", "--mode", "ipc"],
        &["--role", "local", "--mode", "ipc", "--cpus", "1"],
        &["--role", "local", "--mode", "ipc", "--bogus"],
        &["--role", "local", "--mode", "tcp"],
    ] {
        let o = command(&out, &dir, &[args, &base].concat())?.output()?;
        assert!(!o.status.success(), "{args:?}");
        assert!(o.stdout.is_empty(), "no row: {args:?}");
        assert!(!out.join("results.csv").exists(), "{args:?}");
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
