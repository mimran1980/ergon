//! What one allocator-metrics sample costs: min / p50 / p99 / p99.99 / max.
//! It runs on a housekeeping timer every 5 s, in a cycle with no work.

use std::time::Instant;

use ergon_runtime::alloc_stats::AllocStats;
use ergon_runtime::metrics::Metrics;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let mut stats = AllocStats::new(&Metrics::detached());
    let mut ns: Vec<u128> = Vec::with_capacity(10_000);
    for _ in 0..100 {
        stats.sample();
    }
    for _ in 0..10_000 {
        let start = Instant::now();
        stats.sample();
        ns.push(start.elapsed().as_nanos());
    }
    ns.sort_unstable();
    let at = |q: usize| ns[((ns.len() - 1) * q).div_ceil(10_000)];
    println!(
        "PERCENTILES alloc_sample ns n={} min={} p50={} p99={} p99.99={} max={}",
        ns.len(),
        ns[0],
        at(5_000),
        at(9_900),
        at(9_999),
        ns[ns.len() - 1]
    );
}
