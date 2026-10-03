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

#[test]
fn the_counting_allocator_counts_this_thread() {
    let before = thread_allocs();
    let boxed = std::hint::black_box(Box::new([1u8; 128]));
    assert!(thread_allocs() > before);
    drop(boxed);
}
