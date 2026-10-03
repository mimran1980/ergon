//! Allocator and process metrics from mimalloc, sampled on a housekeeping
//! timer, never per message.
//!
//! Release builds of mimalloc keep only its OS-level counters (`MI_STAT=0`):
//! reserved and committed bytes, and the calls that went to the OS. Those are
//! what an HFT loop cares about: any `alloc_os_*` increment during trading is
//! a latency-spike candidate. `MiMalloc::stats_json` allocates, so the C
//! functions are called directly. This is the only module of the client
//! allowed `unsafe`.
//!
//! For a one-off deep dive, `MIMALLOC_SHOW_STATS=1` prints mimalloc's own
//! report at exit and `MIMALLOC_VERBOSE=1` logs its OS calls; neither needs
//! code.
#![allow(unsafe_code)]

#[cfg(feature = "alloc-count")]
use std::alloc::{GlobalAlloc, Layout};
#[cfg(feature = "alloc-count")]
use std::cell::Cell;
use std::time::Instant;

use crate::metrics::{Counter, Gauge, Histogram, Metrics};

/// `mi_stat_count_t`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Count {
    total: i64,
    peak: i64,
    current: i64,
}

/// `mi_stat_counter_t`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Total {
    total: i64,
}

/// `MI_STAT_VERSION` of the vendored mimalloc 3.
const MI_STAT_VERSION: usize = 5;

/// A mirror of mimalloc 3's `mi_stats_t`, up to the fields read; the rest is
/// carried as `tail`. `mi_stats_get` copies nothing unless `size` and
/// `version` match its own, so a layout change returns `false`, never an
/// overrun.
#[repr(C)]
struct MiStats {
    size: usize,
    version: usize,
    pages: Count,
    reserved: Count,
    committed: Count,
    reset: Total,
    purged: Total,
    page_committed: Count,
    pages_abandoned: Count,
    threads: Count,
    malloc_normal: Count,
    malloc_huge: Count,
    malloc_requested: Count,
    mmap_calls: Total,
    commit_calls: Total,
    reset_calls: Total,
    purge_calls: Total,
    arena_count: Total,
    /// From `malloc_normal_count` to the end: 3 + 6 counters, 6 counts,
    /// 5 counters, the reserved 4 counts and 4 counters, then 74 + 74 + 6
    /// size-bin counts.
    tail: [i64; TAIL],
}

const TAIL: usize = (3 + 6) + 6 * 3 + 5 + (4 * 3 + 4) + (74 + 74 + 6) * 3;

impl MiStats {
    fn new() -> Box<Self> {
        Box::new(Self {
            size: size_of::<Self>(),
            version: MI_STAT_VERSION,
            pages: Count::default(),
            reserved: Count::default(),
            committed: Count::default(),
            reset: Total::default(),
            purged: Total::default(),
            page_committed: Count::default(),
            pages_abandoned: Count::default(),
            threads: Count::default(),
            malloc_normal: Count::default(),
            malloc_huge: Count::default(),
            malloc_requested: Count::default(),
            mmap_calls: Total::default(),
            commit_calls: Total::default(),
            reset_calls: Total::default(),
            purge_calls: Total::default(),
            arena_count: Total::default(),
            tail: [0; TAIL],
        })
    }
}

unsafe extern "C" {
    /// Stats aggregated over the current subprocess and its heaps.
    fn mi_stats_get(stats: *mut MiStats) -> bool;
}

#[cfg(feature = "alloc-count")]
thread_local! {
    static THREAD_ALLOCS: Cell<u64> = const { Cell::new(0) };
}

/// `MiMalloc` that counts each thread's allocations.
///
/// Install it as the `#[global_allocator]` and [`AllocStats`] publishes the
/// loop thread's as `loop_allocs`. One thread-local increment per
/// allocation, process wide.
#[cfg(feature = "alloc-count")]
#[derive(Debug, Default)]
pub struct CountingMiMalloc;

