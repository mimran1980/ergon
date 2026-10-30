//! `epoll-spin` and `recv-busy`.
//!
//! - `epoll-spin`: `epoll_wait(timeout = 0)` every poll, then one `recvmsg`
//!   per ready socket (level-triggered: data left behind is reported again).
//!   With `Config::busy_poll_us` (`SO_BUSY_POLL` on every socket), a socket
//!   read with data is read once more at the start of the next poll,
//!   after the replies went out: that read finds the queue empty and makes
//!   one NAPI pass before `EAGAIN`, as mio's read to `WouldBlock` does. A
//!   datagram socket epoll reports is then read once, and its
//!   `datagram_burst` runs from that next-poll read.
//! - `recv-busy`: no epoll. A non-blocking `recvmsg` on every socket every
//!   poll, with `SO_BUSY_POLL` set (by [`super::Driver::add`]). An empty read
//!   runs `sk_busy_loop(sk, nonblock)`: one NAPI pass, then `EAGAIN`. Never
//!   sleeps. One syscall per socket per poll.
//! - `recv-busy` with `Config::per_queue`: one read per NIC queue (one
//!   socket per NAPI id; sockets whose queue is not known yet are all read),
//!   which polls every queue once, then `epoll_wait(0)` for the sockets those
//!   passes (or interrupts) made ready. A socket's queue is looked up after
//!   its first data, and the look-again checks one socket every 16 polls in
//!   turn. No poll makes more than one lookup (one `getsockopt`).
//!
//! Both run over a [`NetStack`] (`Kernel` by default): every socket, poller
//! and receive call goes through it. `fstack-spin` is `epoll-spin` over
//! F-Stack: each poll starts with one pass of the stack
//! ([`NetStack::run_once`], nothing for the kernel), then its kqueue.

use std::os::fd::RawFd;

use super::{Conns, recv_batch, recv_one};
use crate::backend::{Backend, Config};
use crate::endpoint::{ConnId, DriverStats, Endpoint, Handler, Receive};
use crate::error::{Error, Result};
use crate::netstack::{Kernel, NetStack};

/// `recv-busy` with `per_queue`: the look-again checks one socket's queue
/// id every this many polls.
const LOOK_AGAIN_EVERY: u64 = 16;

/// The fewest datagrams one batched send takes (the batch is
/// `datagram_burst` entries when that is larger).
const TX_BATCH: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Spin,
    Recv,
}

#[allow(clippy::struct_excessive_bools)] // independent switches from the configuration
pub(super) struct Epoll<S: NetStack = Kernel> {
    mode: Mode,
    /// `recv-busy` with `Config::per_queue`.
    per_queue: bool,
    /// Per slot: the socket's NAPI id once known (0 until then).
    napi: Vec<u32>,
    /// Per slot: its id was looked up once after data arrived. A socket
    /// still at 0 then has no NAPI id (loopback, a plain veth): it is asked
    /// again only on the periodic look-again.
    asked: Vec<bool>,
    /// The NAPI ids seen while picking (scratch space, kept to avoid an
    /// allocation per rebuild).
    seen: Vec<u32>,
    /// Rebuilds of the representatives (test lever).
    picks: u64,
    /// Queue-id lookups, one `getsockopt` each (test lever).
    lookups: u64,
    /// Per slot: read every poll (one per NIC queue, plus unknown queues).
    polled: Vec<bool>,
    /// The slots in `polled`, in order.
    reps: Vec<usize>,
    /// Sockets came or went, or a queue id changed.
    dirty: bool,
    /// Slots whose first data arrived, to be looked up one a poll.
    ask: Vec<usize>,
    /// The look-again's next slot.
    cursor: usize,
    polls: u64,
    ep: Option<S::Poller>,
    slot: Box<S::RxSlot>,
    /// `Config::epoll_events` entries.
    events: Box<[S::Event]>,
    /// `Config::datagram_burst`: reads per ready datagram socket per poll.
    burst: u32,
    /// Headers for batched datagram receives and sends, on a stack with
    /// them ([`NetStack::BATCH`]).
    mm: Option<Box<crate::sys::MmsgBatch>>,
    /// `epoll-spin` with `Config::busy_poll_us`: a socket read with data is
    /// read once more on the next poll. That read finds the queue
    /// empty and, with `SO_BUSY_POLL` set, makes one NAPI pass first.
    drain: bool,
    /// The slots to read again on the next poll, each once.
    again: Vec<usize>,
    /// `again` while it is being read (kept to avoid an allocation).
    draining: Vec<usize>,
    /// Per slot: in `again`.
    queued: Vec<bool>,
    /// Reading the `again` list (a datagram socket's burst runs there).
    again_pass: bool,
}

