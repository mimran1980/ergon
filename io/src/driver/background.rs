//! Connection set-up on a helper thread, so the polling thread never spends
//! the handshake's CPU.
//!
//! - Runs DNS (the driver's non-blocking resolver: one slow name never holds
//!   up other connects or their deadlines), TCP, TLS and any upgrade.
//! - Finished connections come back over a channel; `poll` checks it only
//!   while background connects are outstanding.
//! - Connections the driver closes are dropped here too: freeing their
//!   buffers never lands on the polling thread.
//! - Its CPU mask is set before it starts (`sys::spawn_pinned`): a thread
//!   inherits its creator's mask, so one that moved itself would first run
//!   on the polling thread's CPU.
//! - Idle, it is parked in the channel: no wakeups without work (a
//!   millisecond poll was 1,000 a second, each one preempting whatever ran
//!   on its CPU). Work handed to it while parked costs the polling thread
//!   one futex wake, a system call; a connect costs far more on the loop
//!   (`Driver::connect`), and closing a connection is not the hot path.
//!   While dials are in flight it steps them at the interval it was started
//!   with ([`crate::placement::dial_step`]: 50 µs on a CPU of its own,
//!   1 ms when it shares `uring-sqpoll`'s spinning CPU, where every wakeup
//!   is a preemption of the spinner), woken early by new work.

use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::Duration;

use std::os::fd::RawFd;

use crate::clock::monotonic_ns;
use crate::dns::{DnsConfig, Lookup, Resolver};
use crate::endpoint::ConnId;
use crate::endpoint::sealed::DialIo;
use crate::error::{Error, Result};
use crate::route::{self, QuickAckCheck, RouteCheck};

/// A dial as the helper thread sees it (no bound on the connection type).
pub(super) trait BgDial<E>: Send {
    fn host(&self) -> &str;
    fn port(&self) -> u16;
    fn addr(&self) -> Option<SocketAddr>;
    fn start(&mut self, addr: SocketAddr) -> Result<()>;
    fn step(&mut self) -> Result<Option<E>>;
    fn awaiting_route(&self) -> Option<RawFd>;
    fn route_checked(&mut self, check: RouteCheck);
}

impl<E, D: DialIo<Conn = E> + Send> BgDial<E> for D {
    fn host(&self) -> &str {
        DialIo::host(self)
    }
    fn port(&self) -> u16 {
        DialIo::port(self)
    }
    fn addr(&self) -> Option<SocketAddr> {
        DialIo::addr(self)
    }
    fn start(&mut self, addr: SocketAddr) -> Result<()> {
        DialIo::start(self, addr)
    }
    fn step(&mut self) -> Result<Option<E>> {
        DialIo::step(self)
    }
    fn awaiting_route(&self) -> Option<RawFd> {
        DialIo::awaiting_route(self)
    }
    fn route_checked(&mut self, check: RouteCheck) {
        DialIo::route_checked(self, check);
    }
}

pub(super) type BoxDial<E> = Box<dyn BgDial<E>>;

/// Applies socket options to a finished connection and runs its checks (on
/// the helper thread; the route already checked at TCP connect under
/// `require`).
pub(super) type Prepare<E> = Box<dyn Fn(&mut E, ConnId) -> Result<()> + Send>;

struct Job<E> {
    slot: usize,
    generation: u32,
    dial: BoxDial<E>,
    prepare: Prepare<E>,
    /// Monotonic nanoseconds.
    deadline_ns: i64,
    started: bool,
}

enum Msg<E> {
    Dial(Job<E>),
    /// A closed connection to free.
    Free(E),
}

/// A finished background connect.
pub(super) struct Done<E> {
    pub(super) slot: usize,
    pub(super) generation: u32,
    pub(super) result: Result<E>,
}

/// The helper thread's mailbox. Dropping it stops the thread once its jobs are
/// done; results nobody receives are dropped there (their sockets close).
pub(super) struct Background<E> {
    msgs: Sender<Msg<E>>,
    done: Receiver<Done<E>>,
    /// Connects submitted and not yet received: (slot, generation).
    outstanding: Vec<(usize, u32)>,
}

