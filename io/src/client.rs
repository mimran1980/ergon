//! A pooled HTTP client; the implementation is configuration.
//!
//! | `HttpImpl` | What | Use it for |
//! |---|---|---|
//! | [`HttpImpl::Native`] (default) | ergon-io's HTTP/1.1 client: a pool of warm connections on a [`Driver`] | latency: on the same interrupt-driven path (`epoll-spin`), 5–12 µs faster per round trip than the source-built libcurl at the median, 1 connection; the p99 and max were lower on Azure, not on AWS (`io/benches/BENCHMARK.md`, "HTTP clients"; each pooled connection is one more socket the loop polls) |
//! | [`HttpImpl::Curl`] (feature `curl`) | libcurl, linked in-process: one multi handle, reused easy handles | endpoints that need redirects, proxies, HTTP/2, other auth schemes or compression |
//!
//! - One API for both: [`HttpClient::request`] and
//!   [`HttpClient::signed_request`] return a request id at once;
//!   [`HttpClient::poll`] (one non-blocking pass) reports each result to
//!   [`ClientHandler`]. Switching costs one predictable branch per call.
//! - Native: each request goes to a connection with nothing in flight; none
//!   free gives [`Error::Busy`] (no queue). A dropped connection is replaced
//!   in the background after a random delay of up to 100 ms (connections
//!   that drop together do not reconnect together), with backoff while
//!   connects fail. [`HttpClient::poll_gated`] puts every connect through
//!   [`ClientHandler::connect_gate`] (a venue's connect limit).
//! - Curl: never blocks. Driven by `curl_multi_perform` on every poll, or by
//!   `curl_multi_socket_action` on libcurl's own sockets ([`CurlDrive`]).
//!   Takes GET, HEAD, OPTIONS and DELETE only, unless
//!   [`HttpClientConfig::curl_allow_non_idempotent_methods`]: libcurl sends
//!   a request again when its reused connection dies before the reply.
//! - A failed request says whether the server can have seen it
//!   ([`ClientHandler::on_error`], [`Sent`]).

use std::str::FromStr;
use std::sync::Arc;

use crate::RxMeta;
use crate::auth::Signer;
use crate::backend::{Backend, Config};
use crate::clock::monotonic_ns;
use crate::driver::Driver;
use crate::endpoint::{ConnId, Handler};
use crate::error::{Error, Result};
use crate::http::{HttpConn, HttpDial, HttpHandler, HttpOptions, Response};
use crate::net::Target;
use crate::outcome::Sent;

/// Which HTTP implementation an [`HttpClient`] uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpImpl {
    /// ergon-io's HTTP/1.1 client on the [`Driver`].
    Native,
    /// libcurl (needs the `curl` feature).
    Curl,
}

impl FromStr for HttpImpl {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "native" => Ok(Self::Native),
            "curl" => Ok(Self::Curl),
            _ => Err(Error::Config(format!("unknown HTTP implementation {s:?}"))),
        }
    }
}

/// How the libcurl implementation is driven (both never block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurlDrive {
    /// `curl_multi_perform` on every poll.
    Perform,
    /// `curl_multi_socket_action` on the sockets libcurl reports ready, from a
    /// zero-timeout `epoll_wait` on libcurl's own sockets.
    SocketAction,
}

