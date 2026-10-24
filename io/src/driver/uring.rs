//! `uring-spin`, `uring-napi` and `uring-sqpoll`.
//!
//! - Each connection keeps one single-shot `IORING_OP_RECVMSG` armed at the
//!   free tail of its receive buffer: the kernel copies straight into the
//!   buffer the TLS layer decrypts in place. (Multishot with a provided-buffer
//!   ring would save the re-arm but needs a copy to stitch TLS records that
//!   straddle ring buffers.)
//! - `uring-spin`: `SINGLE_ISSUER | DEFER_TASKRUN | TASKRUN_FLAG`. `poll`
//!   reads the completion ring and the `IORING_SQ_TASKRUN` flag in shared
//!   memory, and enters the kernel only with submissions or flagged work.
//!   Idle polls make no syscalls.
//! - `uring-napi`: the same ring plus NAPI busy poll (static NAPI ids). An
//!   idle poll waits in `io_uring_enter(GETEVENTS, 1, wait_us)`: the kernel
//!   polls the NIC, then sleeps until a completion or the timeout. Even with
//!   `wait_us = 0` it schedules on an expired timer, so it can sleep; with
//!   `wait_us = 0` the NAPI budget is 0 (one pass).
//! - `uring-sqpoll`: an `SQPOLL` kernel thread (optionally pinned) submits,
//!   completes and polls the NIC; `poll` only reads the completion ring.
//!
//! Safety of in-flight operations:
//!
//! - The kernel holds pointers into each connection's `MsgSlot`, receive
//!   buffer and send buffer until the matching completion is reaped.
//! - Closing a connection cancels its requests and keeps it alive until both
//!   completions arrive; dropping the driver cancels everything and reaps
//!   before freeing.
//! - Requests are tagged with the slot, its generation (bumped for each new
//!   connection in the slot) and the operation; cancels match that exact
//!   tag, so a late cancel never reaches a later connection in the same slot
//!   or descriptor.
//! - Submissions that do not fit in the queue wait in a backlog for the next
//!   poll; they are never failures.
#![allow(unsafe_code)]

use std::collections::VecDeque;

use io_uring::types::{CancelBuilder, Napi, NapiTracking};
use io_uring::{IoUring, opcode, squeue, types};

use super::Conns;
use crate::backend::{Backend, Config};
use crate::endpoint::sealed::Io;
use crate::endpoint::{DriverStats, Endpoint, Handler, Receive};
use crate::error::{Error, Result};
use crate::sys::{self, MsgSlot};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Spin,
    Napi,
    Sqpoll,
}

const RECV: u64 = 0;
const SEND: u64 = 1;
const CANCEL: u64 = 2;
const MAX_CQES: usize = 64;

struct Slot {
    msg: Box<MsgSlot>,
    send_buf: Vec<u8>,
    send_off: usize,
    recv_armed: bool,
    send_active: bool,
    /// Bumped for every connection placed in this slot (24 bits are tagged).
    generation: u32,
    /// Static NAPI tracking: whether this connection's NIC queue is known and
    /// registered (it is learnt from the first packet the socket receives).
    napi_known: bool,
    /// The NAPI id this connection holds a reference to (0: none).
    napi_id: u32,
}

pub(super) struct Uring {
    // Dropped first: closing the ring fd before the buffers below.
    ring: IoUring,
    mode: Mode,
    slots: Vec<Slot>,
    pending: usize,
    wait: types::Timespec,
    /// Static NAPI ids registered with the ring and how many open
    /// connections use each; an id is deleted when its last one closes.
    napi_refs: Vec<(u32, usize)>,
    napi_static: bool,
    /// Submissions that did not fit in the queue, in order.
    backlog: VecDeque<squeue::Entry>,
}

