//! `epoll-spin`, `epoll-busy` and `recv-busy`.
//!
//! - `epoll-spin`: `epoll_wait(timeout = 0)` every poll, then one `recvmsg`
//!   per ready socket (level-triggered: data left behind is reported again).
//! - `epoll-busy`: `epoll_pwait2(wait_us)` with busy poll on the epoll
//!   instance (`EPIOCSPARAMS`, carrying `Config::prefer_busy_poll` and
//!   `busy_poll_budget`). With nothing ready the kernel polls the NIC queue
//!   for up to `busy_poll_us`, then sleeps until an event or the timeout. A
//!   zero timeout returns before the busy loop, so this backend sleeps by
//!   design. An epoll instance tracks one NAPI id: with sockets on several
//!   RX queues, only one queue is polled.
//! - `recv-busy`: no epoll. A non-blocking `recvmsg` on every socket every
//!   poll, with `SO_BUSY_POLL` set (by [`super::Driver::add`]). An empty read
//!   runs `sk_busy_loop(sk, nonblock)`: one NAPI pass, then `EAGAIN`. Never
//!   sleeps. One syscall per socket per poll.
//! - `recv-busy` with `Config::per_queue`: one read per NIC queue (one
//!   socket per NAPI id; sockets whose queue is not known yet are all read),
//!   which polls every queue once, then `epoll_wait(0)` for the sockets those
//!   passes (or interrupts) made ready. A socket's queue is looked up after
//!   its first data and on a look-again every 4096 polls; a socket with no
//!   NAPI id (loopback, a plain veth) costs no lookup in between.
#![allow(unsafe_code)]

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

use super::{Conns, recv_one};
use crate::backend::{Backend, Config};
use crate::endpoint::{ConnId, DriverStats, Endpoint, Handler, Receive};
use crate::error::{Error, Result};
use crate::sys::{self, MsgSlot};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Spin,
    Busy,
    Recv,
}

const MAX_EVENTS: usize = 64;

pub(super) struct Epoll {
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
    /// Per slot: read every poll (one per NIC queue, plus unknown queues).
    polled: Vec<bool>,
    /// The slots in `polled`, in order.
    reps: Vec<usize>,
    /// Sockets came or went, or a queue may have become known.
    dirty: bool,
    /// A socket was added or removed: the cached NAPI ids are stale. A slot
    /// that is reused for a connection on a different queue kept the previous
    /// connection's id, so the new socket was never chosen as a polling
    /// representative and lost busy polling until the end of the run.
    rebind: bool,
    polls: u64,
    ep: Option<OwnedFd>,
    slot: Box<MsgSlot>,
    events: Box<[libc::epoll_event; MAX_EVENTS]>,
    wait: libc::timespec,
}

impl Epoll {
    pub(super) fn new(cfg: &Config) -> Result<Self> {
        let mode = match cfg.backend {
            Backend::EpollBusy => Mode::Busy,
            Backend::RecvBusy => Mode::Recv,
            _ => Mode::Spin,
        };
        let per_queue = mode == Mode::Recv && cfg.per_queue;
        let ep = if mode == Mode::Recv && !per_queue {
            None
        } else {
            // SAFETY: plain syscall.
            let fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            // SAFETY: `fd` is a fresh descriptor we own.
            Some(unsafe { OwnedFd::from_raw_fd(fd) })
        };
        if let (Mode::Busy, Some(ep)) = (mode, &ep) {
            set_epoll_busy_poll(ep.as_raw_fd(), cfg)?;
        }
        Ok(Self {
            mode,
            per_queue,
            napi: Vec::new(),
            asked: Vec::new(),
            seen: Vec::with_capacity(16),
            picks: 0,
            polled: Vec::new(),
            reps: Vec::new(),
            dirty: true,
            rebind: true,
            polls: 0,
            ep,
            slot: MsgSlot::new(),
            events: Box::new([libc::epoll_event { events: 0, u64: 0 }; MAX_EVENTS]),
            wait: libc::timespec {
                tv_sec: libc::time_t::from(cfg.wait_us / 1_000_000),
                tv_nsec: libc::c_long::from(cfg.wait_us % 1_000_000) * 1000,
            },
        })
    }

