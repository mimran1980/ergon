//! The measured loop: the public [`Driver`] API driven exactly as an application
//! would drive it, plus instrumentation that costs the same in every backend
//! (one `rdtsc` per iteration, context switches and CPU time read outside the
//! loop).

use ergon_io::{Backend, Config, Driver, Error, Result, WsConn, WsHandler};

use crate::clock::{monotonic_ns, thread_cpu_ticks, thread_ctx_switches, ticks_per_us};
use crate::stalls::StallLog;

/// An application driven by [`run`].
pub trait App: WsHandler {
    /// Called once per iteration, before `poll` (only if [`App::needs_idle`]).
    /// Sends queued here go out in that `poll`.
    ///
    /// # Errors
    /// Sending failed.
    fn on_idle(&mut self, _driver: &mut Driver<WsConn>) -> Result<()> {
        Ok(())
    }
    /// Whether [`App::on_idle`] must be called.
    fn needs_idle(&self) -> bool {
        false
    }
    /// The first connection error reported through `on_closed`, if any.
    fn take_closed(&mut self) -> Option<Error>;
}

/// Loop counters, reported per thread.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LoopStats {
    /// Loop iterations.
    pub spins: u64,
    /// The driver's kernel-interface counters.
    pub driver: ergon_io::DriverStats,
    /// Thread user CPU ticks during the loop.
    pub utime_ticks: u64,
    /// Thread system CPU ticks during the loop.
    pub stime_ticks: u64,
    /// Whether NIC polling covered every socket at the end.
    pub napi: bool,
    /// Voluntary context switches during the loop (the thread slept).
    pub vol_ctx_switches: u64,
    /// Involuntary context switches during the loop (preempted).
    pub invol_ctx_switches: u64,
    /// Longest loop iteration, in counter ticks.
    pub gap_max_ticks: u64,
    /// Iterations longer than 5, 20 and 100 µs.
    pub gaps_over: [u64; 3],
    /// Counter ticks per microsecond.
    pub ticks_per_us: f64,
}

/// Run options.
#[derive(Debug, Clone, Copy)]
pub struct RunOpts {
    /// Stop at this `CLOCK_MONOTONIC` time.
    pub deadline_mono_ns: i64,
}

/// Builds the driver configuration for a benchmark run.
#[must_use]
pub fn config(
    backend: Backend,
    busy_poll_us: u32,
    wait_us: u32,
    sqpoll_cpu: Option<u32>,
) -> Config {
    let mut c = Config::new(backend);
    c.busy_poll_us = busy_poll_us;
    c.wait_us = wait_us;
    c.sqpoll_cpu = sqpoll_cpu;
    c
}

/// Applies the busy-poll flags under test: `--prefer-busy-poll` (backends that
/// poll the NIC only) and `--busy-poll-budget N` (`recv-busy` / `epoll-busy`).
///
/// # Errors
/// A malformed number.
pub fn busy_poll_flags(c: &mut Config, a: &crate::args::Args) -> Result<()> {
    if a.flag("prefer-busy-poll") && c.backend.polls_nic() {
        c.prefer_busy_poll = true;
    }
    if a.flag("per-queue") && c.backend == Backend::RecvBusy {
        c.per_queue = true;
    }
    let budget: u32 = a.num_or("busy-poll-budget", 0)?;
    if budget > 0 && matches!(c.backend, Backend::RecvBusy | Backend::EpollBusy) {
        c.busy_poll_budget = u16::try_from(budget).ok();
    }
    Ok(())
}

/// Polls `driver` until the deadline. A peer close within two seconds of the
/// deadline is the normal end of a lab run (the server stops once any receiver
/// finishes); any other connection error fails the run.
///
/// # Errors
/// The driver or a connection failed.
pub fn run<A: App>(
    driver: &mut Driver<WsConn>,
    app: &mut A,
    opts: RunOpts,
    stalls: &mut StallLog,
) -> Result<LoopStats> {
    let mut st = LoopStats::default();
    let needs_idle = app.needs_idle();
    st.ticks_per_us = ticks_per_us();
    let (u0, s0) = thread_cpu_ticks();
    let (v0, i0) = thread_ctx_switches();
    stalls.start(driver.stats().messages);
    let result = loop {
        // One counter read per iteration; slow iterations are logged with
        // the work they did.
        stalls.tick(driver.stats().messages);
        st.spins += 1;
        if needs_idle && let Err(e) = app.on_idle(driver) {
            break Err(e);
        }
        if let Err(e) = driver.poll(app) {
            break Err(e);
        }
        if let Some(e) = app.take_closed() {
            let at_end = matches!(e, Error::Closed)
                && monotonic_ns() >= opts.deadline_mono_ns - 2_000_000_000;
            break if at_end { Ok(()) } else { Err(e) };
        }
        // One vDSO clock read every 64 iterations.
        if st.spins & 63 == 0 && monotonic_ns() >= opts.deadline_mono_ns {
            break Ok(());
        }
    };
    let (v1, i1) = thread_ctx_switches();
    let (u1, s1) = thread_cpu_ticks();
    st.gap_max_ticks = stalls.max_ticks;
    st.gaps_over = stalls.over;
    st.vol_ctx_switches = v1 - v0;
    st.invol_ctx_switches = i1 - i0;
    st.utime_ticks = u1 - u0;
    st.stime_ticks = s1 - s0;
    st.driver = *driver.stats();
    st.napi = driver.nic_polling_active();
    result.map(|()| st)
}
