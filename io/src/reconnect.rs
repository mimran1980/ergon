//! Reopening connections with backoff: [`Reconnect`].
//!
//! - Optional. The application owns it (typically in its handler) and feeds
//!   it every `on_open` and `on_closed`; it starts each due attempt from
//!   [`Reconnect::poll`], once per loop pass.
//! - `Driver::close` reports nothing: after it, call
//!   [`Reconnect::on_closed`] yourself (to force a reconnect, say).
//! - Each link has a dial factory: a fresh dial per attempt. On every
//!   backend, through `Driver::connect_background` ([`Retry::background`])
//!   or `Driver::connect`.
//! - Backoff: `initial_ms`, times `multiplier` per failed attempt, up to
//!   `max_ms`, each shortened by up to `jitter` (deterministic for a fixed
//!   `seed`). A connection open for `stable_ms` resets it.
//! - The application stays in control: [`Reconnect::on_open`] says whether
//!   a connection replaces an earlier one (resubscribe, resync a snapshot);
//!   [`Reconnect::stop`] vetoes further attempts, [`Reconnect::resume`]
//!   starts again.
//! - A venue's connect limit: [`Reconnect::poll_gated`] starts an attempt
//!   only when a gate passes it, typically one [`crate::limit::Budget`]
//!   bucket shared by every link on the IP. Fifty links added at once then
//!   start at the gate's pace, not in one burst.
//! - Several source IPs, each with its own connect budget:
//!   [`Reconnect::poll_sourced`] binds each attempt to an IP that
//!   [`SourceIps`] picks and charges, and waits for the soonest one when
//!   none has room; [`Reconnect::source`] names the IP.
//! - The server's wait: [`Reconnect::defer`] keeps a link from connecting
//!   before a time (a refused handshake's `Retry-After`, through
//!   [`crate::outcome::refused_upgrade`]), whatever the backoff says.
//! - Cost: `poll` is one branch while nothing waits. While a link waits, it
//!   reads the counter once per call and the clock every 250 µs.

use std::net::IpAddr;

use crate::clock::{monotonic_ns, rdtsc, ticks_per_us};
use crate::endpoint::ConnId;
use crate::error::{Error, Result};
use crate::limit::{Budget, SourceIps};
use crate::net::BindSource;

/// The backoff policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Retry {
    /// First delay after a close, ms. At least 1.
    pub initial_ms: u32,
    /// Longest delay, ms.
    pub max_ms: u32,
    /// Each failed attempt multiplies the delay by this (at least 1).
    pub multiplier: f64,
    /// Each delay is shortened by up to this fraction, at random (0 to 1).
    pub jitter: f64,
    /// Failed attempts in a row before giving up; `0`: never give up.
    pub max_attempts: u32,
    /// A connection open this long resets the backoff, ms.
    pub stable_ms: u32,
    /// Jitter seed; `0` picks one at random. A fixed seed gives the same
    /// delays every run (tests).
    pub seed: u64,
    /// `true`: connect with `Driver::connect_background` (the handshake off
    /// the loop); `false`: `Driver::connect`. In a process that may use the
    /// polling CPU only (a pod with one CPU, [`crate::placement`]),
    /// `connect_background` refuses and the link gives up
    /// ([`LinkState::GaveUp`]), unless the driver's
    /// `Config::inline_connect_fallback` makes it connect on the loop.
    pub background: bool,
}

impl Default for Retry {
    fn default() -> Self {
        Self {
            initial_ms: 250,
            max_ms: 30_000,
            multiplier: 2.0,
            jitter: 0.2,
            max_attempts: 0,
            stable_ms: 60_000,
            seed: 0,
            background: true,
        }
    }
}

impl Retry {
    fn check(&self) -> Result<()> {
        if self.initial_ms == 0 || self.max_ms < self.initial_ms {
            return Err(Error::Config(format!(
                "retry: initial_ms {} must be at least 1 and at most max_ms {}",
                self.initial_ms, self.max_ms
            )));
        }
        if !(self.multiplier >= 1.0 && self.multiplier.is_finite()) {
            return Err(Error::Config(format!(
                "retry: multiplier {} must be at least 1",
                self.multiplier
            )));
        }
        if !(0.0..=1.0).contains(&self.jitter) {
            return Err(Error::Config(format!(
                "retry: jitter {} must be 0 to 1",
                self.jitter
            )));
        }
        Ok(())
    }

