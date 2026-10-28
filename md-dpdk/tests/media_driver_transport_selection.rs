//! Integration tests: standalone binary transport selection (plan §8).
//!
//! Each test spawns the real `media_driver` binary as a subprocess with a
//! chosen environment and asserts startup behaviour: which backend is logged,
//! whether a selected-DPDK failure exits nonzero, that a failure never falls
//! back to the default socket driver, that SIGTERM (kubelet's stop) shuts the
//! driver down cleanly, and that the socket backend keeps Aeron's receiver IO
//! vector capacity (16 is a `dpdk-ena` default only, set after the transport
//! installs, so no test without ENAs reaches it).
//!
//! Every subprocess gets its own `AERON_DIR` so its driver state cannot collide
//! with another media driver in the workspace, and tests are `#[serial]`
//! (file-locked) like the other media-driver integration tests. Selector
//! parsing runs on every platform; the install path (incomplete/native-failure
//! cases) is `#[cfg(feature = "dpdk")]` because that is where the native code
//! exists.
#![allow(
    missing_docs,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]

use serial_test::serial;

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Unique Aeron dir per test, in the platform temp dir (works on Linux and
/// macOS; `/dev/shm` does not exist on macOS).
fn aeron_dir(tag: &str) -> String {
    format!("{}/aeron-t8-{tag}", std::env::temp_dir().display())
}

fn spawn(envs: &[(&str, &str)]) -> Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_media_driver"));
    cmd.env("AERON_DIR_DELETE_ON_START", "true")
        .env("AERON_DIR_DELETE_ON_SHUTDOWN", "true");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn the media_driver binary")
}

/// Wait (bounded) for a process to exit; panic on timeout.
fn wait_exit(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().expect("try_wait failed") {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("media_driver subprocess did not exit within 30s");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Run the binary to completion, draining stdout and stderr, and return
/// (exit status, stdout, stderr).
fn spawn_and_capture(envs: &[(&str, &str)]) -> (std::process::ExitStatus, String, String) {
    let mut child = spawn(envs);
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut s = String::new();
        let _ = BufReader::new(stdout).read_to_string(&mut s);
        let _ = out_tx.send(s);
    });
    std::thread::spawn(move || {
        let mut s = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut s);
        let _ = err_tx.send(s);
    });

    let status = wait_exit(&mut child);
    let stdout = out_rx.recv().unwrap_or_default();
    let stderr = err_rx.recv().unwrap_or_default();
    (status, stdout, stderr)
}

/// Spawn the binary and wait for `needle` to appear on stdout (the backend log
/// line is printed before the driver starts). Returns the running child so the
/// caller can stop it.
fn spawn_and_wait_for_line(envs: &[(&str, &str)], needle: &str) -> Child {
    let mut child = spawn(envs);
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap_or_default();
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) if line.contains(needle) => return child,
            Ok(_) => {}
            Err(_) if Instant::now() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("timed out waiting for {needle:?} in media_driver output");
            }
            Err(_) => {}
        }
    }
}

fn send_sigint(child: &Child) {
    send_signal(child, "INT");
}

fn send_signal(child: &Child, signal: &str) {
    let pid = child.id().to_string();
    let status = Command::new("kill")
        .args([&format!("-{signal}"), &pid])
        .status()
        .unwrap_or_else(|e| panic!("failed to run kill -{signal}: {e}"));
    assert!(status.success(), "kill -{signal} {pid} failed");
}

/// Spawn the binary, wait for "media driver started", stop it with
/// `kill -<signal>` and wait for it to exit. Returns the exit status and all of
/// stdout, Aeron's configuration print included: the reader keeps draining the
/// pipe to the end, so the child never writes into a closed one.
fn run_and_stop(envs: &[(&str, &str)], signal: &str) -> (std::process::ExitStatus, String) {
    let mut child = spawn(envs);
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut all = String::new();
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap_or_default();
            all.push_str(&line);
            all.push('\n');
            let _ = tx.send(line);
        }
        all
    });

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) if line.contains("media driver started") => break,
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let status = wait_exit(&mut child);
                panic!("media_driver exited ({status:?}) before it started");
            }
            Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("timed out waiting for the media driver to start");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    send_signal(&child, signal);
    let status = wait_exit(&mut child);
    let stdout = reader.join().unwrap_or_default();
    (status, stdout)
}

