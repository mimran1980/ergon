//! Socket options by `setsockopt` / `getsockopt`, for the mio arms (which
//! have no driver to set them) and for reading every arm's sockets back
//! the same way.

use std::io;
use std::os::fd::RawFd;

use super::Error;

/// `TCP_RTO_MIN_US` (`include/uapi/linux/tcp.h`, Linux 6.15); libc does not
/// define it.
pub const TCP_RTO_MIN_US: libc::c_int = 45;
/// `TCP_DELACK_MAX_US` (Linux 6.15).
pub const TCP_DELACK_MAX_US: libc::c_int = 46;
/// `tcpi_options` bit: TCP timestamps negotiated.
const TCPI_OPT_TIMESTAMPS: u8 = 1;

fn int_len() -> libc::socklen_t {
    // a c_int is 4 bytes
    4
}

/// Sets an integer option.
///
/// # Errors
/// `setsockopt` fails.
pub fn set_int(fd: RawFd, level: libc::c_int, opt: libc::c_int, v: libc::c_int) -> io::Result<()> {
    // SAFETY: `v` lives for the call and its length is passed.
    let rc = unsafe { libc::setsockopt(fd, level, opt, (&raw const v).cast(), int_len()) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Reads an integer option.
///
/// # Errors
/// `getsockopt` fails.
pub fn get_int(fd: RawFd, level: libc::c_int, opt: libc::c_int) -> io::Result<libc::c_int> {
    let mut v: libc::c_int = 0;
    let mut len = int_len();
    // SAFETY: `v` and `len` are valid for the call; the kernel writes at
    // most `len` bytes.
    let rc = unsafe { libc::getsockopt(fd, level, opt, (&raw mut v).cast(), &raw mut len) };
    if rc == 0 {
        Ok(v)
    } else {
        Err(io::Error::last_os_error())
    }
}

fn to_int(name: &str, v: u32) -> Result<libc::c_int, Error> {
    libc::c_int::try_from(v).map_err(|_| Error::Config(format!("{name} {v} is over i32::MAX")))
}

/// `SO_RCVBUF`.
///
/// # Errors
/// `setsockopt` fails.
pub fn set_rcvbuf(fd: RawFd, bytes: u32) -> Result<(), Error> {
    Ok(set_int(
        fd,
        libc::SOL_SOCKET,
        libc::SO_RCVBUF,
        to_int("rcvbuf", bytes)?,
    )?)
}

/// `SO_SNDBUF`.
///
/// # Errors
/// `setsockopt` fails.
pub fn set_sndbuf(fd: RawFd, bytes: u32) -> Result<(), Error> {
    Ok(set_int(
        fd,
        libc::SOL_SOCKET,
        libc::SO_SNDBUF,
        to_int("sndbuf", bytes)?,
    )?)
}

/// `SO_BUSY_POLL`, µs, read back.
///
/// # Errors
/// `setsockopt` fails (raising it needs `CAP_NET_ADMIN`), or the kernel
/// keeps another value.
pub fn set_busy_poll(fd: RawFd, us: u32) -> Result<(), Error> {
    set_int(
        fd,
        libc::SOL_SOCKET,
        libc::SO_BUSY_POLL,
        to_int("busy_read_us", us)?,
    )
    .map_err(|e| Error::Config(format!("SO_BUSY_POLL {us} µs: {e}")))?;
    let kept = get_int(fd, libc::SOL_SOCKET, libc::SO_BUSY_POLL)?;
    if i64::from(kept) != i64::from(us) {
        return Err(Error::Config(format!(
            "SO_BUSY_POLL {us} µs: the kernel keeps {kept}"
        )));
    }
    Ok(())
}

/// `TCP_NODELAY` on.
///
/// # Errors
/// `setsockopt` fails.
pub fn set_nodelay(fd: RawFd) -> Result<(), Error> {
    Ok(set_int(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, 1)?)
}

/// `TCP_QUICKACK` on: one `setsockopt`, the per-read re-arm.
///
/// # Errors
/// `setsockopt` fails.
#[inline]
pub fn quickack(fd: RawFd) -> io::Result<()> {
    set_int(fd, libc::IPPROTO_TCP, libc::TCP_QUICKACK, 1)
}

/// Sets `TCP_RTO_MIN_US` or `TCP_DELACK_MAX_US` (`opt`) to `us` and reads
/// it back: fails closed when the kernel does not know the option (before
/// Linux 6.15), refuses the value, or keeps a rounded one.
///
/// # Errors
/// [`Error::Config`] naming the option.
pub fn set_tcp_timer(fd: RawFd, opt: libc::c_int, us: u32) -> Result<(), Error> {
    let name = if opt == TCP_RTO_MIN_US {
        "TCP_RTO_MIN_US"
    } else {
        "TCP_DELACK_MAX_US"
    };
    set_int(fd, libc::IPPROTO_TCP, opt, to_int(name, us)?)
        .map_err(|e| Error::Config(format!("{name} {us} µs (needs Linux 6.15+): {e}")))?;
    let kept = get_int(fd, libc::IPPROTO_TCP, opt)
        .map_err(|e| Error::Config(format!("{name}: reading it back: {e}")))?;
    if i64::from(kept) != i64::from(us) {
        return Err(Error::Config(format!(
            "{name} {us} µs: the kernel keeps {kept} µs (whole timer ticks only)"
        )));
    }
    Ok(())
}

/// What `TCP_INFO` says about a connection that the suite reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpInfo {
    /// `tcpi_total_retrans`: segments retransmitted over its life.
    pub total_retrans: u32,
    /// TCP timestamps negotiated (12 more header bytes per segment).
    pub timestamps: bool,
}

/// Reads `TCP_INFO`: one `getsockopt`, which takes the socket's lock
/// briefly. Never on the hot path.
///
/// # Errors
/// `getsockopt` fails, or the kernel's `tcp_info` stops before
/// `tcpi_total_retrans`.
pub fn tcp_info(fd: RawFd) -> io::Result<TcpInfo> {
    // struct tcp_info: eight u8 fields (tcpi_options at 5), then u32s;
    // tcpi_total_retrans at 100.
    let mut b = [0u8; 256];
    let mut len = libc::socklen_t::try_from(b.len()).map_err(io::Error::other)?;
    // SAFETY: `b` and `len` are valid for the call; the kernel writes at
    // most `len` bytes.
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_INFO,
            b.as_mut_ptr().cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    if (len as usize) < 104 {
        return Err(io::Error::other("tcp_info stops before tcpi_total_retrans"));
    }
    Ok(TcpInfo {
        total_retrans: u32::from_ne_bytes([b[100], b[101], b[102], b[103]]),
        timestamps: b[5] & TCPI_OPT_TIMESTAMPS != 0,
    })
}

/// What every arm reports from its first socket, read the same way:
/// `SO_RCVBUF` and `SO_SNDBUF` as the kernel keeps them, `SO_BUSY_POLL`,
/// and for TCP `TCP_NODELAY` and the two timers (`-` where the kernel has
/// none).
///
/// # Errors
/// A `getsockopt` the kernel always has fails.
pub fn read_back(fd: RawFd, tcp: bool, out: &mut Vec<(String, String)>) -> Result<(), Error> {
    let mut put = |k: &str, v: String| out.push((k.to_owned(), v));
    put(
        "readback.so_rcvbuf",
        get_int(fd, libc::SOL_SOCKET, libc::SO_RCVBUF)?.to_string(),
    );
    put(
        "readback.so_sndbuf",
        get_int(fd, libc::SOL_SOCKET, libc::SO_SNDBUF)?.to_string(),
    );
    put(
        "readback.so_busy_poll",
        get_int(fd, libc::SOL_SOCKET, libc::SO_BUSY_POLL)?.to_string(),
    );
    if tcp {
        put(
            "readback.tcp_nodelay",
            get_int(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY)?.to_string(),
        );
        for (k, opt) in [
            ("readback.tcp_rto_min_us", TCP_RTO_MIN_US),
            ("readback.tcp_delack_max_us", TCP_DELACK_MAX_US),
        ] {
            put(
                k,
                get_int(fd, libc::IPPROTO_TCP, opt)
                    .map_or_else(|_| "-".to_owned(), |v| v.to_string()),
            );
        }
        let info = tcp_info(fd)?;
        put(
            "readback.tcp_timestamps",
            u8::from(info.timestamps).to_string(),
        );
    }
    Ok(())
}
