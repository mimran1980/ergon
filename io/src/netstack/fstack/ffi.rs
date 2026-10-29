//! F-Stack's C API (`ff_api.h`, v2.0 with `ff-run-once.patch`), as the few
//! calls the backend makes, each a plain function over the `ff_*` one.
//!
//! Linked (`cfg(ergon_fstack_linked)`: the build found `FSTACK_PREFIX`) they
//! call F-Stack; otherwise each fails with `ENOSYS`, so a build with the
//! feature but without F-Stack (`--all-features` on a machine without it)
//! still compiles and links, and `super::init` refuses first.
//!
//! The `ff_*` calls set Linux `errno` values (F-Stack's `ff_os_errno`);
//! values they return as data (`SO_ERROR`) are FreeBSD's.
#![allow(unsafe_code)] // F-Stack's C API

use std::ffi::{c_char, c_int, c_void};

/// F-Stack's `struct kevent` (`ff_event.h`: FreeBSD 12+'s layout).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Kevent {
    /// The socket.
    pub ident: usize,
    /// `EVFILT_*`.
    pub filter: i16,
    /// `EV_*`.
    pub flags: u16,
    /// Filter flags.
    pub fflags: u32,
    /// Filter data (bytes readable).
    pub data: i64,
    /// The registration's token.
    pub udata: *mut c_void,
    /// Extensions.
    pub ext: [u64; 4],
}

impl Kevent {
    /// An empty event.
    pub const EMPTY: Self = Self {
        ident: 0,
        filter: 0,
        flags: 0,
        fflags: 0,
        data: 0,
        udata: std::ptr::null_mut(),
        ext: [0; 4],
    };
}

/// `EVFILT_READ`.
pub const EVFILT_READ: i16 = -1;
/// `EV_ADD`.
pub const EV_ADD: u16 = 0x0001;
/// `EV_DELETE`.
pub const EV_DELETE: u16 = 0x0002;
/// Linux's `FIONBIO`, which `ff_ioctl` translates.
const LINUX_FIONBIO: std::ffi::c_ulong = 0x5421;

#[cfg(ergon_fstack_linked)]
mod sys {
    use std::ffi::{c_char, c_int, c_ulong, c_void};

    use super::Kevent;

    // libfstack.a (build.rs links it whole, with its DPDK)
    unsafe extern "C" {
        pub fn ff_init(argc: c_int, argv: *const *mut c_char) -> c_int;
        pub fn ff_run_once() -> c_int;
        pub fn ff_socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
        pub fn ff_setsockopt(
            s: c_int,
            level: c_int,
            opt: c_int,
            val: *const c_void,
            len: libc::socklen_t,
        ) -> c_int;
        pub fn ff_getsockopt(
            s: c_int,
            level: c_int,
            opt: c_int,
            val: *mut c_void,
            len: *mut libc::socklen_t,
        ) -> c_int;
        pub fn ff_getsockopt_freebsd(
            s: c_int,
            level: c_int,
            opt: c_int,
            val: *mut c_void,
            len: *mut libc::socklen_t,
        ) -> c_int;
        pub fn ff_ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
        pub fn ff_bind(s: c_int, addr: *const libc::sockaddr, len: libc::socklen_t) -> c_int;
        pub fn ff_listen(s: c_int, backlog: c_int) -> c_int;
        pub fn ff_accept(s: c_int, addr: *mut libc::sockaddr, len: *mut libc::socklen_t) -> c_int;
        pub fn ff_connect(s: c_int, addr: *const libc::sockaddr, len: libc::socklen_t) -> c_int;
        pub fn ff_getpeername(
            s: c_int,
            addr: *mut libc::sockaddr,
            len: *mut libc::socklen_t,
        ) -> c_int;
        pub fn ff_close(fd: c_int) -> c_int;
        pub fn ff_read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
        pub fn ff_write(fd: c_int, buf: *const c_void, n: usize) -> isize;
        pub fn ff_kqueue() -> c_int;
        pub fn ff_kevent(
            kq: c_int,
            changes: *const Kevent,
            nchanges: c_int,
            events: *mut Kevent,
            nevents: c_int,
            timeout: *const libc::timespec,
        ) -> c_int;
    }
}

/// Without F-Stack: every call fails with `ENOSYS`.
#[cfg(not(ergon_fstack_linked))]
#[allow(clippy::unnecessary_wraps, clippy::missing_const_for_fn)]
mod sys {
    use std::ffi::{c_char, c_int, c_ulong, c_void};

    use super::Kevent;

    fn enosys() -> c_int {
        // SAFETY: the calling thread's errno, a valid int.
        unsafe { *libc::__errno_location() = libc::ENOSYS };
        -1
    }

