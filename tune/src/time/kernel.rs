//! The kernel's view of `CLOCK_REALTIME`: Linux's `adjtimex(2)` with `modes`
//! 0, a read that changes nothing and needs no privilege.
//!
//! This is the crate's one `unsafe` call. rustix (this crate's only
//! dependency) has no wrapper for `adjtimex` or `clock_adjtime` yet, so
//! `adjtimex` below declares the C library's function itself (std already
//! links the C library: no new crate) over a hand-written `struct timex`, whose
//! layout the compile-time assertions below pin to the kernel's
//! `include/uapi/linux/timex.h` on 64-bit Linux (glibc and musl declare the
//! same). It goes when rustix wraps `adjtimex`.

use std::io;

use crate::host::{Host, ReadError};

/// What the readers name the call.
const ADJTIMEX: &str = "adjtimex(2)";

/// The kernel's view of the wall clock: `adjtimex` with `modes` 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KernelSync {
    /// The clock state: 0 `TIME_OK`, 1 `TIME_INS`, 2 `TIME_DEL`, 3
    /// `TIME_OOP` (a leap second in progress), 4 `TIME_WAIT`, 5 `TIME_ERROR`
    /// (unsynchronised).
    pub state: i32,
    /// The status bits (`STA_*`).
    pub status: i32,
    /// The maximum error, µs, as the daemon last set it plus 500 µs a second
    /// since (`second_overflow`, Linux `kernel/time/ntp.c`). chronyd sets it
    /// to its root distance at each update; ntpd keeps it too;
    /// systemd-timesyncd and phc2sys set it to 0, so under them it is not a
    /// bound.
    pub maxerror_us: i64,
    /// The estimated error, µs, as the daemon set it.
    pub esterror_us: i64,
    /// The offset the kernel's PLL is still slewing out, ns.
    pub offset_ns: i64,
    /// The frequency correction, ppm.
    pub freq_ppm: f64,
}

impl KernelSync {
    /// `STA_UNSYNC`: the clock is unsynchronised.
    pub const STA_UNSYNC: i32 = 0x0040;
    /// `STA_INS`: a leap second is to be inserted at midnight UTC.
    pub const STA_INS: i32 = 0x0010;
    /// `STA_DEL`: a leap second is to be deleted at midnight UTC.
    pub const STA_DEL: i32 = 0x0020;
    /// `STA_NANO`: `offset` is in ns, not µs.
    pub const STA_NANO: i32 = 0x2000;
    /// The kernel's `maxerror` limit, µs (`NTP_PHASE_LIMIT`, 16 s): the
    /// kernel adds 500 µs to `maxerror` every second and sets `STA_UNSYNC`
    /// once it passes this, so a `maxerror` below it is being kept by a
    /// daemon.
    pub const MAXERROR_LIMIT_US: i64 = 16_000_000;

    /// The kernel's view now, on the running host.
    ///
    /// # Errors
    /// [`ReadError::NotLive`] on a tree standing for a host (only the
    /// running kernel answers), [`ReadError::NotLinux`] elsewhere than
    /// Linux, and [`ReadError::Io`] when the call fails (a seccomp filter
    /// that denies it) or this architecture's `struct timex` is not one this
    /// crate declares (only 64-bit x86 and Arm).
    pub fn read(host: &Host) -> Result<Self, ReadError> {
        if !host.is_live() {
            return Err(ReadError::NotLive(ADJTIMEX));
        }
        if !cfg!(target_os = "linux") {
            return Err(ReadError::NotLinux(ADJTIMEX));
        }
        Self::read_live().map_err(|kind| ReadError::Io {
            path: ADJTIMEX.to_owned(),
            kind,
        })
    }

    /// `adjtimex` on the running host, with nothing allocated (the error is
    /// a kind, not a message), so housekeeping can call it every few
    /// seconds.
    #[allow(clippy::missing_const_for_fn)] // const only where there is no adjtimex
    pub(crate) fn read_live() -> Result<Self, io::ErrorKind> {
        #[cfg(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ))]
        {
            let (state, tx) = adjtimex()?;
            Ok(Self::from_raw(
                state,
                tx.status,
                tx.offset,
                tx.freq,
                tx.maxerror,
                tx.esterror,
            ))
        }
        #[cfg(not(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )))]
        {
            Err(io::ErrorKind::Unsupported)
        }
    }

