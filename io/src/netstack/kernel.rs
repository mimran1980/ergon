//! [`Kernel`]: the seam over the Linux kernel's sockets. Each method is the
//! call the driver made before the seam existed.

use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, TcpStream, UdpSocket};
use std::os::fd::{AsRawFd, RawFd};

use socket2::{Domain, Protocol, SockRef, Socket, Type};

use super::{Kernel, Sockets, StackId};
use crate::error::{Error, Result};

impl super::Socket for TcpStream {
    const STACK: StackId = StackId::Kernel;

    #[inline]
    fn fd(&self) -> RawFd {
        self.as_raw_fd()
    }

    fn start_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<Self> {
        Kernel::tcp_connect(addr, source)
    }

    fn connect_finished(&self) -> Result<bool> {
        Kernel::connect_finished(self)
    }

    fn configure(&self, sndbuf: Option<u32>) -> Result<()> {
        self.set_nonblocking(true)?;
        self.set_nodelay(true)?;
        if let Some(b) = sndbuf {
            SockRef::from(self)
                .set_send_buffer_size(usize::try_from(b).map_err(|_| {
                    Error::Config(format!("sndbuf {b} does not fit this platform"))
                })?)?;
        }
        Ok(())
    }
}

impl super::Datagram for UdpSocket {
    const STACK: StackId = StackId::Kernel;

    #[inline]
    fn fd(&self) -> RawFd {
        self.as_raw_fd()
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(Self::local_addr(self)?)
    }

    fn peer_addr(&self) -> Option<SocketAddr> {
        Self::peer_addr(self).ok()
    }

    fn recv_buffer_size(&self) -> Result<usize> {
        Ok(SockRef::from(self).recv_buffer_size()?)
    }
}

impl Sockets for Kernel {
    type Tcp = TcpStream;
    type Udp = UdpSocket;

    fn tcp_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<TcpStream> {
        if let Some(ip) = source
            && ip.is_ipv4() != addr.is_ipv4()
        {
            return Err(Error::Config(format!(
                "source {ip} is not of the destination {addr}'s address family"
            )));
        }
        let s = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
        s.set_nonblocking(true)?;
        s.set_tcp_nodelay(true)?;
        if let Some(ip) = source {
            // The port is picked at connect, per destination, not at bind.
            #[cfg(target_os = "linux")]
            crate::sys::set_bind_address_no_port(std::os::fd::AsRawFd::as_raw_fd(&s))?;
            s.bind(&SocketAddr::new(ip, 0).into())?;
        }
        match s.connect(&addr.into()) {
            Ok(()) => {}
            Err(e)
                if e.raw_os_error() == Some(libc::EINPROGRESS)
                    || e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => return Err(e.into()),
        }
        Ok(s.into())
    }

    fn connect_finished(sock: &TcpStream) -> Result<bool> {
        if let Some(e) = sock.take_error()? {
            return Err(e.into());
        }
        match sock.peer_addr() {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotConnected => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    fn udp_bind(local: SocketAddr, sndbuf: Option<u32>) -> Result<UdpSocket> {
        let s = Socket::new(Domain::for_address(local), Type::DGRAM, Some(Protocol::UDP))?;
        s.set_nonblocking(true)?;
        if let Some(b) = sndbuf {
            s.set_send_buffer_size(usize::try_from(b).map_err(|_| {
                Error::Config(format!("udp sndbuf {b} does not fit this platform"))
            })?)?;
        }
        s.bind(&local.into())?;
        Ok(s.into())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::io;
    use std::os::fd::OwnedFd;

    use crate::netstack::{Kernel, NetStack};
    use crate::sys::{self, MsgSlot};

    impl NetStack for Kernel {
        type Poller = OwnedFd;
        type Event = libc::epoll_event;
        type RxSlot = MsgSlot;

        #[inline]
        fn poller() -> io::Result<OwnedFd> {
            sys::epoll_create()
        }

        #[inline]
        fn empty_event() -> libc::epoll_event {
            libc::epoll_event { events: 0, u64: 0 }
        }

        #[inline]
        fn token(ev: &libc::epoll_event) -> u64 {
            ev.u64
        }

        #[inline]
        fn register(poller: &OwnedFd, fd: libc::c_int, token: u64) -> io::Result<()> {
            sys::epoll_add(poller, fd, token)
        }

        #[inline]
        fn deregister(poller: &OwnedFd, fd: libc::c_int) {
            sys::epoll_del(poller, fd);
        }

        #[inline]
        fn poll_now(poller: &OwnedFd, events: &mut [libc::epoll_event]) -> io::Result<usize> {
            sys::epoll_wait_now(poller, events)
        }

        #[inline]
        fn rx_slot() -> Box<MsgSlot> {
            MsgSlot::new()
        }

        #[inline]
        fn arm(slot: &mut MsgSlot, buf: &mut [u8]) {
            slot.arm(buf);
        }

        #[inline]
        fn recv(fd: libc::c_int, slot: &mut MsgSlot) -> io::Result<Option<usize>> {
            sys::recvmsg(fd, slot)
        }

        #[inline]
        fn truncated(slot: &MsgSlot) -> bool {
            slot.truncated()
        }

        #[inline]
        fn rx_meta(slot: &MsgSlot, read_seq: u64) -> crate::RxMeta {
            slot.meta(read_seq)
        }

        #[inline]
        fn send(fd: libc::c_int, buf: &[u8]) -> io::Result<usize> {
            sys::send(fd, buf)
        }

        #[inline]
        fn send_datagram(fd: libc::c_int, buf: &[u8]) -> io::Result<bool> {
            sys::send_datagram(fd, buf)
        }

        const BATCH: bool = true;

        #[inline]
        fn recv_batch(
            fd: libc::c_int,
            b: &mut sys::MmsgBatch,
            n: usize,
        ) -> io::Result<Option<usize>> {
            sys::recvmmsg(fd, b, n)
        }

        #[inline]
        fn send_batch(fd: libc::c_int, b: &mut sys::MmsgBatch, n: usize) -> io::Result<usize> {
            sys::sendmmsg(fd, b, n)
        }

        #[inline]
        fn refused(e: &io::Error) -> bool {
            e.raw_os_error() == Some(libc::ECONNREFUSED)
        }

        fn enable_rx_timestamps(fd: libc::c_int) -> io::Result<()> {
            sys::enable_rx_timestamps(fd)
        }

        fn set_rcvbuf(fd: libc::c_int, bytes: u32) -> io::Result<()> {
            sys::set_rcvbuf(fd, bytes)
        }

        fn set_busy_poll(fd: libc::c_int, usec: u32) -> io::Result<()> {
            sys::set_busy_poll(fd, usec)
        }

        fn set_tcp_keepalive(
            fd: libc::c_int,
            idle_s: u32,
            interval_s: u32,
            count: u32,
        ) -> io::Result<()> {
            sys::set_tcp_keepalive(fd, idle_s, interval_s, count)
        }

        fn set_tcp_user_timeout(fd: libc::c_int, ms: u32) -> io::Result<()> {
            sys::set_tcp_user_timeout(fd, ms)
        }

        fn apply_tcp_timers(
            fd: libc::c_int,
            rto_min_us: Option<u32>,
            delack_max_us: Option<u32>,
        ) -> crate::Result<()> {
            sys::apply_tcp_timers(fd, rto_min_us, delack_max_us)
        }

        fn napi_id(fd: libc::c_int) -> io::Result<u32> {
            sys::incoming_napi_id(fd)
        }

        fn tcp_info(fd: libc::c_int) -> io::Result<crate::TcpInfo> {
            sys::tcp_info(fd)
        }
    }
}
