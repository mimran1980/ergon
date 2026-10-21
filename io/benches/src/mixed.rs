//! The traffic of many connections at once (`replay_server --mode mixed`):
//! every connection slot gets a trickle of messages at its own random
//! interval, and every so often a random subset of slots each gets a burst.
//! The schedule is deterministic for a seed, so every backend measured sees
//! the same traffic.

/// One message to send: when (ns from the start) and to which slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Send {
    /// Due time, ns from the start.
    pub at_ns: i64,
    /// The connection slot it goes to.
    pub slot: usize,
}

/// The schedule's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// Connection slots.
    pub slots: usize,
    /// Each slot's mean trickle interval is drawn once, uniformly between
    /// these (µs); every gap is that mean ±50%.
    pub trickle_min_us: i64,
    /// See [`Shape::trickle_min_us`].
    pub trickle_max_us: i64,
    /// Every this many µs, a burst ...
    pub burst_every_us: i64,
    /// ... to this many distinct random slots ...
    pub burst_slots: usize,
    /// ... of this many messages each, back to back.
    pub burst_len: usize,
    /// The random seed.
    pub seed: u64,
}

/// The messages of a [`Shape`] in time order (an endless iterator).
#[derive(Debug, Clone)]
pub struct Schedule {
    shape: Shape,
    rng: u64,
    mean_ns: Vec<i64>,
    next_ns: Vec<i64>,
    next_burst_ns: i64,
    /// Burst messages still to send at `burst_at_ns`, in order.
    burst: std::collections::VecDeque<usize>,
    burst_at_ns: i64,
}

impl Schedule {
    /// A schedule for `shape` (at least one slot).
    #[must_use]
    pub fn new(shape: Shape) -> Self {
        let slots = shape.slots.max(1);
        let mut s = Self {
            shape: Shape { slots, ..shape },
            // xorshift needs a non-zero state.
            rng: shape.seed | 1,
            mean_ns: Vec::with_capacity(slots),
            next_ns: Vec::with_capacity(slots),
            next_burst_ns: shape.burst_every_us * 1000,
            burst: std::collections::VecDeque::new(),
            burst_at_ns: 0,
        };
        let (lo, hi) = (
            shape.trickle_min_us,
            shape.trickle_max_us.max(shape.trickle_min_us),
        );
        for _ in 0..slots {
            let mean = (lo + s.below(u64::try_from(hi - lo + 1).unwrap_or(1))) * 1000;
            s.mean_ns.push(mean.max(1000));
        }
        for i in 0..slots {
            // Spread the first messages over one interval.
            let first = s.below(u64::try_from(s.mean_ns[i]).unwrap_or(1));
            s.next_ns.push(first);
        }
        s
    }

    /// xorshift64*.
    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: u64) -> i64 {
        i64::try_from(self.rand() % n.max(1)).unwrap_or(0)
    }

    /// Each slot's mean trickle interval, ns.
    #[must_use]
    pub fn trickle_means_ns(&self) -> &[i64] {
        &self.mean_ns
    }
}

impl Iterator for Schedule {
    type Item = Send;

    fn next(&mut self) -> Option<Send> {
        if let Some(slot) = self.burst.pop_front() {
            return Some(Send {
                at_ns: self.burst_at_ns,
                slot,
            });
        }
        let (j, t) = self
            .next_ns
            .iter()
            .copied()
            .enumerate()
            .min_by_key(|&(_, t)| t)?;
        if self.shape.burst_every_us > 0
            && self.shape.burst_slots > 0
            && self.shape.burst_len > 0
            && self.next_burst_ns <= t
        {
            // Distinct slots: a partial Fisher-Yates over the slot indices.
            let mut pick: Vec<usize> = (0..self.shape.slots).collect();
            let k = self.shape.burst_slots.min(self.shape.slots);
            for i in 0..k {
                let r = usize::try_from(self.below((self.shape.slots - i) as u64)).unwrap_or(0);
                pick.swap(i, i + r);
            }
            for &slot in &pick[..k] {
                for _ in 0..self.shape.burst_len {
                    self.burst.push_back(slot);
                }
            }
            self.burst_at_ns = self.next_burst_ns;
            self.next_burst_ns += self.shape.burst_every_us * 1000;
            return self.next();
        }
        // The next gap: the slot's mean ±50%.
        let mean = self.mean_ns[j];
        let gap = mean / 2 + self.below(u64::try_from(mean).unwrap_or(1));
        self.next_ns[j] = t + gap.max(1);
        Some(Send { at_ns: t, slot: j })
    }
}
