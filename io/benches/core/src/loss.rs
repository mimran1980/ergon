//! Loss, reorder and duplicate accounting per stream, from each message's
//! sequence number (from 0, one per message).
//!
//! Each stream keeps a bitmap of the last `window` sequence numbers behind
//! its highest: an arrival in it is a duplicate (seen) or a reorder (a hole
//! filled); older than it, stale (counted, and left lost). Lost is what the
//! stream owed less the distinct sequence numbers that came. Every bitmap is
//! allocated up front, so [`LossTracker::on_message`] never allocates.

use crate::Error;

/// What one arrival was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// The next sequence number.
    InOrder,
    /// Past the next: `missing` sequence numbers skipped.
    Gap {
        /// Sequence numbers skipped over.
        missing: u64,
    },
    /// Behind the highest, filling a hole.
    Reordered,
    /// Seen before.
    Duplicate,
    /// Older than the window: not told apart from a duplicate, left lost.
    Stale,
}

/// One stream's counts, or several summed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LossCounts {
    /// Distinct sequence numbers that came (in the window or in order).
    pub received: u64,
    /// Owed and never came.
    pub lost: u64,
    /// Came behind a later one.
    pub reordered: u64,
    /// Came again.
    pub duplicate: u64,
    /// Came after falling out of the window.
    pub stale: u64,
}

impl LossCounts {
    const fn add(self, o: Self) -> Self {
        Self {
            received: self.received + o.received,
            lost: self.lost + o.lost,
            reordered: self.reordered + o.reordered,
            duplicate: self.duplicate + o.duplicate,
            stale: self.stale + o.stale,
        }
    }
}

#[derive(Debug, Clone)]
struct StreamLoss {
    /// bit `seq % window` is set when `seq` came; only `highest - window + 1
    /// ..= highest` are meaningful
    bits: Vec<u64>,
    /// the next sequence number after the highest seen (0 before any)
    next: u64,
    counts: LossCounts,
}

impl StreamLoss {
    fn bit(seq: u64, window: u64) -> (usize, u64) {
        let i = seq & (window - 1);
        // below `window`, which fits a usize (it came from one)
        (usize::try_from(i >> 6).unwrap_or(0), 1u64 << (i & 63))
    }

    fn seen(&self, seq: u64, window: u64) -> bool {
        let (w, m) = Self::bit(seq, window);
        self.bits.get(w).is_some_and(|b| b & m != 0)
    }

    fn set(&mut self, seq: u64, window: u64, on: bool) {
        let (w, m) = Self::bit(seq, window);
        if let Some(b) = self.bits.get_mut(w) {
            if on {
                *b |= m;
            } else {
                *b &= !m;
            }
        }
    }

    #[inline]
    fn on(&mut self, seq: u64, window: u64) -> Arrival {
        if seq >= self.next {
            let missing = seq - self.next;
            if missing >= window {
                self.bits.fill(0);
            } else {
                for s in self.next..seq {
                    self.set(s, window, false);
                }
            }
            self.set(seq, window, true);
            self.next = seq + 1;
            self.counts.received += 1;
            return if missing == 0 {
                Arrival::InOrder
            } else {
                Arrival::Gap { missing }
            };
        }
        // behind the highest (next - 1)
        if self.next - 1 - seq >= window {
            self.counts.stale += 1;
            return Arrival::Stale;
        }
        if self.seen(seq, window) {
            self.counts.duplicate += 1;
            Arrival::Duplicate
        } else {
            self.set(seq, window, true);
            self.counts.received += 1;
            self.counts.reordered += 1;
            Arrival::Reordered
        }
    }
}

/// Every stream's accounting.
#[derive(Debug, Clone)]
pub struct LossTracker {
    window: u64,
    streams: Vec<StreamLoss>,
}

impl LossTracker {
    /// `streams` streams, each remembering the last `window` sequence numbers
    /// (a power of two, at least 64).
    ///
    /// # Errors
    /// [`Error::Streams`] for no streams, [`Error::Window`] for a bad window.
    pub fn new(streams: u32, window: usize) -> Result<Self, Error> {
        if streams == 0 {
            return Err(Error::Streams { streams: 0, got: 0 });
        }
        if window < 64 || !window.is_power_of_two() {
            return Err(Error::Window(window));
        }
        let stream = StreamLoss {
            bits: vec![0; window / 64],
            next: 0,
            counts: LossCounts::default(),
        };
        Ok(Self {
            window: window as u64,
            streams: (0..streams).map(|_| stream.clone()).collect(),
        })
    }

    fn stream(&self, stream: u32) -> Result<&StreamLoss, Error> {
        self.streams
            .get(stream as usize)
            .ok_or_else(|| self.unknown(stream))
    }

    fn unknown(&self, stream: u32) -> Error {
        Error::Stream {
            stream,
            streams: u32::try_from(self.streams.len()).unwrap_or(u32::MAX),
        }
    }

    /// Message `seq` of `stream` arrived.
    ///
    /// # Errors
    /// [`Error::Stream`] for a stream past the tracker's.
    #[inline]
    pub fn on_message(&mut self, stream: u32, seq: u64) -> Result<Arrival, Error> {
        let window = self.window;
        let streams = u32::try_from(self.streams.len()).unwrap_or(u32::MAX);
        let s = self
            .streams
            .get_mut(stream as usize)
            .ok_or(Error::Stream { stream, streams })?;
        Ok(s.on(seq, window))
    }

    /// `stream`'s counts at the end, when it owed sequence numbers
    /// `0..expected`.
    ///
    /// # Errors
    /// [`Error::Stream`] for a stream past the tracker's.
    pub fn finish(&self, stream: u32, expected: u64) -> Result<LossCounts, Error> {
        let c = self.stream(stream)?.counts;
        Ok(LossCounts {
            lost: expected.saturating_sub(c.received),
            ..c
        })
    }

    /// Every stream's [`finish`](Self::finish) summed; `expected[i]` is what
    /// stream `i` owed.
    ///
    /// # Errors
    /// [`Error::Streams`] when `expected` does not have one value per stream.
    pub fn totals(&self, expected: &[u64]) -> Result<LossCounts, Error> {
        if expected.len() != self.streams.len() {
            return Err(Error::Streams {
                streams: u32::try_from(self.streams.len()).unwrap_or(u32::MAX),
                got: expected.len(),
            });
        }
        let mut sum = LossCounts::default();
        for (i, &e) in (0u32..).zip(expected) {
            sum = sum.add(self.finish(i, e)?);
        }
        Ok(sum)
    }
}