/// [`HttpClient`] settings.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// The implementation.
    pub implementation: HttpImpl,
    /// Native: the loop backend for the pool's driver. Both implementations
    /// take TCP keepalive, `TCP_USER_TIMEOUT` and the quick-ACK route check's
    /// policy from it (`driver.quickack_check`, see [`crate::route`]):
    /// libcurl's connections are checked from its pre-request callback
    /// (libcurl 7.80 or later).
    pub driver: Config,
    /// Connections kept warm (native) or the most libcurl may use at once.
    pub pool: usize,
    /// Time from sending a request to its complete response, in milliseconds.
    pub request_timeout_ms: u32,
    /// Native: receive buffer per connection (must hold the largest response).
    pub buf_size: usize,
    /// Native: open connections on the driver's helper thread
    /// (`Driver::connect_background`), keeping handshake CPU off the loop.
    /// In a process that may use the polling CPU only (a pod with one CPU,
    /// [`crate::placement`]) [`HttpClient::new`] then fails with
    /// [`Error::Cpu`], unless `driver.inline_connect_fallback` is set: the
    /// pool then opens connections on the loop, as with `false`.
    pub background_connect: bool,
    /// Curl: how libcurl is driven.
    pub curl_drive: CurlDrive,
    /// Curl: a CA bundle file instead of the system store. (Native trust comes
    /// from the rustls configuration passed to [`HttpClient::new`].)
    pub curl_ca_file: Option<String>,
    /// Curl: accept POST, PUT, PATCH and every method other than GET, HEAD,
    /// OPTIONS and DELETE. Off by default: libcurl sends a request again, up
    /// to 5 times, when the reused connection it went out on dies before any
    /// reply byte, so an order could run twice. Off, such a request fails
    /// with [`Error::CurlResend`]. Send orders through the native client,
    /// which never re-sends.
    pub curl_allow_non_idempotent_methods: bool,
    /// Native: every pool connect waits for [`ClientHandler::connect_gate`],
    /// the first ones included, so `new` opens nothing and
    /// [`HttpClient::poll`] refuses ([`Error::Config`]). For a venue that
    /// limits connects per IP. Give the pool a gate of its own: venues'
    /// connect limits count WebSocket connects, not HTTP ones. Off
    /// (default): `new` opens every connection at once.
    pub connect_gate: bool,
    /// Native: the seed of the reconnect delays' jitter; `0` picks one at
    /// random. A fixed seed gives the same delays every run (tests).
    pub reconnect_seed: u64,
}

impl HttpClientConfig {
    /// Defaults: native, `recv-busy`, a pool of 2, background connects.
    #[must_use]
    pub const fn new(implementation: HttpImpl) -> Self {
        Self {
            implementation,
            driver: Config::new(Backend::RecvBusy),
            pool: 2,
            request_timeout_ms: 5_000,
            buf_size: 1 << 20,
            background_connect: true,
            curl_drive: CurlDrive::Perform,
            curl_ca_file: None,
            curl_allow_non_idempotent_methods: false,
            connect_gate: false,
            reconnect_seed: 0,
        }
    }
}