    /// The delay before attempt `attempt` (1 for the first retry), ms;
    /// `r` in `[0, 1)` picks the jitter.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn delay_ms(&self, attempt: u32, r: f64) -> u64 {
        let exp = i32::try_from(attempt.saturating_sub(1)).unwrap_or(i32::MAX);
        let base =
            (f64::from(self.initial_ms) * self.multiplier.powi(exp)).min(f64::from(self.max_ms));
        (base * self.jitter.mul_add(-r, 1.0)).round().max(1.0) as u64
    }
}

/// Starts a connection: implemented by `Driver` (Linux), or a stand-in.
pub trait Connector<D> {
    /// Starts connecting `dial` and returns the connection's id.
    ///
    /// # Errors
    /// The connect could not be started.
    fn start(&mut self, dial: D, background: bool) -> Result<ConnId>;
}

#[cfg(target_os = "linux")]
impl<E, D> Connector<D> for crate::driver::Driver<E>
where
    E: crate::endpoint::Endpoint + Send + 'static,
    D: crate::endpoint::Dial<Conn = E> + Send + 'static,
{
    fn start(&mut self, dial: D, background: bool) -> Result<ConnId> {
        if background {
            self.connect_background(dial)
        } else {
            self.connect(dial)
        }
    }
}

/// A link: one logical connection that [`Reconnect`] keeps open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkId(usize);

impl LinkId {
    /// The link's index, in the order links were added.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Where a link is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// Waiting to connect at `at_ns` (`CLOCK_MONOTONIC`); `attempt` failed
    /// attempts in a row so far.
    Waiting {
        /// When the next attempt starts.
        at_ns: i64,
        /// Failed attempts in a row.
        attempt: u32,
    },
    /// Connecting.
    Connecting(ConnId),
    /// Open.
    Open(ConnId),
    /// Stopped by [`Reconnect::stop`].
    Stopped,
    /// Gave up after [`Retry::max_attempts`], or at once when the driver has
    /// no CPU for `connect_background`'s helper ([`Error::Cpu`]: a pod with
    /// one CPU, without `Config::inline_connect_fallback`; one warning line
    /// says so). [`Reconnect::resume`] tries again.
    GaveUp,
}

/// A link's connection opened ([`Reconnect::on_open`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opened {
    /// The link.
    pub link: LinkId,
    /// An earlier connection of this link was open: state from it may be
    /// stale (resync a snapshot). Subscribe either way.
    pub reconnect: bool,
}

/// What follows a link's close ([`Reconnect::on_closed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closed {
    /// The next attempt starts after `delay_ms`.
    Retry {
        /// The link.
        link: LinkId,
        /// Failed attempts in a row, this one included (1 after a close).
        attempt: u32,
        /// Delay before the attempt, ms: the backoff, or longer when
        /// [`Reconnect::defer`] asked for it.
        delay_ms: u64,
    },
    /// [`Retry::max_attempts`] failed in a row: the link stops.
    GaveUp {
        /// The link.
        link: LinkId,
        /// The attempts made.
        attempts: u32,
    },
    /// The link was stopped ([`Reconnect::stop`]): nothing is scheduled.
    Stopped {
        /// The link.
        link: LinkId,
    },
}

type Factory<D> = Box<dyn FnMut() -> Result<D>>;

struct Link<D> {
    make: Factory<D>,
    state: LinkState,
    /// Failed attempts in a row.
    attempts: u32,
    /// When the current connection opened.
    opened_ns: i64,
    /// Any connection of this link has been open.
    was_open: bool,
    stop: bool,
    /// No attempt before this ([`Reconnect::defer`]); `i64::MIN`: none.
    not_before: i64,
    /// The source IP its latest attempt was bound to
    /// ([`Reconnect::poll_sourced`]).
    source: Option<IpAddr>,
}

