//! One clock's parameters shared with other threads: a seqlock in safe
//! atomics, one writer (the clock's owner, after each re-anchor) and any
//! number of readers, none of which ever blocks.
//!
//! The owner disciplines its clock and publishes each new segment
//! ([`ClockPublisher::publish`]); a thread that stamps with the same clock
//! keeps a [`ClockFollower`], whose [`clock`](ClockFollower::clock) costs
//! one acquire load and a compare while nothing changed, and adopts the new
//! parameters (cold: a check and a division, [`Clock::adopt`]) once per
//! change. A follower that reads while a segment is being written keeps
//! its previous one: stale by at most one re-anchor, so its reads are
//! within a nanosecond plus the rate change over the owner's window
//! (anchor read to publish) of the owner's, and never ahead of them by a
//! step.
//!
//! The memory order is Boehm's ("Can seqlocks get along with programming
//! language memory models?", 2012): the writer bumps the sequence to odd,
//! a release fence, the words relaxed, the sequence to even with release;
//! the reader loads the sequence with acquire, the words relaxed, an
//! acquire fence, the sequence again relaxed, and keeps the words only when
//! both loads saw the same even value. The caller allocates the
//! [`SharedClock`] (a `static`, or one leaked per owner): nothing here is
//! global.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering, fence};

use crate::clock::Clock;
use crate::params::{PARAMS_VERSION, Params};

/// Tries a follower makes while a segment is being written before it
/// keeps its cached clock: a writer preempted mid-write never blocks a
/// reader.
const READ_TRIES: u32 = 64;

/// A clock's parameters, shared: a sequence and five words (kind and
/// domain, base ticks, base ns, factor, rate). 56 bytes and a flag; `Sync`.
#[derive(Debug)]
pub struct SharedClock {
    /// Even: stable; odd: being written; 0: never published.
    seq: AtomicU64,
    /// `kind << 32 | domain`, `base_ticks`, `base_ns`, `mult`, `hz`.
    words: [AtomicU64; 5],
    /// A publisher exists.
    claimed: AtomicBool,
}

impl SharedClock {
    /// Nothing published yet: followers keep the clock they were made with.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            seq: AtomicU64::new(0),
            words: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            claimed: AtomicBool::new(false),
        }
    }

    /// How many times a clock has been published (the sequence over two;
    /// a write in progress counts).
    #[must_use]
    pub fn versions(&self) -> u64 {
        self.seq.load(Ordering::Acquire).div_ceil(2)
    }
}

impl Default for SharedClock {
    fn default() -> Self {
        Self::new()
    }
}

/// The one writer of a [`SharedClock`]: not `Clone`, and only one can be
/// made per shared clock.
#[derive(Debug)]
pub struct ClockPublisher<'a> {
    shared: &'a SharedClock,
}

impl<'a> ClockPublisher<'a> {
    /// The writer of `shared`, or `None` where it already has one (two
    /// writers would interleave their words).
    #[must_use]
    pub fn new(shared: &'a SharedClock) -> Option<Self> {
        shared
            .claimed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self { shared })
    }

    /// The writer of a shared clock this crate just made and nobody else
    /// has seen: claimed, with no `Option` to unwrap.
    pub(crate) fn claimed(shared: &'a SharedClock) -> Self {
        shared.claimed.store(true, Ordering::Release);
        Self { shared }
    }

    /// Publish `clock`'s parameters: five relaxed stores between two
    /// sequence stores and a release fence. Never blocks; a follower that
    /// reads meanwhile keeps its previous clock.
    pub fn publish(&mut self, clock: &Clock) {
        let p = clock.params();
        let words = [
            (u64::from(p.kind) << 32) | u64::from(p.domain),
            p.base_ticks,
            p.base_ns,
            p.mult,
            p.hz,
        ];
        let shared = self.shared;
        let s = shared.seq.load(Ordering::Relaxed);
        shared.seq.store(s.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        for (slot, word) in shared.words.iter().zip(words) {
            slot.store(word, Ordering::Relaxed);
        }
        shared.seq.store(s.wrapping_add(2), Ordering::Release);
    }
}

/// A thread's copy of a shared clock, adopting each published segment.
#[derive(Clone, Debug)]
pub struct ClockFollower<'a> {
    shared: &'a SharedClock,
    /// The sequence the cached clock is from (0: the initial clock).
    seq: u64,
    clock: Clock,
}

impl<'a> ClockFollower<'a> {
    /// A follower of `shared`, starting from `initial` (the owner's clock
    /// when the follower is made) until something is published.
    #[must_use]
    pub const fn new(shared: &'a SharedClock, initial: Clock) -> Self {
        Self {
            shared,
            seq: 0,
            clock: initial,
        }
    }

    /// The clock as last published: one acquire load and a compare when
    /// nothing changed; otherwise the new parameters, adopted (cold).
    #[inline]
    pub fn clock(&mut self) -> &Clock {
        let seq = sequence(self.shared);
        if seq != self.seq {
            self.refresh(seq);
        }
        &self.clock
    }

