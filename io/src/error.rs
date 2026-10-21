//! Error type.

use std::fmt;

/// An I/O, protocol or configuration error.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Operating-system I/O failure.
    Io(std::io::Error),
    /// TLS failure reported by rustls.
    Tls(rustls::Error),
    /// Record-layer failure (bad record, AEAD failure, unexpected content).
    Record(&'static str),
    /// WebSocket protocol violation.
    Ws(&'static str),
    /// HTTP failure (a malformed response or upgrade).
    Http(String),
    /// The server answered the WebSocket upgrade with `status`, not 101.
    Upgrade {
        /// HTTP status code.
        status: u16,
        /// The start of the response body (at most 300 bytes).
        body: String,
        /// The server's `Retry-After`, in ms (see
        /// [`crate::outcome::retry_after_ms`]); `None` without one. Wait at
        /// least this long before connecting again (a 429 or 418 refusal).
        retry_after_ms: Option<u64>,
        /// The response had a `Retry-After` that is neither seconds nor an
        /// HTTP date (`retry_after_ms` is then `None`).
        retry_after_invalid: bool,
    },
    /// An HTTP connection closed with requests unanswered. `HttpConn` users
    /// get it in `Handler::on_closed`; `HttpClient` reports each request's
    /// [`crate::outcome::Sent`] to `ClientHandler::on_error` instead. Boxed:
    /// rare.
    Unanswered(Box<Unanswered>),
    /// The libcurl `HttpClient` refused `method`: libcurl sends a request
    /// again, up to 5 times, when the reused connection it went out on dies
    /// before any reply byte, so an order could run twice. Without
    /// `HttpClientConfig::curl_allow_non_idempotent_methods` it takes only
    /// GET, HEAD, OPTIONS and DELETE. The native client never re-sends.
    CurlResend {
        /// The refused method.
        method: String,
    },
    /// The peer closed the connection without a WebSocket close handshake.
    Closed,
    /// A WebSocket close handshake ended the connection: the code and
    /// reason, and who started it. Boxed: rare, and it keeps `Result` small.
    WsClose(Box<crate::ws::Close>),
    /// Bad configuration input.
    Config(String),
    /// The backend is not available on this platform or build.
    Unsupported(&'static str),
    /// Name resolution failed.
    Dns(String),
    /// An operation did not finish in time.
    Timeout(&'static str),
    /// A limit was reached; retry later.
    Busy(&'static str),
    /// The quick-ACK route check refused the connection
    /// ([`crate::route::QuickAckCheck::Require`]). Boxed: it is rare, and
    /// every `Result` of this crate carries the size of the largest variant.
    QuickAck(Box<crate::route::QuickAckError>),
    /// A helper thread has no CPU it may use ([`crate::placement`]): the
    /// process may use the polling CPU only (a pod with one CPU), or a CPU
    /// setting names one outside its cpuset. Boxed: rare.
    Cpu(Box<crate::placement::CpuError>),
    /// The host-check switch (`ERGON_HOST_CHECKS`): it is set to something
    /// other than `off`, `on` or `require`, or it is `require` and a host
    /// check warned ([`crate::checks::HostReport::refusal`]). Boxed: rare.
    HostCheck(Box<ergon_tune::HostCheckError>),
}

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// [`Error::Unanswered`]: why an HTTP connection closed with requests in
/// flight, and which of them never reached the kernel.
#[derive(Debug)]
pub struct Unanswered {
    /// Why the connection closed ([`Error::Closed`], [`Error::Timeout`],
    /// [`Error::Io`], ...).
    pub cause: Error,
    /// The first unanswered request (the id `HttpConn::request` returned)
    /// whose TLS record was not wholly handed to the kernel. It and every
    /// later request cannot have reached the server: the server cannot
    /// decrypt part of a record. `None`: every unanswered request was
    /// handed, so each may have run.
    pub first_unsent: Option<u64>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Tls(e) => write!(f, "tls: {e}"),
            Self::Record(e) => write!(f, "tls record: {e}"),
            Self::Ws(e) => write!(f, "websocket: {e}"),
            Self::Http(e) => write!(f, "http: {e}"),
            Self::Upgrade {
                status,
                body,
                retry_after_ms,
                retry_after_invalid,
            } => {
                write!(f, "websocket upgrade refused: status {status}")?;
                if let Some(ms) = retry_after_ms {
                    write!(f, ", Retry-After {ms} ms")?;
                }
                if *retry_after_invalid {
                    write!(f, ", Retry-After unreadable")?;
                }
                write!(f, ": {body}")
            }
            Self::Unanswered(u) => match u.first_unsent {
                // The cause first: callers match on its text.
                Some(id) => write!(
                    f,
                    "{} (requests unanswered; request {id} and later never reached the kernel)",
                    u.cause
                ),
                None => write!(
                    f,
                    "{} (requests unanswered, all handed to the kernel)",
                    u.cause
                ),
            },
            Self::CurlResend { method } => write!(
                f,
                "libcurl client refused {method}: libcurl may send it again on a reused \
                 connection that dies before the reply; use the native client or set \
                 curl_allow_non_idempotent_methods"
            ),
            Self::Closed => write!(f, "peer closed the connection"),
            Self::WsClose(c) => write!(f, "websocket {c}"),
            Self::Config(e) => write!(f, "config: {e}"),
            Self::Unsupported(e) => write!(f, "unsupported: {e}"),
            Self::Dns(e) => write!(f, "dns: {e}"),
            Self::Timeout(e) => write!(f, "timed out: {e}"),
            Self::Busy(e) => write!(f, "busy: {e}"),
            Self::QuickAck(e) => write!(f, "quickack: {e}"),
            Self::Cpu(e) => write!(f, "cpu: {e}"),
            Self::HostCheck(e) => write!(f, "host check: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Tls(e) => Some(e),
            Self::QuickAck(e) => Some(e.as_ref()),
            Self::Cpu(e) => Some(e.as_ref()),
            Self::HostCheck(e) => Some(e.as_ref()),
            Self::Unanswered(u) => Some(&u.cause),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<crate::placement::CpuError> for Error {
    fn from(e: crate::placement::CpuError) -> Self {
        Self::Cpu(Box::new(e))
    }
}

impl From<ergon_tune::HostCheckError> for Error {
    fn from(e: ergon_tune::HostCheckError) -> Self {
        Self::HostCheck(Box::new(e))
    }
}

impl From<rustls::Error> for Error {
    fn from(e: rustls::Error) -> Self {
        Self::Tls(e)
    }
}
