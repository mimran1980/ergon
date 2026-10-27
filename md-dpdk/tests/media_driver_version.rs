//! `media_driver --version` prints the build and exits without starting a
//! driver: the crate version, the architecture, Rust's target CPU and the flag
//! the native C was built with (the image smoke test runs it).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn version_prints_the_build_and_exits() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_media_driver"))
        .arg("--version")
        .env_remove("RUSTERON_MEDIA_DRIVER_TRANSPORT")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() > deadline {
            child.kill()?;
            child.wait()?;
            return Err("media_driver --version did not exit within 10 s: it started a driver".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let out = child.wait_with_output()?;
    let stdout = String::from_utf8(out.stdout)?;
    assert!(status.success(), "exit {status}: {stdout}");
    let line = stdout.trim_end();
    assert!(
        line.starts_with(&format!("media_driver {} (", env!("CARGO_PKG_VERSION"))),
        "{line}"
    );
    assert!(line.contains(std::env::consts::ARCH), "{line}");
    assert!(line.contains("Rust target CPU "), "{line}");
    assert!(line.contains("native C "), "{line}");
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    Ok(())
}
