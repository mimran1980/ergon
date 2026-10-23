//! The libcurl implementation behind `HttpClient` (feature `curl`).
//!
//! - libcurl is linked in-process through the `curl` crate: no separate
//!   program.
//! - One multi handle; easy handles are reused, so libcurl's connection cache
//!   keeps connections warm.
//! - Driven without blocking.
//! - Its sockets get the driver's TCP keepalive and `TCP_USER_TIMEOUT`
//!   (`HttpClientConfig::driver`), set when libcurl opens them.
#![allow(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::net::SocketAddr;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use curl::easy::{Easy2, Handler as CurlHandler, HttpVersion, List, WriteError};
use curl::multi::{Easy2Handle, Events, Multi};

use crate::client::{ClientHandler, CurlDrive, HttpClientConfig};
use crate::clock::monotonic_ns;
use crate::error::{Error, Result};
use crate::http::Response;
use crate::net::Target;
use crate::outcome::Sent;
use crate::route::{self, Flow, QuickAckCheck, QuickAckError, RouteCheck, RouteError};

// libcurl 7.80's pre-request callback (curl.h): called once a connection is
// set up or reused, before the request goes out, with the connection's
// addresses. The curl crate does not wrap it.
const CURLOPT_PREREQFUNCTION: c_int = 20_312;
const CURLOPT_PREREQDATA: c_int = 10_313;
const CURL_PREREQFUNC_OK: c_int = 0;
const CURL_PREREQFUNC_ABORT: c_int = 1;

type PrereqFn = extern "C" fn(*mut c_void, *mut c_char, *mut c_char, c_int, c_int) -> c_int;

unsafe extern "C" {
    /// libcurl's `curl_easy_setopt`, linked through the curl crate.
    fn curl_easy_setopt(handle: *mut c_void, option: c_int, ...) -> c_int;
}

/// The connections a client's handles have checked, with what the check
/// found. libcurl gives a transfer any idle connection of the client's pool,
/// whichever handle runs it, so this record is the client's: one per handle
/// would look a connection up again each time a handle got another one
/// (nearly every request with a pool of two or more). It keeps the `cap`
/// latest, twice the pool, since libcurl keeps at most `pool` connections to
/// the host open: a connection that outlives that many newer ones is looked
/// up once more.
#[derive(Debug, Default)]
struct Checked {
    flows: VecDeque<(Flow, Arc<RouteCheck>)>,
    cap: usize,
}

impl Checked {
    fn new(pool: usize) -> Self {
        Self {
            flows: VecDeque::new(),
            cap: pool.saturating_mul(2).max(1),
        }
    }

    /// What the check found for `flow`, if it was checked.
    fn get(&self, flow: &Flow) -> Option<Arc<RouteCheck>> {
        self.flows
            .iter()
            .find(|(f, _)| f == flow)
            .map(|(_, found)| Arc::clone(found))
    }

    fn insert(&mut self, flow: Flow, found: Arc<RouteCheck>) {
        if self.flows.len() >= self.cap {
            self.flows.pop_front();
        }
        self.flows.push_back((flow, found));
    }
}

/// TCP keepalive, `TCP_USER_TIMEOUT` and the per-socket TCP timers for
/// libcurl's sockets: the same settings the driver gives its connections.
#[derive(Debug, Clone, Copy, Default)]
struct TcpOpts {
    /// Idle, interval and count; idle 0 leaves keepalive off.
    keepalive: (u32, u32, u32),
    /// 0 leaves the kernel default.
    user_timeout_ms: u32,
    /// `TCP_RTO_MIN_US` and `TCP_DELACK_MAX_US` (`None`: left alone).
    rto_min_us: Option<u32>,
    delack_max_us: Option<u32>,
}

