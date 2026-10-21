//! chronyd's command and monitoring protocol (cmdmon): the one request this
//! crate makes, `tracking`, over UDP to chronyd's command port on the
//! loopback. A monitoring command, so open to any local user without
//! authentication (chronyd's `PERMIT_OPEN`), unlike the Unix socket.
//!
//! The layout is chrony's `candm.h`, protocol version 6, checked against
//! chrony 4.6.1 (`candm.h`, `pktlength.c`, `cmdmon.c` `handle_tracking` and
//! `read_from_cmd_socket`, `util.c` `UTI_FloatNetworkToHost`), which is what
//! chronyc itself sends (`client.c` `process_cmd_tracking`):
//!
//! - every integer is big-endian;
//! - the request is a 20-byte header (version, packet type 1, two reserved
//!   zero bytes, command 33, attempt, sequence, two pad words) and no data,
//!   padded with zeros to the reply's length, 104 bytes: chronyd refuses a
//!   request shorter than its reply (`STT_BADPKTLENGTH`), against traffic
//!   amplification;
//! - the reply is a 28-byte header (version, packet type 2, two reserved
//!   bytes, command, reply 5, status 0, three pads, the request's sequence,
//!   two pad words), then `RPY_Tracking`: reference ID, the reference's
//!   address (16 bytes, family, pad), stratum, leap status, reference time
//!   (12 bytes), then nine of chrony's `Float`s: current correction, last
//!   offset, RMS offset, frequency, residual frequency, skew, root delay,
//!   root dispersion, update interval;
//! - a `Float` is a 7-bit signed exponent over a 25-bit signed coefficient,
//!   `coef * 2^(exp - 25)`.
//!
//! The reply is parsed defensively: a short, foreign, mismatched or
//! out-of-range packet is an error that says what was wrong, never a guess.
//! Parsing allocates nothing: the report and every error are plain values.

use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// chronyd's command port (`DEFAULT_CANDM_PORT`).
pub const PORT: u16 = 323;
/// The protocol version spoken (`PROTO_VERSION_NUMBER`).
const PROTO_VERSION: u8 = 6;
/// `PKT_TYPE_CMD_REQUEST`.
const PKT_REQUEST: u8 = 1;
/// `PKT_TYPE_CMD_REPLY`.
const PKT_REPLY: u8 = 2;
/// `REQ_TRACKING`.
const REQ_TRACKING: u16 = 33;
/// `RPY_TRACKING`.
const RPY_TRACKING: u16 = 5;
/// `STT_SUCCESS`.
const STT_SUCCESS: u16 = 0;
/// The request: its 20-byte header, no data, zeros up to the reply's
/// length (`pktlength.c`: `REQ_LENGTH_ENTRY(null, tracking)`).
const REQUEST_LEN: usize = 104;
/// The reply's header (`offsetof(CMD_Reply, data)`).
const REPLY_HEADER_LEN: usize = 28;
/// The tracking reply (`offsetof(CMD_Reply, data.tracking.EOR)`).
const TRACKING_LEN: usize = 104;
/// The reference ID of chronyd's own clock (`local stratum`):
/// `NTP_REFID_LOCAL`, 127.127.1.1.
const REFID_LOCAL: u32 = 0x7F7F_0101;
/// `LEAP_Unsynchronised`.
const LEAP_UNSYNCHRONISED: u16 = 3;
/// `IPADDR_UNSPEC`, `IPADDR_INET4`, `IPADDR_INET6`, `IPADDR_ID`.
const FAMILY_UNSPEC: u16 = 0;
const FAMILY_INET4: u16 = 1;
const FAMILY_INET6: u16 = 2;
const FAMILY_ID: u16 = 3;
/// The fault of a reply that answers another request.
const ANOTHER_REQUEST: &str = "a reply to another request";

/// A reference clock's ID as chrony prints it (`UTI_RefidToString`): the
/// printable bytes of its four, in order. Held inline: no allocation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefclockId {
    bytes: [u8; 4],
    len: u8,
}

