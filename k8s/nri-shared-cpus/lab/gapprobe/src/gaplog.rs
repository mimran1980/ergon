//! The spin loop's gap log: a fixed-capacity array of `(end ticks, gap
//! ticks)` written before the loop starts, so recording a gap never faults a
//! page in on the hot CPU.
//!
//! A first touch of a fresh page is a minor fault on the CPU that touches
//! it, and it leaves per-CPU state there (the memory cgroup's charge cache,
//! the LRU batch, the vmstat differentials) that the kernel later settles
//! with work queued on that CPU. The log is touched where it is made: on the
//! thread's start CPU, before it pins itself.

use std::hint::black_box;

/// A gap log that holds at most `capacity` gaps and counts the rest.
#[derive(Debug)]
pub struct GapLog {
    slots: Vec<(u64, u64)>,
    len: usize,
    dropped: u64,
}

impl GapLog {
    /// A log of `capacity` gaps, every page of it written once.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        // an opaque value: the writes cannot be folded into a calloc
        slots.resize(capacity, (black_box(0), black_box(0)));
        Self {
            slots: black_box(slots),
            len: 0,
            dropped: 0,
        }
    }

    /// Records a gap; one past the capacity is counted in `dropped` only.
    #[inline]
    pub fn push(&mut self, end: u64, gap: u64) {
        if let Some(s) = self.slots.get_mut(self.len) {
            *s = (end, gap);
            self.len += 1;
        } else {
            self.dropped += 1;
        }
    }

    /// The gaps recorded, in order.
    #[must_use]
    pub fn gaps(&self) -> &[(u64, u64)] {
        self.slots.get(..self.len).unwrap_or(&[])
    }

    /// Gaps over the capacity.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_order_and_counts_what_overflows() {
        let mut g = GapLog::new(2);
        g.push(1, 10);
        g.push(2, 20);
        g.push(3, 30);
        assert_eq!(g.gaps(), &[(1, 10), (2, 20)]);
        assert_eq!(g.dropped(), 1);
        let mut empty = GapLog::new(0);
        empty.push(1, 1);
        assert_eq!(empty.gaps(), &[] as &[(u64, u64)]);
        assert_eq!(empty.dropped(), 1);
    }

    /// The property the log exists for: filling it takes no page fault.
    #[cfg(target_os = "linux")]
    #[test]
    fn filling_the_log_takes_no_page_fault() {
        use std::io::Read;
        // one buffer, touched before the window: a fresh String per read
        // would itself fault between the two counts
        let mut buf = String::with_capacity(4096);
        let mut minflt = || {
            buf.clear();
            let read = std::fs::File::open("/proc/thread-self/stat")
                .and_then(|mut f| f.read_to_string(&mut buf));
            if read.is_err() {
                return u64::MAX;
            }
            crate::procfs::faults(&buf).map_or(u64::MAX, |f| f.0)
        };
        let _ = minflt();
        // 16 MiB: mmap'd, every page fresh
        let cap = 1 << 20;
        let mut g = GapLog::new(cap);
        let before = minflt();
        for i in 0..cap as u64 {
            g.push(i, i);
        }
        let after = minflt();
        assert_eq!(g.gaps().len(), cap);
        assert_eq!(
            after.saturating_sub(before),
            0,
            "minor faults while filling"
        );
    }
}
