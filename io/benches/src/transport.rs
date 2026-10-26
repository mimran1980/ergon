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
//! | [`fstack`] | one TCP connection per stream over F-Stack (FreeBSD's stack on DPDK), `--features fstack`; [`fstack_conf`] its flags, config and `tcp_info` |
//! | [`sock`] | `udp_lat` and `tcp_lat`: their flags, connection set-up, threads and roles |
//! | [`sockopt`] | the socket options the mio arms set and every socket arm reads back |
//! | [`kstat`] | the namespace's TCP counters and a thread's run-queue wait, read around a point's traffic |

#[cfg(target_os = "linux")]
pub mod eio_tcp;
#[cfg(target_os = "linux")]
pub mod eio_udp;
#[cfg(all(target_os = "linux", feature = "fstack"))]
pub mod fstack;
#[cfg(target_os = "linux")]
pub mod fstack_conf;
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