impl RefclockId {
    /// From the reference ID chronyd reports (`PHC0` is
    /// `u32::from_be_bytes(*b"PHC0")`).
    #[must_use]
    pub const fn new(ref_id: u32) -> Self {
        let raw = ref_id.to_be_bytes();
        let mut bytes = [0_u8; 4];
        let mut len = 0_u8;
        let mut i = 0;
        while i < raw.len() {
            let b = raw[i];
            if b.is_ascii_graphic() || b == b' ' {
                bytes[len as usize] = b;
                len += 1;
            }
            i += 1;
        }
        Self { bytes, len }
    }

    /// The ID as text (`PHC0`, `GPS`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        let printable = self.bytes.get(..usize::from(self.len)).unwrap_or_default();
        std::str::from_utf8(printable).unwrap_or_default()
    }
}

impl fmt::Debug for RefclockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RefclockId({:?})", self.as_str())
    }
}

impl fmt::Display for RefclockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What chronyd's clock is synchronised to, from its tracking report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChronyReference {
    /// Nothing: reference ID 0, leap status unsynchronised.
    None,
    /// Its own clock (`local stratum N`): a server of last resort, not
    /// synchronised to anything.
    Local,
    /// An NTP server, by address (`None`: an address chronyd has not
    /// resolved, sent as an ID).
    Server(Option<IpAddr>),
    /// A reference clock, by its reference ID (`PHC0` for the first
    /// `refclock PHC` without a `refid` option: chrony's `refclock.c`
    /// derives it from the driver's name and the clock's index).
    Refclock(RefclockId),
}

/// chronyd's tracking report (`chronyc tracking`), as it answered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChronyTracking {
    /// What the clock is synchronised to.
    pub reference: ChronyReference,
    /// The reference ID.
    pub ref_id: u32,
    /// The stratum (0 when unsynchronised).
    pub stratum: u16,
    /// 0 normal, 1 insert a leap second, 2 delete one, 3 unsynchronised.
    pub leap_status: u16,
    /// How far the system clock is from chronyd's estimate of true time,
    /// s (`System time`): what it is still slewing out. Positive: the clock
    /// is behind (chronyc prints it as slow of NTP time).
    pub correction_s: f64,
    /// The offset at the last update, s.
    pub last_offset_s: f64,
    /// The long-term average offset, s.
    pub rms_offset_s: f64,
    /// The rate correction applied, ppm. Positive: chronyc prints it as
    /// fast.
    pub freq_ppm: f64,
    /// The residual frequency of the reference, ppm.
    pub resid_freq_ppm: f64,
    /// The error bound on the frequency, ppm.
    pub skew_ppm: f64,
    /// Network delay to the stratum-1 source, s.
    pub root_delay_s: f64,
    /// Dispersion accumulated back to the stratum-1 source, s.
    pub root_dispersion_s: f64,
    /// Between the last two updates, s.
    pub update_interval_s: f64,
}

impl ChronyTracking {
    /// Whether chronyd's clock is synchronised to a source: an NTP server,
    /// or a reference clock with a known leap status. Its own clock
    /// (`local`) is not one.
    #[must_use]
    pub const fn synchronised(&self) -> bool {
        match self.reference {
            ChronyReference::Server(_) => true,
            ChronyReference::Refclock(_) => self.leap_status != LEAP_UNSYNCHRONISED,
            ChronyReference::None | ChronyReference::Local => false,
        }
    }

    /// Whether the reference is a PTP hardware clock: a reference clock whose
    /// ID is chrony's default for `refclock PHC` (`PHC0`, `PHC1`, ...).
    /// A PHC given another `refid` is not recognised as one.
    #[must_use]
    pub fn is_phc(&self) -> bool {
        matches!(&self.reference, ChronyReference::Refclock(id) if id.as_str().starts_with("PHC"))
    }

    /// Whether chronyd announces a leap second (leap status 1, insert, or
    /// 2, delete).
    #[must_use]
    pub const fn leap_pending(&self) -> bool {
        matches!(self.leap_status, 1 | 2)
    }

    /// chronyc's absolute bound on the clock's error, ns:
    /// `|system time offset| + root dispersion + root delay / 2`
    /// (chrony 4.6.1 `doc/chronyc.adoc`, `tracking`), "assuming the stratum-1
    /// computer is correct". chronyd's dispersion grows by its
    /// `maxclockerror` (1 ppm by default) a second since the last update, so
    /// it is chronyd's estimate, optimistic beside `ClockBound`'s.
    #[must_use]
    pub fn bound_ns(&self) -> u64 {
        let s = self.correction_s.abs() + self.root_dispersion_s + self.root_delay_s / 2.0;
        seconds_to_ns(s)
    }