impl Uring {
    pub(super) fn new(cfg: &Config) -> Result<Self> {
        let mode = match cfg.backend {
            Backend::UringNapi => Mode::Napi,
            Backend::UringSqpoll => Mode::Sqpoll,
            _ => Mode::Spin,
        };
        let mut b = IoUring::builder();
        if mode == Mode::Sqpoll {
            // NAPI passes do not count as SQPOLL work, so keep the idle window
            // long or the thread sleeps through quiet periods.
            b.setup_sqpoll(cfg.sqpoll_idle_ms);
            // The SQPOLL thread inherits its creator's affinity: with a pinned
            // polling thread it would spin on the poller's own core and steal
            // whole time slices. `Driver::new` has set its CPU
            // (`placement::sqpoll_cpu`: `sqpoll_cpu`, else the first helper
            // CPU — except an explicit helper list, the shared-work CPUs,
            // where it takes a CPU of the process's own outside that list;
            // inside the cpuset, where `io_uring_setup` refuses any other
            // with EINVAL).
            if let Some(cpu) = cfg.sqpoll_cpu {
                b.setup_sqpoll_cpu(cpu);
            }
            b.setup_single_issuer();
        } else {
            b.setup_single_issuer()
                .setup_defer_taskrun()
                .setup_taskrun_flag();
        }
        let ring = b.build(cfg.ring_entries).map_err(|e| {
            if e.raw_os_error() == Some(libc::ENOSYS) || e.raw_os_error() == Some(libc::EPERM) {
                Error::Unsupported("io_uring is disabled or unavailable on this kernel")
            } else {
                Error::Io(e)
            }
        })?;
        let mut napi_static = false;
        if mode != Mode::Spin {
            // With a zero wait the GETEVENTS busy loop is capped only by the NAPI
            // budget, not by the expired timeout: budget 0 makes it one pass.
            let budget = if mode == Mode::Napi && cfg.wait_us == 0 {
                0
            } else {
                cfg.busy_poll_us.max(1)
            };
            // Static tracking (6.13+) polls exactly the queues of our sockets;
            // older kernels (6.9+) track the queues of sockets with requests.
            let register = |tracking| {
                let mut napi = Napi::new()
                    .set_busy_poll_timeout(budget)
                    .set_prefer_busy_poll(cfg.prefer_busy_poll)
                    .set_tracking(tracking);
                ring.submitter().register_napi(&mut napi)
            };
            let napi_error = |e: std::io::Error| match e.raw_os_error() {
                Some(libc::EINVAL) => {
                    Error::Unsupported("IORING_REGISTER_NAPI needs Linux 6.9 or newer")
                }
                Some(libc::EOPNOTSUPP) => Error::Unsupported(
                    "the kernel was built without busy polling (CONFIG_NET_RX_BUSY_POLL)",
                ),
                _ => Error::Io(e),
            };
            match register(NapiTracking::Static) {
                Ok(()) => napi_static = true,
                // Static tracking is 6.13+: only "unknown mode" falls back.
                Err(e) if e.raw_os_error() == Some(libc::EINVAL) => {
                    register(NapiTracking::Dynamic).map_err(napi_error)?;
                }
                Err(e) => return Err(napi_error(e)),
            }
        }
        Ok(Self {
            ring,
            mode,
            slots: Vec::new(),
            pending: 0,
            wait: types::Timespec::new()
                .sec(u64::from(cfg.wait_us / 1_000_000))
                .nsec((cfg.wait_us % 1_000_000) * 1000),
            napi_refs: Vec::new(),
            napi_static,
            backlog: VecDeque::with_capacity(cfg.ring_entries as usize),
        })
    }

    pub(super) fn add<E: Endpoint>(&mut self, ep: &mut E, i: usize) -> Result<()> {
        while self.slots.len() <= i {
            // Written once now: swapped with the connection's queue on every
            // send, so its pages must not fault on the first one.
            let mut send_buf = Vec::with_capacity(1 << 16);
            crate::stream::prefault(&mut send_buf);
            self.slots.push(Slot {
                msg: MsgSlot::new(),
                send_buf,
                send_off: 0,
                recv_armed: false,
                send_active: false,
                generation: 0,
                napi_known: true,
                napi_id: 0,
            });
        }
        // A reused slot gives back the previous connection's queue reference
        // before the new one takes its own: dropping the id without that would
        // leave the queue registered for a connection that is gone.
        self.release_napi(i);
        let s = &mut self.slots[i];
        s.generation = s.generation.wrapping_add(1) & GEN_MASK;
        s.napi_known = !self.napi_static;
        // A socket that has received nothing yet has no NAPI id: its queue is
        // registered on its first receive instead (`on_recv`).
        if self.napi_static {
            let known = self.register_napi_of(ep.fd(), i)?;
            self.slots[i].napi_known = known;
        }
        // Input left from the handshake is decoded first (the driver's next
        // poll), which may move the buffer: arm after that (`arm_initial`).
        if !ep.has_buffered_input() {
            self.arm_recv(ep, i);
        }
        Ok(())
    }