impl TcpOpts {
    const fn of(cfg: &crate::backend::Config) -> Self {
        Self {
            keepalive: (
                cfg.keepalive_idle_s,
                cfg.keepalive_interval_s,
                cfg.keepalive_count,
            ),
            user_timeout_ms: cfg.tcp_user_timeout_ms,
            rto_min_us: cfg.tcp_rto_min_us,
            delack_max_us: cfg.tcp_delack_max_us,
        }
    }

    fn apply(self, fd: RawFd) -> std::io::Result<()> {
        let (idle, interval, count) = self.keepalive;
        if idle > 0 {
            crate::sys::set_tcp_keepalive(fd, idle, interval, count)?;
        }
        if self.user_timeout_ms > 0 {
            crate::sys::set_tcp_user_timeout(fd, self.user_timeout_ms)?;
        }
        crate::sys::apply_tcp_timers(fd, self.rto_min_us, self.delack_max_us)
            .map_err(std::io::Error::other)?;
        Ok(())
    }
}

/// Collects one response, checks each new connection's route for quick
/// ACK, and sets the TCP options on each socket libcurl opens.
#[derive(Default)]
struct Collector {
    head: Vec<u8>,
    body: Vec<u8>,
    tcp: TcpOpts,
    quickack: QuickAckCheck,
    /// The connections the client's handles have checked.
    checked: Arc<Mutex<Checked>>,
    /// The connection this handle last used, once checked.
    last: Option<Flow>,
    /// What the check found for it.
    route: Arc<RouteCheck>,
    /// Why the check refused the transfer it aborted.
    refused: Option<QuickAckError>,
}

impl Collector {
    /// The quick-ACK check for the connection a transfer is about to use: a
    /// connection any handle of the client already checked costs the address
    /// comparison (and, if another handle checked it, the client's record),
    /// no system call.
    fn check(&mut self, flow: std::result::Result<Flow, RouteError>) -> c_int {
        if let Ok(f) = flow {
            if self.last == Some(f) {
                return CURL_PREREQFUNC_OK;
            }
            let seen = self
                .checked
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&f);
            if let Some(found) = seen {
                self.last = Some(f);
                self.route = found;
                return CURL_PREREQFUNC_OK;
            }
        }
        self.last = None;
        match route::check_with(
            self.quickack,
            flow,
            route::lookup,
            &route::WARNED,
            crate::warn::emit,
        ) {
            Ok(found) => {
                let found = Arc::new(found);
                if let Ok(f) = flow {
                    self.checked
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .insert(f, Arc::clone(&found));
                    self.last = Some(f);
                }
                self.route = found;
                CURL_PREREQFUNC_OK
            }
            Err(e) => {
                self.route = Arc::default();
                self.refused = Some(e);
                CURL_PREREQFUNC_ABORT
            }
        }
    }
}

/// `CURLOPT_PREREQFUNCTION`: runs the quick-ACK check on the thread driving
/// the multi handle.
extern "C" fn prereq(
    data: *mut c_void,
    peer: *mut c_char,
    local: *mut c_char,
    peer_port: c_int,
    local_port: c_int,
) -> c_int {
    let run = || {
        // SAFETY: `data` is this handle's `Collector` (CURLOPT_PREREQDATA),
        // which `Easy2` keeps boxed at one address for the handle's life;
        // libcurl calls back only from inside a multi call, while no Rust
        // reference to the collector is live.
        let c = unsafe { &mut *data.cast::<Collector>() };
        let flow = addr(local, local_port)
            .zip(addr(peer, peer_port))
            .map(|(l, p)| Flow::new(l, p))
            .ok_or(RouteError::Malformed(
                "libcurl reported no connection addresses",
            ));
        c.check(flow)
    };
    // A panic must not unwind into libcurl.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or(CURL_PREREQFUNC_ABORT)
}

fn addr(ip: *const c_char, port: c_int) -> Option<SocketAddr> {
    if ip.is_null() {
        return None;
    }
    // SAFETY: libcurl passes a NUL-terminated string valid for the call.
    let s = unsafe { CStr::from_ptr(ip) }.to_str().ok()?;
    Some(SocketAddr::new(s.parse().ok()?, u16::try_from(port).ok()?))
}

