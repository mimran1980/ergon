//! Each line ergon-io's warning thread writes reaches stderr in one
//! `write(2)`, newline included. A process whose stdout and stderr share one
//! file (a runner's `>out 2>&1`) must never see another thread's line land
//! between a warning's text and its newline: that turned a pong's `ready`
//! into `...the polling CPU is 2.ready`, and its runner, waiting for the
//! line `ready`, timed out.
//!
//! The test runs itself again as a child whose stderr is one end of a Unix
//! datagram socket pair, so each `write(2)` arrives as one datagram.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

use std::os::fd::OwnedFd;
use std::os::unix::net::UnixDatagram;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ergon_io::{Backend, Config, Driver, UdpConn, warnings};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Set in the child: it makes the lines instead of checking them.
const CHILD: &str = "ERGON_IO_WARN_LINES_CHILD";
const NAME: &str = "every_warning_line_is_one_write";
const WAIT: Duration = Duration::from_secs(30);

/// The child: a driver whose start writes lines (the helper CPUs on a box
/// with more than one, a receive buffer the kernel caps), then waits until
/// the warning thread wrote them.
fn child() -> R {
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.rcvbuf = Some(1 << 30);
    let d: Driver<UdpConn> = Driver::new(cfg)?;
    if !warnings::flush(WAIT) {
        return Err("the warning thread did not catch up".into());
    }
    drop(d);
    Ok(())
}

#[test]
fn every_warning_line_is_one_write() -> R {
    if std::env::var_os(CHILD).is_some() {
        return child();
    }
    let (ours, theirs) = UnixDatagram::pair()?;
    let mut c = Command::new(std::env::current_exe()?)
        .args([NAME, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env_remove("ERGON_HOST_CHECKS")
        .stdout(Stdio::null())
        .stderr(Stdio::from(OwnedFd::from(theirs)))
        .spawn()?;
    ours.set_read_timeout(Some(Duration::from_millis(100)))?;
    let (mut writes, mut buf, until) = (Vec::new(), vec![0u8; 1 << 16], Instant::now() + WAIT);
    let mut exited = None;
    // Read while the child runs (a datagram socket's queue is short), then
    // what it left queued.
    loop {
        match ours.recv(&mut buf) {
            Ok(n) => writes.push(String::from_utf8_lossy(&buf[..n]).into_owned()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if exited.is_some() {
                    break;
                }
                exited = c.try_wait()?;
                if Instant::now() > until {
                    let _ = c.kill();
                    return Err(format!("the child did not finish: {writes:?}").into());
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    let status = exited.ok_or("no exit status")?;
    assert!(status.success(), "the child failed ({status}): {writes:?}");
    assert!(
        writes.iter().any(|w| w.starts_with("ergon-io ")),
        "no ergon-io line was written: {writes:?}"
    );
    for w in &writes {
        assert!(
            w.ends_with('\n') && w.matches('\n').count() == 1,
            "a write that is not one whole line: {w:?} (all: {writes:?})"
        );
    }
    Ok(())
}