    /// The reference, as `chronyc -n tracking` names it: the server's
    /// address, the reference clock's ID, `local`, or `-`. Allocates.
    #[must_use]
    pub fn reference_name(&self) -> String {
        match &self.reference {
            ChronyReference::None => "-".to_owned(),
            ChronyReference::Local => "local".to_owned(),
            ChronyReference::Server(Some(ip)) => ip.to_string(),
            ChronyReference::Server(None) => format!("id:{:08X}", self.ref_id),
            ChronyReference::Refclock(id) => id.as_str().to_owned(),
        }
    }
}

/// Seconds, a non-negative finite float, as whole ns (rounded up), or
/// `u64::MAX` beyond it.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // clamped to [0, 2^64) first
fn seconds_to_ns(s: f64) -> u64 {
    let ns = (s * 1e9).ceil();
    if ns.is_nan() || ns <= 0.0 {
        0
    } else if ns >= u64::MAX as f64 {
        u64::MAX
    } else {
        ns as u64
    }
}

/// Why chronyd gave no tracking report. A plain value: making one
/// allocates nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChronyError {
    /// A socket call failed: chronyd not listening (the port refused it),
    /// or a socket error.
    Io {
        /// The call (`bind`, `connect`, `send`, `recv`).
        op: &'static str,
        /// What went wrong.
        kind: io::ErrorKind,
    },
    /// No reply within this long (the start-up query).
    Timeout(Duration),
    /// No reply to the last request since the check before
    /// ([`ClockWatch`](crate::ClockWatch)).
    NoReply,
    /// chronyd answered with this status (`STT_*`, not `STT_SUCCESS`).
    Status(u16),
    /// The answer was not a tracking reply to this request: what was wrong.
    Malformed(&'static str),
}

impl fmt::Display for ChronyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { op, kind } => write!(f, "{op}: {kind}"),
            Self::Timeout(t) => write!(f, "no reply within {} ms", t.as_millis()),
            Self::NoReply => f.write_str("no reply since the last check"),
            Self::Status(code) => write!(f, "chronyd answered status {code}"),
            Self::Malformed(why) => write!(f, "malformed reply: {why}"),
        }
    }
}

impl std::error::Error for ChronyError {}

/// The tracking request with this sequence number.
const fn request(sequence: u32) -> [u8; REQUEST_LEN] {
    let mut packet = [0_u8; REQUEST_LEN];
    packet[0] = PROTO_VERSION;
    packet[1] = PKT_REQUEST;
    // res1, res2: 0; command; attempt: 0
    let command = REQ_TRACKING.to_be_bytes();
    packet[4] = command[0];
    packet[5] = command[1];
    let seq = sequence.to_be_bytes();
    packet[8] = seq[0];
    packet[9] = seq[1];
    packet[10] = seq[2];
    packet[11] = seq[3];
    packet
}

/// A big-endian `u16` at `at` (the caller checked the length).
fn be16(bytes: &[u8], at: usize) -> u16 {
    let mut b = [0_u8; 2];
    if let Some(s) = bytes.get(at..at + 2) {
        b.copy_from_slice(s);
    }
    u16::from_be_bytes(b)
}

/// A big-endian `u32` at `at` (the caller checked the length).
fn be32(bytes: &[u8], at: usize) -> u32 {
    let mut b = [0_u8; 4];
    if let Some(s) = bytes.get(at..at + 4) {
        b.copy_from_slice(s);
    }
    u32::from_be_bytes(b)
}

/// chrony's `Float` (`UTI_FloatNetworkToHost`): the top 7 bits a signed
/// exponent, the low 25 a signed coefficient, `coef * 2^(exp - 25)`.
fn float(raw: u32) -> f64 {
    const COEF_BITS: u32 = 25;
    let mut exp = i32::try_from(raw >> COEF_BITS).unwrap_or(0);
    if exp >= 1 << 6 {
        exp -= 1 << 7;
    }
    exp -= 25;
    let mut coef = i32::try_from(raw & ((1 << COEF_BITS) - 1)).unwrap_or(0);
    if coef >= 1 << (COEF_BITS - 1) {
        coef -= 1 << COEF_BITS;
    }
    f64::from(coef) * 2_f64.powi(exp)
}

