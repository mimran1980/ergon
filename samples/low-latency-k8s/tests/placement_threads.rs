//! Where helper threads start: a thread inherits its creator's mask, so a
//! worker spawned from a thread on CPU X runs on X, and `place_worker`
//! moves a worker whose creator was elsewhere onto the helper CPUs. Linux
//! with at least two CPUs (Docker on macOS: `scripts/test-docker.sh`).
#![cfg(target_os = "linux")]

use low_latency_k8s::cpus::{CpuList, current_affinity, pin_current, thread_affinity};
use low_latency_k8s::logging::{LogConfig, non_blocking, place_worker};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Two CPUs this process may use.
fn two_cpus() -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let all = current_affinity()?;
    match (all.nth(0), all.nth(1)) {
        (Some(a), Some(b)) => Ok((a, b)),
        _ => Err(format!("these tests need two CPUs; this process may use {all}").into()),
    }
}

#[test]
fn a_worker_inherits_its_creators_mask() -> TestResult {
    let (x, _) = two_cpus()?;
    let creator = std::thread::spawn(move || -> Result<_, String> {
        pin_current(&CpuList::one(x)).map_err(|e| e.to_string())?;
        let cfg = LogConfig {
            thread_name: "ll-log-inherit".to_owned(),
            ..LogConfig::default()
        };
        let (_nb, guard, _dropped) = non_blocking(std::io::sink(), &cfg);
        let placed = place_worker("ll-log-inherit", &CpuList::one(x)).map_err(|e| e.to_string())?;
        drop(guard);
        Ok(placed)
    });
    let placed = creator.join().map_err(|_| "creator panicked")??;
    assert_eq!(placed.inherited, CpuList::one(x));
    assert_eq!(placed.now, CpuList::one(x));
    assert!(!placed.repinned);
    Ok(())
}

#[test]
fn a_worker_off_the_helpers_is_moved_onto_them() -> TestResult {
    let (x, y) = two_cpus()?;
    let creator = std::thread::spawn(move || -> Result<_, String> {
        // the creator sits where a hot thread would: Y, not the helper X
        pin_current(&CpuList::one(y)).map_err(|e| e.to_string())?;
        let cfg = LogConfig {
            thread_name: "ll-log-repin".to_owned(),
            ..LogConfig::default()
        };
        let (_nb, guard, _dropped) = non_blocking(std::io::sink(), &cfg);
        let placed = place_worker("ll-log-repin", &CpuList::one(x)).map_err(|e| e.to_string())?;
        let mask = thread_affinity(placed.tid).map_err(|e| e.to_string())?;
        drop(guard);
        Ok((placed, mask))
    });
    let (placed, mask) = creator.join().map_err(|_| "creator panicked")??;
    assert_eq!(placed.inherited, CpuList::one(y));
    assert!(placed.repinned);
    assert_eq!(placed.now, CpuList::one(x));
    assert_eq!(mask, CpuList::one(x));
    Ok(())
}

#[test]
fn place_worker_fails_closed_on_a_missing_thread() {
    assert!(place_worker("no-such-thread", &CpuList::one(0)).is_err());
}
