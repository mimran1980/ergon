//! ergon-io's warnings, and the checks behind them.
//!
//! - One thread per process (`ergon-io-warn`), on the helper CPUs the first
//!   driver chose ([`crate::placement`]: inside the process's cpuset, never
//!   the polling CPU when it is known and another CPU is free; `prepare_on`),
//!   or `prepare`'s choice for the libcurl client. In a pod with one CPU it
//!   shares the polling CPU, parked until there is work.
//! - It writes the lines to stderr (which may be a pipe that blocks) and runs
//!   the queued work: each connection's route lookup under
//!   `quickack_check=warn`, and the checks of its egress device.
//! - A polling thread only formats a line or packs a job and queues it. If
//!   the thread is not running, the work is dropped, never done there.
//! - While it watches an egress device it wakes once a second to read that
//!   device's `carrier_changes` (one sysfs read each). It checks the device
//!   again once the counter has moved and then held for
//!   [`checks::SETTLE_READS`] reads. Otherwise it is parked until something
//!   is queued.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SendError, Sender};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use ergon_tune::{Host, HostCheckPolicy};

use crate::checks::{self, DevChecks};
use crate::clock::monotonic_ns;
use crate::error::Error;

const EVERY_NS: i64 = 10_000_000_000;
static LAST_NS: AtomicI64 = AtomicI64::new(i64::MIN);
static HELD_BACK: AtomicU64 = AtomicU64::new(0);
static WRITER: OnceLock<Option<Sender<Msg>>> = OnceLock::new();

/// How often watched devices' `carrier_changes` are read.
const RECHECK: Duration = Duration::from_secs(1);

/// Work for the warning thread.
pub type Job = Box<dyn FnOnce(&mut State) + Send>;

enum Msg {
    /// A line to write.
    Line(String),
    /// A line to write unless the same line was written before.
    Once(String),
    /// Work to run there.
    Run(Job),
}

/// What the warning thread keeps: the lines written once, and the egress
/// devices it checks.
pub struct State {
    host: Host,
    dev: DevChecks,
    seen: HashSet<String>,
    last_recheck: Instant,
    policy: HostCheckPolicy,
}

impl State {
    fn new() -> Self {
        let mut st = Self::at(Host::system());
        // The switch at the thread's start, with the driver's reading of it
        // (`ERGON_HOST_CHECKS`): `off` drops the warning lines here too.
        st.policy = HostCheckPolicy::from_env().unwrap_or_default();
        st
    }

    /// A fresh state reading `host` (a fixture tree in tests).
    pub(crate) fn at(host: Host) -> Self {
        Self {
            host,
            dev: DevChecks::new(),
            seen: HashSet::new(),
            last_recheck: Instant::now(),
            policy: HostCheckPolicy::On,
        }
    }

    /// Writes `line`.
    #[allow(clippy::unused_self)] // on the state, so only its holder writes
    pub fn print(&self, line: String) {
        eprintln!("{line}");
        keep(line);
    }

    /// Writes `line` unless it was written before.
    pub fn print_once(&mut self, line: String) {
        if self.seen.insert(line.clone()) {
            self.print(line);
        }
    }

    /// Whether `key` is new (for reports keyed on less than their text).
    pub fn first(&mut self, key: String) -> bool {
        self.seen.insert(key)
    }

    /// The I-dev checks of egress device `dev` in `ctx`
    /// ([`DevChecks::check`]): each line once, under `ERGON_HOST_CHECKS`.
    pub fn check_device(&mut self, dev: &str, ctx: &checks::DevContext) {
        let found = self.dev.check(&self.host, dev, ctx);
        for l in self.policy.lines("ergon-io", &found) {
            self.print_once(l);
        }
    }

    fn handle(&mut self, m: Msg) {
        match m {
            Msg::Line(l) => self.print(l),
            Msg::Once(l) => self.print_once(l),
            Msg::Run(job) => job(self),
        }
    }

    /// The watched devices whose carrier changed, checked again: each
    /// finding written again (an ENA reset undoes the fixes), what could not
    /// be read only once.
    fn recheck_if_due(&mut self) {
        if !self.dev.watching() || self.last_recheck.elapsed() < RECHECK {
            return;
        }
        self.last_recheck = Instant::now();
        self.recheck_now();
    }

