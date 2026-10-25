//! `mpsc_lat` end to end at a low rate: one way and round trip, 1 and 8
//! streams. Each point's row must read back through the analysis's own
//! readers (md-dpdk/benches/cloud: `transport_points.read_points`,
//! `analyse.read_results`, `spikes.transport_max`), lose nothing, carry the
//! manifest the fairness checks compare, and show that the busy threads
//! allocated nothing while measuring.

use ergon_latency_core::manifest::{CORE_BUILD_HASH, sha256_hex};
use ergon_latency_core::message::template;
use io_bench::transport::point::{CountingAlloc, thread_allocs};
use std::path::{Path, PathBuf};
use std::process::Command;

type R = Result<(), Box<dyn std::error::Error>>;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

const RATE: u64 = 2000;

fn scratch(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("mpsc-lat-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn cloud() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../md-dpdk/benches/cloud")
}

/// Runs `mpsc_lat` with `args` into `out`; its stdout.
fn mpsc_lat(out: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let o = Command::new(env!("CARGO_BIN_EXE_mpsc_lat"))
        .args(args)
        .args(["--out", out.to_str().ok_or("path")?])
        .output()?;
    if !o.status.success() {
        return Err(format!(
            "mpsc_lat {args:?}: {}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(o.stdout)?)
}

/// The analysis's check of one point, as the runner lays it out: the row in
/// `<run>/results.csv` (rep prefixed, the channel column `<transport>-r<rate>`),
/// the manifest and spikes file under `<run>/<set>/` by the point's tag.
const CHECK: &str = r#"
import json, os, shutil, sys
cloud, out, run, kind, streams, sha, core = sys.argv[1:8]
streams = int(streams)
sys.path.insert(0, cloud)
import analyse, spikes, transport_points as tp
line = open(os.path.join(out, "results.csv")).read().strip()
cols = line.split(",")
assert len(cols) == tp.ROW_LEN - 1, (len(cols), line)
label, rate = cols[1], int(cols[4])
cols[2] = f"{cols[2]}-r{rate}"
os.makedirs(os.path.join(run, "1"), exist_ok=True)
open(os.path.join(run, "results.csv"), "w").write("1," + ",".join(cols) + "\n")
tag = tp.tag_of(label, 64, 1, rate, kind)
shutil.copy(os.path.join(out, "manifest.json"), os.path.join(run, "1", f"manifest-{tag}.json"))
shutil.copy(os.path.join(out, f"spikes-{label}.csv"), os.path.join(run, "1", f"spikes-{tag}.csv"))
[p] = tp.read_points(run)
r = p.row
assert (p.set, p.arm, p.kind, p.streams, p.rate, p.size) == ("1", "mpsc", kind, streams, rate, 64), p
assert r["scheduled"] == r["sent"] == r["received"], r
assert r["lost"] == 0 and r["reordered"] == 0 and r["dup"] == 0, r
assert r["samples"] == rate * 2 and r["censored"] == 0, r
assert r["retrans"] is None and r["wire_bytes"] == 64 and r["busy_threads"] == 2, r
assert 0 < r["min"] <= r["p50"] <= r["p99"] <= r["p99.99"] <= r["max"], r
assert 0 < r["stream0_p50"] <= r["stream0_max"] <= r["max"], r
assert tp.row_flags(r) in ([], ["sender late"]), tp.row_flags(r)
m = p.manifest
assert m["payload_sha256"] == sha and m["payload_size"] == 64 and m["wire_bytes"] == 64, m
assert m["core_build_hash"] == core and m["arm"] == "mpsc", m
s = m["settings"]
assert s["transport"] == "mpsc" and s["kind"] == kind and s["streams"] == str(streams), s
assert s["hot_path_allocs"] == "0", s
assert s["max_ns"] == str(r["max"]) and s["lost"] == "0", s
loc = spikes.transport_max(os.path.join(run, "1"), tag, r["max"])
assert loc is not None and loc[2] == r["max"], (loc, r["max"])
if kind == "load":
    res = analyse.read_results(os.path.join(run, "results.csv"))
    [rep] = res.load[(label, 64, f"ipc-r{rate}")]
    assert rep[:6] == [float(r[c]) for c in ("p50", "p90", "p99", "p99.9", "p99.99", "max")], rep
print("ok")
"#;

fn check(out: &Path, kind: &str, streams: u32) -> R {
    let run = out.join("run");
    let o = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(CHECK)
        .arg(cloud())
        .arg(out)
        .arg(&run)
        .arg(kind)
        .arg(streams.to_string())
        .arg(sha256_hex(&template()))
        .arg(CORE_BUILD_HASH)
        .output()?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.status.success() && said == "ok\n", "{said}");
    Ok(())
}

fn point(name: &str, streams: u32, round_trip: bool) -> R {
    let out = scratch(name)?;
    let label = if streams == 1 {
        "1:mpsc".to_owned()
    } else {
        format!("1:mpsc@STREAMS={streams}")
    };
    let (rate, n) = (RATE.to_string(), streams.to_string());
    let mut args = vec![
        "--role",
        "local",
        "--rate",
        &rate,
        "--streams",
        &n,
        "--warmup-s",
        "1",
        "--duration-s",
        "2",
        "--label",
        &label,
    ];
    if round_trip {
        args.push("--round-trip");
    }
    let stdout = mpsc_lat(&out, &args)?;
    let row = std::fs::read_to_string(out.join("results.csv"))?;
    assert_eq!(stdout, row, "the printed row is the written one");
    for f in ["histogram", "histogram-send", "spikes", "stalls"] {
        assert!(out.join(format!("{f}-{label}.csv")).exists(), "{f}");
    }
    check(&out, if round_trip { "load" } else { "oneway" }, streams)?;
    std::fs::remove_dir_all(out)?;
    Ok(())
}

#[test]
fn one_way_one_stream() -> R {
    point("oneway-1", 1, false)
}

#[test]
fn one_way_eight_streams() -> R {
    point("oneway-8", 8, false)
}

#[test]
fn round_trip_one_stream() -> R {
    point("rtt-1", 1, true)
}

#[test]
fn round_trip_eight_streams() -> R {
    point("rtt-8", 8, true)
}

/// The count `hot_path_allocs` reports is real: the allocator the binary
/// installs sees an allocation on this thread.
#[test]
fn the_counting_allocator_counts_this_threads_allocations() {
    let before = thread_allocs();
    let v: Vec<u64> = std::hint::black_box(Vec::with_capacity(16));
    assert!(thread_allocs() > before);
    drop(v);
}

#[test]
fn a_role_other_than_local_a_wrong_cpu_count_and_an_unknown_flag_are_refused() -> R {
    let out = scratch("refused")?;
    for args in [
        &["--role", "ping", "--rate", "10", "--label", "x"][..],
        &[
            "--role", "local", "--rate", "10", "--label", "x", "--cpus", "1",
        ],
        &[
            "--role", "local", "--rate", "10", "--label", "x", "--bogus", "1",
        ],
        &[
            "--role", "local", "--rate", "10", "--label", "x", "--size", "32",
        ],
        &[
            "--role",
            "local",
            "--rate",
            "10",
            "--label",
            "x",
            "--capacity",
            "0",
        ],
    ] {
        let o = Command::new(env!("CARGO_BIN_EXE_mpsc_lat"))
            .args(args)
            .args(["--out", out.to_str().ok_or("path")?])
            .output()?;
        assert!(!o.status.success(), "{args:?}");
        assert!(o.stdout.is_empty(), "no row: {args:?}");
        assert!(!out.join("results.csv").exists(), "{args:?}");
    }
    std::fs::remove_dir_all(out)?;
    Ok(())
}
