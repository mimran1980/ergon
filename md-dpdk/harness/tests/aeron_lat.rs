//! `aeron_lat` end to end at a low rate, each run on a media driver it
//! embeds (`--driver embedded`, the kernel's UDP on 127.0.0.1): IPC, spy and
//! UDP (reliable and not), one way and round trip, 8 streams, the
//! cross-host shape (a `ping` and a `pong` process, each with its own
//! driver; and each with its driver invoked by its one measuring thread,
//! `--driver invoker`), and the spy on a live UDP stream (`--mode spy-udp`: a `local`
//! process and its sink, each with its own driver, every frame shown to
//! cross UDP to the sink). Each point's row must read back through the analysis's own
//! readers (md-dpdk/benches/cloud: `transport_points.read_points`,
//! `analyse.read_results`, `spikes.transport_max`), lose nothing, carry the
//! manifest the fairness checks compare, and show that the busy threads
//! allocated nothing while measuring.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "an integration test fails by panicking: unwrap, expect and panic! are its assertions (as md-dpdk/tests)"
)]
#![allow(clippy::similar_names, reason = "ping and pong name the two ends of each round trip")]

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

/// A base port with `n` (at most 20) free UDP ports from it on 127.0.0.1,
/// in 22000-31999: above the driver's wildcard ranges (20000-21999) and
/// below Linux's ephemeral range (32768-60999), which the kernel hands to
/// any socket bound to port 0 between this check and the driver's bind.
fn free_ports(n: u16) -> Res<u16> {
    let seed = u16::try_from(std::process::id() % 500)?;
    for k in 0..200u16 {
        let base = 22_000 + ((seed + k * 37) % 500) * 20;
        let held: Result<Vec<UdpSocket>, _> = (0..n).map(|i| UdpSocket::bind(("127.0.0.1", base + i))).collect();
        if held.is_ok() {
            return Ok(base);
        }
    }
    Err("no free run of UDP ports".into())
}

fn command(out: &Path, dir: &Path, args: &[&str]) -> Command {
    command_on("embedded", out, dir, args)
}