/// Receives the outcome of every request.
pub trait ClientHandler {
    /// The complete response to `request`; it borrows the client's buffers for
    /// the duration of the call.
    fn on_response(&mut self, request: u64, response: &Response<'_>);
    /// `request` failed with no response (connection error, timeout,
    /// libcurl error, an unreadable response). Every request ends in exactly
    /// one of `on_response` or `on_error`.
    ///
    /// `sent`: [`Sent::No`] when the request cannot have reached the server
    /// (safe to send again); [`Sent::Maybe`] when it may have run (reconcile
    /// an order by its client order id, never send it again blindly). A
    /// timeout is [`Error::Timeout`] (`"http request"`) on both
    /// implementations.
    fn on_error(&mut self, request: u64, sent: Sent, error: Error) {
        let _ = (request, sent, error);
    }
    /// A pooled connection could not be opened (native implementation). The
    /// pool keeps retrying, each time after a random delay below a limit
    /// that starts at 100 ms and doubles per failure in a row, up to 5 s;
    /// `poll` itself does not fail, and [`HttpClient::connect_failures`]
    /// counts them.
    ///
    /// The default prints a warning to stderr, at most one every 10 s across
    /// the process (with the number held back). A thread started with the
    /// client, on the housekeeping CPUs, makes the write, so the polling
    /// thread only formats the line and queues it. Implement it to route
    /// failures to your own logging or alerting.
    fn on_connect_error(&mut self, error: Error) {
        crate::warn::connect_failure("HttpClient could not open a connection", &error);
    }
    /// May a pool connection open now? `Ok(())` to open it, or
    /// `Err(wait_ns)`: that connection asks again after the wait, at most
    /// 5 s later. A refusal is not a failed connect: the backoff does not
    /// grow. Called only for a connection due to open: never on a pass with
    /// nothing to open.
    ///
    /// Lives here, not in a closure passed to [`HttpClient::poll_gated`],
    /// so the gate can share this handler's state (the application's
    /// pacing, which [`on_response`](ClientHandler::on_response) also
    /// updates). ergon-io has no rate limiter: the pacing is the
    /// application's.
    ///
    /// The default passes every connect. Implement it for a venue's connect
    /// limit, with a gate of the pool's own: venues count WebSocket
    /// connects, not HTTP ones.
    ///
    /// # Errors
    /// The nanoseconds to wait before that connection asks again, when the
    /// gate refuses it now.
    fn connect_gate(&mut self) -> std::result::Result<(), u64> {
        Ok(())
    }
}

enum Imp {
    Native(Box<Native>),
    #[cfg(feature = "curl")]
    Curl(Box<crate::curl_client::CurlClient>),
}

/// A pooled HTTP client; see the module documentation.
pub struct HttpClient {
    imp: Imp,
    next_id: u64,
}

impl HttpClient {
    /// A client for `target` (its headers, such as the API key, go with every
    /// request). Native connections start opening at once (with
    /// [`HttpClientConfig::connect_gate`], from the first `poll_gated`); a
    /// connection that cannot be opened is reported to
    /// [`ClientHandler::on_connect_error`] from `poll` and retried.
    ///
    /// # Errors
    /// The implementation is not built in, the configuration is invalid
    /// (headers, TLS config, `connect_gate` with libcurl), or the driver or
    /// libcurl fails to initialise. [`Error::Cpu`]: background connects
    /// with no CPU for the connect helper (see
    /// [`HttpClientConfig::background_connect`]).
    pub fn new(
        cfg: &HttpClientConfig,
        target: Target,
        tls: Arc<rustls::ClientConfig>,
    ) -> Result<Self> {
        if cfg.pool == 0 {
            return Err(Error::Config("pool must be at least 1".into()));
        }
        let imp = match cfg.implementation {
            HttpImpl::Native => {
                // The default `on_connect_error` warning's writer starts in
                // `Driver::new`, on the CPUs it chose for helper threads,
                // rather than on the polling thread at the first failure.
                Imp::Native(Box::new(Native::new(cfg, target, tls)?))
            }
            #[cfg(feature = "curl")]
            HttpImpl::Curl => {
                let _ = tls;
                if cfg.connect_gate {
                    return Err(Error::Config(
                        "connect_gate: libcurl opens its own connections; use the native client"
                            .into(),
                    ));
                }
                if cfg.driver.quickack_check == crate::route::QuickAckCheck::Warn {
                    crate::warn::prepare();
                }
                Imp::Curl(Box::new(crate::curl_client::CurlClient::new(cfg, &target)?))
            }
            #[cfg(not(feature = "curl"))]
            HttpImpl::Curl => {
                return Err(Error::Unsupported(
                    "the libcurl client needs the `curl` feature",
                ));
            }
        };
        Ok(Self { imp, next_id: 0 })
    }

    /// The implementation in use.
    #[must_use]
    pub const fn implementation(&self) -> HttpImpl {
        match self.imp {
            Imp::Native(_) => HttpImpl::Native,
            #[cfg(feature = "curl")]
            Imp::Curl(_) => HttpImpl::Curl,
        }
    }

    /// Connections the native pool failed to open since it was created (each
    /// also went to [`ClientHandler::on_connect_error`]); 0 for libcurl, which
    /// reports connect failures as request errors.
    #[must_use]
    pub fn connect_failures(&self) -> u64 {
        match &self.imp {
            Imp::Native(n) => n.connect_failures,
            #[cfg(feature = "curl")]
            Imp::Curl(_) => 0,
        }
    }

    /// What the quick-ACK route check (`driver.quickack_check`, see
    /// [`crate::route`]) found for each connection: the pool's open
    /// connections (native), or the connection each libcurl handle last
    /// used. Allocates: for diagnostics, not the request path.
    #[must_use]
    pub fn route_checks(&self) -> Vec<crate::route::RouteCheck> {
        match &self.imp {
            Imp::Native(n) => n
                .slots
                .iter()
                .filter(|s| s.open)
                .filter_map(|s| s.id.and_then(|id| n.driver.get(id)))
                .map(|c| c.route().clone())
                .collect(),
            #[cfg(feature = "curl")]
            Imp::Curl(c) => c.route_checks(),
        }
    }