/// The tracking report in `reply`, an answer to the request with
/// `sequence`. Allocates nothing.
///
/// # Errors
/// What was wrong with it: too short, another protocol version or packet
/// type, another command or sequence, a status other than success, another
/// reply type, an address family, stratum or leap status out of range, or a
/// negative root delay or dispersion.
fn parse(reply: &[u8], sequence: u32) -> Result<ChronyTracking, ChronyError> {
    if reply.len() < REPLY_HEADER_LEN {
        return Err(ChronyError::Malformed("shorter than a reply header"));
    }
    if reply[0] != PROTO_VERSION {
        return Err(ChronyError::Malformed("not protocol version 6"));
    }
    if reply[1] != PKT_REPLY || reply[2] != 0 || reply[3] != 0 {
        return Err(ChronyError::Malformed("not a command reply"));
    }
    if be16(reply, 4) != REQ_TRACKING {
        return Err(ChronyError::Malformed("a reply to another command"));
    }
    if be32(reply, 16) != sequence {
        return Err(ChronyError::Malformed(ANOTHER_REQUEST));
    }
    let status = be16(reply, 8);
    if status != STT_SUCCESS {
        return Err(ChronyError::Status(status));
    }
    if be16(reply, 6) != RPY_TRACKING {
        return Err(ChronyError::Malformed("not a tracking reply"));
    }
    if reply.len() < TRACKING_LEN {
        return Err(ChronyError::Malformed("shorter than a tracking reply"));
    }
    let d = REPLY_HEADER_LEN;
    let ref_id = be32(reply, d);
    let family = be16(reply, d + 20);
    let stratum = be16(reply, d + 24);
    let leap_status = be16(reply, d + 26);
    if stratum > 16 {
        return Err(ChronyError::Malformed("stratum over 16"));
    }
    if leap_status > LEAP_UNSYNCHRONISED {
        return Err(ChronyError::Malformed("leap status over 3"));
    }
    let reference = match family {
        FAMILY_INET4 => {
            ChronyReference::Server(Some(IpAddr::V4(Ipv4Addr::from(be32(reply, d + 4)))))
        }
        FAMILY_INET6 => {
            let mut octets = [0_u8; 16];
            octets.copy_from_slice(&reply[d + 4..d + 20]);
            ChronyReference::Server(Some(IpAddr::V6(Ipv6Addr::from(octets))))
        }
        FAMILY_ID => ChronyReference::Server(None),
        FAMILY_UNSPEC => match ref_id {
            0 => ChronyReference::None,
            REFID_LOCAL => ChronyReference::Local,
            _ => ChronyReference::Refclock(RefclockId::new(ref_id)),
        },
        _ => return Err(ChronyError::Malformed("unknown address family")),
    };
    // the reference time (12 bytes at d + 28) is not needed
    let f = |k: usize| float(be32(reply, d + 40 + 4 * k));
    let tracking = ChronyTracking {
        reference,
        ref_id,
        stratum,
        leap_status,
        correction_s: f(0),
        last_offset_s: f(1),
        rms_offset_s: f(2),
        freq_ppm: f(3),
        resid_freq_ppm: f(4),
        skew_ppm: f(5),
        root_delay_s: f(6),
        root_dispersion_s: f(7),
        update_interval_s: f(8),
    };
    if tracking.root_delay_s < 0.0 || tracking.root_dispersion_s < 0.0 {
        return Err(ChronyError::Malformed("negative root delay or dispersion"));
    }
    Ok(tracking)
}

/// A UDP socket on the loopback, connected to `addr` (an address, never a
/// name: nothing is resolved).
fn connect(addr: SocketAddr) -> Result<UdpSocket, ChronyError> {
    let io = |op, e: &io::Error| ChronyError::Io { op, kind: e.kind() };
    let bind: SocketAddr = if addr.is_ipv4() {
        (Ipv4Addr::LOCALHOST, 0).into()
    } else {
        (Ipv6Addr::LOCALHOST, 0).into()
    };
    let socket = UdpSocket::bind(bind).map_err(|e| io("bind", &e))?;
    socket.connect(addr).map_err(|e| io("connect", &e))?;
    Ok(socket)
}