impl CurlHandler for Collector {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        self.body.extend_from_slice(data);
        Ok(data.len())
    }

    fn header(&mut self, data: &[u8]) -> bool {
        // A new status line (after a 100 Continue or a redirect) starts over.
        if data.starts_with(b"HTTP/") {
            self.head.clear();
        }
        self.head.extend_from_slice(data);
        true
    }

    /// Opens the socket as the curl crate does (close-on-exec), then sets
    /// keepalive and `TCP_USER_TIMEOUT` on a TCP one. Runs once per
    /// connection, never per request. `None` fails the connect.
    fn open_socket(&mut self, family: c_int, socktype: c_int, protocol: c_int) -> Option<RawFd> {
        // libcurl passes the type with the creation flags already in it
        // (`SOCK_STREAM | SOCK_CLOEXEC`, 524289, not plain `SOCK_STREAM`, 1),
        // so the type bits have to be masked off before the comparison. A bare
        // `== SOCK_STREAM` never matches, the options are never set, and the
        // socket comes up with the kernel's keepalive defaults (idle 7200,
        // interval 75, count 9).
        const TYPE_MASK: c_int = !(libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK);
        let s = socket2::Socket::new(family.into(), socktype.into(), Some(protocol.into())).ok()?;
        if socktype & TYPE_MASK == libc::SOCK_STREAM {
            self.tcp.apply(s.as_raw_fd()).ok()?;
        }
        Some(s.into_raw_fd())
    }
}

/// Sockets libcurl asked to watch (socket-action mode).
struct Watch {
    /// Closed when the client drops, after `multi` (declared earlier), whose
    /// clean-up still removes sockets from it.
    ep: OwnedFd,
    /// Deadline for `curl_multi_socket_action(CURL_SOCKET_TIMEOUT)`, monotonic
    /// ns; `i64::MAX` when none.
    timer: Arc<AtomicI64>,
}

/// Fields drop in declaration order: `multi` (and with it libcurl's socket
/// callbacks) goes before `watch` closes the epoll descriptor.
pub struct CurlClient {
    multi: Multi,
    free: Vec<Easy2<Collector>>,
    active: Vec<(u64, Easy2Handle<Collector>)>,
    base: String,
    resolve: Option<String>,
    fixed: Vec<String>,
    ca_file: Option<String>,
    timeout: Duration,
    pool: usize,
    /// `HttpClientConfig::curl_allow_non_idempotent_methods`.
    allow_writes: bool,
    tcp: TcpOpts,
    quickack: QuickAckCheck,
    /// The connections the quick-ACK check has seen, shared by every handle.
    checked: Arc<Mutex<Checked>>,
    watch: Option<Watch>,
}