    pub unsafe fn ff_init(_: c_int, _: *const *mut c_char) -> c_int {
        enosys()
    }
    pub unsafe fn ff_run_once() -> c_int {
        0
    }
    pub unsafe fn ff_socket(_: c_int, _: c_int, _: c_int) -> c_int {
        enosys()
    }
    pub unsafe fn ff_setsockopt(
        _: c_int,
        _: c_int,
        _: c_int,
        _: *const c_void,
        _: libc::socklen_t,
    ) -> c_int {
        enosys()
    }
    pub unsafe fn ff_getsockopt(
        _: c_int,
        _: c_int,
        _: c_int,
        _: *mut c_void,
        _: *mut libc::socklen_t,
    ) -> c_int {
        enosys()
    }
    pub unsafe fn ff_getsockopt_freebsd(
        _: c_int,
        _: c_int,
        _: c_int,
        _: *mut c_void,
        _: *mut libc::socklen_t,
    ) -> c_int {
        enosys()
    }
    pub unsafe fn ff_ioctl(_: c_int, _: c_ulong, _: *const c_int) -> c_int {
        enosys()
    }
    pub unsafe fn ff_bind(_: c_int, _: *const libc::sockaddr, _: libc::socklen_t) -> c_int {
        enosys()
    }
    pub unsafe fn ff_listen(_: c_int, _: c_int) -> c_int {
        enosys()
    }
    pub unsafe fn ff_accept(_: c_int, _: *mut libc::sockaddr, _: *mut libc::socklen_t) -> c_int {
        enosys()
    }
    pub unsafe fn ff_connect(_: c_int, _: *const libc::sockaddr, _: libc::socklen_t) -> c_int {
        enosys()
    }
    pub unsafe fn ff_getpeername(
        _: c_int,
        _: *mut libc::sockaddr,
        _: *mut libc::socklen_t,
    ) -> c_int {
        enosys()
    }
    pub unsafe fn ff_close(_: c_int) -> c_int {
        enosys()
    }
    pub unsafe fn ff_read(_: c_int, _: *mut c_void, _: usize) -> isize {
        enosys();
        -1
    }
    pub unsafe fn ff_write(_: c_int, _: *const c_void, _: usize) -> isize {
        enosys();
        -1
    }
    pub unsafe fn ff_kqueue() -> c_int {
        enosys()
    }
    pub unsafe fn ff_kevent(
        _: c_int,
        _: *const Kevent,
        _: c_int,
        _: *mut Kevent,
        _: c_int,
        _: *const libc::timespec,
    ) -> c_int {
        enosys()
    }
}

#[allow(clippy::cast_possible_truncation)] // 16 bytes
const SOCKADDR_LEN: libc::socklen_t = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;

static ZERO: libc::timespec = libc::timespec {
    tv_sec: 0,
    tv_nsec: 0,
};

/// `ff_init(argv)`; `argv` must stay alive for the process (F-Stack and
/// the EAL may keep its strings).
pub fn init(argv: &'static [*mut c_char]) -> c_int {
    let count = c_int::try_from(argv.len().saturating_sub(1)).unwrap_or(0);
    // SAFETY: `argv` is `count` NUL-terminated strings and a NULL, never freed.
    unsafe { sys::ff_init(count, argv.as_ptr()) }
}

/// One F-Stack pass (`ff_run_once`).
#[inline]
pub fn run_once() {
    // SAFETY: plain call; the caller is F-Stack's thread (`super::on_lcore`).
    unsafe { sys::ff_run_once() };
}

/// A TCP socket (`AF_INET`, `SOCK_STREAM`).
pub fn socket_tcp() -> c_int {
    // SAFETY: plain call.
    unsafe { sys::ff_socket(libc::AF_INET, libc::SOCK_STREAM, 0) }
}

/// `FIONBIO` on.
pub fn set_nonblocking(fd: c_int) -> c_int {
    let on: c_int = 1;
    // SAFETY: FIONBIO reads one int through the pointer, alive for the call.
    unsafe { sys::ff_ioctl(fd, LINUX_FIONBIO, &raw const on) }
}

/// `ff_setsockopt` of one int (Linux level and option names).
pub fn setsockopt_int(fd: c_int, level: c_int, opt: c_int, v: c_int) -> c_int {
    // SAFETY: `v` lives for the call and its length is passed.
    unsafe { sys::ff_setsockopt(fd, level, opt, (&raw const v).cast(), 4) }
}