/// chronyd's tracking report from `addr`, within `timeout` all told: one
/// request, and replies read until one answers it or the time is up. A
/// cold path (a socket, a system call or four): start-up, never a hot one.
///
/// # Errors
/// [`ChronyError::Io`] when a socket call failed (chronyd not listening
/// refuses at once on the loopback), [`ChronyError::Timeout`] when nothing
/// answered in time, else the reply's fault.
pub fn query(
    addr: SocketAddr,
    timeout: Duration,
    sequence: u32,
) -> Result<ChronyTracking, ChronyError> {
    let io = |op, e: &io::Error| ChronyError::Io { op, kind: e.kind() };
    let deadline = Instant::now() + timeout;
    let socket = connect(addr)?;
    socket
        .set_write_timeout(Some(timeout.max(Duration::from_millis(1))))
        .map_err(|e| io("set_write_timeout", &e))?;
    socket
        .send(&request(sequence))
        .map_err(|e| io("send", &e))?;
    let mut buf = [0_u8; 512];
    let mut last = ChronyError::Timeout(timeout);
    // a stray datagram (an old request's reply) is read past, a few at most
    for _ in 0..4 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        socket
            .set_read_timeout(Some(left))
            .map_err(|e| io("set_read_timeout", &e))?;
        match socket.recv(&mut buf) {
            Ok(n) => match parse(buf.get(..n).unwrap_or_default(), sequence) {
                Err(ChronyError::Malformed(ANOTHER_REQUEST)) => {
                    last = ChronyError::Malformed(ANOTHER_REQUEST);
                }
                answer => return answer,
            },
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(ChronyError::Timeout(timeout));
            }
            Err(e) => return Err(io("recv", &e)),
        }
    }
    Err(last)
}

/// chronyd asked again without waiting: a non-blocking socket connected
/// once, one request outstanding at a time. [`Self::poll`] makes at most one
/// receive (the reply to the request the previous poll sent) and one send,
/// and allocates nothing.
#[derive(Debug)]
pub struct ChronyPoll {
    socket: UdpSocket,
    /// The sequence of the request awaiting its reply.
    pending: Option<u32>,
    /// Whether one poll has already found no reply to it.
    waited: bool,
    /// The next request's sequence.
    next: u32,
}

impl ChronyPoll {
    /// A non-blocking socket to `addr`, and the first request sent on it.
    ///
    /// # Errors
    /// The socket could not be made, connected or made non-blocking.
    pub fn open(addr: SocketAddr, sequence: u32) -> Result<Self, ChronyError> {
        let socket = connect(addr)?;
        socket.set_nonblocking(true).map_err(|e| ChronyError::Io {
            op: "set_nonblocking",
            kind: e.kind(),
        })?;
        let mut poll = Self {
            socket,
            pending: None,
            waited: false,
            next: sequence,
        };
        // a failed first send shows as the first poll's answer
        let _ = poll.send();
        Ok(poll)
    }

    /// The next request, sent without waiting.
    fn send(&mut self) -> Result<(), ChronyError> {
        let sequence = self.next;
        self.next = self.next.wrapping_add(1);
        self.waited = false;
        match self.socket.send(&request(sequence)) {
            Ok(_) => {
                self.pending = Some(sequence);
                Ok(())
            }
            Err(e) => {
                self.pending = None;
                Err(ChronyError::Io {
                    op: "send",
                    kind: e.kind(),
                })
            }
        }
    }

