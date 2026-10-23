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

    /// The `q` quantile of `sorted`.
    fn quantile(sorted: &[f64], q: f64) -> f64 {
        sorted[((sorted.len() - 1) as f64 * q).round() as usize]
    }

    /// p50, p99 and max of `samples` (sorted in place).
    fn percentiles(samples: &mut [f64]) -> (f64, f64, f64) {
        samples.sort_by(f64::total_cmp);
        (
            quantile(samples, 0.5),
            quantile(samples, 0.99),
            samples[samples.len() - 1],
        )
    }

    /// ns of each of `calls` calls of `one`, each timed alone between two
    /// reads of the transport's own counter (`hz`; the reads' cost
    /// included, alike in every arm): the tail a batch mean averages away.
    /// Preallocated, so recording allocates nothing.
    fn per_call_ns(calls: usize, hz: u64, mut one: impl FnMut() -> c_int) -> Vec<f64> {
        let scale = 1e9 / hz as f64;
        let mut out = Vec::with_capacity(calls);
        for _ in 0..calls {
            let t0 = unsafe { rusteron_dpdk_test_counter() };
            black_box(one());
            let t1 = unsafe { rusteron_dpdk_test_counter() };
            out.push(t1.wrapping_sub(t0) as f64 * scale);
        }
        out
    }

    /// Both passes over `one`: the batch means, then each call timed alone.
    fn passes(hz: u64, mut one: impl FnMut() -> c_int) -> (Vec<f64>, Vec<f64>) {
        const BATCHES: usize = 2000;
        const PER: u32 = 1000;
        const CALLS: usize = 1_000_000;
        let batches = batch_ns(BATCHES, PER, &mut one);
        (batches, per_call_ns(CALLS, hz, one))
    }

    /// ns per call of `one`, per batch of `per`, over `batches` batches.
    fn batch_ns(batches: usize, per: u32, mut one: impl FnMut() -> c_int) -> Vec<f64> {
        (0..batches)
            .map(|_| {
                let start = Instant::now();
                for _ in 0..per {
                    black_box(one());
                }
                start.elapsed().as_nanos() as f64 / f64::from(per)
            })
            .collect()
    }

    /// What the diagnostics cost the send and poll paths, on the counter
    /// clock: off (twice, first and last, for the A/A spread), with idle TX
    /// cleanup on, with RUSTERON_DPDK_GAP_ATTRIB=1 (burst timing: two clock
    /// reads a burst, and the thread CPU clock a poll), with `burst` (the
    /// burst timing alone), and with the event log open as well. The paths:
    /// a 32-byte send, a poll that receives one, and a poll that receives
    /// nothing (an idle agent's every cycle, where idle TX cleanup decides
    /// whether to reclaim). Each row is the p50, p99 and max of the mean ns
    /// per call over 2,000 batches of 1,000 calls, then, from a separate
    /// pass, the p99.99 and max of 1,000,000 calls each timed alone (printed,
    /// not criterion estimates): the gate is off within the A/A spread, on
    /// adding at most about 12 ns.
    pub fn overhead(_c: &mut Criterion) {
        let log = std::env::temp_dir().join(format!("ergon-md-dpdk-bench-events-{}.csv", std::process::id()));
        let log = log.to_string_lossy().into_owned();
        // gap-attrib also reads the thread CPU clock once a poll (a syscall,
        // the gap split that predates burst timing); "+cpu-pinned" pins that
        // clock so the row shows the burst timing's own cost, and
        // "gap-attrib=burst" is the burst timing alone, as the lab runs it.
        // "tx-idle-clean" is RUSTERON_DPDK_TX_IDLE_CLEAN=32: its bookkeeping
        // on a send, and its one branch on a poll that received.
        let arms: [(&str, &[(&'static str, &str)]); 7] = [
            ("off", &[]),
            ("tx-idle-clean", &[("RUSTERON_DPDK_TX_IDLE_CLEAN", "32")]),
            ("gap-attrib", &[("RUSTERON_DPDK_GAP_ATTRIB", "1")]),
            ("gap-attrib+cpu-pinned", &[("RUSTERON_DPDK_GAP_ATTRIB", "1")]),
            ("gap-attrib=burst", &[("RUSTERON_DPDK_GAP_ATTRIB", "burst")]),
            (
                "gap-attrib+event-log",
                &[
                    ("RUSTERON_DPDK_GAP_ATTRIB", "1"),
                    ("RUSTERON_DPDK_EVENT_LOG", log.as_str()),
                ],
            ),
            ("off-again", &[]),
        ];
        let hz = measured_counter_hz();
        println!(
            "overhead (ns per call: p50 / p99 / max of the means of 2,000 batches of 1,000; then per call, \
             p99.99 / max of 1,000,000 calls each timed alone)"
        );
        for (path, label) in [(0, "send_32b"), (1, "poll_32b"), (2, "poll_idle")] {
            for (arm, vars) in arms {
                let _unset = (
                    EnvGuard::unset("RUSTERON_DPDK_GAP_ATTRIB"),
                    EnvGuard::unset("RUSTERON_DPDK_EVENT_LOG"),
                    EnvGuard::unset("RUSTERON_DPDK_TX_IDLE_CLEAN"),
                );
                let _set: Vec<EnvGuard> = vars.iter().map(|(k, v)| EnvGuard::set(k, v)).collect();
                let mut h = Harness::build_after(env_at(hz), rusteron_dpdk_config_t::valid());
                assert_clock(COUNTER_ROW, hz);
                if arm.ends_with("cpu-pinned") {
                    pin_thread_cpu_ns(1);
                }
                let (mut samples, mut calls) = if path == 0 {
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
                    passes(hz, || h.send_iov(&mut dst, &mut iov, &mut bytes))
                } else {
                    h.init_rx(RECEIVER_IP, 60000, 1472);
                    let mut p = h.poller();
                    p.add(&mut *h.rx);
                    let frame = build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, RECEIVER_IP, 7000, 60000, &[7u8; 32]);
                    let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; 16];
                    let mut count = 0u64;
                    let clientd = &mut count as *mut u64 as *mut c_void;
                    let mut bytes = 0i64;
                    // poll_idle: nothing arrives, so every poll finds none
                    let received = c_int::from(path == 1);
                    let mut one = || {
                        if received == 1 {
                            unsafe {
                                rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1)
                            };
                        }
                        p.poll_into(&mut msgvec, Some(count_cb), clientd, &mut bytes)
                    };
                    assert_eq!(one(), received, "poll: {}", last_error());
                    passes(hz, one)
                };
                let (p50, p99, max) = percentiles(&mut samples);
                calls.sort_by(f64::total_cmp);
                let (p9999, call_max) = (quantile(&calls, 0.9999), calls[calls.len() - 1]);
                println!(
                    "{label:<9} {arm:<22} {p50:>7.1} / {p99:>7.1} / {max:>8.1}   per call {p9999:>8.1} / {call_max:>9.1}"
                );
                assert_no_leak();
            }
        }
        let _ = std::fs::remove_file(&log);
    }
}

#[cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
criterion::criterion_group!(benches, bench::reads, bench::send, bench::poll, bench::overhead);
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
