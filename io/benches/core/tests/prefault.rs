//! The spike and stall buffers are written when built, so filling them in a
//! measured run takes no first-touch page fault (Linux: the thread's minor
//! faults, which only `getrusage` reports).

#![cfg(target_os = "linux")]

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::row::{Spikes, Stalls};

fn minor_faults() -> i64 {
    // SAFETY: getrusage writes only the struct it is given.
    #[allow(unsafe_code)]
    let usage = unsafe {
        let mut u = std::mem::zeroed::<libc::rusage>();
        assert_eq!(libc::getrusage(libc::RUSAGE_THREAD, &raw mut u), 0);
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
