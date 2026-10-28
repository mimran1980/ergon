//! `aeron_lat --load mixed` end to end on loopback: a pong process and a
//! ping process, each on a driver of its own (embedded DEDICATED, embedded
//! SHARED, or invoked by the hot stream's thread), nine streams (the hot
//! stream and one background stream of every class: steady, bursty,
//! idle-then-burst, two publishers). Every message must arrive at every
//! subscription of the pong (fan-out 1 to 4), in order, each first
//! subscription's echoed and every echo back at the ping; the sizes mixed
//! (some fragmented); the drivers' bytes at least the frames'; nothing
//! allocated by a busy thread while measuring. The run's manifests go
//! through fairness.py's own mixed-load checks (md-dpdk/benches/cloud), and
//! its row through the analysis's readers.
#![allow(clippy::similar_names, reason = "ping and pong name the two ends of each round trip")]

use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

type R = Result<(), Box<dyn std::error::Error>>;
type Res<T> = Result<T, Box<dyn std::error::Error>>;

const TERM: &str = "65536";
const STREAMS: u32 = 9;
const RATE: u64 = 3000;
const HOT: u64 = 500;

fn scratch(name: &str) -> Res<PathBuf> {
    let dir = std::env::temp_dir().join(format!("aeron-mixed-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn cloud() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benches/cloud")
}

/// A base port with `n` free UDP ports from it on 127.0.0.1, in 22000-31999
/// (above the driver's wildcard ranges, below Linux's ephemeral range).
fn free_ports(n: u16) -> Res<u16> {
    let seed = u16::try_from((std::process::id() + 250) % 500)?;
    for k in 0..200u16 {
        let base = 22_000 + ((seed + k * 37) % 500) * 20;
        let held: Result<Vec<UdpSocket>, _> = (0..n).map(|i| UdpSocket::bind(("127.0.0.1", base + i))).collect();
        if held.is_ok() {
            return Ok(base);
        }
    }
    Err("no free run of UDP ports".into())
}

/// `aeron_lat` with `args` on a driver it runs itself, its threading mode
/// `threading` (`--driver embedded`) or none (`invoker`).
fn command(driver: &str, threading: Option<&str>, out: &Path, dir: &Path, args: &[&str]) -> Command {
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
    if let Some(t) = threading {
        cmd.env("AERON_THREADING_MODE", t);
    }
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

#[allow(unsafe_code)]
fn term(child: &Child) -> Res<()> {
    let pid = libc::pid_t::try_from(child.id())?;
    // SAFETY: kill with a live child's PID and SIGTERM has no memory effects.
    if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
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

fn list(json: &str, key: &str) -> Res<Vec<u64>> {
    Ok(setting(json, key)?
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?)
}

/// fairness.py's mixed-load checks over the run as the runner would lay it
/// out (the row in results.csv, the ping's manifest as "binary", the
/// pong's as "pong"): none must fail; the analysis reads the row.
const CHECK: &str = r#"
import json, os, sys
cloud, ping, pong, busy, mtu = sys.argv[1:6]
sys.path.insert(0, cloud)
import fairness, transport_points as tp
line = open(os.path.join(ping, "results.csv")).read().strip()
cols = line.split(",")
assert len(cols) == tp.ROW_LEN - 1, (len(cols), line)
row = {c: tp.number(v) if c not in ("kind", "label", "point") else v for c, v in zip(tp.COLUMNS, ["1"] + cols)}
binary = json.load(open(os.path.join(ping, "manifest.json")))
pong_m = json.load(open(os.path.join(pong, "manifest.json")))
assert row["kind"] == "load" and row["rate"] == int(binary["settings"]["rate"]), row
assert row["busy_threads"] == int(busy) and row["lost"] == 0 and row["censored"] == 0, row
m = {"binary": binary, "pong": pong_m, "runner": {"arm_hosts": "two"}}
problems = fairness.mixed_count_problems("t", row, binary["settings"], pong_m["settings"])
problems += fairness.mixed_wire_problems("t", row, binary["settings"], pong_m["settings"], int(mtu))
problems += fairness.mixed_admin_problems("t", binary["settings"])
assert problems == [], problems
assert binary["settings"].get("mixed.admin") is not None and int(binary["settings"]["mixed.admin_total"]) > 0
s = binary["settings"]
assert int(s["bg.count"]) > 0 and int(s["all.count"]) == int(s["bg.count"]) + row["samples"], s
assert int(s["all.max_ns"]) >= max(int(s["bg.max_ns"]), row["max"]), s
print("ok")
"#;

/// One mixed point: its pong, then its ping, on drivers of `driver` (with
/// `threading`), the drivers' busy threads `driver_busy`; with `stage`, both
/// ends under `--stage-trace <stage>`.
#[allow(
    clippy::too_many_lines,
    reason = "one point end to end: its run, then every check of its manifests"
)]
fn mixed(name: &str, driver: &str, threading: Option<&str>, driver_busy: u32, stage: Option<&str>) -> R {
    let root = scratch(name)?;
    let base = free_ports(8)?;
    let (ping_ep, reply_ep) = (format!("127.0.0.1:{base}"), format!("127.0.0.1:{}", base + 4));
    let label = format!("mx:aeron-mix@STREAMS={STREAMS}");
    let (n, rate, hot, busy) = (
        STREAMS.to_string(),
        RATE.to_string(),
        HOT.to_string(),
        driver_busy.to_string(),
    );
    let mut common = vec![
        "--mode",
        "udp",
        "--reliable",
        "true",
        "--endpoint",
        &ping_ep,
        "--reply",
        &reply_ep,
        "--streams",
        &n,
        "--rate",
        &rate,
        "--load",
        "mixed",
        "--hot-rate",
        &hot,
        "--label",
        &label,
        "--driver-busy-threads",
        &busy,
    ];
    if let Some(every) = stage {
        common.extend(["--stage-trace", every]);
    }
    let pong_out = root.join("pong");
    let pong = command(
        driver,
        threading,
        &pong_out,
        &root.join("aeron-pong"),
        &[&["--role", "pong"][..], &common].concat(),
    )
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
    let ping_out = root.join("ping");
    let ping = command(
        driver,
        threading,
        &ping_out,
        &root.join("aeron-ping"),
        &[&["--role", "ping", "--warmup-s", "1", "--duration-s", "3"][..], &common].concat(),
    )
    .output();
    // the pong echoes until SIGTERM (the runner's), then drains and writes
    // its manifest
    std::thread::sleep(Duration::from_millis(300));
    let stopped = term(&pong);
    let pong_out_said = wait(pong, Duration::from_secs(30));
    let ping = ping?;
    stopped?;
    let pong_said = pong_out_said?;
    let row = ok("ping", &ping)?;
    assert_eq!(ok("pong", &pong_said)?, "ready\n", "one ready line, no row");
    assert!(row.starts_with("load,"), "{row}");
    let m = std::fs::read_to_string(ping_out.join("manifest.json"))?;
    let p = std::fs::read_to_string(pong_out.join("manifest.json"))?;
    for json in [&m, &p] {
        assert_eq!(setting(json, "load")?, "mixed");
        assert_eq!(setting(json, "mixed.profile")?, "mixed-v1");
        assert_eq!(setting(json, "thread_order")?, "hot,background");
        assert_eq!(number(json, "hot_path_allocs")?, 0, "a busy thread allocated: {json}");
        assert_eq!(number(json, "streams")?, u64::from(STREAMS));
        assert_eq!(number(json, "busy_threads")?, u64::from(2 + driver_busy));
    }
    // every class and fan-out of the table, and the background as scheduled
    assert_eq!(setting(&m, "mixed.classes")?, "SSSSBBIM");
    assert_eq!(list(&m, "mixed.fanout")?, [2, 3, 4, 1, 2, 3, 4, 1]);
    assert_eq!(list(&m, "mixed.publishers")?, [1, 1, 1, 1, 1, 1, 1, 2]);
    let (sent, scheduled) = (list(&m, "mixed.sent")?, list(&m, "mixed.scheduled")?);
    assert_eq!(sent, scheduled, "every background message sent: {m}");
    assert!(sent.iter().all(|&n| n > 0), "{sent:?}");
    // every subscription of the pong got every message, the first echoed it,
    // and every echo came back
    let recv = setting(&p, "mixed.recv")?;
    let fanout = list(&m, "mixed.fanout")?;
    for (i, subs) in recv.split(',').enumerate() {
        let got: Vec<u64> = subs.split('/').map(str::parse).collect::<Result<_, _>>()?;
        assert_eq!(got.len() as u64, fanout[i], "stream {}: {recv}", i + 1);
        assert!(
            got.iter().all(|&g| g == sent[i]),
            "stream {}: sent {}, read {got:?}",
            i + 1,
            sent[i]
        );
    }
    assert_eq!(list(&p, "mixed.echoed")?, sent);
    assert_eq!(list(&m, "mixed.echoes")?, sent);
    for k in ["mixed.recv_gaps", "mixed.recv_behind", "mixed.recv_refused"] {
        assert!(list(&p, k)?.iter().all(|&v| v == 0), "{k}: {p}");
    }
    for k in ["mixed.echo_gaps", "mixed.echo_behind"] {
        assert!(list(&m, k)?.iter().all(|&v| v == 0), "{k}: {m}");
    }
    // back-pressure is counted (a loaded test host's driver can stall past a
    // 64 KiB term's window), the measured window's within the whole's
    let (bp, bpm) = (list(&m, "mixed.bp")?, list(&m, "mixed.bp_measured")?);
    assert!(
        bp.len() == 8 && bp.iter().zip(&bpm).all(|(a, b)| b <= a),
        "{bp:?} {bpm:?}"
    );
    term_rotations(&m)?;
    // the sizes mixed: more frames than messages (4096 B goes in three), and
    // a steady stream's mean size above the smallest
    let (frames, total) = (number(&m, "mixed.frames_sent")?, number(&m, "mixed.sent_total")?);
    assert!(
        frames > total,
        "no message fragmented: {frames} frames for {total} messages"
    );
    let bytes = list(&m, "mixed.payload_bytes")?;
    assert!(bytes[0] > 64 * sent[0] && bytes[6] < 300 * sent[6], "{bytes:?}");
    assert_eq!(number(&p, "mixed.hot_echoes")?, number(&m, "mixed.hot_sent")?);
    let o = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(CHECK)
        .arg(cloud())
        .arg(&ping_out)
        .arg(&pong_out)
        .arg((2 + driver_busy).to_string())
        .arg("1408")
        .output()?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.status.success() && said == "ok\n", "{said}");
    if let Some(every) = stage {
        hot_stages(&ping_out, &pong_out, every, number(&m, "mixed.hot_sent")?)?;
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}

/// Aeron's `ADMIN_ACTION` offer results (a term rotation) counted apart from
/// back-pressure: each publication's one per term it filled, so a stream's
/// `mixed.admin` is its publications' terms filled (`mixed.position`, every
/// publication's final position, over the term length; a publication
/// whose last message ended its term exactly has not rotated yet), the
/// measured window's within the whole's; the hot stream's `admin_actions`
/// its 96-byte frames' terms filled.
fn term_rotations(m: &str) -> R {
    let term: u64 = TERM.parse()?;
    let (admin, measured) = (list(m, "mixed.admin")?, list(m, "mixed.admin_measured")?);
    let (pos, pubs) = (list(m, "mixed.position")?, list(m, "mixed.publishers")?);
    assert!(admin.len() == 8 && pos.len() == 8, "{admin:?} {pos:?}");
    for i in 0..8 {
        // a publication's terms filled: its position over the term length,
        // and a stream's publications each rotate their own
        let most = pos[i] / term;
        assert!(
            admin[i] <= most && admin[i] + pubs[i] >= most && measured[i] <= admin[i],
            "stream {}: {} admin actions, {} measured, position {} ({most} terms)",
            i + 1,
            admin[i],
            measured[i],
            pos[i]
        );
    }
    assert!(admin.iter().sum::<u64>() > 0, "no term rotated: {admin:?}");
    assert_eq!(number(m, "mixed.admin_total")?, admin.iter().sum::<u64>());
    let hot = number(m, "mixed.hot_sent")?;
    assert_eq!(number(m, "admin_actions")?, hot.saturating_sub(1) / (term / 96), "{m}");
    Ok(())
}

/// `--stage-trace` under `--load mixed`: each end's `stages-app.csv` holds
/// the hot stream's stamps alone (stream id 1001, its echoes 11001; none of
/// the background's), every sampled hot message offered on one end read on
/// the other, and each manifest says so.
fn hot_stages(ping_out: &Path, pong_out: &Path, every: &str, hot_sent: u64) -> R {
    // (stage, stream id) -> count
    type Counts = std::collections::BTreeMap<(String, u32), u64>;
    let n: u64 = every.parse()?;
    let read = |dir: &Path| -> Res<(Counts, String)> {
        let text = std::fs::read_to_string(dir.join("stages-app.csv"))?;
        let mut counts = std::collections::BTreeMap::new();
        let mut notes = String::new();
        for l in text.lines() {
            if l.starts_with('#') {
                notes.push_str(l);
                notes.push('\n');
                continue;
            }
            if l.starts_with("stage,") {
                continue;
            }
            let f: Vec<&str> = l.split(',').collect();
            assert_eq!(f.len(), 7, "{l}");
            *counts.entry((f[0].to_owned(), f[2].parse()?)).or_insert(0) += 1;
        }
        let m = std::fs::read_to_string(dir.join("manifest.json"))?;
        assert_eq!(setting(&m, "stage_trace")?, every, "{}", dir.display());
        Ok((counts, notes))
    };
    let ((ping, ping_notes), (pong, pong_notes)) = (read(ping_out)?, read(pong_out)?);
    for (notes, role) in [(&ping_notes, "role=ping"), (&pong_notes, "role=pong")] {
        for want in [role, "streams=1", "stream_id=1001", "dropped=0"] {
            assert!(notes.contains(want), "{want}: {notes}");
        }
    }
    // the hot messages are seq 0.. in order: every nth sampled
    let sampled = hot_sent.div_ceil(n);
    assert!(sampled > 0, "no hot message sampled");
    let want = |stage: &str, id: u32| ((stage.to_owned(), id), sampled);
    let ping_want: std::collections::BTreeMap<_, _> = [
        want("app-offer", 1001),
        want("app-offered", 1001),
        want("app-read", 11_001),
    ]
    .into();
    let pong_want: std::collections::BTreeMap<_, _> = [
        want("app-read", 1001),
        want("app-offer", 11_001),
        want("app-offered", 11_001),
    ]
    .into();
    assert_eq!(ping, ping_want, "the ping's stamps");
    assert_eq!(pong, pong_want, "the pong's stamps");
    Ok(())
}

#[test]
fn mixed_load_on_dedicated_embedded_drivers() -> R {
    mixed("emb", "embedded", None, 3, None)
}

#[test]
fn mixed_load_on_shared_embedded_drivers() -> R {
    mixed("shared", "embedded", Some("SHARED"), 1, None)
}

/// The hot stream's thread runs its driver: the background's own client
/// talks to that driver, whose sender and receiver the hot thread runs
/// for every stream.
#[test]
fn mixed_load_on_invoked_drivers() -> R {
    mixed("inv", "invoker", None, 0, None)
}

/// `--stage-trace` traces the hot stream alone, both ends.
#[test]
fn mixed_load_stage_traces_the_hot_stream() -> R {
    mixed("stage", "embedded", None, 3, Some("4"))
}

#[test]
fn a_mix_that_cannot_run_is_refused() -> R {
    let root = scratch("refused")?;
    let (out, dir) = (root.join("out"), root.join("aeron"));
    let eps = ["--endpoint", "127.0.0.1:39000", "--reply", "127.0.0.1:39100"];
    for (args, why) in [
        (
            &["--role", "ping", "--mode", "udp", "--load", "mixed", "--streams", "8"][..],
            "--hot-rate",
        ),
        (
            &[
                "--role",
                "ping",
                "--mode",
                "udp",
                "--load",
                "mixed",
                "--hot-rate",
                "100",
                "--streams",
                "1",
            ],
            "at least one background",
        ),
        (
            &[
                "--role",
                "ping",
                "--mode",
                "udp",
                "--load",
                "mixed",
                "--hot-rate",
                "1000",
                "--streams",
                "8",
            ],
            "under --rate",
        ),
        (
            &[
                "--role",
                "local",
                "--round-trip",
                "--mode",
                "udp",
                "--load",
                "mixed",
                "--hot-rate",
                "1",
            ],
            "cross-host",
        ),
        (
            &["--role", "ping", "--mode", "udp", "--hot-rate", "100"],
            "--load mixed's",
        ),
        (
            &["--role", "ping", "--mode", "udp", "--load", "zipf"],
            "uniform or mixed",
        ),
    ] {
        let mut a = args.to_vec();
        a.extend(eps);
        a.extend(["--rate", "1000", "--label", "x:y"]);
        let o = command("embedded", None, &out, &dir, &a).output()?;
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(!o.status.success() && o.stdout.is_empty(), "{args:?}: {err}");
        assert!(err.contains(why), "{args:?}: {err}");
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