/// `ff_getsockopt` of one int (Linux level and option names) into `out`.
pub fn getsockopt_int(fd: c_int, level: c_int, opt: c_int, out: &mut c_int) -> c_int {
    let mut len: libc::socklen_t = 4;
    // SAFETY: `out` and `len` live for the call; `len` is `out`'s size.
    unsafe { sys::ff_getsockopt(fd, level, opt, std::ptr::from_mut(out).cast(), &raw mut len) }
}

/// `ff_getsockopt_freebsd` (FreeBSD's level and option numbers) into
/// `buf`: the return code and the bytes written.
pub fn getsockopt_freebsd(fd: c_int, level: c_int, opt: c_int, buf: &mut [u8]) -> (c_int, usize) {
    let mut len = libc::socklen_t::try_from(buf.len()).unwrap_or(libc::socklen_t::MAX);
    // SAFETY: `buf` and `len` live for the call; `len` is at most `buf`'s size.
    let rc = unsafe {
        sys::ff_getsockopt_freebsd(fd, level, opt, buf.as_mut_ptr().cast(), &raw mut len)
    };
    (rc, (len as usize).min(buf.len()))
}

/// `ff_bind` to `sa`.
pub fn bind(fd: c_int, sa: &libc::sockaddr_in) -> c_int {
    // SAFETY: `sa` is a sockaddr_in of SOCKADDR_LEN bytes, alive for the call.
    unsafe { sys::ff_bind(fd, std::ptr::from_ref(sa).cast(), SOCKADDR_LEN) }
}

/// `ff_listen`.
pub fn listen(fd: c_int, backlog: c_int) -> c_int {
    // SAFETY: plain call.
    unsafe { sys::ff_listen(fd, backlog) }
}

/// `ff_accept`, no address asked for.
pub fn accept(fd: c_int) -> c_int {
    // SAFETY: NULL address and length ask for none.
    unsafe { sys::ff_accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) }
}

/// `ff_connect` to `sa`.
pub fn connect(fd: c_int, sa: &libc::sockaddr_in) -> c_int {
    // SAFETY: `sa` is a sockaddr_in of SOCKADDR_LEN bytes, alive for the call.
    unsafe { sys::ff_connect(fd, std::ptr::from_ref(sa).cast(), SOCKADDR_LEN) }
}

/// `ff_getpeername` into a scratch address (whether there is a peer).
pub fn getpeername(fd: c_int) -> c_int {
    // SAFETY: an all-zero sockaddr_storage is valid.
    let mut sa: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::sockaddr_storage>())
        .unwrap_or(libc::socklen_t::MAX);
    // SAFETY: `sa` and `len` live for the call; `len` is `sa`'s size.
    unsafe { sys::ff_getpeername(fd, (&raw mut sa).cast(), &raw mut len) }
}

/// `ff_close`.
pub fn close(fd: c_int) -> c_int {
    // SAFETY: plain call; the caller owns `fd`.
    unsafe { sys::ff_close(fd) }
}

/// `ff_read` into `len` bytes at `buf`.
///
/// The caller guarantees `buf..buf + len` is writable for the call (the
/// receive slot's contract: armed with a live buffer before every read).
#[inline]
pub fn read_raw(fd: c_int, buf: *mut u8, len: usize) -> isize {
    // SAFETY: as the caller guarantees.
    unsafe { sys::ff_read(fd, buf.cast(), len) }
}

/// `ff_read` into `buf`.
#[inline]
pub fn read(fd: c_int, buf: &mut [u8]) -> isize {
    read_raw(fd, buf.as_mut_ptr(), buf.len())
}

/// `ff_write` from `buf`.
#[inline]
pub fn write(fd: c_int, buf: &[u8]) -> isize {
    // SAFETY: reads `buf`, alive for the call.
    unsafe { sys::ff_write(fd, buf.as_ptr().cast(), buf.len()) }
}

/// `ff_kqueue`.
pub fn kqueue() -> c_int {
    // SAFETY: plain call.
    unsafe { sys::ff_kqueue() }
}

/// `ff_kevent` with a zero timeout: `changes` in, ready events into
/// `events`.
#[inline]
pub fn kevent(kq: c_int, changes: &[Kevent], events: &mut [Kevent]) -> c_int {
    let nc = c_int::try_from(changes.len()).unwrap_or(c_int::MAX);
    let ne = c_int::try_from(events.len()).unwrap_or(c_int::MAX);
    // SAFETY: both slices live for the call, their lengths passed.
    unsafe {
        sys::ff_kevent(
            kq,
            changes.as_ptr(),
            nc,
            events.as_mut_ptr(),
            ne,
            &raw const ZERO,
        )
    }
}