    /// From `adjtimex`'s return value and fields: `offset` scaled to ns
    /// (it is in µs unless `STA_NANO`), `freq` from ppm scaled by 2^16.
    #[cfg(any(
        test,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    #[must_use]
    pub(crate) fn from_raw(
        state: i32,
        status: i32,
        offset: i64,
        freq: i64,
        maxerror_us: i64,
        esterror_us: i64,
    ) -> Self {
        #[allow(clippy::cast_precision_loss)] // ppm scaled by 2^16: far under 2^52
        let freq_ppm = freq as f64 / 65_536.0;
        Self {
            state,
            status,
            maxerror_us,
            esterror_us,
            offset_ns: if status & Self::STA_NANO != 0 {
                offset
            } else {
                offset.saturating_mul(1_000)
            },
            freq_ppm,
        }
    }

    /// Whether the kernel says the clock is unsynchronised (`STA_UNSYNC`).
    #[must_use]
    pub const fn unsynchronised(&self) -> bool {
        self.status & Self::STA_UNSYNC != 0
    }

    /// Whether something keeps `maxerror` below the kernel's limit: a
    /// daemon has updated it in the last nine hours or so.
    #[must_use]
    pub const fn maxerror_kept(&self) -> bool {
        self.maxerror_us < Self::MAXERROR_LIMIT_US
    }

    /// Whether a leap second is pending or in progress: `STA_INS` or
    /// `STA_DEL` set, or the state `TIME_INS`, `TIME_DEL` or `TIME_OOP`.
    #[must_use]
    pub const fn leap_pending(&self) -> bool {
        self.status & (Self::STA_INS | Self::STA_DEL) != 0 || matches!(self.state, 1..=3)
    }

    /// `maxerror` in ns (0 if negative).
    #[must_use]
    pub(crate) fn maxerror_ns(&self) -> u64 {
        u64::try_from(self.maxerror_us)
            .unwrap_or(0)
            .saturating_mul(1_000)
    }
}

/// `struct timex` on 64-bit Linux (`include/uapi/linux/timex.h`, the same in
/// glibc's and musl's `<sys/timex.h>`): `long` and `time_t` are 64 bits,
/// with the C padding written out. Only the fields read are named.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[repr(C)]
#[derive(Clone, Copy)]
struct Timex {
    /// 0: read only.
    _modes: u32,
    _pad0: u32,
    offset: i64,
    freq: i64,
    maxerror: i64,
    esterror: i64,
    status: i32,
    _pad1: u32,
    _constant: i64,
    _precision: i64,
    _tolerance: i64,
    /// `struct timeval`: `tv_sec`, `tv_usec`.
    _time: [i64; 2],
    _tick: i64,
    _ppsfreq: i64,
    _jitter: i64,
    _shift: i32,
    _pad2: u32,
    _stabil: i64,
    _jitcnt: i64,
    _calcnt: i64,
    _errcnt: i64,
    _stbcnt: i64,
    _tai: i32,
    /// The eleven reserved `int :32` fields.
    _reserved: [i32; 11],
}

// The kernel's layout, offset by offset (`struct __kernel_timex` spells the
// padding out: an `int :32` after `modes`, `status` and `shift`).
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const _: () = {
    use std::mem::{align_of, offset_of, size_of};
    assert!(size_of::<Timex>() == 208);
    assert!(align_of::<Timex>() == 8);
    assert!(offset_of!(Timex, _modes) == 0);
    assert!(offset_of!(Timex, offset) == 8);
    assert!(offset_of!(Timex, freq) == 16);
    assert!(offset_of!(Timex, maxerror) == 24);
    assert!(offset_of!(Timex, esterror) == 32);
    assert!(offset_of!(Timex, status) == 40);
    assert!(offset_of!(Timex, _constant) == 48);
    assert!(offset_of!(Timex, _time) == 72);
    assert!(offset_of!(Timex, _tick) == 88);
    assert!(offset_of!(Timex, _shift) == 112);
    assert!(offset_of!(Timex, _stabil) == 120);
    assert!(offset_of!(Timex, _tai) == 160);
    assert!(offset_of!(Timex, _reserved) == 164);
};