impl<E: Send + 'static> Background<E> {
    /// Starts the helper thread on `cpus` (`placement::dial_cpus`: never the
    /// polling CPU when it is known; empty keeps the caller's mask, for tests
    /// only). `step` is how often a dial in flight is stepped
    /// ([`crate::placement::dial_step`]).
    pub(super) fn start(cpus: &[usize], dns: DnsConfig, step: Duration) -> Result<Self> {
        let (msgs, msg_rx) = mpsc::channel::<Msg<E>>();
        let (done_tx, done) = mpsc::channel::<Done<E>>();
        crate::sys::spawn_pinned("ergon-io-dial", cpus, move || {
            run(&msg_rx, &done_tx, Resolver::new(dns), step);
        })?;
        Ok(Self {
            msgs,
            done,
            outstanding: Vec::new(),
        })
    }

    pub(super) fn submit(
        &mut self,
        slot: usize,
        generation: u32,
        dial: BoxDial<E>,
        prepare: Prepare<E>,
        timeout: Duration,
    ) -> Result<()> {
        self.msgs
            .send(Msg::Dial(Job {
                slot,
                generation,
                dial,
                prepare,
                deadline_ns: monotonic_ns()
                    .saturating_add(i64::try_from(timeout.as_nanos()).unwrap_or(i64::MAX)),
                started: false,
            }))
            .map_err(|_| Error::Config("background connect thread stopped".into()))?;
        self.outstanding.push((slot, generation));
        Ok(())
    }
}

impl<E> Background<E> {
    /// Frees `conn` on the helper thread (here, if the thread has stopped).
    pub(super) fn free(&self, conn: E) {
        let _ = self.msgs.send(Msg::Free(conn));
    }

    /// Whether connects are still to be taken.
    pub(super) const fn has_outstanding(&self) -> bool {
        !self.outstanding.is_empty()
    }

    /// A finished connect, if any, without blocking. If the helper thread has
    /// stopped, every outstanding connect is reported as failed.
    pub(super) fn try_take(&mut self) -> Option<Done<E>> {
        match self.done.try_recv() {
            Ok(d) => {
                self.outstanding.retain(|&o| o != (d.slot, d.generation));
                Some(d)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.outstanding.pop().map(|(slot, generation)| Done {
                    slot,
                    generation,
                    result: Err(Error::Config("background connect thread stopped".into())),
                })
            }
        }
    }
}

