//! The transport latency suite's arms, one module each, public so other
//! benchmark crates reuse them. Every arm sends `ergon_latency_core`'s
//! 64-byte message; the socket arms run epoll-spin only.
//!
//! [`point`] is the load point every arm runs (flags, pacing, recording,
//! echo, files): an arm adds only its transport, as a `point::Tx` and a
//! `point::Rx`. md-dpdk/harness's `aeron_lat` takes it by path.
//!
//! | module | transport |
//! |---|---|
//! | [`mpsc`] | `std::sync::mpsc::sync_channel` (IPC baseline) |
//! | [`mio_udp`], [`eio_udp`] | one connected UDP socket per stream (stream `i` on base port + `i`), raw mio and ergon-io |
//! | [`mio_tcp`], [`eio_tcp`] | one TCP connection per stream, `ergon_latency_core::frame`'s u32 LE frames, raw mio and ergon-io |
//! | [`sock`] | `udp_lat` and `tcp_lat`: their flags, connection set-up, threads and roles |
//! | [`sockopt`] | the socket options the mio arms set and every socket arm reads back |
//! | [`kstat`] | the namespace's TCP counters and a thread's run-queue wait, read around a point's traffic |

#[cfg(target_os = "linux")]
pub mod eio_tcp;
#[cfg(target_os = "linux")]
pub mod eio_udp;
#[cfg(target_os = "linux")]
pub mod kstat;
#[cfg(target_os = "linux")]
pub mod mio_tcp;
#[cfg(target_os = "linux")]
pub mod mio_udp;
pub mod mpsc;
pub mod point;
#[cfg(target_os = "linux")]
pub mod sock;
#[cfg(target_os = "linux")]
pub mod sockopt;

/// The longest UDP datagram either UDP arm takes, bytes: ergon-io's
/// `UdpOptions::max_datagram` default, and the mio arm's receive buffer
/// and send slot.
pub const MAX_DATAGRAM: usize = 2_048;
/// UDP send slots per socket, both arms: ergon-io's `UdpOptions::tx_slots`
/// default.
pub const TX_SLOTS: usize = 64;
/// The longest TCP frame payload either TCP arm takes: ergon-io's
/// `FrameOptions::max_frame` default.
pub const MAX_FRAME: usize = 64 << 10;
/// TCP receive buffer per connection, both arms: what
/// `ergon_latency_core::frame::Deframer` holds for [`MAX_FRAME`] (two whole
/// frames), so each read asks the kernel for the same room.
pub const TCP_RX_BUF: usize = 2 * (MAX_FRAME + ergon_latency_core::frame::FRAME_HEADER);
/// TCP send queue per connection, both arms: ergon-io's
/// `FrameOptions::tx_buf` default.
pub const TCP_TX_BUF: usize = 256 << 10;
/// `SO_RCVBUF` both socket arms get unless `--rcvbuf` says otherwise:
/// ergon-io's `Config::rcvbuf` default, set on the mio sockets too.
pub const DEFAULT_RCVBUF: u32 = 4 << 20;

/// Ethernet (14) and IPv4 (20) or IPv6 (40) header bytes in front of a
/// transport header.
#[must_use]
pub const fn ip_overhead(ipv6: bool) -> u64 {
    14 + if ipv6 { 40 } else { 20 }
}

/// The media driver's wildcard port ranges in the lab (sender 20000-20999,
/// receiver 21000-21999: vm.sh `common_env`): no socket arm binds or sends
/// to one.
pub const DPDK_PORTS: std::ops::RangeInclusive<u16> = 20_000..=21_999;

/// `base` with its port moved up by `i`.
///
/// # Errors
/// [`point::Error::Usage`] for port 0, a port past 65 535, or one in
/// [`DPDK_PORTS`].
pub fn port_plus(base: std::net::SocketAddr, i: u32) -> Result<std::net::SocketAddr, point::Error> {
    if base.port() == 0 {
        return Err(point::Error::Usage(format!(
            "{base}: a base port is needed (stream i uses port + i)"
        )));
    }
    let port = u16::try_from(u32::from(base.port()) + i)
        .map_err(|_| point::Error::Usage(format!("{base} + {i}: past port 65535")))?;
    if DPDK_PORTS.contains(&port) {
        return Err(point::Error::Usage(format!(
            "port {port} (stream {i} of base {base}): 20000-21999 are the media driver's wildcard ranges"
        )));
    }
    Ok(std::net::SocketAddr::new(base.ip(), port))
}

/// Echoes the ergon-io framed-TCP arm (`eio_tcp`) can be handed inside
/// one `Driver::poll`, at most: room the arm stages them in, as an echo
/// cannot be sent while the driver polls. A poll reads each connection
/// epoll reports once (at most `epoll_events` of the `streams`) and, with
/// `again_pass` (`SO_BUSY_POLL` set: ergon-io reads a connection that
/// returned data once more at the start of the next poll), each such
/// connection once more. A read takes at most [`TCP_RX_BUF`] bytes; the
/// whole frames in the buffer after it are at most [`TCP_RX_BUF`] / 68, one
/// more for slack. A smaller queue let two full reads in one poll overflow
/// it under a backlog, and the echo then spun inside the poll for good.
#[must_use]
pub const fn tcp_echo_capacity(streams: usize, epoll_events: usize, again_pass: bool) -> usize {
    let per_read = TCP_RX_BUF
        / (ergon_latency_core::frame::FRAME_HEADER + ergon_latency_core::message::MESSAGE_LEN)
        + 1;
    let reported = if epoll_events < streams {
        epoll_events
    } else {
        streams
    };
    let again = if again_pass { streams } else { 0 };
    (reported + again) * per_read
}