/// The value Aeron's configuration print gives `name` (`    name=value`).
fn printed_setting<'a>(stdout: &'a str, name: &str) -> Option<&'a str> {
    stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix(name)?.strip_prefix('='))
}

// --- Default path: absent / `default` selector --------------------------------

#[serial]
#[test]
fn absent_selector_starts_socket_backend() {
    let dir = aeron_dir("absent");
    let envs = [("AERON_DIR", dir.as_str())];
    // Wait for the LAST startup stdout line ("media driver started") before
    // SIGINT. Waiting on "transport backend: socket" races the child's startup:
    // spawn_and_wait_for_line drops the channel receiver on return, which closes
    // the stdout pipe while the child is still writing "media driver started",
    // and the child's println! then dies with EPIPE (exit 101).
    let mut child = spawn_and_wait_for_line(&envs, "media driver started");
    send_sigint(&child);
    let status = wait_exit(&mut child);
    assert_eq!(status.code(), Some(0), "graceful SIGINT shutdown must exit 0");
}

#[serial]
#[test]
fn default_selector_starts_socket_backend() {
    let dir = aeron_dir("default");
    let envs = [
        ("AERON_DIR", dir.as_str()),
        ("RUSTERON_MEDIA_DRIVER_TRANSPORT", "default"),
    ];
    // Wait for the LAST startup stdout line ("media driver started") before
    // SIGINT. Waiting on "transport backend: socket" races the child's startup:
    // spawn_and_wait_for_line drops the channel receiver on return, which closes
    // the stdout pipe while the child is still writing "media driver started",
    // and the child's println! then dies with EPIPE (exit 101).
    let mut child = spawn_and_wait_for_line(&envs, "media driver started");
    send_sigint(&child);
    let status = wait_exit(&mut child);
    assert_eq!(status.code(), Some(0), "graceful SIGINT shutdown must exit 0");
}

// --- SIGTERM: how kubelet stops a pod ------------------------------------------

#[serial]
#[test]
fn sigterm_shuts_the_driver_down_gracefully() {
    let dir = aeron_dir("sigterm");
    // Without a SIGTERM handler the default action kills the process (and as a
    // container's PID 1 it ignores SIGTERM until kubelet's SIGKILL), so the
    // driver and the DPDK transport are never closed.
    let (status, _out) = run_and_stop(&[("AERON_DIR", dir.as_str())], "TERM");
    assert_eq!(
        status.code(),
        Some(0),
        "SIGTERM must stop the driver like SIGINT and exit 0: {status:?}"
    );
}

// --- CPU pinning in the shared threading modes ------------------------------------

/// The CPUs in a `Cpus_allowed_list:` line of a /proc status file.
#[cfg(target_os = "linux")]
fn allowed_cpus(status: &str) -> Vec<usize> {
    let list = status
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .expect("no Cpus_allowed_list in the status file")
        .trim();
    list.split(',')
        .flat_map(|part| {
            let (low, high) = part.split_once('-').unwrap_or((part, part));
            low.parse::<usize>().unwrap()..=high.parse::<usize>().unwrap()
        })
        .collect()
}