    pub(super) fn add(&mut self, fd: RawFd, id: ConnId) -> Result<()> {
        self.dirty = true;
        // The slot may be a reused one whose cached NAPI id belongs to the
        // connection that left it.
        self.rebind = true;
        if let Some(n) = self.napi.get_mut(id.index()) {
            *n = 0;
        }
        if let Some(a) = self.asked.get_mut(id.index()) {
            *a = false;
        }
        let Some(ep) = &self.ep else {
            return Ok(());
        };
        let mut ev = libc::epoll_event {
            events: libc::EPOLLIN.cast_unsigned(),
            u64: id.index() as u64,
        };
        // SAFETY: valid epoll fd, socket fd and event.
        if unsafe { libc::epoll_ctl(ep.as_raw_fd(), libc::EPOLL_CTL_ADD, fd, &raw mut ev) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub(super) fn remove(&mut self, fd: RawFd) {
        self.dirty = true;
        // Whoever takes this slot next must learn its own queue.
        self.rebind = true;
        if let Some(ep) = &self.ep {
            // SAFETY: valid fds; a null event is allowed for DEL.
            unsafe {
                libc::epoll_ctl(
                    ep.as_raw_fd(),
                    libc::EPOLL_CTL_DEL,
                    fd,
                    std::ptr::null_mut(),
                )
            };
        }
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
        if self.mode == Mode::Recv && !self.per_queue {
            for i in 0..conns.len_slots() {
                self.recv_index(conns, i, handler, stats, read_seq);
            }
        } else if self.per_queue {
            self.refresh_reps(conns);
            for k in 0..self.reps.len() {
                let i = self.reps[k];
                self.recv_index(conns, i, handler, stats, read_seq);
            }
            let n = self.wait(stats)?;
            for k in 0..n {
                let i = usize::try_from(self.events[k].u64).unwrap_or(usize::MAX);
                // Read already this poll (anything left is reported again).
                if self.polled.get(i).copied().unwrap_or(false) {
                    continue;
                }
                self.recv_index(conns, i, handler, stats, read_seq);
            }
        } else {
            let n = self.wait(stats)?;
            for k in 0..n {
                let i = usize::try_from(self.events[k].u64).unwrap_or(usize::MAX);
                self.recv_index(conns, i, handler, stats, read_seq);
            }
        }
        Ok(())
    }

    /// Rebuilds the representatives when due: a socket came or went, an
    /// unknown socket's first data arrived (its queue is learnt from it), or
    /// the periodic look-again (every 4096 polls). Otherwise one branch.
    #[inline]
    fn refresh_reps<E: Endpoint>(&mut self, conns: &mut Conns<E>) {
        self.polls += 1;
        let periodic = self.polls.is_multiple_of(4096);
        if self.dirty || periodic {
            // A socket coming or going, and the periodic look-again, both
            // refresh every id: a NIC can move a queue under a live socket
            // (RSS remap), and a reused slot can hold the previous
            // connection's queue. A rebuild only to fill ids that are still
            // unknown reads nothing new.
            let refresh_all = self.rebind || periodic;
            self.pick_reps(conns, refresh_all);
        }
    }

    /// Data arrived on slot `i`: when its queue is unknown, look it up once
    /// (on the next poll), not every poll.
    #[inline]
    fn data_arrived(&mut self, i: usize) {
        if self.per_queue && self.napi.get(i) == Some(&0) && self.asked.get(i) == Some(&false) {
            self.asked[i] = true;
            self.dirty = true;
        }
    }

    #[inline]
    fn recv_index<E, H>(
        &mut self,
        conns: &mut Conns<E>,
        i: usize,
        handler: &mut H,
        stats: &mut DriverStats,
        read_seq: &mut u64,
    ) where
        E: Receive<H>,
        H: Handler,
    {
        let id = conns.id(i);
        let Some(conn) = conns.at(i) else {
            return;
        };
        match recv_one(conn, id, &mut self.slot, handler, stats, read_seq) {
            Ok(false) => {}
            // A reply (a Pong) may be queued: list it for the send pass.
            Ok(true) => {
                if !conn.tx_queue().is_empty() {
                    conns.mark_tx(i);
                }
                self.data_arrived(i);
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

    /// One slot per known NIC queue, plus every slot whose queue is unknown.
    ///
    /// `refresh_all` drops every cached queue id first, so each live socket is
    /// asked again which queue it is on. Without it only ids that are still
    /// unknown are read, which never corrects an id that has gone stale.
    fn pick_reps<E: Endpoint>(&mut self, conns: &mut Conns<E>, refresh_all: bool) {
        self.picks += 1;
        let n = conns.len_slots();
        self.napi.resize(n, 0);
        self.asked.resize(n, false);
        if refresh_all {
            self.rebind = false;
            self.napi.fill(0);
        }
        self.polled.clear();
        self.polled.resize(n, false);
        self.reps.clear();
        self.seen.clear();
        for i in 0..n {
            let Some(c) = conns.at(i) else {
                self.napi[i] = 0;
                self.asked[i] = false;
                continue;
            };
            if self.napi[i] == 0 {
                self.napi[i] = sys::incoming_napi_id(c.fd()).unwrap_or(0);
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
        // Unknown ids wait for their socket's first data or the periodic
        // look-again, not the next poll.
        self.dirty = false;
    }

    fn wait(&mut self, stats: &mut DriverStats) -> Result<usize> {
        let Some(ep) = &self.ep else {
            return Ok(0);
        };
        stats.epoll_wait_calls += 1;
        #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
        let max = MAX_EVENTS as libc::c_int;
        let n = if self.mode == Mode::Busy {
            // SAFETY: valid fd, event buffer of MAX_EVENTS, timeout pointer, no sigmask.
            unsafe {
                libc::syscall(
                    libc::SYS_epoll_pwait2,
                    ep.as_raw_fd(),
                    self.events.as_mut_ptr(),
                    max,
                    &raw const self.wait,
                    std::ptr::null::<libc::sigset_t>(),
                    0usize,
                )
            }
        } else {
            // SAFETY: `events` has room for MAX_EVENTS entries.
            libc::c_long::from(unsafe {
                libc::epoll_wait(ep.as_raw_fd(), self.events.as_mut_ptr(), max, 0)
            })
        };
        if let Ok(n) = usize::try_from(n) {
            return Ok(n);
        }
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::EINTR) {
            Ok(0)
        } else {
            Err(Error::Io(e))
        }
    }
}

/// `struct epoll_params` (Linux 6.9+).
#[repr(C)]
struct EpollParams {
    busy_poll_usecs: u32,
    busy_poll_budget: u16,
    prefer_busy_poll: u8,
    pad: u8,
}

/// `EPIOCSPARAMS = _IOW(0x8A, 0x01, struct epoll_params)`.
const EPIOCSPARAMS: libc::c_ulong = (1 << 30) | (8 << 16) | (0x8A << 8) | 0x01;

fn set_epoll_busy_poll(ep: RawFd, cfg: &Config) -> Result<()> {
    let p = EpollParams {
        busy_poll_usecs: cfg.busy_poll_us,
        busy_poll_budget: cfg.busy_poll_budget.unwrap_or(8),
        prefer_busy_poll: u8::from(cfg.prefer_busy_poll),
        pad: 0,
    };
    // SAFETY: `p` is a valid epoll_params for the duration of the call.
    if unsafe { libc::ioctl(ep, EPIOCSPARAMS as _, &raw const p) } != 0 {
        let e = std::io::Error::last_os_error();
        return Err(match e.raw_os_error() {
            Some(libc::ENOTTY) => {
                Error::Unsupported("EPIOCSPARAMS (epoll busy poll) needs Linux 6.9 or newer")
            }
            Some(libc::EOPNOTSUPP) => Error::Unsupported(
                "the kernel was built without busy polling (CONFIG_NET_RX_BUSY_POLL)",
            ),
            Some(libc::EPERM) => {
                Error::Config("busy_poll_budget above 64 needs CAP_NET_ADMIN".to_owned())
            }
            _ => Error::Io(e),
        });
    }
    Ok(())
}

#[cfg(test)]
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
        ep.rebind = false;
        ep.pick_reps(&mut conns, false);
        assert_eq!(ep.reps, [0], "{:?}", ep.reps);

        // Slot 1 is reused for a connection the NIC has put on another queue.
        ep.remove(conns.at(1).ok_or("slot 1")?.fd());
        conns.remove(1);
        let i = install(&mut ep, &mut conns)?;
        assert_eq!(i, 1, "the slot is reused");

        assert_eq!(ep.napi[1], 0, "the reused slot kept the old queue id");
        assert!(ep.rebind, "the cache is not marked stale");
        ep.pick_reps(&mut conns, ep.rebind);
        assert!(
            ep.reps.contains(&1),
            "the replacement is never a polling representative: {:?}",
            ep.reps
        );
        Ok(())
    }

    /// Sockets with no NAPI id (loopback, a plain veth) never learn one:
    /// the representatives are not rebuilt every poll (one getsockopt per
    /// socket and an allocation each time), only on the periodic
    /// look-again, and the rebuild reuses its scratch space.
    #[test]
    fn unknown_queues_are_looked_up_again_only_now_and_then() -> R {
        let (mut ep, mut conns) = per_queue()?;
        install(&mut ep, &mut conns)?;
        install(&mut ep, &mut conns)?;
        for _ in 0..10_000 {
            ep.refresh_reps(&mut conns);
        }
        // The first poll after the adds, then the look-again at 4096 and 8192.
        assert_eq!(ep.picks, 3, "picks over 10,000 polls");
        assert_eq!(ep.reps, [0, 1], "unknown queues are all still read");
        let cap = ep.seen.capacity();
        for _ in 0..10_000 {
            ep.refresh_reps(&mut conns);
        }
        assert_eq!(ep.seen.capacity(), cap, "the scratch space grew");
        Ok(())
    }

    /// A socket's first data asks for its queue once: the next poll looks it
    /// up, and later data on a socket that still has no id (loopback, a
    /// plain veth) asks nothing until the periodic look-again.
    #[test]
    fn first_data_looks_a_queue_up_once() -> R {
        let (mut ep, mut conns) = per_queue()?;
        install(&mut ep, &mut conns)?;
        install(&mut ep, &mut conns)?;
        ep.refresh_reps(&mut conns);
        assert_eq!(ep.picks, 1, "the adds");
        for _ in 0..100 {
            ep.data_arrived(0);
            ep.refresh_reps(&mut conns);
        }
        assert_eq!(ep.picks, 2, "one look-up after the first data");
        ep.data_arrived(1);
        ep.refresh_reps(&mut conns);
        assert_eq!(ep.picks, 3, "the other socket's first data");
        Ok(())
    }

    /// A periodic look-again has to correct an id that has gone stale (the NIC
    /// moved a queue under a live socket), not only ids that are still unknown.
    #[test]
    fn a_refresh_forgets_every_cached_queue_id() -> R {
        let (mut ep, mut conns) = per_queue()?;
        install(&mut ep, &mut conns)?;
        install(&mut ep, &mut conns)?;
        ep.napi.resize(conns.len_slots(), 0);
        ep.napi.fill(9);
        ep.pick_reps(&mut conns, true);
        assert!(
            ep.napi.iter().all(|&id| id == 0),
            "a stale id survived the refresh: {:?}",
            ep.napi
        );
        Ok(())
    }
}