    /// Registers the NIC queue that `fd`'s last packet came from (static
    /// tracking) for slot `i`; `false` while the socket has received nothing.
    /// A getsockopt, plus a register call for a new queue: once per
    /// connection, at set-up or on its first receive, on the polling thread.
    fn register_napi_of(&mut self, fd: std::os::fd::RawFd, i: usize) -> Result<bool> {
        let id = sys::incoming_napi_id(fd).unwrap_or(0);
        if id == 0 {
            return Ok(false);
        }
        if let Some(r) = self.napi_refs.iter_mut().find(|r| r.0 == id) {
            r.1 += 1;
        } else {
            self.ring.submitter().register_napi_add_id(id)?;
            self.napi_refs.push((id, 1));
        }
        self.slots[i].napi_id = id;
        Ok(true)
    }

    /// Drops slot `i`'s NAPI reference; the queue stops being polled when no
    /// open connection uses it.
    fn release_napi(&mut self, i: usize) {
        let id = std::mem::take(&mut self.slots[i].napi_id);
        if id == 0 {
            return;
        }
        if let Some(k) = self.napi_refs.iter().position(|r| r.0 == id) {
            self.napi_refs[k].1 -= 1;
            if self.napi_refs[k].1 == 0 {
                self.napi_refs.swap_remove(k);
                let _ = self.ring.submitter().register_napi_del_id(id);
            }
        }
    }

    /// Arms the first receive of a connection added with buffered input.
    pub(super) fn arm_initial<E: Io>(&mut self, ep: &mut E, i: usize) {
        if self.slots.get(i).is_some_and(|s| !s.recv_armed) {
            self.arm_recv(ep, i);
        }
    }

    /// Starts closing slot `i`: cancels its requests, or drops it at once when
    /// nothing is in flight.
    pub(super) fn close<E: Endpoint>(&mut self, conns: &mut Conns<E>, i: usize) {
        if conns.at(i).is_none() {
            return;
        }
        conns.begin_close(i);
        // Requests still in the backlog never reached the kernel: drop them.
        self.purge_backlog(i);
        if let Some(s) = self.slots.get(i) {
            let g = s.generation;
            for (armed, op) in [(s.recv_armed, RECV), (s.send_active, SEND)] {
                if armed {
                    let e = opcode::AsyncCancel2::new(CancelBuilder::user_data(tag(i, g, op)))
                        .build()
                        .user_data(tag(i, g, CANCEL));
                    self.push(e);
                }
            }
        }
        self.maybe_finish(conns, i);
    }

    /// Removes slot `i`'s queued submissions (its current generation) and
    /// clears the flags of the requests they would have started.
    fn purge_backlog(&mut self, slot: usize) {
        if self.backlog.is_empty() {
            return;
        }
        let Some(state) = self.slots.get_mut(slot) else {
            return;
        };
        let generation = state.generation;
        self.backlog.retain(|entry| {
            let (other, other_generation, op) = untag(entry.get_user_data());
            if other != slot || other_generation != generation {
                return true;
            }
            match op {
                RECV => state.recv_armed = false,
                SEND => state.send_active = false,
                _ => {}
            }
            false
        });
    }

    fn maybe_finish<E>(&mut self, conns: &mut Conns<E>, i: usize) {
        if conns.is_closing(i)
            && self
                .slots
                .get(i)
                .is_none_or(|s| !s.recv_armed && !s.send_active)
        {
            // Leftover cancels for this connection are not needed any more.
            self.purge_backlog(i);
            if let Some(s) = self.slots.get_mut(i) {
                s.send_buf.clear();
                s.send_off = 0;
            }
            if i < self.slots.len() {
                self.release_napi(i);
            }
            conns.finish_close(i);
        }
    }

    fn fail<E: Endpoint, H: Handler>(
        &mut self,
        conns: &mut Conns<E>,
        i: usize,
        err: Error,
        handler: &mut H,
        stats: &mut DriverStats,
    ) {
        let id = conns.id(i);
        if !conns.is_open(i) {
            return;
        }
        let err = match conns.at(i) {
            Some(c) => c.close_reason(err),
            None => err,
        };
        self.close(conns, i);
        stats.closed += 1;
        handler.on_closed(id, err);
    }