    /// Requests that can be sent now without [`Error::Busy`]: idle open
    /// connections (native) or free handles (libcurl).
    #[must_use]
    pub fn idle(&self) -> usize {
        match &self.imp {
            Imp::Native(n) => n.idle(),
            #[cfg(feature = "curl")]
            Imp::Curl(c) => c.idle(),
        }
    }

    /// Sends a request (`path` may carry a query). Returns its id.
    ///
    /// # Errors
    /// [`Error::Busy`] when no connection or handle is free, or an invalid
    /// request.
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<u64> {
        let id = self.next_id;
        match &mut self.imp {
            Imp::Native(n) => n.send(id, |c| c.request(method, path, headers, body))?,
            #[cfg(feature = "curl")]
            Imp::Curl(c) => c.request(id, method, path, None, headers, body)?,
        }
        self.next_id += 1;
        Ok(id)
    }

    /// Sends a signed request: `params`, `timestamp` (this host's UTC, ms)
    /// and `signature` in the query string, as [`HttpConn::signed_request`].
    /// Returns its id.
    ///
    /// # Errors
    /// As [`HttpClient::request`], or `params` needs percent-encoding.
    pub fn signed_request(
        &mut self,
        method: &str,
        path: &str,
        params: &str,
        headers: &[(&str, &str)],
        signer: &Signer,
    ) -> Result<u64> {
        let now_ms = u64::try_from(crate::clock::realtime_ns() / 1_000_000).unwrap_or(0);
        self.signed_request_at(method, path, params, headers, signer, now_ms)
    }

    /// [`HttpClient::signed_request`] with the caller's timestamp, in ms
    /// since 1970: a clock of the caller's choice, or replay time.
    ///
    /// # Errors
    /// As [`HttpClient::signed_request`], or `path` has a query (put it in
    /// `params`).
    pub fn signed_request_at(
        &mut self,
        method: &str,
        path: &str,
        params: &str,
        headers: &[(&str, &str)],
        signer: &Signer,
        timestamp_ms: u64,
    ) -> Result<u64> {
        let id = self.next_id;
        match &mut self.imp {
            Imp::Native(n) => {
                n.send(id, |c| {
                    c.signed_request_at(method, path, params, headers, signer, timestamp_ms)
                })?;
            }
            #[cfg(feature = "curl")]
            Imp::Curl(c) => {
                crate::auth::validate_params(params)?;
                if path.contains('?') {
                    return Err(Error::Http("put query parameters in `params`".into()));
                }
                let mut q = Vec::with_capacity(params.len() + 128);
                crate::auth::append_signed_params(&mut q, params, timestamp_ms, signer);
                let q = String::from_utf8(q).map_err(|_| Error::Http("non-UTF-8 query".into()))?;
                c.request(id, method, path, Some(&q), headers, &[])?;
            }
        }
        self.next_id += 1;
        Ok(id)
    }

    /// One non-blocking iteration: moves bytes, reports finished requests, and
    /// replaces dropped connections. Returns requests finished.
    ///
    /// # Errors
    /// The driver or libcurl failed as a whole (per-request failures go to
    /// [`ClientHandler::on_error`]). [`Error::Config`] for a pool with
    /// [`HttpClientConfig::connect_gate`]: use [`HttpClient::poll_gated`].
    pub fn poll<H: ClientHandler>(&mut self, handler: &mut H) -> Result<usize> {
        match &mut self.imp {
            Imp::Native(n) => {
                if n.gated {
                    return Err(Error::Config(
                        "connect_gate is set: poll with poll_gated".into(),
                    ));
                }
                n.poll(handler, false)
            }
            #[cfg(feature = "curl")]
            Imp::Curl(c) => c.poll(handler),
        }
    }

