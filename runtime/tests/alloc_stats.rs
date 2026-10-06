//! Allocator metrics see the allocator: a block larger than mimalloc's
//! up-front arena raises committed memory and counts an OS call, and the
//! loop thread's allocations are counted.

use ergon_runtime::alloc_stats::{AllocStats, CountingMiMalloc, thread_allocs};
use ergon_runtime::metrics::Metrics;

#[global_allocator]
static GLOBAL: CountingMiMalloc = CountingMiMalloc;

#[test]
fn fresh_pages_raise_commit_and_count_os_calls() {
    let mut stats = AllocStats::new(&Metrics::detached());
    assert!(stats.sample(), "mi_stats_get refused the layout mirror");
    let (calls, committed) = (stats.os_calls(), stats.committed_bytes());
    // Bigger than the arena mimalloc reserves up front, so it must go to the
    // OS. Zeroed fresh pages are left untouched: no RSS is spent.
    let block = vec![0u8; 2 << 30];
    std::hint::black_box(&block);
    assert!(stats.sample());
    assert!(
        stats.committed_bytes() > committed,
        "committed {committed} -> {}",
        stats.committed_bytes()
    );
    assert!(stats.os_calls() > calls, "no mmap or commit call counted");
    drop(block);
}

/// A loop thread that has to share its core is preempted, and the sample
/// counts it: pinned to one core beside a second spinning thread, it is
/// switched out involuntarily. `RUSAGE_THREAD` is Linux's alone.
#[cfg(target_os = "linux")]
#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "a rival thread on the loop's core is what this measures"
)]
fn a_shared_core_counts_involuntary_switches() {
    use std::time::{Duration, Instant};

    #[allow(unsafe_code)]
    fn pin_to_core_zero() {
        // SAFETY: an all-zero `cpu_set_t` is the empty set, `CPU_SET` adds
        // core 0 to it, and `sched_setaffinity(0, ..)` only reads it, for
        // the calling thread.
        let pinned = unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_SET(0, &mut set);
            libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &raw const set)
        };
        assert_eq!(pinned, 0, "sched_setaffinity to core 0 failed");
    }
    fn spin(time: Duration) {
        let end = Instant::now() + time;
        while Instant::now() < end {
            std::hint::spin_loop();
        }
    }

    pin_to_core_zero();
    let mut stats = AllocStats::new(&Metrics::detached());
    assert!(stats.sample());
    let before = stats.loop_involuntary();
    let rival = std::thread::spawn(|| {
        pin_to_core_zero();
        spin(Duration::from_millis(300));
    });
    spin(Duration::from_millis(300));
    assert!(rival.join().is_ok(), "the rival thread panicked");
    assert!(stats.sample());
    // Beside the rival it is preempted every few milliseconds, about 50
    // times; alone on its core, only a handful of times.
    assert!(
        stats.loop_involuntary() >= before + 20,
        "{} involuntary switches counted beside a spinning rival",
        stats.loop_involuntary() - before
    );
}

#[test]
fn the_counting_allocator_counts_this_thread() {
    let before = thread_allocs();
    let boxed = std::hint::black_box(Box::new([1u8; 128]));
    assert!(thread_allocs() > before);
    drop(boxed);
}
