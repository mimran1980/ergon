//! What the transport's clock costs: each clock read on its own (the
//! hardware-counter clock at the counter's own rate, as DPDK's EAL gives it,
//! its `CLOCK_MONOTONIC` fallback, the millisecond clock it replaced, and for
//! reference `std::time::Instant`; on aarch64 also the bare counter reads,
//! `mrs CNTVCT_EL0` as the transport reads it and after an `isb` as the lab
//! clients do), then one 32-byte datagram through the genuine send and poll
//! callbacks against the fake ports, once with each transport clock. The
//! fakes stand in for the NIC and the EAL (whose rate for the counter is
//! measured here), so the path numbers are the transport's own cost, not a
//! wire latency.
//!
//! `cargo bench -p ergon-md-dpdk --features dpdk --bench clock`
//! (Linux x86_64 or aarch64; pin it, e.g. `taskset -c 2`). Elsewhere it does
//! nothing.
#![allow(
    missing_docs,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]

#[cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[path = "../tests/common/mod.rs"]
mod common;

#[cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod bench {
    use super::common::transport::*;
    use super::common::*;
    use criterion::Criterion;
    use rusteron_media_driver::bindings::iovec;
    use std::hint::black_box;
    use std::os::raw::{c_int, c_void};
    use std::time::Instant;

    /// The hardware-counter clock's row: the TSC on x86_64, the generic
    /// timer's virtual counter on aarch64.
    const COUNTER_ROW: &str = if cfg!(target_arch = "aarch64") {
        "cntvct_ns"
    } else {
        "tsc_ns"
    };

    /// (label, the counter's rate the fake EAL gives) for each transport
    /// clock: the counter at its measured rate, and `CLOCK_MONOTONIC` (no
    /// rate).
    fn clocks() -> [(&'static str, u64); 2] {
        [("monotonic_ns", 0), (COUNTER_ROW, measured_counter_hz())]
    }

    /// A test environment whose fake EAL gives the counter `hz` (0: none).
    fn env_at(hz: u64) -> TestEnv {
        let env = TestEnv::new();
        env.set_counter_rate(hz);
        env
    }

    /// The clock a transport started is the one `name` asks for.
    fn assert_clock(name: &str, hz: u64) {
        let source = unsafe { rusteron_dpdk_clock_source() };
        let wanted = if hz == 0 { SOURCE_MONOTONIC } else { COUNTER_SOURCE };
        assert_eq!(source, wanted, "{name}: transport clock source {source}");
    }

    /// The generic timer's virtual counter as the transport reads it.
    #[cfg(target_arch = "aarch64")]
    fn cntvct() -> u64 {
        let ticks: u64;
        // SAFETY: Linux lets EL0 read CNTVCT_EL0; the read touches no memory.
        unsafe { std::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack, preserves_flags)) };
        ticks
    }

    /// The same after an `isb`, so it is not taken before the instructions
    /// ahead of it: the lab clients' timestamp.
    #[cfg(target_arch = "aarch64")]
    fn isb_cntvct() -> u64 {
        let ticks: u64;
        // SAFETY: as cntvct; isb only orders the instruction stream.
        unsafe { std::arch::asm!("isb", "mrs {}, cntvct_el0", out(reg) ticks, options(nostack, preserves_flags)) };
        ticks
    }

    pub fn reads(c: &mut Criterion) {
        let mut g = c.benchmark_group("clock_read");
        g.bench_function("legacy_monotonic_ms", |b| {
            b.iter(|| black_box(unsafe { rusteron_dpdk_fake_legacy_clock_ms() }))
        });
        for (name, hz) in clocks() {
            // The clock alone, started as create starts it, no transport.
            let _env = env_at(hz);
            unsafe { rusteron_dpdk_clock_start(c_int::from(CLOCK_CHOICE_COUNTER)) };
            assert_clock(name, hz);
            g.bench_function(name, |b| b.iter(|| black_box(unsafe { rusteron_dpdk_clock_ns() })));
        }
        #[cfg(target_arch = "aarch64")]
        {
            g.bench_function("cntvct", |b| b.iter(|| black_box(cntvct())));
            g.bench_function("isb_cntvct", |b| b.iter(|| black_box(isb_cntvct())));
        }
        g.bench_function("std_instant", |b| b.iter(|| black_box(Instant::now())));
        g.finish();
    }

    pub fn send(c: &mut Criterion) {
        let mut g = c.benchmark_group("send_32b");
        for (name, hz) in clocks() {
            let mut h = Harness::build_after(env_at(hz), rusteron_dpdk_config_t::valid());
            assert_clock(name, hz);
            h.init_tx(SENDER_IP, 40000, None, 1472);
            h.arp_seed(PEER_IP, PEER_MAC);
            let payload = [7u8; 32];
            let mut dst = SockAddrIn::new(PEER_IP, 40123);
            let mut iov = [iovec {
                iov_base: payload.as_ptr() as *mut c_void,
                iov_len: payload.len(),
            }];
            let mut bytes = 0i64;
            assert_eq!(h.send_iov(&mut dst, &mut iov, &mut bytes), 1, "send: {}", last_error());
            g.bench_function(name, |b| {
                b.iter(|| black_box(h.send_iov(&mut dst, &mut iov, &mut bytes)))
            });
            assert_no_leak();
        }
        g.finish();
    }

    pub fn poll(c: &mut Criterion) {
        let mut g = c.benchmark_group("poll_32b");
        for (name, hz) in clocks() {
            let mut h = Harness::build_after(env_at(hz), rusteron_dpdk_config_t::valid());
            assert_clock(name, hz);
            h.init_rx(RECEIVER_IP, 60000, 1472);
            let mut p = h.poller();
            p.add(&mut *h.rx);
            let frame = build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, RECEIVER_IP, 7000, 60000, &[7u8; 32]);
            let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; 16];
            let mut count = 0u64;
            let clientd = &mut count as *mut u64 as *mut c_void;
            let mut bytes = 0i64;
            let mut one = || {
                unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) };
                p.poll_into(&mut msgvec, Some(count_cb), clientd, &mut bytes)
            };
            assert_eq!(one(), 1, "poll: {}", last_error());
            g.bench_function(name, |b| b.iter(|| black_box(one())));
            drop(p);
            assert_no_leak();
        }
        g.finish();
    }
}

#[cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
criterion::criterion_group!(benches, bench::reads, bench::send, bench::poll);
#[cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
criterion::criterion_main!(benches);

#[cfg(not(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
fn main() {}