    /// [`HttpClient::poll`], but a pool connect starts only when
    /// [`ClientHandler::connect_gate`] passes it: a venue's limit on
    /// connects per IP. The gate is the handler's, so it can share the
    /// handler's state (see [`ClientHandler::connect_gate`]).
    ///
    /// # Errors
    /// As [`HttpClient::poll`]; [`Error::Unsupported`] for libcurl, which
    /// opens its own connections.
    pub fn poll_gated<H: ClientHandler>(&mut self, handler: &mut H) -> Result<usize> {
        match &mut self.imp {
            Imp::Native(n) => n.poll(handler, true),
            #[cfg(feature = "curl")]
            Imp::Curl(_) => Err(Error::Unsupported(
                "the libcurl client opens its own connections: no connect gate",
            )),
        }
    }
}

struct Slot {
    id: Option<ConnId>,
    open: bool,
    /// The client's request id in flight, and the connection's own.
    request: Option<(u64, u64)>,
    /// No connect before this (`CLOCK_MONOTONIC`).
    retry_at_ns: i64,
    /// Failed connects in a row.
    failures: u32,
    /// Closed during the last driver pass: its reconnect is not scheduled
    /// yet.
    closed: bool,
}

/// The first reconnect's delay is random, below this; the limit doubles per
/// failed connect in a row.
const REOPEN_NS: f64 = 100e6;
/// The longest reconnect delay, and the longest wait between asking the
/// gate.
const REOPEN_MAX_NS: i64 = 5_000_000_000;

struct Native {
    driver: Driver<HttpConn>,
    target: Target,
    tls: Arc<rustls::ClientConfig>,
    opts: HttpOptions,
    background: bool,
    slots: Vec<Slot>,
    next: usize,
    connect_failures: u64,
    /// Every connect goes through the caller's gate.
    gated: bool,
    /// The reconnect jitter's state.
    rng: u64,
}

impl Native {
    fn new(cfg: &HttpClientConfig, target: Target, tls: Arc<rustls::ClientConfig>) -> Result<Self> {
        let mut n = Self {
            driver: Driver::new(cfg.driver.clone())?,
            target,
            tls,
            opts: HttpOptions {
                request_timeout_ms: cfg.request_timeout_ms,
                max_in_flight: 1,
                buf_size: cfg.buf_size,
                ..HttpOptions::default()
            },
            background: cfg.background_connect,
            slots: (0..cfg.pool)
                .map(|_| Slot {
                    id: None,
                    open: false,
                    request: None,
                    retry_at_ns: i64::MIN,
                    failures: 0,
                    closed: false,
                })
                .collect(),
            next: 0,
            connect_failures: 0,
            gated: cfg.connect_gate,
            rng: crate::reconnect::rng_state(cfg.reconnect_seed)?,
        };
        // Configuration errors (headers, TLS config) fail here; connect
        // failures go to the handler from `poll`, which retries them.
        HttpDial::new(n.target.clone(), n.tls.clone(), n.opts)?;
        // No CPU for the driver's connect helper (a pod with one CPU, without
        // `inline_connect_fallback`): fail now, as every open would.
        if n.background {
            n.driver.background_ready()?;
        }
        // With a gate, the first opens wait for it too.
        if !n.gated {
            for i in 0..n.slots.len() {
                let _ = n.open(i);
            }
        }
        Ok(n)
    }

    fn open(&mut self, i: usize) -> Result<()> {
        let dial = HttpDial::new(self.target.clone(), self.tls.clone(), self.opts)?;
        let id = if self.background {
            self.driver.connect_background(dial)?
        } else {
            self.driver.connect(dial)?
        };
        self.slots[i].id = Some(id);
        Ok(())
    }

    fn idle(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.open && s.request.is_none())
            .count()
    }

    fn send(&mut self, id: u64, f: impl FnOnce(&mut HttpConn) -> Result<u64>) -> Result<()> {
        let n = self.slots.len();
        let Some(i) = (0..n)
            .map(|k| (self.next + k) % n)
            .find(|&i| self.slots[i].open && self.slots[i].request.is_none())
        else {
            return Err(Error::Busy("no idle connection in the pool"));
        };
        let mut conn = self.slots[i]
            .id
            .and_then(|c| self.driver.get_mut(c))
            .ok_or(Error::Closed)?;
        let on_conn = f(conn.inner())?;
        self.slots[i].request = Some((id, on_conn));
        self.next = (i + 1) % n;
        Ok(())
    }