/// Keeps links open: reconnects each with backoff after it closes.
pub struct Reconnect<D> {
    retry: Retry,
    links: Vec<Link<D>>,
    rng: u64,
    /// Links in [`LinkState::Waiting`]: `poll` does nothing while 0.
    waiting: usize,
    next_check: u64,
    check_ticks: u64,
    /// Where the next pass starts: after the last link a gate passed, so
    /// no link starves behind others.
    cursor: usize,
}

/// How often a waiting link's due time is checked, µs.
const CHECK_US: f64 = 250.0;

impl<D> Reconnect<D> {
    /// A helper with no links.
    ///
    /// # Errors
    /// A policy out of range: `initial_ms` 0 or above `max_ms`,
    /// `multiplier` below 1, `jitter` outside 0 to 1.
    pub fn new(retry: Retry) -> Result<Self> {
        retry.check()?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let check_ticks = (CHECK_US * ticks_per_us()) as u64;
        Ok(Self {
            retry,
            links: Vec::new(),
            rng: rng_state(retry.seed)?,
            waiting: 0,
            next_check: 0,
            check_ticks,
            cursor: 0,
        })
    }

    /// Adds a link; `make` builds a fresh dial for each attempt. The first
    /// attempt starts on the next `poll`.
    pub fn add(&mut self, make: impl FnMut() -> Result<D> + 'static) -> LinkId {
        self.links.push(Link {
            make: Box::new(make),
            state: LinkState::Waiting {
                at_ns: i64::MIN,
                attempt: 0,
            },
            attempts: 0,
            opened_ns: 0,
            was_open: false,
            stop: false,
            not_before: i64::MIN,
            source: None,
        });
        self.waiting += 1;
        LinkId(self.links.len() - 1)
    }

    /// The source IP `link`'s latest attempt was bound to by
    /// [`Reconnect::poll_sourced`]: charge requests on its connection to
    /// that IP's [`SourceIp::sends`](crate::limit::SourceIp::sends).
    /// `None` before its first sourced attempt, or for an unknown link.
    #[must_use]
    pub fn source(&self, link: LinkId) -> Option<IpAddr> {
        self.links.get(link.0).and_then(|l| l.source)
    }

    /// Where `link` is; `None` for an unknown link.
    #[must_use]
    pub fn state(&self, link: LinkId) -> Option<LinkState> {
        self.links.get(link.0).map(|l| l.state)
    }

    /// The link `id` belongs to, while it is connecting or open.
    #[must_use]
    pub fn link_of(&self, id: ConnId) -> Option<LinkId> {
        self.links
            .iter()
            .position(
                |l| matches!(l.state, LinkState::Connecting(c) | LinkState::Open(c) if c == id),
            )
            .map(LinkId)
    }

    /// Feed `Handler::on_open` here. `Some` when `id` is one of the links.
    pub fn on_open(&mut self, id: ConnId) -> Option<Opened> {
        self.on_open_at(id, monotonic_ns())
    }

    /// [`Reconnect::on_open`] at `now_ns` (`CLOCK_MONOTONIC`).
    pub fn on_open_at(&mut self, id: ConnId, now_ns: i64) -> Option<Opened> {
        let link = self.link_of(id)?;
        let l = &mut self.links[link.0];
        l.state = LinkState::Open(id);
        l.opened_ns = now_ns;
        let reconnect = l.was_open;
        l.was_open = true;
        Some(Opened { link, reconnect })
    }

    /// Feed `Handler::on_closed` here (connects that failed included), and
    /// call it after `Driver::close`, which reports nothing.
    /// Schedules the next attempt unless the link was stopped or gave up.
    /// `None` when `id` is not one of the links.
    pub fn on_closed(&mut self, id: ConnId) -> Option<Closed> {
        self.on_closed_at(id, monotonic_ns())
    }