#[cfg(feature = "alloc-count")]
// SAFETY: every call delegates to `MiMalloc` with the caller's arguments;
// the thread-local counter is const-initialised and allocates nothing.
unsafe impl GlobalAlloc for CountingMiMalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        THREAD_ALLOCS.with(|n| n.set(n.get() + 1));
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { mimalloc::MiMalloc.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        THREAD_ALLOCS.with(|n| n.set(n.get() + 1));
        // SAFETY: as `alloc`.
        unsafe { mimalloc::MiMalloc.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator, which is `MiMalloc`.
        unsafe { mimalloc::MiMalloc.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        THREAD_ALLOCS.with(|n| n.set(n.get() + 1));
        // SAFETY: `ptr` came from this allocator for `layout`.
        unsafe { mimalloc::MiMalloc.realloc(ptr, layout, new_size) }
    }
}

/// Allocations [`CountingMiMalloc`] counted on the calling thread.
#[cfg(feature = "alloc-count")]
#[must_use]
pub fn thread_allocs() -> u64 {
    THREAD_ALLOCS.with(Cell::get)
}

/// Monotonic totals, kept to publish deltas.
#[derive(Clone, Copy, Default)]
struct Totals {
    mmap: i64,
    commit: i64,
    purge: i64,
    purged: i64,
    user_ms: usize,
    sys_ms: usize,
    faults_major: usize,
    loop_minor: i64,
    loop_major: i64,
    loop_voluntary: i64,
    loop_involuntary: i64,
    loop_allocs: u64,
}

/// What one sample publishes, and the storage it reuses: nothing allocates
/// after [`AllocStats::new`].
pub struct AllocStats {
    stats: Box<MiStats>,
    last: Totals,
    warned: bool,
    committed: Gauge,
    committed_peak: Gauge,
    reserved: Gauge,
    rss_peak: Gauge,
    arenas: Gauge,
    threads: Gauge,
    abandoned: Gauge,
    mmap_calls: Counter,
    commit_calls: Counter,
    purge_calls: Counter,
    purged: Counter,
    cpu_user: Counter,
    cpu_sys: Counter,
    faults_major: Counter,
    loop_minor: Counter,
    loop_major: Counter,
    loop_voluntary: Counter,
    loop_involuntary: Counter,
    sample_ns: Histogram,
    #[cfg(feature = "alloc-count")]
    loop_allocs: Counter,
}

impl AllocStats {
    /// Register the series on `metrics`. Call it on the loop's thread: the
    /// `loop_*` counters are that thread's.
    #[must_use]
    pub fn new(metrics: &Metrics) -> Self {
        Self {
            stats: MiStats::new(),
            last: Totals::default(),
            warned: false,
            committed: metrics.gauge("mem_committed_bytes", &[]),
            committed_peak: metrics.gauge("mem_committed_peak_bytes", &[]),
            reserved: metrics.gauge("mem_reserved_bytes", &[]),
            rss_peak: metrics.gauge("mem_rss_peak_bytes", &[]),
            arenas: metrics.gauge("alloc_arenas", &[]),
            threads: metrics.gauge("alloc_threads", &[]),
            abandoned: metrics.gauge("alloc_pages_abandoned", &[]),
            mmap_calls: metrics.counter("alloc_os_mmap_calls", &[]),
            commit_calls: metrics.counter("alloc_os_commit_calls", &[]),
            purge_calls: metrics.counter("alloc_os_purge_calls", &[]),
            purged: metrics.counter("alloc_purged_bytes", &[]),
            cpu_user: metrics.counter("cpu_user_ms", &[]),
            cpu_sys: metrics.counter("cpu_sys_ms", &[]),
            faults_major: metrics.counter("faults_major", &[]),
            loop_minor: metrics.counter("loop_faults_minor", &[]),
            loop_major: metrics.counter("loop_faults_major", &[]),
            loop_voluntary: metrics.counter("loop_ctx_switches_voluntary", &[]),
            loop_involuntary: metrics.counter("loop_ctx_switches_involuntary", &[]),
            sample_ns: metrics.histogram("alloc_sample_ns", &[]),
            #[cfg(feature = "alloc-count")]
            loop_allocs: metrics.counter("loop_allocs", &[]),
        }
    }

    /// Read mimalloc and the process, and publish. Returns `false` when
    /// mimalloc refused the stats layout (logged once): the process metrics
    /// are still published.
    pub fn sample(&mut self) -> bool {
        let started = Instant::now();
        let mut now = self.last;
        let ok = self.read_mimalloc(&mut now);
        self.read_process(&mut now);
        self.read_thread(&mut now);
        #[cfg(feature = "alloc-count")]
        {
            now.loop_allocs = thread_allocs();
        }
        self.publish(now);
        self.last = now;
        self.sample_ns
            .record(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        ok
    }

    fn read_mimalloc(&mut self, now: &mut Totals) -> bool {
        // SAFETY: `stats` is a live, exclusively borrowed `mi_stats_t` mirror
        // whose `size` and `version` fields mimalloc checks before writing; it
        // writes exactly `size` bytes or nothing.
        let ok = unsafe { mi_stats_get(&raw mut *self.stats) };
        if !ok {
            if !self.warned {
                log::error!(
                    "mimalloc refused the stats layout ({} bytes, version {MI_STAT_VERSION}): allocator metrics are off",
                    size_of::<MiStats>()
                );
                self.warned = true;
            }
            return false;
        }
        let s = &self.stats;
        self.reserved.set(f(s.reserved.current));
        self.arenas.set(f(s.arena_count.total));
        self.threads.set(f(s.threads.current));
        self.abandoned.set(f(s.pages_abandoned.current));
        now.mmap = s.mmap_calls.total;
        now.commit = s.commit_calls.total;
        now.purge = s.purge_calls.total;
        now.purged = s.purged.total;
        true
    }

    fn read_process(&self, now: &mut Totals) {
        let (mut elapsed, mut user, mut sys) = (0, 0, 0);
        let (mut rss, mut rss_peak, mut commit, mut commit_peak, mut faults) = (0, 0, 0, 0, 0);
        // SAFETY: every argument points at a local `usize`, which is what
        // `mi_process_info` writes through each pointer.
        unsafe {
            libmimalloc_sys::mi_process_info(
                &raw mut elapsed,
                &raw mut user,
                &raw mut sys,
                &raw mut rss,
                &raw mut rss_peak,
                &raw mut commit,
                &raw mut commit_peak,
                &raw mut faults,
            );
        }
        self.committed.set(f(commit));
        self.committed_peak.set(f(commit_peak));
        self.rss_peak.set(f(rss_peak));
        now.user_ms = user;
        now.sys_ms = sys;
        now.faults_major = faults;
    }

    #[cfg(target_os = "linux")]
    #[allow(clippy::unused_self)]
    fn read_thread(&self, now: &mut Totals) {
        // SAFETY: `rusage` is plain data; all-zero is a valid value.
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        // SAFETY: `usage` is a valid, exclusively borrowed `rusage`.
        if unsafe { libc::getrusage(libc::RUSAGE_THREAD, &raw mut usage) } == 0 {
            now.loop_minor = usage.ru_minflt;
            now.loop_major = usage.ru_majflt;
            now.loop_voluntary = usage.ru_nvcsw;
            now.loop_involuntary = usage.ru_nivcsw;
        }
    }

    /// `RUSAGE_THREAD` is Linux's alone.
    #[cfg(not(target_os = "linux"))]
    #[allow(clippy::unused_self)]
    const fn read_thread(&self, _now: &mut Totals) {}

    fn publish(&self, now: Totals) {
        let last = self.last;
        let up = |a: i64, b: i64| u64::try_from(a.saturating_sub(b)).unwrap_or(0);
        let up_u = |a: usize, b: usize| u64::try_from(a.saturating_sub(b)).unwrap_or(0);
        self.mmap_calls.add(up(now.mmap, last.mmap));
        self.commit_calls.add(up(now.commit, last.commit));
        self.purge_calls.add(up(now.purge, last.purge));
        self.purged.add(up(now.purged, last.purged));
        self.cpu_user.add(up_u(now.user_ms, last.user_ms));
        self.cpu_sys.add(up_u(now.sys_ms, last.sys_ms));
        self.faults_major
            .add(up_u(now.faults_major, last.faults_major));
        self.loop_minor.add(up(now.loop_minor, last.loop_minor));
        self.loop_major.add(up(now.loop_major, last.loop_major));
        self.loop_voluntary
            .add(up(now.loop_voluntary, last.loop_voluntary));
        self.loop_involuntary
            .add(up(now.loop_involuntary, last.loop_involuntary));
        #[cfg(feature = "alloc-count")]
        self.loop_allocs
            .add(now.loop_allocs.saturating_sub(last.loop_allocs));
    }

    /// Total `alloc_os_mmap_calls` and `alloc_os_commit_calls` published.
    #[must_use]
    pub fn os_calls(&self) -> u64 {
        self.mmap_calls.get() + self.commit_calls.get()
    }

    /// The last `mem_committed_bytes`.
    #[must_use]
    pub fn committed_bytes(&self) -> f64 {
        self.committed.get()
    }
}

/// A gauge value: byte and thread counts stay far below 2^53, where `f64`
/// starts to round.
#[allow(clippy::cast_precision_loss)]
fn f(x: impl TryInto<i64>) -> f64 {
    x.try_into().unwrap_or(i64::MAX) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mirror_matches_the_vendored_layout() {
        assert_eq!(size_of::<MiStats>(), 4368);
        let mut stats = AllocStats::new(&Metrics::detached());
        assert!(
            stats.sample(),
            "mi_stats_get refused the mirror: mimalloc's mi_stats_t changed"
        );
    }
}