    #[inline]
    pub(super) fn poll<E, H>(
        &mut self,
        conns: &mut Conns<E>,
        handler: &mut H,
        stats: &mut DriverStats,
        read_seq: &mut u64,
    ) -> Result<()>
    where
        E: Receive<H>,
        H: Handler,
    {
        if !self.backlog.is_empty() {
            self.drain_backlog();
        }
        if self.ring.completion().is_empty() {
            // Idle fast path: nothing to reap.
            self.push_sends(conns, stats);
            return self.enter(true, stats);
        }
        let mut cqes = [(0u64, 0i32); MAX_CQES];
        let mut n = 0;
        for cqe in self.ring.completion() {
            cqes[n] = (cqe.user_data(), cqe.result());
            n += 1;
            if n == MAX_CQES {
                break;
            }
        }
        for &(ud, res) in &cqes[..n] {
            let (i, g, op) = untag(ud);
            // Every request of a slot completes before the slot is reused, so
            // a mismatch is a cancel's own completion at most; never act on it.
            if self.slots.get(i).is_none_or(|s| s.generation != g) {
                continue;
            }
            match op {
                RECV => self.on_recv(conns, i, res, handler, stats, read_seq),
                SEND => self.on_send(conns, i, res, handler, stats),
                _ => {}
            }
        }
        self.push_sends(conns, stats);
        self.enter(n == 0, stats)
    }

    #[inline]
    fn on_recv<E, H>(
        &mut self,
        conns: &mut Conns<E>,
        i: usize,
        res: i32,
        handler: &mut H,
        stats: &mut DriverStats,
        read_seq: &mut u64,
    ) where
        E: Receive<H>,
        H: Handler,
    {
        let Some(slot) = self.slots.get_mut(i) else {
            return;
        };
        slot.recv_armed = false;
        if !conns.is_open(i) {
            self.maybe_finish(conns, i);
            return;
        }
        let id = conns.id(i);
        let Some(conn) = conns.at(i) else {
            return;
        };
        let napi_known = slot.napi_known;
        let mut queued = false;
        let outcome = match usize::try_from(res) {
            Ok(0) => Err(Error::Closed),
            Ok(len) => {
                *read_seq += 1;
                stats.completions += 1;
                let meta = slot.msg.meta(*read_seq);
                let r = conn
                    .on_received(id, len, &meta, handler)
                    .map(|m| stats.messages += m as u64);
                // A reply (a Pong) may be queued: list it for `push_sends`.
                queued = !conn.tx_queue().is_empty();
                r
            }
            Err(_) if res == -libc::EAGAIN || res == -libc::EINTR => Ok(()),
            Err(_) => Err(Error::Io(std::io::Error::from_raw_os_error(-res))),
        };
        // The first packet tells which NIC queue to poll (one getsockopt per
        // connection).
        let outcome = outcome.and_then(|()| {
            if !napi_known && res > 0 {
                let known = self.register_napi_of(conn.fd(), i)?;
                self.slots[i].napi_known = known;
            }
            Ok(())
        });
        let outcome = outcome.map(|()| self.arm_recv(conn, i));
        match outcome {
            Err(e) => self.fail(conns, i, e, handler, stats),
            Ok(()) if queued => conns.mark_tx(i),
            Ok(()) => {}
        }
    }

    fn on_send<E: Endpoint, H: Handler>(
        &mut self,
        conns: &mut Conns<E>,
        i: usize,
        res: i32,
        handler: &mut H,
        stats: &mut DriverStats,
    ) {
        let Some(slot) = self.slots.get_mut(i) else {
            return;
        };
        slot.send_active = false;
        if !conns.is_open(i) {
            self.maybe_finish(conns, i);
            return;
        }
        let Ok(sent) = usize::try_from(res) else {
            let e = Error::Io(std::io::Error::from_raw_os_error(-res));
            self.fail(conns, i, e, handler, stats);
            return;
        };
        slot.send_off += sent;
        if slot.send_off < slot.send_buf.len() {
            self.push_send(conns, i, stats);
        } else {
            slot.send_buf.clear();
            slot.send_off = 0;
        }
    }