    /// [`Reconnect::on_closed`] at `now_ns` (`CLOCK_MONOTONIC`).
    pub fn on_closed_at(&mut self, id: ConnId, now_ns: i64) -> Option<Closed> {
        let link = self.link_of(id)?;
        let stable_ns = i64::from(self.retry.stable_ms) * 1_000_000;
        let l = &mut self.links[link.0];
        if matches!(l.state, LinkState::Open(_)) && now_ns - l.opened_ns >= stable_ns {
            l.attempts = 0;
        }
        if l.stop {
            l.state = LinkState::Stopped;
            return Some(Closed::Stopped { link });
        }
        Some(self.schedule(link, now_ns))
    }

    /// Stops `link`: no more attempts. A connection still open stays open;
    /// its close is then [`Closed::Stopped`]. To close it, use
    /// `WsConn::close`, or `Driver::close` and then call
    /// [`Reconnect::on_closed`] yourself: otherwise the link stays `Open`
    /// and [`Reconnect::resume`] cannot restart it.
    pub fn stop(&mut self, link: LinkId) {
        let Some(l) = self.links.get_mut(link.0) else {
            return;
        };
        l.stop = true;
        if matches!(l.state, LinkState::Waiting { .. }) {
            l.state = LinkState::Stopped;
            self.waiting -= 1;
        }
    }

    /// Starts a stopped or given-up link again, with no backoff: it connects
    /// on the next `poll` (not before a [`Reconnect::defer`] ends).
    pub fn resume(&mut self, link: LinkId) {
        let Some(l) = self.links.get_mut(link.0) else {
            return;
        };
        l.stop = false;
        if matches!(l.state, LinkState::Stopped | LinkState::GaveUp) {
            l.attempts = 0;
            l.state = LinkState::Waiting {
                at_ns: l.not_before,
                attempt: 0,
            };
            self.waiting += 1;
            self.next_check = 0;
        }
    }

    /// Starts every attempt that is due; call once per loop pass. Returns
    /// the attempts started. One branch while no link waits.
    #[inline]
    pub fn poll(&mut self, c: &mut impl Connector<D>) -> usize {
        if self.waiting == 0 {
            return 0;
        }
        let t = rdtsc();
        if t < self.next_check {
            return 0;
        }
        self.next_check = t.wrapping_add(self.check_ticks);
        self.poll_at(c, monotonic_ns())
    }

    /// [`Reconnect::poll`] at `now_ns` (`CLOCK_MONOTONIC`), every time.
    /// A dial that cannot be built or started counts as a failed attempt.
    #[cold]
    pub fn poll_at(&mut self, c: &mut impl Connector<D>, now_ns: i64) -> usize {
        self.poll_at_gated(c, now_ns, || Ok(()))
    }

    /// [`Reconnect::poll`], but each attempt must pass `gate` first, the
    /// first attempt after [`Reconnect::add`] included. Use it for a venue's
    /// limit on connect attempts per IP: one gate (one
    /// [`crate::limit::Budget`] bucket) for every link on that IP.
    ///
    /// - `gate` answers `Ok(())` to connect now, or `Err(wait_ns)`: the link
    ///   asks again after that wait, at most [`Retry::max_ms`] later (a
    ///   halted budget answers about 292 years; asking again restarts the
    ///   link soon after a person clears the halt).
    /// - A refusal is not a failed attempt: the backoff does not grow, and
    ///   [`Retry::max_attempts`] does not count it.
    /// - Each pass starts at the link after the last one the gate passed, so
    ///   links that keep failing cannot starve the others.
    /// - `gate` is called only for a link that is due: never on a pass with
    ///   nothing to start.
    ///
    /// A gate over a budget, with `now` the budget's clock:
    /// `|| budget.try_charge(&[connects], Class::Normal, now).map(drop).map_err(|l| l.wait_ns(now))`.
    /// The gate returns a wait, not a time, so the budget's clock need not
    /// be `CLOCK_MONOTONIC`.
    #[inline]
    pub fn poll_gated(
        &mut self,
        c: &mut impl Connector<D>,
        gate: impl FnMut() -> std::result::Result<(), u64>,
    ) -> usize {
        if self.waiting == 0 {
            return 0;
        }
        let t = rdtsc();
        if t < self.next_check {
            return 0;
        }
        self.next_check = t.wrapping_add(self.check_ticks);
        self.poll_at_gated(c, monotonic_ns(), gate)
    }