    /// [`DevChecks::recheck`] now, its findings written (under
    /// `ERGON_HOST_CHECKS`: `off` drops the warning lines).
    pub(crate) fn recheck_now(&mut self) {
        let found = self.dev.recheck(&self.host);
        for f in &found {
            if self.policy == HostCheckPolicy::Off && f.is_warning() {
                continue;
            }
            if let Some(l) = checks::line(f) {
                if f.is_unknown() {
                    self.print_once(l);
                } else {
                    self.print(l);
                }
            }
        }
    }
}

/// Starts the writer thread where no driver chose its CPUs (the libcurl
/// client): `ERGON_HELPER_CPUS`, else the helper CPUs
/// [`crate::placement`] chooses for the calling thread's CPU. Called where
/// clients are created, so neither the sysfs reads nor the thread start land
/// on a polling thread. The fallback masks keep off every CPU the calling
/// thread may run on (not only one pinned CPU): when that thread is
/// unpinned the polling CPU is unknown, and the writer is parked, but a
/// spare CPU is still the better home.
#[cfg(feature = "curl")]
pub fn prepare() {
    WRITER.get_or_init(|| {
        let host = Host::system();
        let affinity = ergon_tune::thread_affinity(&host, ergon_tune::Thread::Current)
            .map(|s| list(&s))
            .unwrap_or_default();
        let polling = affinity.first().copied().filter(|_| affinity.len() == 1);
        let env = std::env::var(crate::placement::HELPER_CPUS_ENV).ok();
        let (cpus, allowed) = match crate::placement::of(
            &host,
            &[],
            env.as_deref(),
            polling.and_then(|c| u32::try_from(c).ok()),
        ) {
            Ok(p) => (list(&p.cpus), list(&p.allowed)),
            Err(_) => (Vec::new(), list(&crate::placement::process_cpus(&host).0)),
        };
        start(&writer_masks(&cpus, &allowed, &affinity))
    });
}

/// Starts the writer thread on `cpus`, the CPUs `Driver::new` chose for its
/// helper threads in placement `p` ([`crate::placement`]). The first driver
/// or client in the process starts it; later calls do nothing.
pub fn prepare_on(p: &crate::placement::Placement, cpus: &[usize]) {
    WRITER.get_or_init(|| {
        let avoid: Vec<usize> = p
            .polling
            .and_then(|c| usize::try_from(c).ok())
            .into_iter()
            .collect();
        start(&writer_masks(cpus, &list(&p.allowed), &avoid))
    });
}

/// `set` as CPU numbers for `sys::spawn_pinned`.
fn list(set: &ergon_tune::CpuSet) -> Vec<usize> {
    set.iter().filter_map(|c| usize::try_from(c).ok()).collect()
}

/// Starts the writer with the first of `masks` the kernel accepts, and says
/// so in one line when that is not the first.
///
/// The first mask is always inside the process's cpuset
/// ([`crate::placement`]), so a refusal means something else changed (a
/// CPU went offline). In a process that may use the polling CPU only, every
/// mask is the creating thread's own: the writer shares the polling CPU,
/// parked until there is work, which `Driver::new`'s one-CPU warning says.
fn start(masks: &[Vec<usize>]) -> Option<Sender<Msg>> {
    let (tx, which) = start_with(masks, &mut |cpus, serve| {
        crate::sys::spawn_pinned("ergon-io-warn", cpus, serve)
    })?;
    if which > 0 {
        let list = |m: &[usize]| {
            m.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        let now = masks.get(which).map_or_else(String::new, |m| list(m));
        let _ = tx.send(Msg::Once(format!(
            "ergon-io info: ergon-io's warning thread could not be started on CPUs {} (the \
             kernel refused that mask). It runs on {}; it is parked until there is work, and \
             wakes once a second while it watches an egress device.",
            masks.first().map_or_else(String::new, |m| list(m)),
            if now.is_empty() {
                "the CPUs of the thread that created the first driver or client".to_owned()
            } else {
                format!("those of CPUs {now} that the cpuset allows")
            }
        )));
    }
    Some(tx)
}

/// The CPU masks to start the writer with, most preferred first, each
/// once: the helper CPUs (`preferred`), every CPU the process may use but
/// `avoid` (the creating thread's CPUs: with an unpinned creator all of
/// them), and the creating thread's own mask (empty: inherited) last.
fn writer_masks(preferred: &[usize], allowed: &[usize], avoid: &[usize]) -> Vec<Vec<usize>> {
    let others: Vec<usize> = allowed
        .iter()
        .copied()
        .filter(|c| !avoid.contains(c))
        .collect();
    let mut out: Vec<Vec<usize>> = Vec::new();
    for m in [preferred.to_vec(), others] {
        if !m.is_empty() && !out.contains(&m) {
            out.push(m);
        }
    }
    out.push(Vec::new());
    out
}

/// Starts a thread with a CPU mask (`sys::spawn_pinned`, or a stand-in in
/// tests).
type Spawn<'a> = dyn FnMut(&[usize], Box<dyn FnOnce() + Send>) -> std::io::Result<()> + 'a;

/// Starts the writer with the first of `masks` that `spawn` accepts: its
/// channel and which mask.
fn start_with(masks: &[Vec<usize>], spawn: &mut Spawn<'_>) -> Option<(Sender<Msg>, usize)> {
    masks.iter().enumerate().find_map(|(i, cpus)| {
        let (tx, rx) = mpsc::channel::<Msg>();
        spawn(cpus, Box::new(move || serve(&rx)))
            .ok()
            .map(|()| (tx, i))
    })
}

/// The writer's loop: lines and work as they come, and the watched
/// devices' re-checks once a second.
fn serve(rx: &Receiver<Msg>) {
    let mut st = State::new();
    loop {
        let msg = if st.dev.watching() {
            match rx.recv_timeout(RECHECK) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => return,
            }
        };
        if let Some(m) = msg {
            st.handle(m);
        }
        st.recheck_if_due();
    }
}

