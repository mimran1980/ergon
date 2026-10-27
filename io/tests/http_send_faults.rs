//! A fresh `HttpConn`'s first requests take no page faults: its send queue
//! is written once at construction. Its own test binary with one test, so
//! the queue is not carved from heap pages an earlier test already touched.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod http_rig;
mod support;

use ergon_io::HttpOptions;
use http_rig::HttpRig;
use support::{Lab, R};

/// Minor faults of this thread so far (`/proc/thread-self/stat`, field 10).
fn minor_faults() -> R<u64> {
    let s = std::fs::read_to_string("/proc/thread-self/stat")?;
    let close = s.rfind(')').ok_or("no comm")?;
    let f: Vec<&str> = s[close + 1..].split_whitespace().collect();
    Ok(f.get(7).ok_or("short stat")?.parse()?)
}

/// Queueing 60 KB of requests on a fresh connection takes no page faults
/// (64 KiB of untouched queue would take one per page).
#[test]
fn the_first_requests_take_no_page_faults() -> R {
    let lab = Lab::new()?;
    let mut r = HttpRig::new(
        &lab,
        HttpOptions {
            max_in_flight: 8,
            ..HttpOptions::default()
        },
    )?;
    let body = vec![b'x'; 15_000];
    // Warm the code path (its pages fault too) on a request with no body,
    // which touches only the queue's first page; then empty the queue.
    r.conn.request("GET", "/", &[], &[])?;
    r.conn.clear_tx();
    let before = minor_faults()?;
    for _ in 0..4 {
        r.conn.request("POST", "/", &[], &body)?;
    }
    let after = minor_faults()?;
    assert!(r.conn.queued_tx().len() > 60_000);
    assert!(
        after - before <= 2,
        "{} minor faults queueing 60 KB on a fresh connection",
        after - before
    );
    Ok(())
}