    /// Moves the queued bytes of each connection in the send set with no send
    /// in flight into its in-flight send buffer (a swap, no copy) and queues
    /// the send. A connection whose send is still in flight stays listed
    /// while it has bytes queued; the others leave the set.
    #[inline]
    fn push_sends<E: Endpoint>(&mut self, conns: &mut Conns<E>, stats: &mut DriverStats) {
        let mut k = 0;
        while let Some(&i) = conns.tx_dirty.get(k) {
            let open = conns.is_open(i);
            let (Some(slot), Some(conn)) = (self.slots.get_mut(i), conns.at(i).filter(|_| open))
            else {
                conns.unlist_tx(k);
                continue;
            };
            let queue = conn.tx_queue();
            if queue.is_empty() {
                conns.unlist_tx(k);
                continue;
            }
            if slot.send_active {
                k += 1;
                continue;
            }
            let len = queue.len();
            slot.send_off = queue.swap_out(&mut slot.send_buf);
            // Counted as handed once queued for the kernel: an upper bound,
            // so a close before the kernel takes it errs towards "maybe sent".
            conn.on_handed(len);
            self.push_send(conns, i, stats);
            conns.unlist_tx(k);
        }
    }

    pub(super) fn flush<E: Endpoint>(
        &mut self,
        conns: &mut Conns<E>,
        stats: &mut DriverStats,
    ) -> Result<()> {
        if !self.backlog.is_empty() {
            self.drain_backlog();
        }
        self.push_sends(conns, stats);
        self.enter(false, stats)
    }

    #[inline]
    fn enter(&mut self, idle: bool, stats: &mut DriverStats) -> Result<()> {
        match self.mode {
            Mode::Sqpoll => {
                // Completions that overflowed the ring wait in the kernel until
                // an enter collects them.
                let overflow = self.ring.submission().cq_overflow();
                if self.pending > 0 || overflow {
                    // A syscall happens only when the SQPOLL thread went to
                    // sleep or completions overflowed.
                    if overflow || self.ring.submission().need_wakeup() {
                        stats.enter_calls += 1;
                    }
                    self.ring.submit()?;
                    self.pending = 0;
                }
            }
            Mode::Napi if idle => {
                stats.enter_calls += 1;
                let args = types::SubmitArgs::new().timespec(&self.wait);
                match self.ring.submitter().submit_with_args(1, &args) {
                    Ok(_) => {}
                    Err(e)
                        if matches!(
                            e.raw_os_error(),
                            Some(libc::ETIME | libc::EINTR | libc::EBUSY)
                        ) => {}
                    Err(e) => return Err(e.into()),
                }
                self.pending = 0;
            }
            _ => {
                let work = {
                    let sq = self.ring.submission();
                    sq.taskrun() || sq.cq_overflow()
                };
                if self.pending > 0 || work {
                    stats.enter_calls += 1;
                    self.ring.submitter().submit_and_wait(0)?;
                    self.pending = 0;
                }
            }
        }
        Ok(())
    }

    fn arm_recv<E: Endpoint>(&mut self, conn: &mut E, i: usize) {
        let s = &mut self.slots[i];
        s.msg.arm(conn.rx_space());
        let e = opcode::RecvMsg::new(types::Fd(conn.fd()), &raw mut s.msg.hdr)
            .build()
            .user_data(tag(i, s.generation, RECV));
        s.recv_armed = true;
        self.push(e);
    }

    fn push_send<E: Endpoint>(&mut self, conns: &mut Conns<E>, i: usize, stats: &mut DriverStats) {
        let Some(fd) = conns.at(i).map(|c| c.fd()) else {
            return;
        };
        let s = &mut self.slots[i];
        let rest = &s.send_buf[s.send_off..];
        // The kernel sends at most MAX_RW_COUNT per call anyway; `on_send`
        // queues the rest.
        let len = u32::try_from(rest.len()).unwrap_or(u32::MAX);
        let e = opcode::Send::new(types::Fd(fd), rest.as_ptr(), len)
            .flags(libc::MSG_NOSIGNAL)
            .build()
            .user_data(tag(i, s.generation, SEND));
        s.send_active = true;
        // Submitted through io_uring_enter (or the SQPOLL thread): no syscall
        // of its own.
        stats.sends += 1;
        self.push(e);
    }

