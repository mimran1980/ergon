//! `aeron_lat --stage-trace N`: the application's end of md-dpdk's stage
//! trace, on drivers the binaries embed (the kernel's UDP on 127.0.0.1).
//!
//! A ping and a pong each stamp every Nth message (by seq) at its offer's
//! start and return and at its read, into `<out>/stages-app.csv`, and still
//! allocate nothing while measuring. With the `dpdk` feature their embedded
//! drivers also trace (`RUSTERON_DPDK_STAGE_TRACE`: the socket transport's
//! wrapped bindings), and md-dpdk/benches/cloud/transport_stages.py joins the
//! four files into a breakdown whose host and wire times add up to the round
//! trip. Without the feature a driver asked to trace refuses to start.
//! `--stage-trace` that is no power of two, or on an end that does not both
//! offer and read, is refused.

use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

type R = Result<(), Box<dyn std::error::Error>>;
type Res<T> = Result<T, Box<dyn std::error::Error>>;
/// A stage row's stage, stream id, seq and ring.
type Row = (String, i64, u64, u32);

fn scratch(name: &str) -> Res<PathBuf> {
    let dir = std::env::temp_dir().join(format!("aeron-lat-stages-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// A base port with `n` free UDP ports from it on 127.0.0.1, clear of the
/// driver's wildcard ranges and Linux's ephemeral range.
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

fn command(out: &Path, dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aeron_lat"));
    cmd.args(args)
        .args([
            "--driver",
            "embedded",
            "--term-length",
            "65536",
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
        .env_remove("RUSTERON_MEDIA_DRIVER_DIR")
        .env_remove("RUSTERON_DPDK_STAGE_TRACE")
        .env_remove("RUSTERON_DPDK_STAGE_TRACE_FILE");
    cmd
}

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

/// (stage, stream id, seq, ring) of each row, and the `#` lines.
fn rows(path: &Path) -> Res<(Vec<Row>, Vec<String>)> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    let mut notes = Vec::new();
    for l in text.lines() {
        if l.starts_with('#') {
            notes.push(l.to_owned());
            continue;
        }
        if l.starts_with("stage,") {
            assert_eq!(l, "stage,tsc,stream_id,seq,term_offset,value,ring");
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        assert_eq!(f.len(), 7, "{l}");
        out.push((f[0].to_owned(), f[2].parse()?, f[3].parse()?, f[6].parse()?));
    }
    Ok((out, notes))
}

/// A ping and a pong over UDP, 2 streams at 1000 msg/s for 1 s of warm-up
/// and 1 of measuring, both `--stage-trace 4`; with `driver_trace`, each
/// embedded driver traced too, into `<root>/driver-<end>.csv`.
fn ping_pong(root: &Path, driver_trace: bool) -> Res<(Output, Output)> {
    let base = free_ports(8)?;
    let (ep, rp) = (format!("127.0.0.1:{base}"), format!("127.0.0.1:{}", base + 4));
    let common = [
        "--mode",
        "udp",
        "--reliable",
        "true",
        "--endpoint",
        &ep,
        "--reply",
        &rp,
        "--streams",
        "2",
        "--label",
        "t1:aeron-udp@STREAMS=2",
        "--stage-trace",
        "4",
        "--rate",
        "1000",
        "--warmup-s",
        "1",
        "--duration-s",
        "1",
    ];
    let traced = |cmd: &mut Command, end: &str| {
        if driver_trace {
            cmd.env("RUSTERON_DPDK_STAGE_TRACE", "4")
                .env("RUSTERON_DPDK_STAGE_TRACE_FILE", root.join(format!("driver-{end}.csv")));
        }
    };
    let mut pong = command(
        &root.join("pong"),
        &root.join("aeron-pong"),
        &[&["--role", "pong"][..], &common].concat(),
    );
    traced(&mut pong, "pong");
    let pong = pong.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let mut ping = command(
        &root.join("ping"),
        &root.join("aeron-ping"),
        &[&["--role", "ping"][..], &common].concat(),
    );
    traced(&mut ping, "ping");
    let ping = ping.output()?;
    let pong = wait(pong, Duration::from_secs(60))?;
    Ok((ping, pong))
}

#[test]
fn a_ping_and_a_pong_stamp_every_nth_message() -> R {
    let root = scratch("app")?;
    let (ping, pong) = ping_pong(&root, false)?;
    ok("ping", &ping)?;
    ok("pong", &pong)?;
    let (pr, notes) = rows(&root.join("ping/stages-app.csv"))?;
    let head = notes.join("\n");
    for want in [
        "role=ping",
        "every=4",
        "streams=2",
        "stream_id=1001",
        "echo_offset=10000",
        "dropped=0",
    ] {
        assert!(head.contains(want), "{want}: {head}");
    }
    // 2000 messages a run, 1000 a stream (seq 0..999): seq % 4 == 0 is 250 a stream
    let count = |r: &[Row], stage: &str, stream: i64| r.iter().filter(|x| x.0 == stage && x.1 == stream).count();
    for stream in [1001, 1002] {
        assert_eq!(count(&pr, "app-offer", stream), 250, "ping offers on {stream}");
        assert_eq!(count(&pr, "app-offered", stream), 250);
        assert_eq!(
            count(&pr, "app-read", stream + 10_000),
            250,
            "echoes read on {}",
            stream + 10_000
        );
    }
    assert!(pr.iter().all(|x| x.2 % 4 == 0), "only every 4th seq");
    assert!(
        pr.iter()
            .all(|x| (x.0 == "app-read") == (x.3 == 1) && (x.0 != "app-read") == (x.3 == 0)),
        "offers ring 0, reads ring 1"
    );
    let (qr, notes) = rows(&root.join("pong/stages-app.csv"))?;
    assert!(notes.join("\n").contains("role=pong"), "{notes:?}");
    for stream in [1001, 1002] {
        assert_eq!(count(&qr, "app-read", stream), 250, "pings read on {stream}");
        assert_eq!(count(&qr, "app-offer", stream + 10_000), 250, "echoes offered");
    }
    for (end, manifest) in [("ping", "ping/manifest.json"), ("pong", "pong/manifest.json")] {
        let m = std::fs::read_to_string(root.join(manifest))?;
        assert!(m.contains("\"stage_trace\": \"4\""), "{end}: {m}");
        assert!(
            m.contains("\"hot_path_allocs\": \"0\""),
            "{end} allocated while measuring: {m}"
        );
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}

/// The embedded drivers traced as well (the socket transport's wrapped
/// bindings), the four files joined: every measured sampled message
/// complete, each host's time and the wire adding up to the round trip.
#[cfg(all(feature = "dpdk", target_os = "linux"))]
#[test]
fn the_drivers_stamps_join_the_apps_into_a_breakdown() -> R {
    let root = scratch("joined")?;
    let (ping, pong) = ping_pong(&root, true)?;
    ok("ping", &ping)?;
    ok("pong", &pong)?;
    let (dr, notes) = rows(&root.join("driver-ping.csv"))?;
    assert!(notes.iter().any(|n| n.contains("transport=kernel")), "{notes:?}");
    assert!(
        notes.iter().any(|n| n.starts_with("# clock close ")),
        "closed with the driver: {notes:?}"
    );
    for stage in [
        "tx-send",
        "tx-call",
        "tx-done",
        "rx-burst",
        "rx-dispatch",
        "rx-inserted",
    ] {
        assert!(dr.iter().any(|x| x.0 == stage), "{stage}");
    }
    let cloud = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benches/cloud/transport_stages.py");
    let o = Command::new("python3")
        .arg("-I")
        .arg(cloud)
        .arg("join")
        .args([
            root.join("ping/stages-app.csv"),
            root.join("pong/stages-app.csv"),
            root.join("driver-ping.csv"),
            root.join("driver-pong.csv"),
        ])
        .output()?;
    let table = ok("transport_stages.py join", &o)?;
    // the breakdown, for a reader of the test's output (--nocapture)
    println!("{table}");
    assert!(table.contains("250 measured sampled messages"), "{table}");
    for stage in [
        "a app->sender",
        "a tx burst",
        "b rx->dispatch",
        "b insert",
        "b insert->app",
        "b turnaround",
        "b tx burst",
        "a insert->app",
        "host a",
        "host b",
        "wire",
        "round trip",
    ] {
        assert!(table.contains(&format!("| {stage} | 250 |")), "{stage}: {table}");
    }
    assert!(
        !table.contains("messages without"),
        "every sampled message complete: {table}"
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}

/// Without the `dpdk` feature the stage trace's native half is not there: a
/// driver asked to trace refuses to start rather than run untraced.
#[cfg(not(feature = "dpdk"))]
#[test]
fn a_driver_asked_to_trace_without_the_feature_refuses() -> R {
    let root = scratch("nofeature")?;
    let o = command(
        &root.join("out"),
        &root.join("aeron"),
        &[
            "--role",
            "local",
            "--round-trip",
            "--mode",
            "ipc",
            "--rate",
            "10",
            "--label",
            "t:x",
        ],
    )
    .env("RUSTERON_DPDK_STAGE_TRACE", "4")
    .env("RUSTERON_DPDK_STAGE_TRACE_FILE", root.join("d.csv"))
    .output()?;
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(!o.status.success(), "{err}");
    assert!(err.contains("built with the `dpdk` feature only"), "{err}");
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn a_local_round_trip_stamps_both_threads() -> R {
    let root = scratch("local")?;
    let o = command(
        &root.join("out"),
        &root.join("aeron"),
        &[
            "--role",
            "local",
            "--round-trip",
            "--mode",
            "ipc",
            "--rate",
            "1000",
            "--warmup-s",
            "1",
            "--duration-s",
            "1",
            "--label",
            "1:aeron-ipc",
            "--stage-trace",
            "2",
        ],
    )
    .output()?;
    ok("local", &o)?;
    let (r, _) = rows(&root.join("out/stages-app.csv"))?;
    let n = |stage: &str, ring: u32| r.iter().filter(|x| x.0 == stage && x.3 == ring).count();
    // the ping thread's rings 0 and 1, the echo thread's 2 and 3
    assert_eq!((n("app-offer", 0), n("app-read", 1)), (1000, 1000));
    assert_eq!((n("app-read", 3), n("app-offer", 2)), (1000, 1000));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn a_stage_trace_that_cannot_be_kept_is_refused() -> R {
    let root = scratch("refused")?;
    for (args, why) in [
        (
            &["--role", "local", "--round-trip", "--stage-trace", "3"][..],
            "a power of two",
        ),
        (
            &["--role", "local", "--stage-trace", "4"][..],
            "an end that offers and reads",
        ),
    ] {
        let o = command(
            &root.join("out"),
            &root.join("aeron"),
            &[args, &["--mode", "ipc", "--rate", "10", "--label", "t:x"]].concat(),
        )
        .output()?;
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(!o.status.success() && err.contains(why), "{args:?}: {err}");
        assert!(!root.join("aeron/cnc.dat").exists(), "no driver started: {args:?}");
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