    /// The clock as cached, without looking for a newer one.
    #[must_use]
    pub const fn cached(&self) -> &Clock {
        &self.clock
    }

    /// [`newer`] the cached clock, kept where nothing newer was read.
    #[cold]
    #[inline(never)]
    fn refresh(&mut self, s1: u64) {
        if let Some((seq, clock)) = newer(self.shared, self.seq, s1, &self.clock) {
            self.seq = seq;
            self.clock = clock;
        }
    }
}

/// The sequence now (acquire): what a follower compares with its own on
/// every read.
#[inline]
pub(crate) fn sequence(shared: &SharedClock) -> u64 {
    shared.seq.load(Ordering::Acquire)
}

/// The version after `cached`, from a sequence `s1` just loaded (acquire)
/// that differs from it: the published words read consistently and
/// adopted from `clock` ([`Clock::adopt`]; parameters this process cannot
/// use leave `clock` as it is, under the new sequence). `None` where every
/// one of [`READ_TRIES`] tries met a write in progress, or the sequence came
/// back to `cached`: the caller keeps its clock (stale, valid, never
/// blocked). The one seqlock read every follower shares.
pub(crate) fn newer(
    shared: &SharedClock,
    cached: u64,
    mut s1: u64,
    clock: &Clock,
) -> Option<(u64, Clock)> {
    for _ in 0..READ_TRIES {
        if s1 & 1 == 0 {
            let words: [u64; 5] = core::array::from_fn(|i| shared.words[i].load(Ordering::Relaxed));
            fence(Ordering::Acquire);
            let s2 = shared.seq.load(Ordering::Relaxed);
            if s1 == s2 {
                return Some((s1, adopted(clock, words)));
            }
        }
        core::hint::spin_loop();
        s1 = shared.seq.load(Ordering::Acquire);
        if s1 == cached {
            return None;
        }
    }
    None
}