    /// [`Reconnect::poll_gated`] at `now_ns` (`CLOCK_MONOTONIC`), every
    /// time.
    #[cold]
    pub fn poll_at_gated(
        &mut self,
        c: &mut impl Connector<D>,
        now_ns: i64,
        mut gate: impl FnMut() -> std::result::Result<(), u64>,
    ) -> usize {
        self.run(c, now_ns, || gate().map(|()| None), |_, _| {})
    }

    /// [`Reconnect::poll`], with every attempt from a source IP that
    /// [`SourceIps::try_connect`] picks and charges in `budget` at
    /// `budget_now_ns` (the budget's clock): the connect gate for a venue
    /// that limits connects per IP, with several IPs each on its own
    /// bucket. The dial is bound to the pick ([`BindSource`]) and
    /// [`Reconnect::source`] names it.
    ///
    /// - No IP has room: the link waits for the soonest one, at most
    ///   [`Retry::max_ms`]. As with [`Reconnect::poll_gated`], a refusal is
    ///   not a failed attempt.
    /// - A dial that cannot be built or started has still spent its
    ///   connect unit: the count errs on the safe side.
    #[inline]
    pub fn poll_sourced(
        &mut self,
        c: &mut impl Connector<D>,
        ips: &mut SourceIps,
        budget: &mut Budget,
        budget_now_ns: i64,
    ) -> usize
    where
        D: BindSource,
    {
        if self.waiting == 0 {
            return 0;
        }
        let t = rdtsc();
        if t < self.next_check {
            return 0;
        }
        self.next_check = t.wrapping_add(self.check_ticks);
        self.poll_at_sourced(c, monotonic_ns(), ips, budget, budget_now_ns)
    }

    /// [`Reconnect::poll_sourced`] at `now_ns` (`CLOCK_MONOTONIC`), every
    /// time.
    #[cold]
    pub fn poll_at_sourced(
        &mut self,
        c: &mut impl Connector<D>,
        now_ns: i64,
        ips: &mut SourceIps,
        budget: &mut Budget,
        budget_now_ns: i64,
    ) -> usize
    where
        D: BindSource,
    {
        self.run(
            c,
            now_ns,
            || {
                ips.try_connect(budget, budget_now_ns)
                    .map(|p| Some(p.ip))
                    .map_err(|l| l.wait_ns(budget_now_ns))
            },
            BindSource::bind_source,
        )
    }

    /// Starts every due link that `gate` passes; a passed link's dial is
    /// bound to the IP the gate names, if any.
    fn run(
        &mut self,
        c: &mut impl Connector<D>,
        now_ns: i64,
        mut gate: impl FnMut() -> std::result::Result<Option<IpAddr>, u64>,
        mut bind: impl FnMut(&mut D, IpAddr),
    ) -> usize {
        let links = self.links.len();
        let first = if self.cursor < links { self.cursor } else { 0 };
        let longest = ms_to_ns(u64::from(self.retry.max_ms));
        let bg = self.retry.background;
        let mut started = 0;
        for step in 0..links {
            let i = (first + step) % links;
            let LinkState::Waiting { at_ns, attempt } = self.links[i].state else {
                continue;
            };
            if at_ns > now_ns {
                continue;
            }
            let source = match gate() {
                Ok(source) => source,
                Err(wait_ns) => {
                    let wait = i64::try_from(wait_ns).unwrap_or(i64::MAX).min(longest);
                    self.links[i].state = LinkState::Waiting {
                        at_ns: now_ns.saturating_add(wait),
                        attempt,
                    };
                    continue;
                }
            };
            self.cursor = (i + 1) % links;
            self.waiting -= 1;
            let l = &mut self.links[i];
            if source.is_some() {
                l.source = source;
            }
            let made = (l.make)().map(|mut d| {
                if let Some(ip) = source {
                    bind(&mut d, ip);
                }
                d
            });
            match made.and_then(|d| c.start(d, bg)) {
                Ok(id) => {
                    l.state = LinkState::Connecting(id);
                    started += 1;
                }
                // No CPU for `connect_background`'s helper (a pod with one
                // CPU): no attempt can succeed, so the link gives up now
                // rather than retrying unseen. One line says why.
                Err(Error::Cpu(e)) => {
                    l.state = LinkState::GaveUp;
                    #[cfg(target_os = "linux")]
                    crate::warn::once(format!(
                        "ergon-io warning: Reconnect gave up link {i}: cpu: {e}"
                    ));
                    #[cfg(not(target_os = "linux"))]
                    let _ = e;
                }
                Err(_) => {
                    let _ = self.schedule(LinkId(i), now_ns);
                }
            }
        }
        started
    }