    /// chronyd's answer now, given the one before (`last`): the reply to the
    /// previous poll's request if it came, then the next request sent.
    ///
    /// - A reply to this request: its report, or its fault.
    /// - A reply to an older request: read past; the request stays
    ///   outstanding and nothing is sent, so the next poll reads on.
    /// - Nothing yet: the first time, `last` stands and nothing is sent; the
    ///   second, [`ChronyError::NoReply`] and a new request.
    /// - A socket error (chronyd not listening): that error.
    pub fn poll(&mut self, last: super::ChronyAnswer) -> super::ChronyAnswer {
        use super::ChronyAnswer;
        let Some(sequence) = self.pending else {
            return match self.send() {
                Ok(()) => last,
                Err(e) => ChronyAnswer::Failed(e),
            };
        };
        let mut buf = [0_u8; 512];
        let answer = match self.socket.recv(&mut buf) {
            Ok(n) => match parse(buf.get(..n).unwrap_or_default(), sequence) {
                Ok(t) => ChronyAnswer::Tracking(t),
                Err(ChronyError::Malformed(ANOTHER_REQUEST)) => return last,
                Err(e) => ChronyAnswer::Failed(e),
            },
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if !self.waited {
                    self.waited = true;
                    return last;
                }
                ChronyAnswer::Failed(ChronyError::NoReply)
            }
            Err(e) => ChronyAnswer::Failed(ChronyError::Io {
                op: "recv",
                kind: e.kind(),
            }),
        };
        // the answer stands; a send that fails shows at the next poll
        let _ = self.send();
        answer
    }
}

#[cfg(test)]
mod tests {
    use super::super::ChronyAnswer;
    use super::*;