/// `clock` adopting one consistent version's words; parameters this
/// process cannot use are skipped: `clock` is kept.
fn adopted(clock: &Clock, words: [u64; 5]) -> Clock {
    #[allow(clippy::cast_possible_truncation)] // the two halves, on purpose
    let p = Params {
        version: PARAMS_VERSION,
        kind: (words[0] >> 32) as u32,
        domain: words[0] as u32,
        reserved: 0,
        base_ticks: words[1],
        base_ns: words[2],
        mult: words[3],
        hz: words[4],
    };
    clock.adopt(&p).unwrap_or(*clock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{CounterClock, Domain};
    use crate::detect::OrderedRead;
    use crate::scale::Scale;
    use std::sync::atomic::AtomicBool;

    type Res = Result<(), Box<dyn std::error::Error>>;

    fn xorshift(x: &mut u64) -> u64 {
        *x ^= *x >> 12;
        *x ^= *x << 25;
        *x ^= *x >> 27;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn clock_at(base_ticks: u64, base_ns: u64) -> Result<Clock, Box<dyn std::error::Error>> {
        let scale = Scale::from_hz(1_000_000_000, base_ticks, base_ns).ok_or("rate")?;
        Ok(Clock::Counter(CounterClock::new(
            scale,
            Domain::Realtime,
            OrderedRead::Cntvctss,
        )))
    }

    /// A follower keeps its initial clock until something is published,
    /// then adopts each version (keeping its own ordered read); a shared
    /// clock has one publisher.
    #[test]
    fn a_follower_adopts_each_published_clock() -> Res {
        let shared = SharedClock::new();
        let initial = clock_at(0, 1_000)?;
        let mut follower = ClockFollower::new(&shared, initial);
        assert_eq!(*follower.clock(), initial, "nothing published yet");
        let mut publisher = ClockPublisher::new(&shared).ok_or("claimed")?;
        assert!(ClockPublisher::new(&shared).is_none(), "one writer");
        let next = initial.retuned(500, (1 << 32) + 4_000, 7).ok_or("retune")?;
        publisher.publish(&next);
        assert_eq!(*follower.clock(), next);
        assert_eq!(shared.versions(), 1);
        let after = next.retuned(900, (1 << 32) - 4_000, 0).ok_or("retune")?;
        publisher.publish(&after);
        assert_eq!(*follower.clock(), after);
        assert_eq!(*follower.cached(), after);
        Ok(())
    }

    /// A writer that stopped mid-write (the sequence odd, the words half
    /// written) never blocks a follower and is never adopted: after 64
    /// tries the cached clock comes back.
    #[test]
    fn a_write_in_progress_leaves_the_cached_clock() -> Res {
        let shared = SharedClock::new();
        let initial = clock_at(0, 1_000)?;
        let mut follower = ClockFollower::new(&shared, initial);
        let mut publisher = ClockPublisher::new(&shared).ok_or("claimed")?;
        let next = initial.retuned(500, (1 << 32) + 4_000, 7).ok_or("retune")?;
        publisher.publish(&next);
        assert_eq!(*follower.clock(), next);
        // a writer preempted between its two sequence stores, its words
        // half written: a valid clock's words, so adopting them would show
        let torn = clock_at(77, 99)?.params();
        shared.seq.store(3, Ordering::Release);
        shared.words[1].store(torn.base_ticks, Ordering::Relaxed);
        shared.words[2].store(torn.base_ns, Ordering::Relaxed);
        assert_eq!(
            *follower.clock(),
            next,
            "the cached clock, not the half-written one"
        );
        assert_eq!(*follower.clock(), next, "and again: never blocked");
        Ok(())
    }

    /// P12: a writer publishes a chain of 10^6 random segments, each
    /// anchored where the last reads at its anchor (`B = prev.to_ns(b)`,
    /// plus a forward step), while 3 followers read. Every read's segment is
    /// one that was published, whole (no torn words); and each follower's
    /// reads at a shared, advancing tick never go back by more than 1 ns
    /// plus the rate change over the ticks between the read and the anchor
    /// it had not yet seen (the owner's window).
    #[test]
    fn followers_never_see_a_torn_segment() -> Res {
        const VERSIONS: usize = 1_000_000;
        let shared = SharedClock::new();
        let tick = AtomicU64::new(1_000_000);
        let done = AtomicBool::new(false);
        let first = clock_at(1_000_000, 1_760_000_000_000_000_000)?;
        let chain = std::thread::scope(|scope| -> Result<Vec<Clock>, String> {
            let readers: Vec<_> = (0..3_u64)
                .map(|r| {
                    let (shared, tick, done) = (&shared, &tick, &done);
                    scope.spawn(move || {
                        let mut follower = ClockFollower::new(shared, first);
                        let mut seen = Vec::with_capacity(1 << 16);
                        let mut last = (first, 0_u64);
                        let mut spins = r;
                        while !done.load(Ordering::Acquire) {
                            let clock = *follower.clock();
                            let t = tick.load(Ordering::Acquire);
                            if clock != last.0 || spins % 64 == 0 {
                                seen.push((clock, t));
                                last = (clock, t);
                            }
                            spins += 1;
                        }
                        seen
                    })
                })
                .collect();
            let mut publisher = ClockPublisher::new(&shared).ok_or("claimed")?;
            let mut chain = Vec::with_capacity(VERSIONS + 1);
            chain.push(first);
            let mut x = 0x1234_5678_9abc_def1_u64;
            let mut clock = first;
            for _ in 0..VERSIONS {
                let at = tick.load(Ordering::Relaxed) + 1 + xorshift(&mut x) % 2_000;
                tick.store(at, Ordering::Release);
                let mult = (1 << 32) - (1 << 21) + xorshift(&mut x) % (1 << 22);
                let forward = if xorshift(&mut x).is_multiple_of(8) {
                    xorshift(&mut x) % 1_000
                } else {
                    0
                };
                clock = clock.retuned(at, mult, forward).ok_or("retune")?;
                publisher.publish(&clock);
                chain.push(clock);
            }
            done.store(true, Ordering::Release);
            let mut all = Vec::new();
            for reader in readers {
                all.push(reader.join().map_err(|_| "a reader panicked")?);
            }
            for (r, seen) in all.iter().enumerate() {
                check_reads(&chain, seen).map_err(|e| format!("reader {r}: {e}"))?;
            }
            Ok(chain)
        })?;
        assert_eq!(chain.len(), VERSIONS + 1);
        Ok(())
    }

    /// Each read's clock is a published version, and consecutive reads go
    /// back by at most 1 ns plus the stale segments' rate differences over
    /// the ticks from the anchors they missed.
    fn check_reads(chain: &[Clock], seen: &[(Clock, u64)]) -> Result<(), String> {
        let index = |c: &Clock| -> Result<usize, String> {
            let b = c.params().base_ticks;
            let i = chain.partition_point(|k| k.params().base_ticks < b);
            match chain.get(i) {
                Some(k) if k == c => Ok(i),
                _ => Err(format!("a torn or unpublished segment: {c:?}")),
            }
        };
        let mut prev: Option<(usize, u64, u64)> = None;
        for (clock, t) in seen {
            let i = index(clock)?;
            let v = clock.to_ns(*t);
            if let Some((j, t0, v0)) = prev {
                if i < j {
                    return Err(format!("an older segment after a newer one: {j} then {i}"));
                }
                if v < v0 {
                    // allowed: the earlier read's segment was stale at t0
                    let mut slack = 1_u128;
                    for k in j + 1..=i {
                        let a = chain[k].params().base_ticks;
                        let dm = chain[k].params().mult.abs_diff(chain[k - 1].params().mult);
                        slack += ((u128::from(t0.saturating_sub(a)) * u128::from(dm)) >> 32) + 1;
                    }
                    if u128::from(v0 - v) > slack {
                        return Err(format!("back by {} ns, allowed {slack}", v0 - v));
                    }
                }
            }
            prev = Some((i, *t, v));
        }
        Ok(())
    }
}
