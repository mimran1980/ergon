//! The spike and stall buffers are written when built, so filling them in a
//! measured run takes no first-touch page fault (the minor faults
//! `getrusage` reports: the thread's on Linux, the process's elsewhere, so
//! run single-threaded).

#![cfg(unix)]

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::row::{Spikes, Stalls};

fn minor_faults() -> i64 {
    // SAFETY: getrusage writes only the struct it is given.
    #[allow(unsafe_code)]
    let usage = unsafe {
        let mut u = std::mem::zeroed::<libc::rusage>();
        #[cfg(target_os = "linux")]
        let who = libc::RUSAGE_THREAD;
        #[cfg(not(target_os = "linux"))]
        let who = libc::RUSAGE_SELF;
        assert_eq!(libc::getrusage(who, &raw mut u), 0);
        u
    };
    usage.ru_minflt
}

#[test]
fn filling_spike_and_stall_buffers_takes_no_page_fault() {
    const N: usize = 1 << 16;
    let clock = TscClock::new();
    let t0 = clock.raw();
    let mut spikes = Spikes::new(0, N);
    let mut stalls = Stalls::new(0, N, t0);
    let before = minor_faults();
    for i in 0..N as u64 {
        spikes.record(1, i, i + 1, i);
        stalls.pass(&clock, t0 + i);
    }
    let faults = minor_faults() - before;
    assert_eq!(faults, 0, "{faults} page faults filling {N}-slot buffers");
}

#[test]
fn recording_across_the_histogram_takes_no_page_fault() -> Result<(), Box<dyn std::error::Error>> {
    use ergon_latency_core::hist::Latency;
    let mut lat = Latency::new()?;
    let before = minor_faults();
    // every power of two from 1 ns to 60 s, and a dense run in the µs range:
    // each lands in its own bucket, so each would touch a fresh page if the
    // counts were still the kernel's lazy zero pages
    let mut v: u64 = 1;
    while v < 60_000_000_000 {
        lat.record(v);
        v = v * 2 + 1;
    }
    for ns in (1_000..2_000_000).step_by(997) {
        lat.record(ns);
    }
    let faults = minor_faults() - before;
    assert_eq!(
        faults, 0,
        "{faults} page faults recording into a fresh histogram"
    );
    Ok(())
}