    /// chrony's `UTI_FloatHostToNetwork`, for building replies.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    pub fn float_bits(x: f64) -> u32 {
        let (neg, x) = if x < 0.0 { (1, -x) } else { (0, x) };
        if x < 1.0e-100 {
            return 0;
        }
        let mut exp = x.log2() as i32 + 1;
        let mut coef = x.mul_add(2_f64.powi(-exp + 25), 0.5) as i32;
        while coef > (1 << 24) - 1 + neg {
            coef >>= 1;
            exp += 1;
        }
        if neg == 1 {
            coef = -coef;
        }
        ((exp as u32 & 0x7f) << 25) | (coef as u32 & 0x1ff_ffff)
    }

    /// A tracking reply as chronyd 4.6.1 builds one.
    pub fn reply(
        sequence: u32,
        ref_id: u32,
        family: u16,
        addr: [u8; 16],
        stratum: u16,
        leap: u16,
        floats: [f64; 9],
    ) -> Vec<u8> {
        let mut p = vec![0_u8; TRACKING_LEN];
        p[0] = 6;
        p[1] = 2;
        p[4..6].copy_from_slice(&33_u16.to_be_bytes());
        p[6..8].copy_from_slice(&5_u16.to_be_bytes());
        p[16..20].copy_from_slice(&sequence.to_be_bytes());
        p[28..32].copy_from_slice(&ref_id.to_be_bytes());
        p[32..48].copy_from_slice(&addr);
        p[48..50].copy_from_slice(&family.to_be_bytes());
        p[52..54].copy_from_slice(&stratum.to_be_bytes());
        p[54..56].copy_from_slice(&leap.to_be_bytes());
        for (k, v) in floats.iter().enumerate() {
            p[68 + 4 * k..72 + 4 * k].copy_from_slice(&float_bits(*v).to_be_bytes());
        }
        p
    }

    fn v4(a: [u8; 4]) -> [u8; 16] {
        let mut addr = [0_u8; 16];
        addr[..4].copy_from_slice(&a);
        addr
    }

    const FLOATS: [f64; 9] = [
        -0.000_012_5, // correction
        0.000_003,
        0.000_010,
        -12.25,
        0.001,
        0.05,
        0.000_250, // root delay
        0.000_100, // root dispersion
        16.0,
    ];

    /// The request is the header chronyc sends, padded to the reply's
    /// length: version 6, a request, command 33, the sequence, zeros.
    #[test]
    fn the_request_is_chronycs_padded_to_the_replys_length() {
        let r = request(0x0102_0304);
        assert_eq!(r.len(), 104);
        assert_eq!(&r[..12], &[6, 1, 0, 0, 0, 33, 0, 0, 1, 2, 3, 4]);
        assert!(r[12..].iter().all(|&b| b == 0));
    }

    /// chrony's float: a 7-bit signed exponent over a 25-bit signed
    /// coefficient; the encoder's values come back to within its precision.
    #[test]
    fn chronys_float_decodes() {
        assert!(float(0).abs() < f64::EPSILON);
        // exponent 2, coefficient 2^23: 2^23 * 2^(2 - 25) = 1
        assert!((float((2 << 25) | (1 << 23)) - 1.0).abs() < f64::EPSILON);
        // bit 24 is the coefficient's sign: exponent 1, coefficient -2^24
        assert!((float((1 << 25) | (1 << 24)) + 1.0).abs() < f64::EPSILON);
        // exponent -1 (0x7f), coefficient -2^24: -2^24 * 2^-26 = -0.25
        assert!((float((0x7f << 25) | (1 << 24)) + 0.25).abs() < 1e-12);
        for x in [1.0, -1.0, 0.000_25, -12.25, 16.0, 1.234_567e-9, 3.5e5] {
            let back = float(float_bits(x));
            assert!((back - x).abs() <= x.abs() * 1e-7, "{x}: {back}");
        }
    }

    /// An NTP server's report: its address, the floats in order, the bound
    /// chronyc gives (|correction| + dispersion + delay / 2).
    #[test]
    fn a_servers_report_parses() -> Result<(), ChronyError> {
        let p = reply(7, 0xA9FE_A97B, 1, v4([169, 254, 169, 123]), 4, 0, FLOATS);
        let t = parse(&p, 7)?;
        assert_eq!(
            t.reference,
            ChronyReference::Server(Some(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 123))))
        );
        assert!(t.synchronised() && !t.is_phc());
        assert_eq!((t.stratum, t.leap_status), (4, 0));
        assert!((t.correction_s + 0.000_012_5).abs() < 1e-12);
        assert!((t.freq_ppm + 12.25).abs() < 1e-6);
        assert!((t.update_interval_s - 16.0).abs() < 1e-6);
        // 12.5 + 100 + 125 us
        let b = t.bound_ns();
        assert!(b.abs_diff(237_500) <= 2, "{b}");
        assert_eq!(t.reference_name(), "169.254.169.123");
        Ok(())
    }

    /// A PHC reference clock: no address, reference ID `PHC0`; a reference
    /// ID's unprintable bytes are left out (`GPS\0` is `GPS`).
    #[test]
    fn a_phc_refclocks_report_parses() -> Result<(), ChronyError> {
        let p = reply(9, u32::from_be_bytes(*b"PHC0"), 0, [0; 16], 1, 0, FLOATS);
        let t = parse(&p, 9)?;
        assert_eq!(
            t.reference,
            ChronyReference::Refclock(RefclockId::new(u32::from_be_bytes(*b"PHC0")))
        );
        assert!(t.synchronised() && t.is_phc());
        assert_eq!(t.reference_name(), "PHC0");
        let gps = parse(
            &reply(9, u32::from_be_bytes(*b"GPS\0"), 0, [0; 16], 1, 0, FLOATS),
            9,
        )?;
        assert_eq!(
            gps.reference,
            ChronyReference::Refclock(RefclockId::new(u32::from_be_bytes(*b"GPS\0")))
        );
        assert_eq!(
            RefclockId::new(u32::from_be_bytes(*b"GPS\0")).as_str(),
            "GPS"
        );
        assert!(gps.synchronised() && !gps.is_phc());
        Ok(())
    }

    /// Unsynchronised (reference ID 0, leap 3) and chronyd's own clock
    /// (`local`, 127.127.1.1) are not synchronised.
    #[test]
    fn unsynchronised_and_local_are_not_synchronised() -> Result<(), ChronyError> {
        let none = parse(&reply(1, 0, 0, [0; 16], 0, 3, [0.0; 9]), 1)?;
        assert_eq!(none.reference, ChronyReference::None);
        assert!(!none.synchronised());
        let local = parse(&reply(1, REFID_LOCAL, 0, [0; 16], 10, 0, [0.0; 9]), 1)?;
        assert_eq!(local.reference, ChronyReference::Local);
        assert!(!local.synchronised());
        Ok(())
    }

    /// Every malformed reply is refused with its fault, never read as a
    /// report: short, another version, type, command, sequence, status or
    /// reply, a family, stratum or leap status out of range, a negative
    /// root delay.
    #[test]
    fn malformed_replies_are_refused() {
        let good = reply(5, 0, 1, v4([10, 0, 0, 1]), 3, 0, FLOATS);
        assert!(parse(&good, 5).is_ok());
        let with = |at: usize, v: u8| {
            let mut p = good.clone();
            p[at] = v;
            p
        };
        let m = |p: &[u8], seq| parse(p, seq).map(|t| t.stratum);
        let bad = |why| Err(ChronyError::Malformed(why));
        assert_eq!(m(&good[..27], 5), bad("shorter than a reply header"));
        assert_eq!(m(&good[..103], 5), bad("shorter than a tracking reply"));
        assert_eq!(m(&[], 5), bad("shorter than a reply header"));
        assert_eq!(m(&with(0, 5), 5), bad("not protocol version 6"));
        assert_eq!(m(&with(1, 1), 5), bad("not a command reply"));
        assert_eq!(m(&with(2, 1), 5), bad("not a command reply"));
        assert_eq!(m(&with(5, 34), 5), bad("a reply to another command"));
        assert_eq!(m(&good, 6), bad("a reply to another request"));
        assert_eq!(m(&with(7, 6), 5), bad("not a tracking reply"));
        assert_eq!(m(&with(49, 4), 5), bad("unknown address family"));
        assert_eq!(m(&with(53, 17), 5), bad("stratum over 16"));
        assert_eq!(m(&with(55, 4), 5), bad("leap status over 3"));
        let mut negative = good.clone();
        negative[92..96].copy_from_slice(&float_bits(-0.001).to_be_bytes());
        assert_eq!(m(&negative, 5), bad("negative root delay or dispersion"));
        assert_eq!(parse(&with(9, 3), 5), Err(ChronyError::Status(3)));
        assert_eq!(parse(&with(9, 18), 5), Err(ChronyError::Status(18)));
    }

    /// The errors read as one line each.
    #[test]
    fn the_errors_read_as_one_line() {
        let cases = [
            (
                ChronyError::Io {
                    op: "recv",
                    kind: io::ErrorKind::ConnectionRefused,
                },
                "recv: connection refused",
            ),
            (
                ChronyError::Timeout(Duration::from_millis(50)),
                "no reply within 50 ms",
            ),
            (ChronyError::NoReply, "no reply since the last check"),
            (ChronyError::Status(3), "chronyd answered status 3"),
            (
                ChronyError::Malformed("stratum over 16"),
                "malformed reply: stratum over 16",
            ),
        ];
        for (e, want) in cases {
            assert_eq!(e.to_string(), want);
        }
    }

    /// A non-blocking poll against a loopback fake: the reply to one poll's
    /// request is the next poll's answer; a reply to an older request is
    /// read past with nothing sent; no reply leaves the last answer once,
    /// then is [`ChronyError::NoReply`].
    #[test]
    fn a_poll_reads_the_previous_reply_and_never_waits() -> Result<(), Box<dyn std::error::Error>> {
        let server = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
        server.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut poll = ChronyPoll::open(server.local_addr()?, 100)?;
        let mut buf = [0_u8; 512];
        let (n, client) = server.recv_from(&mut buf)?;
        assert_eq!(n, 104);
        assert_eq!(be32(&buf, 8), 100);
        let before = ChronyAnswer::NotAsked;
        // the reply to request 100, and a stray one to request 99 ahead of it
        server.send_to(&reply(99, 0, 1, v4([10, 0, 0, 9]), 3, 0, FLOATS), client)?;
        server.send_to(&reply(100, 0, 1, v4([10, 0, 0, 1]), 3, 0, FLOATS), client)?;
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(poll.poll(before), before, "the stray reply is read past");
        let answer = poll.poll(before);
        assert!(
            matches!(answer, ChronyAnswer::Tracking(t) if t.reference_name() == "10.0.0.1"),
            "{answer:?}"
        );
        // that poll sent request 101, which nobody answers
        let (_, _) = server.recv_from(&mut buf)?;
        assert_eq!(be32(&buf, 8), 101);
        assert_eq!(poll.poll(answer), answer, "a first miss keeps the answer");
        assert_eq!(
            poll.poll(answer),
            ChronyAnswer::Failed(ChronyError::NoReply)
        );
        let (_, _) = server.recv_from(&mut buf)?;
        assert_eq!(be32(&buf, 8), 102, "a new request after the miss");
        Ok(())
    }
}
