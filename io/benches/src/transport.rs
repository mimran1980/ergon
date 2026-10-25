//! The transport latency suite's socket arms, one module each, public so
//! other benchmark crates reuse them. Every arm sends
//! `ergon_latency_core`'s 64-byte message, epoll-spin only.
//!
//! | module | what it is |
//! |---|---|
//! | [`mio_udp`], [`eio_udp`] | one connected UDP socket per stream (stream `i` on base port + `i`) |
//! | [`mio_tcp`], [`eio_tcp`] | one TCP connection per stream, `ergon_latency_core::frame`'s u32 LE frames |
//! | [`point`] | the measurement both ends of every arm share: pacing, stamps, decode, echo, histograms, loss |
//! | [`bench`] | the `udp_lat` / `tcp_lat` command lines, threads and output files |
//! | [`sockopt`] | the socket options the mio arms set and every arm reads back |
//!
//! An arm is a [`Link`]: [`Link::send`] hands one message to the
//! transport now, [`Link::poll`] gives every message received to a
//! callback the moment the transport hands it over, [`Link::echo`] turns
//! every ping received into its echo on the same stream. Everything else
//! (when to send, the stamps, what is recorded) is [`point`]'s, the same
//! code for every arm.

#[cfg(target_os = "linux")]
pub mod bench;
#[cfg(target_os = "linux")]
pub mod eio_tcp;
#[cfg(target_os = "linux")]
pub mod eio_udp;
#[cfg(target_os = "linux")]
pub mod mio_tcp;
#[cfg(target_os = "linux")]
pub mod mio_udp;
#[cfg(target_os = "linux")]
pub mod point;
#[cfg(target_os = "linux")]
pub mod sockopt;

use std::fmt;

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
/// `SO_RCVBUF` both arms get unless `--rcvbuf` says otherwise: ergon-io's
/// `Config::rcvbuf` default, set on the mio sockets too.
pub const DEFAULT_RCVBUF: u32 = 4 << 20;

/// Ethernet (14), IPv4 (20) or IPv6 (40) header bytes in front of a
/// transport header.
#[must_use]
pub const fn ip_overhead(ipv6: bool) -> u64 {
    14 + if ipv6 { 40 } else { 20 }
}

/// What an arm refuses or reports.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A system call failed.
    Io(std::io::Error),
    /// The measurement core refused something (a malformed message, a bad
    /// schedule).
    Core(ergon_latency_core::Error),
    /// ergon-io failed.
    Eio(ergon_io::Error),
    /// A setting that cannot run, or a command-line error.
    Config(String),
    /// The peer closed a connection.
    Closed(String),
    /// A message that should not have arrived: why.
    Protocol(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Core(e) => write!(f, "core: {e}"),
            Self::Eio(e) => write!(f, "ergon-io: {e}"),
            Self::Config(e) => write!(f, "config: {e}"),
            Self::Closed(e) => write!(f, "closed: {e}"),
            Self::Protocol(e) => write!(f, "protocol: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ergon_latency_core::Error> for Error {
    fn from(e: ergon_latency_core::Error) -> Self {
        Self::Core(e)
    }
}

impl From<ergon_io::Error> for Error {
    fn from(e: ergon_io::Error) -> Self {
        match e {
            ergon_io::Error::Closed => Self::Closed("the peer closed the connection".into()),
            e => Self::Eio(e),
        }
    }
}

/// One arm's end of N streams.
pub trait Link {
    /// Streams it carries.
    fn streams(&self) -> u32;

    /// Hands `payload` for `stream` to the transport now. `Ok(false)`:
    /// back-pressure, nothing was taken; offer it again later.
    ///
    /// # Errors
    /// The transport failed, or `stream` is not one of its streams.
    fn send(&mut self, stream: u32, payload: &[u8]) -> Result<bool, Error>;

    /// One pass of the receive side: every message the transport has goes
    /// to `on_msg(stream, payload)` as it hands it over. Also retries sends
    /// the kernel had no room for. Never blocks. Returns messages delivered.
    ///
    /// # Errors
    /// The transport failed, a connection closed, or `on_msg` failed (the
    /// first error stops the pass).
    fn poll<F>(&mut self, on_msg: &mut F) -> Result<usize, Error>
    where
        F: FnMut(u32, &[u8]) -> Result<(), Error>;

    /// One pass of the echo side: every ping received is turned into its
    /// echo in place (`ergon_latency_core::message::echo_in_place`) and
    /// sent back on its own stream. Never blocks. Returns echoes sent.
    ///
    /// # Errors
    /// As [`poll`](Self::poll); [`Error::Closed`] when a peer closed.
    fn echo(&mut self) -> Result<usize, Error>;

    /// Segments retransmitted so far, summed over the streams
    /// (`TCP_INFO` `tcpi_total_retrans`); `None` for a transport without
    /// retransmits. One `getsockopt` per stream: never on the hot path.
    ///
    /// # Errors
    /// `getsockopt` fails.
    fn retrans(&self) -> Result<Option<u64>, Error>;

    /// Bytes on the wire per message: payload, framing, transport, IP and
    /// Ethernet headers.
    ///
    /// # Errors
    /// The headers cannot be read back (`TCP_INFO`).
    fn wire_bytes(&self) -> Result<u64, Error>;

    /// The settings as applied and read back from stream 0's socket, and
    /// the arm's counters, as `(name, value)`.
    ///
    /// # Errors
    /// A read-back fails.
    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error>;
}
