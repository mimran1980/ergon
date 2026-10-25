//! Coordinated-omission correction in `load`, the open-loop rate generator.
//!
//! A closed-loop client (one in flight, the next sent only when the echo is
//! back) hides every stall of the far end: nothing is in flight during it, so
//! the stall is absent from the histogram. An open-loop generator sends on its
//! schedule whatever the replies do, and stamps each message with when it was
//! *due*, not when the offer finally went through, so a message offered late
//! still counts the wait against the path. That is the correction: a stall of
//! the far end appears in the latency of the messages that were due during it.
//!
//! Here the far end is frozen with SIGSTOP for a while, after warm-up, so the
//! generator is back-pressured and falls behind its schedule. When it is let
//! go, the backlog goes out stamped with the times it was due. The run's
//! `max send lag` is the longest a measured message waited past its due time
//! to be offered, and that message's latency is the wait plus the round trip,
//! so the histogram's max must exceed the max send lag. Measured from the
//! offer instead, the same run would report only the round trip and the echo
//! queue — milliseconds, not the lag.

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// A free UDP port on 127.0.0.1 (released for the client to bind).
fn endpoint() -> String {
    let port = std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .expect("a free UDP port");
    format!("127.0.0.1:{port}")
}

/// A fresh Aeron directory for one client's embedded driver.
fn aeron_dir(tag: &str) -> String {
    let dir = format!(
        "{}/aeron-harness-omission-{tag}-{}",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A client process, its output collected as it runs.
struct Running {
    what: String,
    child: Child,
    stdout: std::thread::JoinHandle<String>,
    stderr: std::thread::JoinHandle<String>,
}

/// What a client printed, and how it exited.
struct Finished {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

/// Start `bin` with `args` on an embedded driver (`HARNESS_DRIVER=invoker`)
/// at `dir`, a 64 KiB term so a 32 KiB window back-pressures a publication
/// within a few hundred messages of a frozen reader.
fn start(bin: &str, args: &[&str], dir: &str) -> Running {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .env("AERON_DIR", dir)
        .env("AERON_DIR_DELETE_ON_START", "true")
        .env("AERON_TERM_BUFFER_LENGTH", "65536")
        .env("HARNESS_DRIVER", "invoker")
        .env("LABEL", "omission-test")
        .env("RTT_SECONDS", "10");
    for var in [
        "PUB_CHANNEL_PARAMS",
        "SUB_CHANNEL_PARAMS",
        "LOOP_STALLS_DIR",
        "LOAD_SPIKES",
        "RTT_HISTOGRAM",
        "RUSTERON_MEDIA_DRIVER_TRANSPORT",
    ] {
        cmd.env_remove(var);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawning {bin}: {e}"));
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    Running {
        what: format!("{} {}", bin.rsplit('/').next().unwrap(), args.join(" ")),
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
    /// Freezes it (and the driver its thread runs), so it stops reading and
    /// its echoes stop: the far end of an open-loop run that went away for a
    /// while, without this test having to build one.
    fn freeze(&self) {
        signal(&self.child, "-STOP");
    }

    /// Lets it run again.
    fn thaw(&self) {
        signal(&self.child, "-CONT");
    }

    /// Waits up to `secs` for it to exit, killing it (by its PID) past that.
    fn finish(mut self, secs: u64) -> Finished {
        let deadline = Instant::now() + Duration::from_secs(secs);
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let stderr = self.stderr.join().unwrap_or_default();
                panic!("{} did not exit within {secs} s; stderr:\n{stderr}", self.what);
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        Finished {
            status,
            stdout: self.stdout.join().unwrap_or_default(),
            stderr: self.stderr.join().unwrap_or_default(),
        }
    }

    /// SIGTERM, which `rtt xpong` takes as the end of its run.
    fn terminate(self, secs: u64) -> Finished {
        signal(&self.child, "-TERM");
        self.finish(secs)
    }
}

fn signal(child: &Child, what: &str) {
    let pid = child.id().to_string();
    let sent = Command::new("kill").args([what, &pid]).status().expect("kill");
    assert!(sent.success(), "kill {what} {pid}");
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

    /// Its one result line with `prefix`, split on commas.
    fn result(&self, prefix: &str) -> Vec<String> {
        let lines: Vec<&str> = self.stdout.lines().collect();
        assert!(
            lines.len() == 1 && lines[0].starts_with(prefix),
            "expected one {prefix:?} line on stdout, got:\n{}",
            self.stdout
        );
        lines[0].split(',').map(str::to_string).collect()
    }
}

/// The open-loop generator's result line: `load,label,udp,size,rate,burst,
/// p50,p90,p99,p99.9,p99.99,max,mean,samples,sent,received,back-pressured,
/// achieved,max send lag`. Indices 11 (max), 14 (sent), 15 (received), 16
/// (back-pressured) and 18 (max send lag) are the ones asserted here.
#[test]
fn a_message_offered_late_still_counts_the_wait_in_its_latency() {
    let load = env!("CARGO_BIN_EXE_load");
    let rtt = env!("CARGO_BIN_EXE_rtt");
    let (ping, pong) = (endpoint(), endpoint());
    let (near, far) = (aeron_dir("near"), aeron_dir("far"));

    let echo = start(rtt, &["xpong", &ping, &pong, "-"], &far);
    // three seconds of measured run after the warm-up second, so a freeze at
    // wall-clock three seconds lands well inside the measured window even
    // though the schedule only starts once the path is up and the clock is
    // calibrated
    let offered = start(load, &["xping", &ping, &pong, "5000", "3", "32", "-"], &near);

    // Past warm-up (the first second, not recorded), freeze the echo for
    // 400 ms. Its window stops advancing, the generator's publication fills
    // and back-pressures, and the messages due during the freeze pile up.
    std::thread::sleep(Duration::from_millis(3000));
    let frozen_at = Instant::now();
    echo.freeze();
    std::thread::sleep(Duration::from_millis(400));
    echo.thaw();
    let frozen = frozen_at.elapsed();

    let offered = offered.finish(60);
    let echoed = echo.terminate(30);
    offered.assert_ok("load xping");
    echoed.assert_ok("rtt xpong");

    let fields = offered.result("load,omission-test,udp,32,5000,1,");
    let (max, sent, received, backpressured, max_lag): (u64, u64, u64, u64, u64) = (
        fields[11].parse().expect("max"),
        fields[14].parse().expect("sent"),
        fields[15].parse().expect("received"),
        fields[16].parse().expect("back-pressured"),
        fields[18].parse().expect("max send lag"),
    );
    eprintln!("omission: froze the echo {frozen:?}; {fields:?}");

    // The freeze did what it was for: the generator fell behind, and every
    // message of the schedule still went out and came back, so the histogram
    // covers the stall rather than the run being reported lossy. The lag has
    // to be long enough to be unmistakable against a local round trip (tens
    // of microseconds): a quarter of the freeze is the window filling part of
    // the way through it, and 20 ms is already two orders above a round trip.
    assert!(
        backpressured > 0 && max_lag > 0,
        "never back-pressured, so nothing was late: the freeze of {frozen:?} did not stall the path: {fields:?}"
    );
    assert!(
        max_lag >= frozen.as_nanos() as u64 / 4 && max_lag > 20_000_000,
        "the max send lag ({max_lag} ns) is far under the {frozen:?} freeze: the generator did not fall behind it: {fields:?}"
    );
    assert!(sent == received && sent == 20_000, "messages went missing: {fields:?}");

    // The correction: the wait to be offered is inside the latency, so the
    // slowest round trip is slower than the longest any message waited to be
    // sent at all. Measured from the offer, max would be the round trip and
    // the echo queue — milliseconds — against this lag.
    assert!(
        max > max_lag,
        "latency was not measured from the due time: max {max} ns does not exceed the max send lag {max_lag} ns. \
         Measured from the offer, the stall would be invisible: {fields:?}"
    );

    for dir in [near, far] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// The generator keeps its schedule whatever the replies do: with the echo
/// frozen it still offers every message that comes due (late), so `achieved`
/// stays near the offered rate and `sent` is the whole schedule. A closed
/// loop would have sent nothing during the freeze and reported a shorter run.
#[test]
fn the_schedule_runs_on_whatever_the_replies_do() {
    let load = env!("CARGO_BIN_EXE_load");
    let rtt = env!("CARGO_BIN_EXE_rtt");
    let (ping, pong) = (endpoint(), endpoint());
    let (near, far) = (aeron_dir("sched-near"), aeron_dir("sched-far"));

    let echo = start(rtt, &["xpong", &ping, &pong, "-"], &far);
    let offered = start(load, &["xping", &ping, &pong, "2000", "2", "32", "-"], &near);
    std::thread::sleep(Duration::from_millis(200));
    echo.freeze();
    std::thread::sleep(Duration::from_millis(300));
    echo.thaw();

    let offered = offered.finish(60);
    let echoed = echo.terminate(30);
    offered.assert_ok("load xping");
    echoed.assert_ok("rtt xpong");

    let fields = offered.result("load,omission-test,udp,32,2000,1,");
    let (sent, received, achieved): (u64, u64, f64) = (
        fields[14].parse().expect("sent"),
        fields[15].parse().expect("received"),
        fields[17].parse().expect("achieved"),
    );
    // the whole schedule (a warm-up second and two), not a run cut short by
    // the freeze, and the rate actually offered is the one asked for
    assert!(sent == 6000, "the schedule was not completed: {fields:?}");
    assert!(sent == received, "messages went missing: {fields:?}");
    assert!(
        (achieved - 2000.0).abs() < 100.0,
        "achieved {achieved} is not the offered rate: {fields:?}"
    );

    for dir in [near, far] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// A message whose echo never came is in the percentiles, not left out of
/// them. The echo is frozen for the rest of the run past warm-up, so the
/// last part of the schedule is never echoed (or never offered, once the
/// window is full). Each such measured message is recorded at what it is
/// known to be at least, the run's end minus its due time, and counted as
/// censored: `samples` is more than the echoes, and the max is the stall.
/// Left out, the run would report only the round trips that got through.
#[test]
fn a_message_whose_echo_never_came_is_in_the_percentiles() {
    let load = env!("CARGO_BIN_EXE_load");
    let rtt = env!("CARGO_BIN_EXE_rtt");
    let (ping, pong) = (endpoint(), endpoint());
    let (near, far) = (aeron_dir("lost-near"), aeron_dir("lost-far"));

    let echo = start(rtt, &["xpong", &ping, &pong, "-"], &far);
    // a warm-up second and two measured ones; the freeze lands in the
    // measured part and lasts until the generator has given up
    let offered = start(load, &["xping", &ping, &pong, "2000", "2", "32", "-"], &near);
    std::thread::sleep(Duration::from_millis(3000));
    echo.freeze();
    let offered = offered.finish(60);
    echo.thaw();
    let _ = echo.terminate(30);
    offered.assert_ok("load xping");

    let fields = offered.result("load,omission-test,udp,32,2000,1,");
    let field = |i: usize, what: &str| -> u64 { fields[i].parse().unwrap_or_else(|_| panic!("{what}: {fields:?}")) };
    let (max, samples, sent, received, scheduled, censored) = (
        field(11, "max"),
        field(13, "samples"),
        field(14, "sent"),
        field(15, "received"),
        field(19, "scheduled"),
        field(21, "censored"),
    );
    eprintln!("lost: {fields:?}");
    let warmup = 2000;
    assert!(
        received < scheduled,
        "the freeze lost no echo, so this proves nothing: {fields:?}"
    );
    // the freeze may land in warm-up on a slow start: then no echo is
    // measured, and every measured message is owed without one
    let echoed = received.saturating_sub(warmup);
    assert!(
        samples > echoed,
        "the percentiles hold only the {echoed} echoes: the {} messages owed without one are missing: {fields:?}",
        scheduled - received
    );
    assert!(
        censored >= samples - echoed,
        "a message without an echo is not counted censored: {fields:?}"
    );
    assert!(sent <= scheduled);
    // each was waited for at least until the run gave up on it: the max is
    // the stall, not the round trips that got through
    assert!(max >= 1_000_000_000, "max {max} ns is not the stall: {fields:?}");

    for dir in [near, far] {
        let _ = std::fs::remove_dir_all(dir);
    }
}
