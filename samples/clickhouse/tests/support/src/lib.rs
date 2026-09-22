//! Shared integration-test harness: boots pinned service containers
//! (ClickHouse, SeaweedFS S3) via Docker and tears them down after.
//!
//! Containers are started with unique names/ports per test process; the
//! harness fails (rather than skips) when Docker is unavailable — service
//! tests are part of the lane.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A running service container.
pub struct Container {
    pub name: String,
    pub port: u16,
}

impl Drop for Container {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .output();
    }
}

fn docker_available() -> bool {
    Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn free_port() -> Option<u16> {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .ok()?
        .local_addr()
        .ok()
        .map(|a| a.port())
}

fn http_ready(url: &str, deadline: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if let Ok(resp) = ureq::get(url).call() {
            let _ = resp.into_string();
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

/// Start a pinned ClickHouse server; returns its HTTP port.
pub fn start_clickhouse(tag: &str) -> Result<Container, Box<dyn std::error::Error>> {
    assert!(
        docker_available(),
        "docker must be available for service tests"
    );
    let name = format!("ergo-ch-test-{tag}");
    let _ = Command::new("docker").args(["rm", "-f", &name]).output();
    let port = free_port().ok_or("no free port")?;
    Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            &name,
            "-p",
            &format!("{port}:8123"),
            "-e",
            "CLICKHOUSE_USER=default",
            "-e",
            "CLICKHOUSE_PASSWORD=ergo_test",
            "clickhouse/clickhouse-server:24.8-alpine",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !http_ready(
        &format!("http://127.0.0.1:{port}/ping"),
        Duration::from_secs(120),
    ) {
        let logs = Command::new("docker").args(["logs", &name]).output()?;
        return Err(format!(
            "clickhouse not ready in 120s; logs: {}",
            String::from_utf8_lossy(&logs.stdout)
        )
        .into());
    }
    Ok(Container { name, port })
}

/// Start a pinned SeaweedFS S3 endpoint; returns its port.
pub fn start_seaweedfs(tag: &str) -> Result<Container, Box<dyn std::error::Error>> {
    assert!(
        docker_available(),
        "docker must be available for service tests"
    );
    let name = format!("ergo-s3-test-{tag}");
    let _ = Command::new("docker").args(["rm", "-f", &name]).output();
    let port = free_port().ok_or("no free port")?;
    Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            &name,
            "-p",
            &format!("{port}:8333"),
            "chrislusf/seaweedfs:3.80",
            "server",
            "-s3",
            "-volume.max=0",
            "-master.volumeSizeLimitMB=1024",
            "-ip=127.0.0.1",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !http_ready(
        &format!("http://127.0.0.1:{port}/"),
        Duration::from_secs(120),
    ) {
        let logs = Command::new("docker").args(["logs", &name]).output()?;
        return Err(format!(
            "seaweedfs not ready in 120s; logs: {}",
            String::from_utf8_lossy(&logs.stdout)
        )
        .into());
    }
    Ok(Container { name, port })
}