impl<S: NetStack> Epoll<S> {
    pub(super) fn new(cfg: &Config) -> Result<Self> {
        let mode = if cfg.backend == Backend::RecvBusy {
            Mode::Recv
        } else {
            Mode::Spin
        };
        let per_queue = mode == Mode::Recv && cfg.per_queue;
        let ep = if mode == Mode::Recv && !per_queue {
            None
        } else {
            Some(S::poller()?)
        };
        Ok(Self {
            mode,
            per_queue,
            napi: Vec::new(),
            asked: Vec::new(),
            seen: Vec::with_capacity(16),
            picks: 0,
            lookups: 0,
            polled: Vec::new(),
            reps: Vec::new(),
            dirty: true,
            ask: Vec::new(),
            cursor: 0,
            polls: 0,
            ep,
            slot: S::rx_slot(),
            events: vec![S::empty_event(); cfg.epoll_events as usize].into_boxed_slice(),
            burst: cfg.datagram_burst,
            mm: S::BATCH.then(|| {
                Box::new(crate::sys::MmsgBatch::new(
                    (cfg.datagram_burst as usize).max(TX_BATCH),
                ))
            }),
            drain: mode == Mode::Spin && cfg.busy_poll_us > 0,
            again: Vec::new(),
            draining: Vec::new(),
            queued: Vec::new(),
            again_pass: false,
        })
    }

    pub(super) fn add(&mut self, fd: RawFd, id: ConnId) -> Result<()> {
        self.dirty = true;
        // The slot may be a reused one whose cached NAPI id belongs to the
        // connection that left it.
        if let Some(n) = self.napi.get_mut(id.index()) {
            *n = 0;
        }
        if let Some(a) = self.asked.get_mut(id.index()) {
            *a = false;
        }
        // A lookup still queued for the connection that left the slot.
        self.ask.retain(|&j| j != id.index());
        if self.per_queue && self.ask.capacity() <= id.index() {
            // Room for every slot now, so the poll never allocates.
            self.ask.reserve(id.index() + 1 - self.ask.len());
        }
        if self.drain && self.queued.len() <= id.index() {
            // Room for every slot now, so the poll never allocates.
            let n = id.index() + 1;
            self.queued.resize(n, false);
            self.again.reserve(n.saturating_sub(self.again.len()));
            self.draining.reserve(n.saturating_sub(self.draining.len()));
        }
        let Some(ep) = &self.ep else {
            return Ok(());
        };
        S::register(ep, fd, id.index() as u64)?;
        Ok(())
    }

    pub(super) fn remove(&mut self, fd: RawFd) {
        self.dirty = true;
        if let Some(ep) = &self.ep {
            S::deregister(ep, fd);
        }
    }

    #[inline]
    pub(super) fn poll<E, H>(
        &mut self,
        conns: &mut Conns<E>,
        handler: &mut H,
        stats: &mut DriverStats,
        reads: &mut super::Reads,
    ) -> Result<()>
    where
        E: Receive<H>,
        H: Handler,
    {
        // A user-space stack's pass (RX burst, timers, TX drain); nothing
        // for the kernel.
        S::run_once();
        if self.mode == Mode::Recv && !self.per_queue {
            for i in 0..conns.len_slots() {
                self.recv_index(conns, i, handler, stats, reads);
            }
        } else if self.per_queue {
            self.refresh_reps(conns);
            for k in 0..self.reps.len() {
                let i = self.reps[k];
                self.recv_index(conns, i, handler, stats, reads);
            }
            let n = self.wait(stats)?;
            for k in 0..n {
                let i = usize::try_from(S::token(&self.events[k])).unwrap_or(usize::MAX);
                // Read already this poll (anything left is reported again).
                if self.polled.get(i).copied().unwrap_or(false) {
                    continue;
                }
                self.recv_index(conns, i, handler, stats, reads);
            }
        } else {
            if !self.again.is_empty() {
                // The read to empty after last poll's data: the NIC poll.
                std::mem::swap(&mut self.again, &mut self.draining);
                self.again_pass = true;
                for k in 0..self.draining.len() {
                    let i = self.draining[k];
                    if let Some(q) = self.queued.get_mut(i) {
                        *q = false;
                    }
                    self.recv_index(conns, i, handler, stats, reads);
                }
                self.draining.clear();
                self.again_pass = false;
            }
            let n = self.wait(stats)?;
            for k in 0..n {
                let i = usize::try_from(S::token(&self.events[k])).unwrap_or(usize::MAX);
                self.recv_index(conns, i, handler, stats, reads);
            }
        }
        Ok(())
    }

