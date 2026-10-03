//! Replay must finish even when the archive contains term-padding frames.
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

struct Processes {
    children: Vec<Child>,
    directory: PathBuf,
}

impl Drop for Processes {
    fn drop(&mut self) {
        for child in self.children.iter_mut().rev() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn finishes(
    child: &mut Child,
    description: &str,
) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "{description} did not finish within 15 seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn replay_finishes_after_consuming_a_recording_with_term_padding()
-> Result<(), Box<dyn std::error::Error>> {
    let jar = std::env::var_os("AERON_TEST_JAR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/aeron-all-1.52.2.jar"),
        PathBuf::from,
    );
    assert!(
        jar.is_file(),
        "run the sample's just test to fetch the archive test dependency"
    );
    let directory = std::env::temp_dir().join(format!("aeron-replay-test-{}", std::process::id()));
    std::fs::create_dir(&directory)?;
    let driver_dir = directory.join("driver");
    let mut processes = Processes {
        children: Vec::new(),
        directory,
    };
    let archive_output = std::fs::File::create(processes.directory.join("archive.log"))?;
    processes.children.push(
        Command::new("java")
            .args([
                "-Xms32m",
                "-Xmx128m",
                "-XX:MaxDirectMemorySize=64m",
                "--add-opens",
                "java.base/jdk.internal.misc=ALL-UNNAMED",
                "-cp",
            ])
            .arg(jar)
            .arg(format!("-Daeron.dir={}", driver_dir.display()))
            .arg("-Daeron.dir.delete.on.start=true")
            .arg(format!(
                "-Daeron.archive.dir={}",
                processes.directory.join("archive").display()
            ))
            .args([
                "-Daeron.archive.segment.file.length=1m",
                "-Daeron.archive.control.channel=aeron:udp?endpoint=localhost:0",
                "-Daeron.archive.replication.channel=aeron:udp?endpoint=localhost:0",
                "io.aeron.archive.ArchivingMediaDriver",
            ])
            .stdout(archive_output.try_clone()?)
            .stderr(archive_output)
            .spawn()?,
    );
    let started = Instant::now();
    while !driver_dir.join("cnc.dat").exists() {
        assert!(
            processes.children[0].try_wait()?.is_none(),
            "archive exited during startup"
        );
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "archive startup timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let command = |role: &str| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_aeron-bench"));
        cmd.env("AERON_DIR", &driver_dir).arg(role);
        cmd
    };
    let channels = [
        "--ping",
        "aeron:ipc?term-length=64k",
        "--pong",
        "aeron:ipc?term-length=64k",
    ];
    processes.children.push(
        command("pong")
            .args(channels)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let ping_output = std::fs::File::create(processes.directory.join("ping.log"))?;
    processes.children.push(
        command("ping")
            .args(channels)
            .args([
                "--record", "yes", "--rate", "10000", "--size", "64", "--count", "700", "--warmup",
                "0",
            ])
            .stdout(ping_output.try_clone()?)
            .stderr(ping_output)
            .spawn()?,
    );
    let ping = finishes(
        processes.children.last_mut().ok_or("missing ping")?,
        "recorded ping",
    )?;
    assert!(ping.success(), "recorded ping failed");
    // 700 aligned 96-byte frames exceed one 64-KiB term and leave a padding
    // frame. Padding advances the stream position without invoking the callback.
    let replay_output = std::fs::File::create(processes.directory.join("replay.log"))?;
    processes.children.push(
        command("replay")
            .stdout(replay_output.try_clone()?)
            .stderr(replay_output)
            .spawn()?,
    );
    let replay = finishes(
        processes.children.last_mut().ok_or("missing replay")?,
        "replay across term boundary",
    )?;
    let output = std::fs::read_to_string(processes.directory.join("replay.log"))?;
    assert!(replay.success(), "replay failed: {output}");
    assert!(
        output.contains("\"messages\":700,"),
        "replay did not deliver every message: {output}"
    );
    assert!(
        output.contains("\"bytes\":67200,"),
        "padding must not inflate delivered-frame throughput: {output}"
    );
    Ok(())
}