/// Hands `m` to the writer.
fn send(m: Msg) {
    deliver(WRITER.get().and_then(Option::as_ref), m);
}

/// Whether work was dropped for want of a writer (reported once).
static SKIPPED: AtomicBool = AtomicBool::new(false);

/// Hands `m` to `writer`. Without one (it could not start, or it ended) a
/// line is written here, but work is not done: the caller may be a polling
/// thread, and the work is route lookups and device checks. One line says
/// it was skipped.
fn deliver(writer: Option<&Sender<Msg>>, m: Msg) {
    let m = match writer {
        Some(tx) => match tx.send(m) {
            Ok(()) => return,
            Err(SendError(m)) => m,
        },
        None => m,
    };
    let line = match m {
        Msg::Line(l) | Msg::Once(l) => l,
        Msg::Run(_) => {
            if SKIPPED.swap(true, Ordering::Relaxed) {
                return;
            }
            "ergon-io warning: ergon-io's warning thread is not running, so the work queued to \
             it is skipped, never done on the polling thread: the route lookups of \
             quickack_check=warn (those connections' RouteCheck stays Pending), the NAPI-id check \
             and the egress-device checks."
                .to_owned()
        }
    };
    eprintln!("{line}");
    keep(line);
}

/// Queues `line` for the writer; written here only if the writer is not
/// running. There is one writer per process, parked in `recv`: each line costs
/// the caller an allocation and a futex wake, so callers bound how often
/// they warn.
pub fn emit(line: String) {
    send(Msg::Line(line));
}

/// Queues `line`, written once per process.
pub fn once(line: String) {
    send(Msg::Once(line));
}

/// Queues `job` to run on the writer thread, in order with the lines.
/// Dropped (never run here) if the writer is not running.
pub fn run(job: impl FnOnce(&mut State) + Send + 'static) {
    send(Msg::Run(Box::new(job)));
}