fn run<E>(msgs: &Receiver<Msg<E>>, done: &Sender<Done<E>>, mut resolver: Resolver, step: Duration) {
    let mut active: Vec<Job<E>> = Vec::new();
    // Once the driver is gone, `recv_timeout` would return at once, so the
    // in-flight dials' last steps sleep instead.
    let mut alive = true;
    let take = |m: Msg<E>, active: &mut Vec<Job<E>>| match m {
        Msg::Dial(j) => active.push(j),
        Msg::Free(c) => drop(c),
    };
    loop {
        if active.is_empty() {
            // Parked until there is work, or the driver is gone.
            match msgs.recv() {
                Ok(m) => take(m, &mut active),
                Err(_) => return,
            }
        }
        loop {
            match msgs.try_recv() {
                Ok(m) => take(m, &mut active),
                // The driver is gone: finish what is in hand (the results are
                // dropped, closing their sockets), then stop.
                Err(TryRecvError::Disconnected) if active.is_empty() => return,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        if active.is_empty() {
            // Only connections to free came: park again.
            continue;
        }
        let now = monotonic_ns();
        resolver.poll(now);
        let mut i = 0;
        while i < active.len() {
            match advance(&mut active[i], &mut resolver, now) {
                Ok(None) => i += 1,
                result => {
                    let job = active.swap_remove(i);
                    // Socket options are set here too: the first
                    // SO_TIMESTAMPING in a process flips a kernel static key
                    // (code patched on every CPU, ~100 µs), which must not
                    // land on the polling thread. So is the quick-ACK route
                    // check's netlink exchange.
                    let id =
                        ConnId::new(u32::try_from(job.slot).unwrap_or(u32::MAX), job.generation);
                    let result = result
                        .and_then(|c| c.ok_or(Error::Closed))
                        .and_then(|mut c| (job.prepare)(&mut c, id).map(|()| c));
                    // The driver is gone: dropping the result closes the socket.
                    let _ = done.send(Done {
                        slot: job.slot,
                        generation: job.generation,
                        result,
                    });
                }
            }
        }
        if !active.is_empty() {
            // Step again in `step`, but take new work at once: a plain sleep
            // would delay every connect queued meanwhile for a whole step.
            if alive {
                match msgs.recv_timeout(step) {
                    Ok(m) => take(m, &mut active),
                    Err(RecvTimeoutError::Timeout) => {}
                    // Disconnected: no new work can arrive. Do not spin on a
                    // closed channel; sleep until the dials in hand are done.
                    Err(RecvTimeoutError::Disconnected) => alive = false,
                }
            } else {
                std::thread::sleep(step);
            }
        }
    }
}

fn advance<E>(job: &mut Job<E>, resolver: &mut Resolver, now: i64) -> Result<Option<E>> {
    if now > job.deadline_ns {
        return Err(Error::Timeout("connect"));
    }
    if !job.started {
        let addr = match job.dial.addr() {
            Some(a) => a,
            None => match resolver.lookup(job.dial.host(), now)? {
                Lookup::Ready(ip) => SocketAddr::new(IpAddr::V4(ip), job.dial.port()),
                Lookup::Pending => return Ok(None),
            },
        };
        job.dial.start(addr)?;
        job.started = true;
    }
    let r = job.dial.step();
    if let (Ok(None), Some(fd)) = (&r, job.dial.awaiting_route()) {
        // Under `require`: TCP is connected, TLS not started: a refused
        // route costs no handshake.
        let found = route::check_socket(QuickAckCheck::Require, fd)
            .map_err(|e| Error::QuickAck(Box::new(e)))?;
        job.dial.route_checked(found);
    }
    r
}

/// Held by every unit test in this crate that may start an `ergon-io-dial`
/// helper. Unit tests share one process and may run in parallel
/// (`--test-threads`); a test that finds its helper by diffing the process's
/// dial threads must not pick up another test's.
#[cfg(test)]
pub fn dial_helper_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    /// A dial whose first step panics, killing the helper thread.
    struct Boom;

    impl BgDial<u8> for Boom {
        fn host(&self) -> &'static str {
            "boom.test"
        }
        fn port(&self) -> u16 {
            1
        }
        fn addr(&self) -> Option<SocketAddr> {
            Some(SocketAddr::from(([127, 0, 0, 1], 1)))
        }
        fn start(&mut self, _: SocketAddr) -> Result<()> {
            Ok(())
        }
        #[allow(clippy::panic)]
        fn step(&mut self) -> Result<Option<u8>> {
            panic!("dial step panicked");
        }
        fn awaiting_route(&self) -> Option<RawFd> {
            None
        }
        fn route_checked(&mut self, _: RouteCheck) {}
    }

    #[test]
    fn a_stopped_helper_fails_every_outstanding_connect() -> Result<()> {
        let _helpers = super::dial_helper_test_lock();
        let mut bg = Background::<u8>::start(
            &[],
            DnsConfig::system(),
            crate::placement::DIAL_STEP_OWN_CPU,
        )?;
        bg.submit(
            3,
            7,
            Box::new(Boom),
            Box::new(|_, _| Ok(())),
            Duration::from_secs(5),
        )?;
        bg.submit(
            4,
            9,
            Box::new(Boom),
            Box::new(|_, _| Ok(())),
            Duration::from_secs(5),
        )?;
        let end = Instant::now() + Duration::from_secs(5);
        let mut failed = Vec::new();
        while bg.has_outstanding() {
            if let Some(d) = bg.try_take() {
                assert!(d.result.is_err());
                failed.push((d.slot, d.generation));
            }
            assert!(Instant::now() < end, "outstanding connects never reported");
        }
        failed.sort_unstable();
        assert_eq!(failed, vec![(3, 7), (4, 9)]);
        Ok(())
    }

    type R<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// The thread ids of this process's dial helpers.
    fn dial_threads() -> R<Vec<String>> {
        let mut out = Vec::new();
        for e in std::fs::read_dir("/proc/self/task")? {
            let p = e?.path();
            if std::fs::read_to_string(p.join("comm")).is_ok_and(|c| c.trim() == "ergon-io-dial") {
                out.extend(p.file_name().and_then(|n| n.to_str()).map(str::to_owned));
            }
        }
        Ok(out)
    }

    /// The dial helper `start` adds (the lane runs one test at a time).
    fn started_helper(before: &[String]) -> R<String> {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(t) = dial_threads()?.into_iter().find(|t| !before.contains(t)) {
                return Ok(t);
            }
            if Instant::now() > end {
                return Err("the dial helper never started".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// A dial that stays in flight forever (a connect to nowhere).
    struct Hang;

    impl BgDial<u8> for Hang {
        fn host(&self) -> &'static str {
            "hang.test"
        }
        fn port(&self) -> u16 {
            1
        }
        fn addr(&self) -> Option<SocketAddr> {
            Some(SocketAddr::from(([127, 0, 0, 1], 1)))
        }
        fn start(&mut self, _: SocketAddr) -> Result<()> {
            Ok(())
        }
        fn step(&mut self) -> Result<Option<u8>> {
            Ok(None)
        }
        fn awaiting_route(&self) -> Option<RawFd> {
            None
        }
        fn route_checked(&mut self, _: RouteCheck) {}
    }

    /// Times thread `tid` gave up its CPU (each wakeup from a sleep or a
    /// wait is one).
    fn voluntary_switches(tid: &str) -> R<u64> {
        let status = std::fs::read_to_string(format!("/proc/self/task/{tid}/status"))?;
        Ok(status
            .lines()
            .find_map(|l| l.strip_prefix("voluntary_ctxt_switches:"))
            .ok_or("no voluntary_ctxt_switches")?
            .trim()
            .parse()?)
    }

    /// With nothing to connect, the helper is parked in its channel: no
    /// wakeups (a 1 ms poll was 1,000 a second, each one preempting
    /// whatever else ran on its CPU), and dropping the mailbox stops it.
    #[test]
    fn an_idle_helper_never_wakes_and_stops_when_dropped() -> R {
        let _helpers = super::dial_helper_test_lock();
        let before = dial_threads()?;
        let bg = Background::<u8>::start(
            &[],
            DnsConfig::system(),
            crate::placement::DIAL_STEP_OWN_CPU,
        )?;
        let tid = started_helper(&before)?;
        std::thread::sleep(Duration::from_millis(50));
        let first = voluntary_switches(&tid)?;
        std::thread::sleep(Duration::from_millis(300));
        let woke = voluntary_switches(&tid)?.saturating_sub(first);
        assert!(woke <= 2, "the idle helper woke {woke} times in 300 ms");
        drop(bg);
        let end = Instant::now() + Duration::from_secs(2);
        while std::path::Path::new(&format!("/proc/self/task/{tid}")).exists() {
            assert!(Instant::now() < end, "the helper outlived its driver");
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    /// Work handed to a parked helper wakes it: a dial submitted after a
    /// long idle spell is still taken and reported.
    #[test]
    fn a_parked_helper_wakes_for_work() -> R {
        let _helpers = super::dial_helper_test_lock();
        let mut bg = Background::<u8>::start(
            &[],
            DnsConfig::system(),
            crate::placement::DIAL_STEP_OWN_CPU,
        )?;
        std::thread::sleep(Duration::from_millis(50));
        bg.submit(
            1,
            1,
            Box::new(Boom),
            Box::new(|_, _| Ok(())),
            Duration::from_secs(5),
        )?;
        let end = Instant::now() + Duration::from_secs(5);
        while bg.has_outstanding() {
            if let Some(d) = bg.try_take() {
                assert_eq!((d.slot, d.generation), (1, 1));
                assert!(d.result.is_err());
            }
            assert!(
                Instant::now() < end,
                "the parked helper never took the dial"
            );
        }
        Ok(())
    }

    /// While a dial is in flight the helper steps it at the interval it was
    /// started with. A fixed 50 µs poll is 20,000 wakeups a second, each
    /// one a preemption of whatever shares its CPU (in the x86 layout
    /// `uring-sqpoll`'s spinner).
    #[test]
    fn a_dial_in_flight_steps_at_the_given_interval() -> R {
        let _helpers = super::dial_helper_test_lock();
        let before = dial_threads()?;
        let mut bg = Background::<u8>::start(&[], DnsConfig::system(), Duration::from_millis(1))?;
        let tid = started_helper(&before)?;
        bg.submit(
            1,
            1,
            Box::new(Hang),
            Box::new(|_, _| Ok(())),
            Duration::from_secs(5),
        )?;
        std::thread::sleep(Duration::from_millis(50));
        let first = voluntary_switches(&tid)?;
        std::thread::sleep(Duration::from_millis(300));
        let woke = voluntary_switches(&tid)?.saturating_sub(first);
        assert!(
            woke <= 500,
            "the helper woke {woke} times in 300 ms while a dial was in flight"
        );
        Ok(())
    }

    /// Work arriving while a dial is in flight is taken at once, not after
    /// the next step: the helper waits on its channel with a timeout, it
    /// does not sleep between steps.
    #[test]
    fn work_arrives_at_once_while_a_dial_is_in_flight() -> R {
        let _helpers = super::dial_helper_test_lock();
        let mut bg = Background::<u8>::start(&[], DnsConfig::system(), Duration::from_millis(200))?;
        bg.submit(
            1,
            1,
            Box::new(Hang),
            Box::new(|_, _| Ok(())),
            Duration::from_secs(5),
        )?;
        std::thread::sleep(Duration::from_millis(20));
        let t0 = Instant::now();
        bg.submit(
            2,
            2,
            Box::new(Boom),
            Box::new(|_, _| Ok(())),
            Duration::from_secs(5),
        )?;
        let end = Instant::now() + Duration::from_millis(50);
        let mut got = false;
        while !got {
            if let Some(d) = bg.try_take() {
                assert!(d.result.is_err());
                got = (d.slot, d.generation) == (2, 2);
            }
            assert!(
                Instant::now() < end,
                "the in-flight dial delayed new work past {t0:?}"
            );
        }
        Ok(())
    }
}