impl CurlClient {
    pub(crate) fn new(cfg: &HttpClientConfig, target: &Target) -> Result<Self> {
        // Its keepalive settings apply here too: refused, never ignored.
        cfg.driver.validate()?;
        // As `Driver::new`: a TCP timer this kernel lacks or would round is
        // refused here, typed, not at libcurl's first socket.
        crate::sys::probe_tcp_timers(cfg.driver.tcp_rto_min_us, cfg.driver.tcp_delack_max_us)?;
        curl::init();
        let mut multi = Multi::new();
        multi
            .set_max_host_connections(cfg.pool)
            .map_err(|e| Error::Http(format!("libcurl: {e}")))?;
        let base = if target.port == 443 {
            format!("https://{}", target.host)
        } else {
            format!("https://{}:{}", target.host, target.port)
        };
        let watch = if cfg.curl_drive == CurlDrive::SocketAction {
            // SAFETY: plain syscall.
            let raw = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
            if raw < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            // SAFETY: a fresh descriptor we own; closed on every return path.
            let ep_owned = unsafe { OwnedFd::from_raw_fd(raw) };
            let ep = ep_owned.as_raw_fd();
            let timer = Arc::new(AtomicI64::new(i64::MAX));
            let t = timer.clone();
            multi
                .timer_function(move |d| {
                    let at = d.map_or(i64::MAX, |d| {
                        monotonic_ns()
                            .saturating_add(i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
                    });
                    t.store(at, Ordering::Relaxed);
                    true
                })
                .map_err(|e| Error::Http(format!("libcurl: {e}")))?;
            // Interest per socket, owned by the socket callback.
            let int: Mutex<HashMap<RawFd, u32>> = Mutex::new(HashMap::new());
            multi
                .socket_function(move |sock, events, _token| {
                    let mut flags = 0u32;
                    if events.input() {
                        flags |= libc::EPOLLIN.cast_unsigned();
                    }
                    if events.output() {
                        flags |= libc::EPOLLOUT.cast_unsigned();
                    }
                    let mut ev = libc::epoll_event {
                        events: flags,
                        u64: u64::try_from(sock).unwrap_or(0),
                    };
                    let Ok(mut known) = int.lock() else {
                        return;
                    };
                    // SAFETY: valid epoll fd and event; libcurl owns `sock`.
                    unsafe {
                        if events.remove() {
                            libc::epoll_ctl(ep, libc::EPOLL_CTL_DEL, sock, std::ptr::null_mut());
                            known.remove(&sock);
                        } else if known.insert(sock, flags).is_some() {
                            libc::epoll_ctl(ep, libc::EPOLL_CTL_MOD, sock, &raw mut ev);
                        } else {
                            libc::epoll_ctl(ep, libc::EPOLL_CTL_ADD, sock, &raw mut ev);
                        }
                    }
                })
                .map_err(|e| Error::Http(format!("libcurl: {e}")))?;
            Some(Watch {
                ep: ep_owned,
                timer,
            })
        } else {
            None
        };
        let mut c = Self {
            multi,
            free: Vec::new(),
            active: Vec::new(),
            base,
            resolve: target
                .addr
                .map(|a| format!("{}:{}:{}", target.host, target.port, a.ip())),
            fixed: target
                .headers
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect(),
            ca_file: cfg.curl_ca_file.clone(),
            timeout: Duration::from_millis(u64::from(cfg.request_timeout_ms)),
            pool: cfg.pool,
            allow_writes: cfg.curl_allow_non_idempotent_methods,
            tcp: TcpOpts::of(&cfg.driver),
            quickack: cfg.driver.quickack_check,
            checked: Arc::new(Mutex::new(Checked::new(cfg.pool))),
            watch,
        };
        if c.quickack != QuickAckCheck::Off {
            // One handle now, so a libcurl that cannot run the check is
            // reported here rather than at the first request.
            let first = c.easy()?;
            c.free.push(first);
        }
        Ok(c)
    }

    /// What the quick-ACK check found for the connection each handle last
    /// used.
    pub(crate) fn route_checks(&self) -> Vec<RouteCheck> {
        self.free
            .iter()
            .map(|e| RouteCheck::clone(&e.get_ref().route))
            .chain(
                self.active
                    .iter()
                    .map(|(_, h)| RouteCheck::clone(&h.get_ref().route)),
            )
            .collect()
    }

    pub(crate) const fn idle(&self) -> usize {
        self.pool.saturating_sub(self.active.len())
    }

    fn easy(&mut self) -> Result<Easy2<Collector>> {
        if let Some(e) = self.free.pop() {
            return Ok(e);
        }
        let map = |e: curl::Error| Error::Http(format!("libcurl: {e}"));
        let mut e = Easy2::new(Collector::default());
        e.get_mut().tcp = self.tcp;
        e.http_version(HttpVersion::V11).map_err(map)?;
        e.signal(false).map_err(map)?;
        e.tcp_nodelay(true).map_err(map)?;
        e.timeout(self.timeout).map_err(map)?;
        if let Some(r) = &self.resolve {
            let mut l = List::new();
            l.append(r).map_err(map)?;
            e.resolve(l).map_err(map)?;
        }
        if let Some(ca) = &self.ca_file {
            e.cainfo(ca).map_err(map)?;
        }
        if self.quickack != QuickAckCheck::Off {
            e.get_mut().quickack = self.quickack;
            e.get_mut().checked = Arc::clone(&self.checked);
            let data = std::ptr::from_mut(e.get_mut()).cast::<c_void>();
            let f: PrereqFn = prereq;
            // SAFETY: a valid handle and the argument types curl.h gives
            // these options; `data` stays valid for the handle's life (see
            // `prereq`).
            let rc = unsafe {
                match curl_easy_setopt(e.raw().cast(), CURLOPT_PREREQDATA, data) {
                    0 => curl_easy_setopt(e.raw().cast(), CURLOPT_PREREQFUNCTION, f),
                    rc => rc,
                }
            };
            if rc != 0 {
                // Before 7.80 (CURLE_UNKNOWN_OPTION): warn once, or refuse
                // under `require`.
                let why = "libcurl before 7.80 cannot report its connections";
                e.get_mut().route = Arc::new(
                    route::check_with(
                        self.quickack,
                        Err(RouteError::Unsupported(why)),
                        route::lookup,
                        &route::WARNED,
                        crate::warn::emit,
                    )
                    .map_err(|e| Error::QuickAck(Box::new(e)))?,
                );
            }
        }
        Ok(e)
    }

    pub(crate) fn request(
        &mut self,
        id: u64,
        method: &str,
        path: &str,
        query: Option<&str>,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<()> {
        // libcurl re-sends a request when its reused connection dies before
        // any reply byte (`Curl_retry_request`): only methods a second copy
        // cannot harm, unless the caller accepts that.
        if !self.allow_writes && !matches!(method, "GET" | "HEAD" | "OPTIONS" | "DELETE") {
            return Err(Error::CurlResend {
                method: method.to_owned(),
            });
        }
        if self.active.len() >= self.pool {
            return Err(Error::Busy("libcurl pool fully in use"));
        }
        let map = |e: curl::Error| Error::Http(format!("libcurl: {e}"));
        let mut e = self.easy()?;
        let mut url =
            String::with_capacity(self.base.len() + path.len() + 1 + query.map_or(0, str::len));
        url.push_str(&self.base);
        url.push_str(path);
        if let Some(q) = query {
            url.push('?');
            url.push_str(q);
        }
        e.url(&url).map_err(map)?;
        let mut list = List::new();
        for h in &self.fixed {
            list.append(h).map_err(map)?;
        }
        for (k, v) in headers {
            list.append(&format!("{k}: {v}")).map_err(map)?;
        }
        // Never wait for a 100-continue.
        list.append("Expect:").map_err(map)?;
        e.http_headers(list).map_err(map)?;
        // HEAD: libcurl must not wait for the announced body (NOBODY); reused
        // handles reset it for every other method.
        let head = method == "HEAD";
        e.nobody(head).map_err(map)?;
        if head {
        } else if !body.is_empty() || matches!(method, "POST" | "PUT" | "PATCH") {
            e.post(true).map_err(map)?;
            e.post_fields_copy(body).map_err(map)?;
        } else {
            e.get(true).map_err(map)?;
        }
        e.custom_request(method).map_err(map)?;
        e.get_mut().head.clear();
        e.get_mut().body.clear();
        e.get_mut().refused = None;
        let h = self
            .multi
            .add2(e)
            .map_err(|e| Error::Http(format!("libcurl: {e}")))?;
        self.active.push((id, h));
        Ok(())
    }

    pub(crate) fn poll<H: ClientHandler>(&mut self, handler: &mut H) -> Result<usize> {
        if self.active.is_empty() {
            return Ok(0);
        }
        let mmap = |e: curl::MultiError| Error::Http(format!("libcurl: {e}"));
        match &self.watch {
            None => {
                self.multi.perform().map_err(mmap)?;
            }
            Some(w) => {
                let mut evs = [libc::epoll_event { events: 0, u64: 0 }; 16];
                // SAFETY: valid epoll fd and event buffer.
                let n = unsafe { libc::epoll_wait(w.ep.as_raw_fd(), evs.as_mut_ptr(), 16, 0) };
                for ev in evs.iter().take(usize::try_from(n).unwrap_or(0)) {
                    let fd = RawFd::try_from(ev.u64).unwrap_or(-1);
                    let mut e = Events::new();
                    e.input(ev.events & libc::EPOLLIN.cast_unsigned() != 0);
                    e.output(ev.events & libc::EPOLLOUT.cast_unsigned() != 0);
                    self.multi.action(fd, &e).map_err(mmap)?;
                }
                if monotonic_ns() >= w.timer.load(Ordering::Relaxed) {
                    w.timer.store(i64::MAX, Ordering::Relaxed);
                    self.multi.timeout().map_err(mmap)?;
                }
            }
        }
        let mut finished: Vec<(usize, std::result::Result<(), curl::Error>)> = Vec::new();
        let active = &self.active;
        self.multi.messages(|m| {
            for (i, (_, h)) in active.iter().enumerate() {
                if let Some(r) = m.result_for2(h) {
                    finished.push((i, r));
                }
            }
        });
        if finished.is_empty() {
            return Ok(0);
        }
        // Remove from the back so indices stay valid.
        finished.sort_unstable_by_key(|f| std::cmp::Reverse(f.0));
        let done = finished.len();
        for (i, result) in finished {
            let (id, h) = self.active.swap_remove(i);
            let mut e = self.multi.remove2(h).map_err(mmap)?;
            match result {
                Ok(()) => deliver(id, &e, handler),
                Err(err) => {
                    // No request byte ever went to the connection (libcurl
                    // counts them over every re-send): the server cannot
                    // have seen it.
                    let sent = match e.request_size() {
                        Ok(0) => Sent::No,
                        _ => Sent::Maybe,
                    };
                    // A transfer the quick-ACK check aborted reports why; a
                    // timeout is reported as the native client reports it.
                    let error = e.get_mut().refused.take().map_or_else(
                        || {
                            if err.is_operation_timedout() {
                                Error::Timeout("http request")
                            } else {
                                Error::Http(format!("libcurl: {err}"))
                            }
                        },
                        |why| Error::QuickAck(Box::new(why)),
                    );
                    handler.on_error(id, sent, error);
                }
            }
            self.free.push(e);
        }
        Ok(done)
    }
}

/// The response, or an error when its header block cannot be read (more
/// than 64 headers, say): never an empty header list, which would hide
/// `Retry-After` and the usage headers. The request ran, so `Sent::Maybe`.
fn deliver<H: ClientHandler>(id: u64, e: &Easy2<Collector>, handler: &mut H) {
    let c = e.get_ref();
    let mut hdrs = [httparse::EMPTY_HEADER; 64];
    let mut resp = httparse::Response::new(&mut hdrs);
    let parsed = match resp.parse(&c.head) {
        Ok(httparse::Status::Complete(_)) => resp.code.ok_or_else(|| "no status code".to_owned()),
        Ok(httparse::Status::Partial) => Err("incomplete header block".to_owned()),
        Err(err) => Err(err.to_string()),
    };
    match parsed {
        Ok(status) => handler.on_response(
            id,
            &Response {
                status,
                headers: resp.headers,
                body: &c.body,
            },
        ),
        Err(why) => handler.on_error(
            id,
            Sent::Maybe,
            Error::Http(format!("libcurl response: {why}")),
        ),
    }
}

impl Drop for CurlClient {
    fn drop(&mut self) {
        // Easy handles must leave the multi before it is cleaned up.
        for (_, h) in self.active.drain(..) {
            let _ = self.multi.remove2(h);
        }
        // The epoll descriptor closes with `watch`, after `multi`.
    }
}