    /// Looks up at most one queue id (one `getsockopt`): a socket whose
    /// first data arrived, or else, every [`LOOK_AGAIN_EVERY`] polls, the
    /// next live socket in turn, which catches a NIC moving a queue under a
    /// live socket (RSS remap). Rebuilds the representatives only when a
    /// socket came or went or an id changed. Otherwise a few branches.
    #[inline]
    fn refresh_reps<E: Endpoint>(&mut self, conns: &mut Conns<E>) {
        self.polls += 1;
        let n = conns.len_slots();
        if self.napi.len() < n {
            self.napi.resize(n, 0);
            self.asked.resize(n, false);
        }
        let slot = if let Some(i) = self.ask.pop() {
            Some(i)
        } else if self.polls.is_multiple_of(LOOK_AGAIN_EVERY) && n > 0 {
            // The next live slot, at most one lap.
            (0..n)
                .map(|k| (self.cursor + k) % n)
                .find(|&i| conns.at(i).is_some())
                .inspect(|&i| self.cursor = (i + 1) % n)
        } else {
            None
        };
        if let Some(i) = slot
            && let Some(c) = conns.at(i)
        {
            self.lookups += 1;
            let id = S::napi_id(c.fd()).unwrap_or(0);
            if self.napi[i] != id {
                self.napi[i] = id;
                self.dirty = true;
            }
        }
        if self.dirty {
            self.pick_reps(conns);
        }
    }

    /// The batched-datagram headers, on a stack with them.
    #[inline]
    pub(super) fn batch(&mut self) -> Option<&mut crate::sys::MmsgBatch> {
        self.mm.as_deref_mut()
    }

    /// Data arrived on slot `i`: when its queue is unknown, look it up once
    /// (on the next poll), not every poll.
    #[inline]
    fn data_arrived(&mut self, i: usize) {
        if self.per_queue && self.napi.get(i) == Some(&0) && self.asked.get(i) == Some(&false) {
            self.asked[i] = true;
            self.ask.push(i);
        }
    }

    #[inline]
    fn recv_index<E, H>(
        &mut self,
        conns: &mut Conns<E>,
        i: usize,
        handler: &mut H,
        stats: &mut DriverStats,
        reads: &mut super::Reads,
    ) where
        E: Receive<H>,
        H: Handler,
    {
        let id = conns.id(i);
        let Some(conn) = conns.at(i) else {
            return;
        };
        // With `drain`, a datagram socket epoll reports is read once and
        // its burst runs from the again list on the next poll, so the empty
        // read that ends it (the NIC poll) comes after this poll's replies.
        let burst = E::DATAGRAM && (!self.drain || self.again_pass);
        let mut r = match self.mm.as_deref_mut() {
            Some(mm) if burst => {
                recv_batch::<S, _, _>(conn, id, mm, self.burst, handler, stats, reads)
            }
            _ => recv_one::<S, _, _>(conn, id, &mut self.slot, handler, stats, reads),
        };
        if burst && self.mm.is_none() {
            // Up to `burst` datagrams from this socket, then the next one;
            // what is left is reported again by the next `epoll_wait`.
            let mut left = self.burst;
            while matches!(r, Ok(true)) && left > 1 {
                left -= 1;
                r = recv_one::<S, _, _>(conn, id, &mut self.slot, handler, stats, reads);
            }
        }
        match r {
            Ok(false) => {}
            // A reply (a Pong) may be queued: list it for the send pass.
            Ok(true) => {
                if conn.has_tx() {
                    conns.mark_tx(i);
                }
                self.data_arrived(i);
                // Read again on the next poll: to `EAGAIN`, which polls the NIC.
                if self.drain && self.queued.get(i) == Some(&false) {
                    self.queued[i] = true;
                    self.again.push(i);
                }
            }
            Err(e) => {
                let e = conn.close_reason(e);
                self.remove(conn.fd());
                conns.remove(i);
                stats.closed += 1;
                handler.on_closed(id, e);
            }
        }
    }