    fn poll<H: ClientHandler>(&mut self, handler: &mut H, gated: bool) -> Result<usize> {
        let mut a = Adapter {
            h: handler,
            slots: &mut self.slots,
            done: 0,
            connect_failures: 0,
        };
        self.driver.poll(&mut a)?;
        let done = a.done;
        self.connect_failures += a.connect_failures;
        if self.slots.iter().any(|s| s.id.is_none()) {
            self.reopen(handler, gated);
        }
        Ok(done)
    }

    /// Replaces dropped connections: each after a random delay, the limit
    /// doubling per failed connect in a row, and each through
    /// [`ClientHandler::connect_gate`] when `gated`.
    #[cold]
    fn reopen<H: ClientHandler>(&mut self, handler: &mut H, gated: bool) {
        let now = monotonic_ns();
        let n = self.slots.len();
        for i in 0..n {
            if std::mem::take(&mut self.slots[i].closed) {
                let delay = self.reopen_delay_ns(self.slots[i].failures);
                self.slots[i].retry_at_ns = now.saturating_add(delay);
            }
        }
        for i in 0..n {
            if self.slots[i].id.is_some() || now < self.slots[i].retry_at_ns {
                continue;
            }
            let gate = if gated {
                handler.connect_gate()
            } else {
                Ok(())
            };
            if let Err(wait_ns) = gate {
                let wait = i64::try_from(wait_ns)
                    .unwrap_or(i64::MAX)
                    .min(REOPEN_MAX_NS);
                self.slots[i].retry_at_ns = now.saturating_add(wait);
                continue;
            }
            // A refused or unroutable connect fails here at once: report it
            // and retry later, as for one that fails while connecting.
            if let Err(e) = self.open(i) {
                self.connect_failures += 1;
                let failures = self.slots[i].failures.saturating_add(1);
                self.slots[i].failures = failures;
                let delay = self.reopen_delay_ns(failures);
                self.slots[i].retry_at_ns = now.saturating_add(delay);
                handler.on_connect_error(e);
            }
        }
    }

    /// A random delay below 100 ms × 2^`failures`, at most 5 s ("full
    /// jitter": connections that drop together spread out).
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn reopen_delay_ns(&mut self, failures: u32) -> i64 {
        let limit = (REOPEN_NS * f64::from(1u32 << failures.min(6))).min(REOPEN_MAX_NS as f64);
        (crate::reconnect::unit(&mut self.rng) * limit) as i64
    }
}

/// Maps driver callbacks onto pool slots and client request ids.
struct Adapter<'a, H> {
    h: &'a mut H,
    slots: &'a mut [Slot],
    done: usize,
    connect_failures: u64,
}

impl<H> Adapter<'_, H> {
    fn slot(&mut self, id: ConnId) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|s| s.id == Some(id))
    }
}

impl<H: ClientHandler> Handler for Adapter<'_, H> {
    fn on_open(&mut self, id: ConnId) {
        if let Some(s) = self.slot(id) {
            s.open = true;
            s.failures = 0;
        }
    }

    fn on_closed(&mut self, id: ConnId, error: Error) {
        let Some(s) = self.slot(id) else {
            return;
        };
        s.id = None;
        s.closed = true;
        let was_open = std::mem::replace(&mut s.open, false);
        if !was_open {
            s.failures = s.failures.saturating_add(1);
        }
        if let Some((r, on_conn)) = s.request.take() {
            self.done += 1;
            // The connection names the first request the kernel never
            // wholly took; anything else may have reached the server.
            let (sent, error) = match error {
                Error::Unanswered(u) => {
                    let sent = match u.first_unsent {
                        Some(first) if first <= on_conn => Sent::No,
                        _ => Sent::Maybe,
                    };
                    (sent, u.cause)
                }
                e => (Sent::Maybe, e),
            };
            self.h.on_error(r, sent, error);
        } else if !was_open {
            // It never opened: a failed connect.
            self.connect_failures += 1;
            self.h.on_connect_error(error);
        }
    }
}

impl<H: ClientHandler> HttpHandler for Adapter<'_, H> {
    fn on_response(&mut self, conn: ConnId, _request: u64, response: &Response<'_>, _: &RxMeta) {
        if let Some((r, _)) = self.slot(conn).and_then(|s| s.request.take()) {
            self.done += 1;
            self.h.on_response(r, response);
        }
    }
}