/// Linux's `adjtimex` with `modes` 0: the kernel's NTP state, read only
/// (only a non-zero `modes` needs `CAP_SYS_TIME`), so any process may call
/// it. The clock state (`TIME_OK` 0 ... `TIME_ERROR` 5) and the `timex`, or
/// the error's kind. Allocates nothing.
///
/// The crate's one `unsafe` call: see the module documentation.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[allow(unsafe_code)]
fn adjtimex() -> Result<(i32, Timex), io::ErrorKind> {
    unsafe extern "C" {
        /// glibc's and musl's `int adjtimex(struct timex *buf)`.
        fn adjtimex(buf: *mut Timex) -> std::ffi::c_int;
    }
    let mut tx = Timex {
        _modes: 0,
        _pad0: 0,
        offset: 0,
        freq: 0,
        maxerror: 0,
        esterror: 0,
        status: 0,
        _pad1: 0,
        _constant: 0,
        _precision: 0,
        _tolerance: 0,
        _time: [0; 2],
        _tick: 0,
        _ppsfreq: 0,
        _jitter: 0,
        _shift: 0,
        _pad2: 0,
        _stabil: 0,
        _jitcnt: 0,
        _calcnt: 0,
        _errcnt: 0,
        _stbcnt: 0,
        _tai: 0,
        _reserved: [0; 11],
    };
    // SAFETY: `adjtimex` reads and writes the one `struct timex` that `buf`
    // points to, and nothing else. `tx` is a live, initialised, writable and
    // aligned `Timex` for the whole call, with the C layout the assertions
    // above check, so every byte the kernel writes lands inside it. `modes`
    // is 0: the kernel only reports its state and changes nothing.
    let state = unsafe { adjtimex(&raw mut tx) };
    if state < 0 {
        return Err(io::Error::last_os_error().kind());
    }
    Ok((state, tx))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `offset` is µs unless `STA_NANO`, `freq` is ppm scaled by 2^16, and
    /// the flags read as the kernel defines them.
    #[test]
    fn the_raw_fields_scale_as_the_kernel_defines_them() {
        let k = KernelSync::from_raw(0, 0x0001, -250, -819_200, 1_500, 20);
        assert_eq!(k.offset_ns, -250_000);
        assert!((k.freq_ppm + 12.5).abs() < f64::EPSILON);
        assert_eq!(k.maxerror_ns(), 1_500_000);
        assert!(!k.unsynchronised() && k.maxerror_kept() && !k.leap_pending());
        let k = KernelSync::from_raw(5, 0x2041, -250, 0, 16_000_000, 0);
        assert_eq!(k.offset_ns, -250);
        assert!(k.unsynchronised() && !k.maxerror_kept());
        for (state, status) in [(0, 0x0010), (0, 0x0020), (1, 0), (2, 0), (3, 0)] {
            assert!(KernelSync::from_raw(state, status, 0, 0, 0, 0).leap_pending());
        }
        assert!(!KernelSync::from_raw(4, 0, 0, 0, 0, 0).leap_pending());
    }

    /// On a tree only the running host answers; on the running host,
    /// Linux's kernel answers unprivileged, and elsewhere it is not Linux.
    #[test]
    fn only_the_running_linux_host_is_asked() {
        let tree = Host::at(std::env::temp_dir());
        assert_eq!(KernelSync::read(&tree), Err(ReadError::NotLive(ADJTIMEX)));
        let live = KernelSync::read(&Host::system());
        if cfg!(target_os = "linux") {
            assert!(
                live.as_ref().is_ok_and(|k| (0..=5).contains(&k.state)),
                "{live:?}"
            );
        } else {
            assert_eq!(live, Err(ReadError::NotLinux(ADJTIMEX)));
        }
    }
}