    /// No attempt for `link` before `wait_ns` from now: a refused
    /// handshake's `Retry-After`, a ban, announced maintenance.
    ///
    /// Kept with the link: every later attempt, after a close, a failed
    /// attempt or [`Reconnect::resume`], waits for it as well as for the
    /// backoff. A shorter wait never brings an attempt forward. An unknown
    /// link is ignored.
    pub fn defer(&mut self, link: LinkId, wait_ns: u64) {
        self.defer_at(link, wait_ns, monotonic_ns());
    }

    /// [`Reconnect::defer`] from `now_ns` (`CLOCK_MONOTONIC`).
    pub fn defer_at(&mut self, link: LinkId, wait_ns: u64, now_ns: i64) {
        let Some(l) = self.links.get_mut(link.0) else {
            return;
        };
        let until = now_ns.saturating_add(i64::try_from(wait_ns).unwrap_or(i64::MAX));
        l.not_before = l.not_before.max(until);
        if let LinkState::Waiting { at_ns, attempt } = l.state {
            l.state = LinkState::Waiting {
                at_ns: at_ns.max(l.not_before),
                attempt,
            };
        }
    }

    /// Counts a failed attempt and schedules the next, or gives up.
    fn schedule(&mut self, link: LinkId, now_ns: i64) -> Closed {
        let r = self.random();
        let l = &mut self.links[link.0];
        let attempt = l.attempts + 1;
        if self.retry.max_attempts > 0 && attempt > self.retry.max_attempts {
            l.state = LinkState::GaveUp;
            return Closed::GaveUp {
                link,
                attempts: l.attempts,
            };
        }
        l.attempts = attempt;
        let backoff_ms = self.retry.delay_ms(attempt, r);
        let backoff_at = now_ns.saturating_add(ms_to_ns(backoff_ms));
        let at_ns = backoff_at.max(l.not_before);
        let delay_ms = if at_ns == backoff_at {
            backoff_ms
        } else {
            u64::try_from(at_ns.saturating_sub(now_ns))
                .unwrap_or(0)
                .div_ceil(1_000_000)
        };
        l.state = LinkState::Waiting { at_ns, attempt };
        self.waiting += 1;
        Closed::Retry {
            link,
            attempt,
            delay_ms,
        }
    }

    /// A number in `[0, 1)`.
    const fn random(&mut self) -> f64 {
        unit(&mut self.rng)
    }
}

/// `ms` in nanoseconds, saturating.
fn ms_to_ns(ms: u64) -> i64 {
    i64::try_from(ms)
        .unwrap_or(i64::MAX)
        .saturating_mul(1_000_000)
}

/// A jitter generator's state from `seed`; `0` takes one from the system.
///
/// # Errors
/// [`Error::Config`]: no system randomness.
pub(crate) fn rng_state(seed: u64) -> Result<u64> {
    let seed = if seed == 0 {
        let mut b = [0u8; 8];
        ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut b)
            .map_err(|_| Error::Config("no system randomness".into()))?;
        u64::from_le_bytes(b)
    } else {
        seed
    };
    // SplitMix64 spreads nearby seeds apart; xorshift needs a state other
    // than 0.
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    Ok(z.max(1))
}

/// The next number in `[0, 1)` from a [`rng_state`] (xorshift64).
#[allow(clippy::cast_precision_loss)]
pub(crate) const fn unit(rng: &mut u64) -> f64 {
    let mut x = *rng;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *rng = x;
    (x >> 11) as f64 / (1u64 << 53) as f64
}