/// `aeron_lat` with `args` on a driver it runs itself (`--driver`
/// `embedded` or `invoker`).
fn command_on(driver: &str, out: &Path, dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aeron_lat"));
    cmd.args(args)
        .args(["--driver", driver, "--term-length", TERM, "--connect-timeout-s", "20"])
        .arg("--out")
        .arg(out)
        .arg("--aeron-dir")
        .arg(dir)
        .env("ERGON_HOST_CHECKS", "off")
        .env_remove("AERON_THREADING_MODE")
        .env_remove("RUSTERON_MEDIA_DRIVER_TRANSPORT")
        .env_remove("RUSTERON_MEDIA_DRIVER_DIR");
    cmd
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
    let o = command(&out, &dir, &args).output()?;
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

/// A manifest's setting `key` (the core's JSON: `"key": "value"`).
fn setting(json: &str, key: &str) -> Res<String> {
    let at = format!("\"{key}\": \"");
    let from = json.find(&at).ok_or_else(|| format!("no {key} in {json}"))? + at.len();
    let len = json[from..].find('"').ok_or("an unterminated value")?;
    Ok(json[from..from + len].to_owned())
}

fn number(json: &str, key: &str) -> Res<u64> {
    Ok(setting(json, key)?.parse()?)
}

/// The published spy arm's publication has no destination: its driver
/// sends no byte, whatever the spy reads (`driver.bytes-sent` 0).
#[test]
fn spy_round_trip_sends_nothing_on_the_wire() -> R {
    let base = scratch("spy-wire")?;
    let (out, dir) = (base.join("out"), base.join("aeron"));
    let rate = RATE.to_string();
    let args = [
        "--role",
        "local",
        "--round-trip",
        "--mode",
        "spy",
        "--rate",
        &rate,
        "--warmup-s",
        "1",
        "--duration-s",
        "1",
        "--label",
        "2b:aeron-spy",
    ];
    ok("spy", &command(&out, &dir, &args).output()?)?;
    let m = std::fs::read_to_string(out.join("manifest.json"))?;
    let (sent, echoes, bytes) = (
        number(&m, "sent")?,
        number(&m, "echoes_sent")?,
        number(&m, "driver.bytes-sent")?,
    );
    assert!(sent > 0 && echoes == sent, "{m}");
    assert_eq!(bytes, 0, "the spy arm's driver sent {bytes} bytes");
    std::fs::remove_dir_all(base)?;
    Ok(())
}

/// `--mode spy-udp` on one host: the sink (`--role pong`) on a driver of its
/// own, the measuring `local` on another, the two channels on loopback. The
/// row reads back as every other; the sink got every ping (and every echo)
/// in order, and the measuring end's driver sent at least every frame's
/// bytes: the spy read publications that really sent.
fn spy_udp(name: &str, round_trip: bool) -> R {
    let root = scratch(name)?;
    let base = free_ports(16)?;
    let (ping_ep, reply_ep) = (format!("127.0.0.1:{base}"), format!("127.0.0.1:{}", base + 8));
    let label = label("aeron-spy-udp", "spy", 8);
    let common = [
        "--mode",
        "spy-udp",
        "--endpoint",
        &ping_ep,
        "--reply",
        &reply_ep,
        "--streams",
        "8",
        "--label",
        &label,
    ];
    let sink_out = root.join("sink");
    let sink = command(
        &sink_out,
        &root.join("aeron-sink"),
        &[&["--role", "pong"][..], &common].concat(),
    )
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
    let rate = RATE.to_string();
    let out = root.join("local");
    let mut args = vec![
        "--role",
        "local",
        "--rate",
        &rate,
        "--warmup-s",
        "1",
        "--duration-s",
        "1",
    ];
    if round_trip {
        args.push("--round-trip");
    }
    let local = command(&out, &root.join("aeron-local"), &[&args[..], &common].concat()).output();
    // the sink ends by itself once the publications' images go
    let sink = wait(sink, Duration::from_secs(60))?;
    let row = ok("local", &local?)?;
    assert_eq!(ok("sink", &sink)?, "ready\n", "a sink prints one ready line and no row");
    assert!(!sink_out.join("results.csv").exists(), "a sink writes no row");
    let kind = if round_trip { "load" } else { "oneway" };
    assert!(row.starts_with(&format!("{kind},")), "{row}");
    check(&out, &expect(kind, "aeron-spy-udp", "udp", 138, 8))?;
    let m = std::fs::read_to_string(out.join("manifest.json"))?;
    let s = std::fs::read_to_string(sink_out.join("manifest.json"))?;
    assert!(
        setting(&m, "ping_pub_uri")?.contains(&format!("endpoint={ping_ep}|ssc=false|")),
        "{m}"
    );
    assert!(
        setting(&m, "ping_sub_uri")?.starts_with(&format!("aeron-spy:aeron:udp?endpoint={ping_ep}")),
        "{m}"
    );
    assert_eq!(setting(&m, "reliable")?, "true");
    assert_eq!(setting(&s, "thread_order")?, "sink");
    let sent = number(&m, "sent")?;
    let echoes = if round_trip { number(&m, "echoes_sent")? } else { 0 };
    assert!(sent > 0 && (!round_trip || echoes == sent), "{m}");
    assert_eq!(
        number(&s, "sink.ping_frames")?,
        sent,
        "every ping crossed UDP to the sink: {s}"
    );
    assert_eq!(
        number(&s, "sink.echo_frames")?,
        echoes,
        "every echo crossed UDP to the sink: {s}"
    );
    for k in ["sink.gaps", "sink.behind", "sink.refused"] {
        assert_eq!(number(&s, k)?, 0, "{k}: {s}");
    }
    assert_eq!(number(&s, "sink.images")?, if round_trip { 16 } else { 8 }, "{s}");
    let bytes = number(&m, "driver.bytes-sent")?;
    assert!(
        bytes >= (sent + echoes) * 96,
        "the measuring end's driver sent {bytes} bytes for {sent} + {echoes} frames"
    );
    assert!(number(&s, "driver.bytes-received")? >= (sent + echoes) * 96, "{s}");
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn spy_udp_round_trip_eight_streams_reaches_its_sink() -> R {
    spy_udp("spy-udp-rtt-8", true)
}

#[test]
fn spy_udp_one_way_eight_streams_reaches_its_sink() -> R {
    spy_udp("spy-udp-8", false)
}

/// Without its sink a spy-udp publication never connects: the spy alone
/// does not (`ssc=false`), so the point fails rather than measuring a
/// publication that sends nowhere.
#[test]
fn spy_udp_without_a_sink_never_connects() -> R {
    let root = scratch("spy-udp-nosink")?;
    let base = free_ports(2)?;
    let (ping_ep, reply_ep) = (format!("127.0.0.1:{base}"), format!("127.0.0.1:{}", base + 1));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aeron_lat"));
    cmd.args([
        "--role",
        "local",
        "--round-trip",
        "--mode",
        "spy-udp",
        "--rate",
        "100",
        "--label",
        "spy:x",
    ])
    .args([
        "--endpoint",
        &ping_ep,
        "--reply",
        &reply_ep,
        "--driver",
        "embedded",
        "--term-length",
        TERM,
    ])
    .args(["--connect-timeout-s", "3", "--warmup-s", "1", "--duration-s", "1"])
    .arg("--out")
    .arg(root.join("out"))
    .arg("--aeron-dir")
    .arg(root.join("aeron"))
    .env("ERGON_HOST_CHECKS", "off")
    .env_remove("AERON_THREADING_MODE")
    .env_remove("RUSTERON_MEDIA_DRIVER_TRANSPORT")
    .env_remove("RUSTERON_MEDIA_DRIVER_DIR");
    let o = cmd.output()?;
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(!o.status.success() && o.stdout.is_empty(), "{err}");
    assert!(err.contains("publications connected false"), "{err}");
    std::fs::remove_dir_all(root)?;
    Ok(())
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
/// each on a driver of its own (`driver`: on a thread of its own, or
/// invoked by the measuring thread), over UDP (reliable and not). The pong
/// says `ready` once its channels are added (the runner starts the ping on
/// it), ends by itself once the ping's images go, and writes its manifest
/// alone.
fn ping_pong(name: &str, reliable: &str, driver: &str) -> R {
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
    let mut pong = command_on(
        driver,
        &pong_out,
        &root.join("aeron-pong"),
        &[&["--role", "pong"][..], &common].concat(),
    );
    let pong = pong.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let rate = RATE.to_string();
    let ping_out = root.join("ping");
    let ping = command_on(
        driver,
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
    )
    .output();
    let pong = wait(pong, Duration::from_secs(60))?;
    let row = ok("ping", &ping?)?;
    assert_eq!(
        ok("pong", &pong)?,
        "ready\n",
        "a pong prints one ready line (its channels added) and no row"
    );
    assert!(!pong_out.join("results.csv").exists(), "a pong writes no row");
    let m = std::fs::read_to_string(pong_out.join("manifest.json"))?;
    // the pong's busy threads: its echo and, but invoked, the driver's three
    let (busy, threading) = if driver == "invoker" {
        (1, "INVOKER")
    } else {
        (4, "DEDICATED")
    };
    let pong_busy = format!("\"busy_threads\": \"{busy}\"");
    let pong_driver = format!("\"driver\": \"{driver}\"");
    let pong_threading = format!("\"driver_threading\": \"{threading}\"");
    for want in [
        "\"role\": \"pong\"",
        "\"hot_path_allocs\": \"0\"",
        "\"transport\": \"aeron-udp\"",
        &pong_busy,
        &pong_driver,
        &pong_threading,
    ] {
        assert!(m.contains(want), "{want}: {m}");
    }
    let pm = std::fs::read_to_string(ping_out.join("manifest.json"))?;
    assert!(pm.contains(&pong_driver) && pm.contains(&pong_threading), "{pm}");
    assert!(row.starts_with("load,"), "{row}");
    check(
        &ping_out,
        &Expect {
            kind: "load",
            transport: "aeron-udp",
            channel: "udp",
            wire_bytes: 138,
            busy_threads: busy,
            streams: 8,
        },
    )?;
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn udp_reliable_ping_pong_eight_streams() -> R {
    ping_pong("pp-rel-8", "true", "embedded")
}

#[test]
fn udp_unreliable_ping_pong_eight_streams() -> R {
    ping_pong("pp-unrel-8", "false", "embedded")
}

/// Each end's driver invoked by its one measuring thread: the ping's
/// thread sends, runs its driver and client conductor, and reads the
/// echoes; the pong's the same for the echo. No driver thread: one busy
/// thread an end, every round trip back, nothing allocated while measuring.
#[test]
fn udp_reliable_ping_pong_on_invoked_drivers() -> R {
    ping_pong("pp-rel-inv", "true", "invoker")
}

#[test]
fn udp_unreliable_ping_pong_on_invoked_drivers() -> R {
    ping_pong("pp-unrel-inv", "false", "invoker")
}

/// `--driver invoker` is one measuring thread a process over `--mode udp`:
/// `--role local` (two threads), a spy-udp sink, IPC and a driver busy
/// thread are refused before anything starts.
#[test]
fn an_invoked_driver_off_its_one_thread_is_refused() -> R {
    let root = scratch("inv-refused")?;
    let (out, dir) = (root.join("out"), root.join("aeron"));
    let eps = ["--endpoint", "127.0.0.1:39000", "--reply", "127.0.0.1:39100"];
    for (args, why) in [
        (
            &["--role", "local", "--round-trip", "--mode", "udp"][..],
            "--role ping or pong",
        ),
        (&["--role", "local", "--mode", "ipc"][..], "--mode udp"),
        (&["--role", "pong", "--mode", "spy-udp"][..], "--role ping or pong"),
        (
            &["--role", "ping", "--mode", "udp", "--driver-busy-threads", "3"][..],
            "no thread of its own",
        ),
    ] {
        let mut a = args.to_vec();
        if a.contains(&"udp") || a.contains(&"spy-udp") {
            a.extend(eps);
        }
        a.extend(["--rate", "10", "--label", "x:y"]);
        let o = command_on("invoker", &out, &dir, &a).output()?;
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(!o.status.success() && o.stdout.is_empty(), "{args:?}: {err}");
        assert!(err.contains(why), "{args:?}: {err}");
        assert!(!dir.join("cnc.dat").exists(), "no driver started: {args:?}");
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
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
        &["--role", "local", "--mode", "spy-udp"],
        &[
            "--role",
            "local",
            "--mode",
            "spy-udp",
            "--endpoint",
            "127.0.0.1:39000",
            "--reliable",
            "false",
        ],
        &[
            "--role",
            "ping",
            "--mode",
            "spy-udp",
            "--endpoint",
            "127.0.0.1:39000",
            "--reply",
            "127.0.0.1:39100",
        ],
        &["--role", "pong", "--mode", "spy-udp", "--endpoint", "127.0.0.1:39000"],
    ] {
        let o = command(&out, &dir, &[args, &base].concat()).output()?;
        assert!(!o.status.success(), "{args:?}");
        assert!(o.stdout.is_empty(), "no row: {args:?}");
        assert!(!out.join("results.csv").exists(), "{args:?}");
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn help_prints_the_usage() -> R {
    let o = Command::new(env!("CARGO_BIN_EXE_aeron_lat")).arg("--help").output()?;
    let said = String::from_utf8(o.stdout)?;
    assert!(o.status.success(), "{said}");
    for flag in [
        "--role",
        "--round-trip",
        "--mode",
        "--reliable",
        "--endpoint",
        "--reply",
        "--interface",
        "--aeron-dir",
        "--driver",
        "--driver-busy-threads",
        "--term-length",
        "--mtu",
        "--stream-id",
        "--connect-timeout-s",
        "--cpus",
    ] {
        assert!(said.contains(flag), "{flag}: {said}");
    }
    Ok(())
}

#[test]
fn a_udp_port_in_the_drivers_wildcard_ranges_is_refused() -> R {
    let root = scratch("wildcard")?;
    let (out, dir) = (root.join("out"), root.join("aeron"));
    for (flag, ep) in [("--endpoint", "127.0.0.1:21998"), ("--reply", "127.0.0.1:20500")] {
        let mut args = vec![
            "--role",
            "local",
            "--round-trip",
            "--mode",
            "udp",
            "--rate",
            "10",
            "--label",
            "x:y",
        ];
        args.extend(["--endpoint", "127.0.0.1:39000", "--reply", "127.0.0.1:39100"]);
        let at = args.iter().position(|a| *a == flag).ok_or("no flag")?;
        args[at + 1] = ep;
        let o = command(&out, &dir, &args).output()?;
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(!o.status.success() && o.stdout.is_empty(), "{flag} {ep}: {err}");
        assert!(err.contains("20000-21999"), "{flag} {ep}: {err}");
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