/// Each live thread of `pid`: (tid, name, the CPUs it may run on).
#[cfg(target_os = "linux")]
fn threads_of(pid: u32) -> Vec<(u32, String, Vec<usize>)> {
    let mut threads = Vec::new();
    for entry in std::fs::read_dir(format!("/proc/{pid}/task")).expect("the driver's /proc task dir") {
        let task = entry.expect("a /proc task entry").path();
        let tid = task.file_name().unwrap().to_string_lossy().parse().unwrap();
        // a thread that exited between the listing and the reads is not one
        if let (Ok(name), Ok(status)) = (
            std::fs::read_to_string(task.join("comm")),
            std::fs::read_to_string(task.join("status")),
        ) {
            threads.push((tid, name.trim().to_string(), allowed_cpus(&status)));
        }
    }
    threads
}

/// Aeron's affinity hook keys on its agents' role names, so the shared
/// threading modes' agents are pinned too: under SHARED_NETWORK the network
/// agent (`[sender, receiver]`) to the sender's CPU and the conductor (the
/// main thread, under the binary's manual main loop) to the conductor's; under
/// SHARED the one agent, the main thread, to the conductor's. Comparing the
/// threading modes on kernel UDP rests on it.
#[cfg(target_os = "linux")]
#[serial]
#[test]
fn shared_threading_modes_pin_their_agents_to_the_configured_cpus() {
    let cpus = allowed_cpus(&std::fs::read_to_string("/proc/self/status").unwrap());
    assert!(cpus.len() >= 2, "two CPUs are needed to tell the pins apart: {cpus:?}");
    let (conductor, sender) = (cpus[0], cpus[cpus.len() - 1]);
    let (conductor_cpu, sender_cpu) = (conductor.to_string(), sender.to_string());
    for mode in ["SHARED_NETWORK", "SHARED"] {
        let dir = aeron_dir(&format!("pin-{mode}"));
        let envs = [
            ("AERON_DIR", dir.as_str()),
            ("AERON_THREADING_MODE", mode),
            ("AERON_CONDUCTOR_CPU_AFFINITY", conductor_cpu.as_str()),
            ("AERON_SENDER_CPU_AFFINITY", sender_cpu.as_str()),
        ];
        let mut child = spawn_and_wait_for_line(&envs, "media driver started");
        let pid = child.id();
        // An agent thread pins itself as it starts, which may follow the line.
        let deadline = Instant::now() + Duration::from_secs(10);
        let (pinned, threads) = loop {
            let threads = threads_of(pid);
            let main = threads.iter().any(|(tid, _, on)| *tid == pid && on[..] == [conductor]);
            let network = mode == "SHARED"
                || threads
                    .iter()
                    .any(|(_, name, on)| name.starts_with("[sender, rec") && on[..] == [sender]);
            if (main && network) || Instant::now() > deadline {
                break (main && network, threads);
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        send_sigint(&child);
        let status = wait_exit(&mut child);
        assert!(
            pinned,
            "{mode}: the main thread on {conductor} and any network agent on {sender}: {threads:?}"
        );
        assert_eq!(status.code(), Some(0), "{mode}: graceful SIGINT shutdown must exit 0");
    }
}

/// CPUs given through the context's setters, not the environment, pin the
/// agents too. Since Aeron 1.54 the setters write `*_cpu_affinity_no` while
/// Aeron's hook pins from `*_cpu_affinity_resolved`, which only the affinity
/// validation fills, so `pin_agents_to_configured_cpus` must run it.
#[cfg(target_os = "linux")]
#[serial]
#[test]
fn setter_cpus_pin_the_dedicated_agents() {
    use rusteron_media_driver::bindings::aeron_threading_mode_t;
    use rusteron_media_driver::{AeronDriver, AeronDriverContext};

    let cpus = allowed_cpus(&std::fs::read_to_string("/proc/self/status").unwrap());
    assert!(cpus.len() >= 2, "two CPUs are needed to tell the pins apart: {cpus:?}");
    let (sender, receiver) = (cpus[0], cpus[cpus.len() - 1]);
    let dir = std::ffi::CString::new(aeron_dir("pin-setters")).unwrap();
    let context = AeronDriverContext::new().unwrap();
    context.set_dir(&dir).unwrap();
    context.set_dir_delete_on_start(true).unwrap();
    context.set_dir_delete_on_shutdown(true).unwrap();
    context
        .set_threading_mode(aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED)
        .unwrap();
    context.set_sender_cpu_affinity(i32::try_from(sender).unwrap()).unwrap();
    context
        .set_receiver_cpu_affinity(i32::try_from(receiver).unwrap())
        .unwrap();
    ergon_md_dpdk::pin_agents_to_configured_cpus(&context).unwrap();
    let driver = AeronDriver::launch_embedded_guard(context, false);
    let deadline = Instant::now() + Duration::from_secs(10);
    let (pinned, threads) = loop {
        let threads = threads_of(std::process::id());
        let on = |role: &str, cpu: usize| {
            threads
                .iter()
                .any(|(_, name, allowed)| name == role && allowed[..] == [cpu])
        };
        let pinned = (on("sender", sender) || on("aeron-md-snd", sender))
            && (on("receiver", receiver) || on("aeron-md-rcv", receiver));
        if pinned || Instant::now() > deadline {
            break (pinned, threads);
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    drop(driver);
    assert!(pinned, "sender on {sender} and receiver on {receiver}: {threads:?}");
}

// --- Receiver IO vectors: 16 only under dpdk-ena --------------------------------

#[serial]
#[test]
fn socket_backend_keeps_aerons_receiver_io_vector_capacity() {
    let dir = aeron_dir("iovec-default");
    let (status, out) = run_and_stop(&[("AERON_DIR", dir.as_str())], "INT");
    assert_eq!(status.code(), Some(0), "graceful SIGINT shutdown must exit 0");
    // No committed run separates 16 on the socket transport (md-dpdk/
    // BENCHMARKS.md), so the socket backend keeps Aeron's default of 4.
    assert_eq!(
        printed_setting(&out, "receiver_io_vector_capacity"),
        Some("4"),
        "the socket backend must keep Aeron's default:\n{out}"
    );
}

#[serial]
#[test]
fn explicit_receiver_io_vector_capacity_wins() {
    let dir = aeron_dir("iovec-explicit");
    let envs = [("AERON_DIR", dir.as_str()), ("AERON_RECEIVER_IO_VECTOR_CAPACITY", "8")];
    let (status, out) = run_and_stop(&envs, "INT");
    assert_eq!(status.code(), Some(0), "graceful SIGINT shutdown must exit 0");
    assert_eq!(
        printed_setting(&out, "receiver_io_vector_capacity"),
        Some("8"),
        "AERON_RECEIVER_IO_VECTOR_CAPACITY must win:\n{out}"
    );
}

// --- Unknown selector ---------------------------------------------------------

#[serial]
#[test]
fn unknown_selector_exits_nonzero_without_launching_a_driver() {
    let dir = aeron_dir("unknown");
    let envs = [
        ("AERON_DIR", dir.as_str()),
        ("RUSTERON_MEDIA_DRIVER_TRANSPORT", "vanilla"),
    ];
    let (status, _out, stderr) = spawn_and_capture(&envs);
    assert_ne!(status.code(), Some(0), "unknown selector must not launch a driver");
    assert!(
        stderr.contains("RUSTERON_MEDIA_DRIVER_TRANSPORT"),
        "stderr must name the selector variable: {stderr}"
    );
}

// --- Selected-DPDK failures (feature-gated: native code exists only with it) --

#[cfg(feature = "dpdk")]
#[serial]
#[test]
fn selected_dpdk_with_incomplete_env_exits_nonzero() {
    let dir = aeron_dir("incomplete");
    // Every required §6.2/§6.3 variable except the sender PCI, which is
    // deliberately absent so environment parsing fails deterministically.
    let mut envs = vec![
        ("AERON_DIR", dir.as_str()),
        ("RUSTERON_MEDIA_DRIVER_TRANSPORT", "dpdk-ena"),
        ("RUSTERON_DPDK_FILE_PREFIX", "rusteron-ena"),
        ("RUSTERON_DPDK_SENDER_IPV4_CIDR", "10.0.0.1/24"),
        ("RUSTERON_DPDK_SENDER_GATEWAY", "10.0.0.254"),
        ("RUSTERON_DPDK_RECEIVER_PCI", "0000:00:02.0"),
        ("RUSTERON_DPDK_RECEIVER_IPV4_CIDR", "10.0.1.1/24"),
        ("RUSTERON_DPDK_RECEIVER_GATEWAY", "10.0.1.254"),
    ];
    envs.retain(|(k, _)| *k != "RUSTERON_DPDK_SENDER_PCI");
    let (status, _out, stderr) = spawn_and_capture(&envs);
    assert_ne!(status.code(), Some(0), "missing DPDK env must not launch a driver");
    assert!(
        stderr.contains("RUSTERON_DPDK_SENDER_PCI"),
        "stderr must name the missing variable: {stderr}"
    );
}

#[cfg(feature = "dpdk")]
#[serial]
#[test]
fn selected_dpdk_native_failure_exits_nonzero() {
    let dir = aeron_dir("native");
    // Full §6.2/§6.3 env plus the §6.4 context env (distinct CPUs, spin idle,
    // disjoint wildcard port ranges), so validation passes and the native EAL
    // init runs — and fails in the container, which has no hugepages or ENAs.
    let envs = [
        ("AERON_DIR", dir.as_str()),
        ("RUSTERON_MEDIA_DRIVER_TRANSPORT", "dpdk-ena"),
        ("RUSTERON_DPDK_FILE_PREFIX", "rusteron-ena"),
        ("RUSTERON_DPDK_SENDER_PCI", "0000:00:01.0"),
        ("RUSTERON_DPDK_SENDER_IPV4_CIDR", "10.0.0.1/24"),
        ("RUSTERON_DPDK_SENDER_GATEWAY", "10.0.0.254"),
        ("RUSTERON_DPDK_RECEIVER_PCI", "0000:00:02.0"),
        ("RUSTERON_DPDK_RECEIVER_IPV4_CIDR", "10.0.1.1/24"),
        ("RUSTERON_DPDK_RECEIVER_GATEWAY", "10.0.1.254"),
        ("AERON_SENDER_CPU_AFFINITY", "1"),
        ("AERON_RECEIVER_CPU_AFFINITY", "2"),
        ("AERON_SENDER_IDLE_STRATEGY", "spin"),
        ("AERON_RECEIVER_IDLE_STRATEGY", "spin"),
        ("AERON_SENDER_WILDCARD_PORT_RANGE", "20000 20999"),
        ("AERON_RECEIVER_WILDCARD_PORT_RANGE", "21000 21999"),
        ("AERON_MTU_LENGTH", "1408"),
    ];
    let (status, _out, stderr) = spawn_and_capture(&envs);
    assert_ne!(
        status.code(),
        Some(0),
        "a selected-DPDK native failure must not launch the default driver"
    );
    assert!(!stderr.is_empty(), "stderr must report the native failure");
}

// --- dpdk-ena selected without the feature (runs everywhere without `dpdk`) ---

#[cfg(not(feature = "dpdk"))]
#[serial]
#[test]
fn selected_dpdk_without_feature_exits_nonzero() {
    let dir = aeron_dir("disabled");
    let envs = [
        ("AERON_DIR", dir.as_str()),
        ("RUSTERON_MEDIA_DRIVER_TRANSPORT", "dpdk-ena"),
    ];
    let (status, _out, stderr) = spawn_and_capture(&envs);
    assert_ne!(
        status.code(),
        Some(0),
        "dpdk-ena without the feature must not launch a driver"
    );
    // Rust's `main` Err path prints the Debug form (`Error: {err:?}`).
    assert!(
        stderr.contains("FeatureDisabled"),
        "stderr must name the missing feature: {stderr}"
    );
}
