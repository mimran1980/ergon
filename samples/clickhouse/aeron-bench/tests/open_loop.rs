//! Exercise both bounded IPC queues through the actual ping and pong processes.
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn an_overdue_sender_drains_replies_before_queues_fill() -> Result<(), Box<dyn std::error::Error>> {
    drains_replies("spin", "4096")
}

#[test]
fn a_sleeping_sender_does_not_idle_after_successful_offers()
-> Result<(), Box<dyn std::error::Error>> {
    drains_replies("sleep", "32768")
}

fn drains_replies(idle: &str, count: &str) -> Result<(), Box<dyn std::error::Error>> {
    let directory =
        std::env::temp_dir().join(format!("aeron-open-loop-{}-{idle}", std::process::id()));
    let channel = "aeron:ipc?term-length=64k";
    let command = |role: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_aeron-bench"));
        command
            .env("AERON_DIR", &directory)
            .env("AERON_DIR_DELETE_ON_START", "true")
            .env("AERON_SHARED_IDLE_STRATEGY", "spin")
            .args([role, "--ping", channel, "--pong", channel]);
        command
    };
    let mut pong = Running(
        command("pong")
            .args(["--driver", "shared"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let connected = Instant::now();
    while !directory.join("cnc.dat").exists() {
        assert!(
            pong.0.try_wait()?.is_none(),
            "pong exited before driver startup"
        );
        assert!(
            connected.elapsed() < Duration::from_secs(10),
            "driver startup timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // Offered load far exceeds processing capacity. Both channels hold only
    // 64-KiB terms, while this run sends over 4 MiB in each direction.
    let mut ping = Running(
        command("ping")
            .args([
                "--rate",
                "1000000000",
                "--size",
                "1024",
                "--count",
                count,
                "--warmup",
                "512",
                "--idle",
                idle,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let started = Instant::now();
    let status = loop {
        if let Some(status) = ping.0.try_wait()? {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "open-loop sender stalled without draining its reply queue"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    use std::io::Read;
    let mut output = String::new();
    ping.0
        .stdout
        .take()
        .ok_or("missing stdout")?
        .read_to_string(&mut output)?;
    let mut errors = String::new();
    ping.0
        .stderr
        .take()
        .ok_or("missing stderr")?
        .read_to_string(&mut errors)?;
    assert!(status.success(), "ping failed: {errors}");
    assert!(
        output.contains(&format!("\"count\":{count},")),
        "wrong measured sample count: {output}"
    );
    drop(ping);
    drop(pong);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