    /// One slot per known NIC queue, plus every slot whose queue is unknown,
    /// from the cached ids: no lookups here.
    fn pick_reps<E: Endpoint>(&mut self, conns: &mut Conns<E>) {
        self.picks += 1;
        let n = conns.len_slots();
        self.napi.resize(n, 0);
        self.asked.resize(n, false);
        self.polled.clear();
        self.polled.resize(n, false);
        self.reps.clear();
        self.seen.clear();
        for i in 0..n {
            if conns.at(i).is_none() {
                self.napi[i] = 0;
                self.asked[i] = false;
                continue;
            }
            let id = self.napi[i];
            if id != 0 {
                if self.seen.contains(&id) {
                    continue;
                }
                self.seen.push(id);
            }
            // Unknown queues are all read: nothing is missed meanwhile.
            self.polled[i] = true;
            self.reps.push(i);
        }
        self.dirty = false;
    }

    fn wait(&mut self, stats: &mut DriverStats) -> Result<usize> {
        let Some(ep) = &self.ep else {
            return Ok(0);
        };
        stats.epoll_wait_calls += 1;
        S::poll_now(ep, &mut self.events).map_err(Error::Io)
    }

    /// The connection's `TCP_INFO`, from this backend's stack.
    #[allow(clippy::unused_self)] // the stack is the backend's type
    pub(super) fn tcp_info(&self, fd: RawFd) -> std::io::Result<crate::TcpInfo> {
        S::tcp_info(fd)
    }

    /// The NIC queue of the socket's last packet (0: none known), from this
    /// backend's stack.
    #[allow(clippy::unused_self)] // the stack is the backend's type
    pub(super) fn napi_id(&self, fd: RawFd) -> u32 {
        S::napi_id(fd).unwrap_or(0)
    }
}

#[cfg(test)]
#[allow(unsafe_code)] // socketpair(2) for the stand-in sockets
mod tests {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    use super::*;
    use crate::endpoint::sealed::{Io, RouteSlot};

    type R<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// A stand-in connection: a real socket and nothing else.
    struct Stub {
        fd: OwnedFd,
        rx: Vec<u8>,
        tx: crate::stream::TxQueue,
        route: RouteSlot,
    }

    impl Stub {
        fn socket() -> R<Self> {
            let mut fds = [0; 2];
            // SAFETY: two fresh descriptors are written into `fds`.
            if unsafe {
                libc::socketpair(
                    libc::AF_UNIX,
                    libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
                    0,
                    fds.as_mut_ptr(),
                )
            } != 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            // SAFETY: each entry of `fds` is a descriptor we now own.
            let (fd, peer) =
                unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
            drop(peer);
            Ok(Self {
                fd,
                rx: Vec::new(),
                tx: crate::stream::TxQueue::default(),
                route: RouteSlot::default(),
            })
        }
    }

    impl Io for Stub {
        fn fd(&self) -> RawFd {
            self.fd.as_raw_fd()
        }
        fn rx_space(&mut self) -> &mut [u8] {
            &mut self.rx
        }
        fn tx_queue(&mut self) -> &mut crate::stream::TxQueue {
            &mut self.tx
        }
        fn has_buffered_input(&self) -> bool {
            false
        }
        fn set_route(&mut self, route: RouteSlot) {
            self.route = route;
        }
        fn route_state(&self) -> &RouteSlot {
            &self.route
        }
    }

    fn per_queue() -> R<(Epoll, Conns<Stub>)> {
        let mut cfg = Config::new(Backend::RecvBusy);
        cfg.per_queue = true;
        Ok((Epoll::new(&cfg)?, Conns::with_capacity(0)))
    }

    fn install(ep: &mut Epoll, conns: &mut Conns<Stub>) -> R<usize> {
        let i = conns.reserve();
        let stub = Stub::socket()?;
        ep.add(stub.fd(), conns.id(i))?;
        conns.install(i, stub);
        Ok(i)
    }