    /// Queues `e`, behind any backlog so submissions keep their order. A full
    /// queue is not an error: the entry waits in the backlog for the next poll.
    fn push(&mut self, e: squeue::Entry) {
        if self.backlog.is_empty() {
            // SAFETY: every pointer in `e` (msghdr slot, receive buffer tail, send
            // buffer) stays valid and untouched until its completion is reaped:
            // slots are boxed and owned here, receive buffers belong to endpoints
            // that are only dropped after their completions (`maybe_finish`,
            // `shutdown`), and send buffers are only cleared after completion.
            // Entries waiting in the backlog keep the same guarantees.
            if unsafe { self.ring.submission().push(&e) }.is_ok() {
                self.pending += 1;
                return;
            }
        }
        self.backlog.push_back(e);
    }

    /// Moves backlog entries into the submission queue while they fit. Without
    /// `SQPOLL` an enter first makes room (the kernel consumes the whole queue).
    #[cold]
    fn drain_backlog(&mut self) {
        if self.mode != Mode::Sqpoll
            && self.ring.submission().is_full()
            && self.ring.submit().is_ok()
        {
            self.pending = 0;
        }
        while let Some(e) = self.backlog.front() {
            // SAFETY: as in `push`.
            if unsafe { self.ring.submission().push(e) }.is_err() {
                break;
            }
            self.backlog.pop_front();
            self.pending += 1;
        }
    }

    /// Cancels every request and reaps until the kernel holds no buffer, for at
    /// most one second. Returns whether it got there; if not, the caller must
    /// leak the endpoints.
    pub(super) fn shutdown(&mut self) -> bool {
        let inflight = |s: &[Slot]| s.iter().any(|s| s.recv_armed || s.send_active);
        if !inflight(&self.slots) {
            return true;
        }
        // Backlog entries never reached the kernel.
        for e in std::mem::take(&mut self.backlog) {
            let (i, g, op) = untag(e.get_user_data());
            if let Some(s) = self.slots.get_mut(i).filter(|s| s.generation == g) {
                match op {
                    RECV => s.recv_armed = false,
                    SEND => s.send_active = false,
                    _ => {}
                }
            }
        }
        if !inflight(&self.slots) {
            return true;
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        // Every queued SQE must be issued before a cancel can see it (with
        // SQPOLL the kernel thread picks them up asynchronously).
        while !self.ring.submission().is_empty() && std::time::Instant::now() < deadline {
            let _ = self.ring.submit();
            std::hint::spin_loop();
        }
        while std::time::Instant::now() < deadline {
            // A None timeout is `tv_sec = tv_nsec = -1`: the kernel waits
            // until something is cancelled, which can outlast the one second
            // this shutdown promises and defeat the safe-leak fallback. Pass
            // what is left of the bound.
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let _ = self
                .ring
                .submitter()
                .register_sync_cancel(Some(remaining.into()), CancelBuilder::any());
            let _ = self.ring.submitter().submit_and_wait(0);
            for cqe in self.ring.completion() {
                let (i, g, op) = untag(cqe.user_data());
                if let Some(s) = self.slots.get_mut(i).filter(|s| s.generation == g) {
                    match op {
                        RECV => s.recv_armed = false,
                        SEND => s.send_active = false,
                        _ => {}
                    }
                }
            }
            if !inflight(&self.slots) {
                return true;
            }
            std::hint::spin_loop();
        }
        // The kernel may still write into these: never free them.
        std::mem::forget(std::mem::take(&mut self.slots));
        false
    }
}

/// Generations are tagged with 24 bits.
const GEN_MASK: u32 = 0x00FF_FFFF;

/// `user_data`: generation (24 bits) | slot (32 bits) | operation (8 bits).
const fn tag(i: usize, generation: u32, op: u64) -> u64 {
    ((generation & GEN_MASK) as u64) << 40 | ((i as u64) & 0xFFFF_FFFF) << 8 | op
}

const fn untag(ud: u64) -> (usize, u32, u64) {
    (
        ((ud >> 8) & 0xFFFF_FFFF) as usize,
        (ud >> 40) as u32,
        ud & 0xFF,
    )
}