/// Queues `what: error` for a connect failure the application does not
/// handle itself, at most once every 10 s across the process (with the
/// number held back).
pub fn connect_failure(what: &str, error: &Error) {
    let now = monotonic_ns();
    let last = LAST_NS.load(Ordering::Relaxed);
    let due = last == i64::MIN || now - last >= EVERY_NS;
    // One winner per 10 s, even with clients on several threads.
    if !due
        || LAST_NS
            .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
    {
        HELD_BACK.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let held = HELD_BACK.swap(0, Ordering::Relaxed);
    emit(format!(
        "ergon-io warning: {what}: {error}{}; retrying (implement \
         ClientHandler::on_connect_error to handle this yourself)",
        if held > 0 {
            format!(" ({held} more since the last warning)")
        } else {
            String::new()
        }
    ));
}

/// Waits until the warning thread has handled everything queued before the
/// call.
///
/// That is: written its lines, made its route lookups and checked their
/// devices. `false` if that took longer than `timeout`, or at once if the
/// warning thread is not running. For tests and start-up health checks;
/// never call it on a polling thread.
#[must_use]
pub fn flush(timeout: Duration) -> bool {
    let (tx, rx) = mpsc::channel::<()>();
    run(move |_| {
        let _ = tx.send(());
    });
    rx.recv_timeout(timeout).is_ok()
}

static KEEP: AtomicBool = AtomicBool::new(false);
static KEPT: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn keep(line: String) {
    if KEEP.load(Ordering::Relaxed) {
        KEPT.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// Test lever: from now on, keep a copy of every line written.
#[doc(hidden)]
pub fn keep_lines() {
    KEEP.store(true, Ordering::Relaxed);
}

/// Test lever: the lines kept so far, taken.
#[doc(hidden)]
#[must_use]
pub fn take_lines() -> Vec<String> {
    std::mem::take(&mut *KEPT.lock().unwrap_or_else(PoisonError::into_inner))
}

/// Test lever: forget which lines were written so far, so `once` prints them
/// again. Queued ahead of any later line, so the next report is counted.
#[doc(hidden)]
pub fn forget_seen() {
    run(|st| {
        st.seen.clear();
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use super::{Msg, deliver, start_with, writer_masks};

    type R = Result<(), Box<dyn std::error::Error>>;

    fn einval() -> std::io::Error {
        std::io::Error::from_raw_os_error(libc::EINVAL)
    }

    /// A pod whose static CPU set holds isolated CPUs only (2 and 3 of
    /// 0-7, the housekeeping CPUs 0 and 1 outside it): the kernel refuses a
    /// mask with none of the set's CPUs (EINVAL, `sched_setaffinity(2)`), so
    /// the writer must start on a mask it accepts, never not at all.
    #[test]
    fn the_writer_starts_when_the_housekeeping_cpus_are_outside_the_cpuset() -> R {
        let allowed = [2_usize, 3];
        let masks = writer_masks(&[0, 1], &[0, 1, 2, 3, 4, 5, 6, 7], &[2]);
        let mut tried = Vec::new();
        let started = start_with(&masks, &mut |cpus, serve| {
            tried.push(cpus.to_vec());
            if !cpus.is_empty() && !cpus.iter().any(|c| allowed.contains(c)) {
                return Err(einval());
            }
            std::thread::spawn(serve);
            Ok(())
        });
        let (tx, which) = started.ok_or("the writer did not start")?;
        assert_eq!(tried[0], vec![0, 1], "housekeeping first");
        assert!(which > 0, "{tried:?}");
        // Not the creating thread's CPU when another is allowed.
        assert!(!tried[which].contains(&2), "{tried:?}");
        // It runs work on its own thread.
        let (done, got) = std::sync::mpsc::channel();
        tx.send(Msg::Run(Box::new(move |_| {
            let _ = done.send(std::thread::current().id());
        })))?;
        let ran_on = got.recv_timeout(Duration::from_secs(5))?;
        assert_ne!(ran_on, std::thread::current().id());
        Ok(())
    }

    /// An unpinned creating thread (it may run on 0-2 of 0-4) is avoided
    /// whole, not just one CPU: the writer's fallback mask stays off every
    /// CPU that thread may run on, and only then inherits it.
    #[test]
    fn the_writer_avoids_every_cpu_an_unpinned_creator_may_run_on() {
        assert_eq!(
            writer_masks(&[], &[0, 1, 2, 3, 4], &[0, 1, 2]),
            vec![vec![3, 4], vec![]]
        );
    }

    /// One CPU in the set, the creating thread's: only its own mask fits.
    #[test]
    fn a_single_cpu_cpuset_falls_back_to_the_creating_threads_mask() -> R {
        let masks = writer_masks(&[0, 1, 2, 3, 4, 6, 7], &[0, 1, 2, 3, 4, 5, 6, 7], &[5]);
        let mut tried = Vec::new();
        let started = start_with(&masks, &mut |cpus, serve| {
            tried.push(cpus.to_vec());
            if !cpus.is_empty() && !cpus.contains(&5) {
                return Err(einval());
            }
            std::thread::spawn(serve);
            Ok(())
        });
        let (_tx, which) = started.ok_or("the writer did not start")?;
        assert_eq!(tried.last(), Some(&Vec::new()), "{tried:?}");
        assert_eq!(tried.len(), which + 1);
        // The same mask is not tried twice.
        let mut unique = tried.clone();
        unique.dedup();
        assert_eq!(unique, tried);
        Ok(())
    }

    /// Without a writer (it could not start at all), work queued for it
    /// is not done on the calling thread, which may be a polling thread.
    #[test]
    fn without_a_writer_work_is_not_done_on_the_calling_thread() {
        let ran = Arc::new(AtomicBool::new(false));
        let r = Arc::clone(&ran);
        deliver(
            None,
            Msg::Run(Box::new(move |_| r.store(true, Ordering::Relaxed))),
        );
        assert!(!ran.load(Ordering::Relaxed));
    }
}