    /// Two sockets the cache says share a queue: only the first is polled.
    /// A slot reused for a connection on another queue must drop the previous
    /// connection's id, or the replacement is judged to still share the old
    /// queue, is never chosen as a representative, and its own queue is never
    /// busy-polled.
    #[test]
    fn a_reused_slot_drops_the_previous_connections_queue_id() -> R {
        let (mut ep, mut conns) = per_queue()?;
        install(&mut ep, &mut conns)?;
        install(&mut ep, &mut conns)?;
        ep.napi.resize(conns.len_slots(), 0);
        ep.napi.fill(5);
        ep.pick_reps(&mut conns);
        assert_eq!(ep.reps, [0], "{:?}", ep.reps);

        // Slot 1 is reused for a connection the NIC has put on another queue.
        ep.remove(conns.at(1).ok_or("slot 1")?.fd());
        conns.remove(1);
        let i = install(&mut ep, &mut conns)?;
        assert_eq!(i, 1, "the slot is reused");

        assert_eq!(ep.napi[1], 0, "the reused slot kept the old queue id");
        ep.refresh_reps(&mut conns);
        assert!(
            ep.reps.contains(&1),
            "the replacement is never a polling representative: {:?}",
            ep.reps
        );
        Ok(())
    }

    /// However many sockets there are, and however many see their first data
    /// at once, no poll asks the kernel for more than one queue id: the
    /// look-again checks one socket at a time, never all of them in one poll.
    #[test]
    fn no_poll_looks_up_more_than_one_queue_id() -> R {
        let (mut ep, mut conns) = per_queue()?;
        for _ in 0..256 {
            install(&mut ep, &mut conns)?;
        }
        for i in 0..256 {
            ep.data_arrived(i);
        }
        let mut most = 0;
        for _ in 0..20_000 {
            let before = ep.lookups;
            ep.refresh_reps(&mut conns);
            most = most.max(ep.lookups - before);
        }
        assert_eq!(most, 1, "the most queue-id lookups in one poll");
        assert!(
            ep.lookups >= 256,
            "every first data was looked up: {}",
            ep.lookups
        );
        Ok(())
    }

    /// A NIC can move a queue under a live socket (RSS remap). The look-again
    /// corrects a stale id within `LOOK_AGAIN_EVERY` polls per socket.
    #[test]
    fn a_stale_queue_id_is_corrected_by_the_look_again() -> R {
        let (mut ep, mut conns) = per_queue()?;
        install(&mut ep, &mut conns)?;
        install(&mut ep, &mut conns)?;
        ep.refresh_reps(&mut conns);
        // Both sockets cached on one queue; the kernel says neither is.
        ep.napi.fill(9);
        ep.pick_reps(&mut conns);
        assert_eq!(ep.reps, [0], "{:?}", ep.reps);
        for _ in 0..2 * 16 {
            ep.refresh_reps(&mut conns);
        }
        assert!(
            ep.napi.iter().all(|&id| id == 0),
            "a stale id survived: {:?}",
            ep.napi
        );
        assert_eq!(ep.reps, [0, 1], "{:?}", ep.reps);
        Ok(())
    }

    /// Sockets with no NAPI id (loopback, a plain veth) never learn one: the
    /// representatives are rebuilt for the adds and never again, and a
    /// rebuild reuses its scratch space.
    #[test]
    fn unknown_queues_are_not_rebuilt_every_poll() -> R {
        let (mut ep, mut conns) = per_queue()?;
        install(&mut ep, &mut conns)?;
        install(&mut ep, &mut conns)?;
        for _ in 0..10_000 {
            ep.refresh_reps(&mut conns);
        }
        assert_eq!(ep.picks, 1, "picks over 10,000 polls");
        assert_eq!(ep.reps, [0, 1], "unknown queues are all still read");
        let cap = ep.seen.capacity();
        for _ in 0..10_000 {
            ep.refresh_reps(&mut conns);
        }
        assert_eq!(ep.seen.capacity(), cap, "the scratch space grew");
        Ok(())
    }

    /// A socket's first data asks for its queue once; later data on a socket
    /// that still has no id asks nothing more. Over 100 polls with data every
    /// poll there is that one lookup plus the look-again's one per 16 polls.
    #[test]
    fn first_data_looks_a_queue_up_once() -> R {
        let (mut ep, mut conns) = per_queue()?;
        install(&mut ep, &mut conns)?;
        install(&mut ep, &mut conns)?;
        ep.refresh_reps(&mut conns);
        let before = ep.lookups;
        for _ in 0..100 {
            ep.data_arrived(0);
            ep.refresh_reps(&mut conns);
        }
        let n = ep.lookups - before;
        assert!(n <= 1 + 100 / 16 + 1, "{n} lookups over 100 polls");
        Ok(())
    }
}
