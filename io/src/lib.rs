#![doc = include_str!("../README.md")]
//!
//! ## Modules
//!
//! - [`backend`]: [`Backend`] and [`Config`] ([`Config::parse`] takes
//!   `backend[,key=value...]`).
//! - [`ws_conn`]: [`WsConn`], [`WsDial`], [`WsOptions`], [`WsHandler`];
//!   [`ws`] is the frame codec.
//! - [`reconnect`]: [`Reconnect`] and its backoff policy, [`Retry`].
//! - [`http`]: [`HttpConn`], [`HttpDial`], [`HttpHandler`]. On Linux,
//!   `client` pools connections as `HttpClient`.
//! - [`text`]: the text check on WebSocket text and HTTP bodies
//!   ([`TextCheck`], [`TextStatus`]) and [`AsciiText`], a byte view of text
//!   trusted to be ASCII.
//! - [`tls`]: the TLS 1.3 client configuration ([`tls::client_config`]).
//! - [`auth`]: request signing ([`Signer`]).
//! - [`endpoint`]: [`Handler`] (`on_open`, `on_closed`), [`ConnId`], and
//!   what a `Driver` takes ([`Endpoint`], [`Dial`]).
//! - [`checks`] and [`route`]: the host, NIC and quick-ACK route checks,
//!   built on ergon-tune.
//! - [`placement`]: where the helper threads run (the connect helper, the
//!   warning thread, `SQPOLL`), inside the process's cpuset.
//! - [`limit`]: the send budget for venue rate limits ([`Budget`]).
//! - [`outcome`]: what became of a request ([`Sent`], [`Verdict`]) and
//!   `Retry-After` parsing.
//! - [`net`] ([`Target`]), [`dns`], [`any`] ([`AnyConn`]:
//!   WebSocket and HTTP on one driver), [`clock`], [`error`].
//!
//! `Driver`, `HttpClient`, `sys` and `warnings` are Linux-only, so they are
//! named here without links.

pub mod any;
pub mod auth;
pub mod backend;
pub mod checks;
pub mod clock;
pub mod dns;
pub mod endpoint;
pub mod error;
pub mod http;
pub mod limit;
pub mod net;
pub mod outcome;
pub mod placement;
pub mod reconnect;
pub mod route;
mod stream;
pub mod text;
pub mod tls;
pub mod ws;
pub mod ws_conn;

#[cfg(target_os = "linux")]
pub mod client;
#[cfg(all(target_os = "linux", feature = "curl"))]
mod curl_client;
#[cfg(target_os = "linux")]
mod driver;
#[cfg(target_os = "linux")]
pub mod sys;
#[cfg(target_os = "linux")]
mod warn;

/// ergon-io's warning thread (see [`checks`]): waiting for it to catch up.
#[cfg(target_os = "linux")]
pub mod warnings {
    pub use crate::warn::flush;
    /// Hidden test levers: capture and reset the lines the thread wrote
    /// (`io/tests/driver_checks.rs`).
    #[doc(hidden)]
    pub use crate::warn::{forget_seen, keep_lines, take_lines};
}

pub use any::{AnyConn, AnyDial};
pub use auth::{SignatureKind, Signer};
pub use backend::{Backend, Config};
#[cfg(target_os = "linux")]
pub use client::{ClientHandler, CurlDrive, HttpClient, HttpClientConfig, HttpImpl};
#[cfg(target_os = "linux")]
pub use driver::Driver;
pub use endpoint::{ConnId, ConnMut, Dial, DriverStats, Endpoint, Handler, Receive, TcpInfo};
pub use error::{Error, Result};
pub use http::{
    HttpConn, HttpDial, HttpHandler, HttpOptions, HttpStats, NonAsciiHead, RequestTemplate,
    Response,
};
pub use limit::{
    BucketId, BucketSpec, BucketStats, Budget, Charge, Class, Limited, Resend, Ticket, Why,
};
pub use net::Target;
pub use outcome::{Now, Sent, Verdict};
pub use reconnect::{Reconnect, Retry};
pub use text::{AsciiText, NonAscii, TextCheck, TextStatus};
pub use ws_conn::{
    Flow, FragmentIter, Fragments, Interest, MAX_FRAGMENTS, WsConn, WsDial, WsHandler, WsOptions,
    WsStats,
};

/// Receive metadata for one read, and for each WebSocket message or HTTP
/// body what its text check found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RxMeta {
    /// Kernel software receive timestamp (`CLOCK_REALTIME` ns), 0 if absent.
    pub rx_sw_ns: i64,
    /// NIC hardware timestamp (raw PHC ns), 0 if absent.
    pub rx_hw_ns: i64,
    /// Sequence number of this read within its driver.
    pub read_seq: u64,
    /// What the text check ([`text`]) found for the payload being
    /// delivered: a WebSocket message under [`WsOptions::text_check`], an
    /// HTTP body under [`HttpOptions::body_check`]
    /// ([`TextStatus::Ascii`]: every byte is one character;
    /// [`TextStatus::AssumedAscii`]: trusted, never read).
    /// [`TextStatus::Unchecked`] everywhere else.
    pub text: TextStatus,
}
