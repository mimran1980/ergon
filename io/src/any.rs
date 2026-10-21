//! One endpoint type for both protocols, so WebSocket and HTTP connections
//! share one driver (one `poll` per loop iteration).

use std::net::SocketAddr;

use crate::RxMeta;
use crate::endpoint::sealed::{Deliver, DialIo, Io};
use crate::endpoint::{ConnId, ConnMut};
use crate::error::Result;
use crate::http::{HttpConn, HttpDial, HttpHandler};
use crate::ws_conn::{WsConn, WsDial, WsHandler};

/// A WebSocket or an HTTP connection.
pub enum AnyConn {
    /// WebSocket.
    Ws(WsConn),
    /// HTTP/1.1.
    Http(HttpConn),
}

impl AnyConn {
    /// The WebSocket connection, if it is one.
    pub const fn ws(&mut self) -> Option<&mut WsConn> {
        match self {
            Self::Ws(c) => Some(c),
            Self::Http(_) => None,
        }
    }

    /// The HTTP connection, if it is one.
    pub const fn http(&mut self) -> Option<&mut HttpConn> {
        match self {
            Self::Http(c) => Some(c),
            Self::Ws(_) => None,
        }
    }

    /// What the quick-ACK route check found for this connection
    /// ([`crate::route::RouteCheck::Pending`] while ergon-io's warning
    /// thread is still looking it up).
    #[must_use]
    pub fn route(&self) -> &crate::route::RouteCheck {
        match self {
            Self::Ws(c) => c.route(),
            Self::Http(c) => c.route(),
        }
    }
}

/// A driver-owned [`AnyConn`]: the protocol-specific handle.
impl ConnMut<'_, AnyConn> {
    /// The WebSocket connection, if it is one.
    pub fn ws(&mut self) -> Option<ConnMut<'_, WsConn>> {
        self.inner().ws().map(ConnMut::new)
    }

    /// The HTTP connection, if it is one.
    pub fn http(&mut self) -> Option<ConnMut<'_, HttpConn>> {
        self.inner().http().map(ConnMut::new)
    }
}

impl Io for AnyConn {
    const DEADLINES: bool = true;

    #[inline]
    fn fd(&self) -> std::os::fd::RawFd {
        match self {
            Self::Ws(c) => c.fd(),
            Self::Http(c) => c.fd(),
        }
    }
    #[inline]
    fn rx_space(&mut self) -> &mut [u8] {
        match self {
            Self::Ws(c) => c.rx_space(),
            Self::Http(c) => c.rx_space(),
        }
    }
    #[inline]
    fn tx_queue(&mut self) -> &mut Vec<u8> {
        match self {
            Self::Ws(c) => c.tx_queue(),
            Self::Http(c) => c.tx_queue(),
        }
    }
    fn has_buffered_input(&self) -> bool {
        match self {
            Self::Ws(c) => c.has_buffered_input(),
            Self::Http(c) => c.has_buffered_input(),
        }
    }
    fn set_route(&mut self, route: crate::endpoint::sealed::RouteSlot) {
        match self {
            Self::Ws(c) => c.set_route(route),
            Self::Http(c) => c.set_route(route),
        }
    }
    fn route_state(&self) -> &crate::endpoint::sealed::RouteSlot {
        match self {
            Self::Ws(c) => c.route_state(),
            Self::Http(c) => c.route_state(),
        }
    }
    fn expire(&mut self, now_ns: i64) -> Result<()> {
        match self {
            Self::Ws(c) => c.expire(now_ns),
            Self::Http(c) => c.expire(now_ns),
        }
    }
    fn close_reason(&mut self, error: crate::error::Error) -> crate::error::Error {
        match self {
            Self::Ws(c) => c.close_reason(error),
            Self::Http(c) => c.close_reason(error),
        }
    }
    #[inline]
    fn on_handed(&mut self, n: usize) {
        match self {
            Self::Ws(c) => c.on_handed(n),
            Self::Http(c) => c.on_handed(n),
        }
    }
}

impl<H: WsHandler + HttpHandler> Deliver<H> for AnyConn {
    #[inline]
    fn on_received(
        &mut self,
        id: ConnId,
        n: usize,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        match self {
            Self::Ws(c) => c.on_received(id, n, meta, handler),
            Self::Http(c) => c.on_received(id, n, meta, handler),
        }
    }
}

/// A WebSocket or an HTTP dial, for `Driver<AnyConn>::connect`.
pub enum AnyDial {
    /// WebSocket.
    Ws(WsDial),
    /// HTTP/1.1.
    Http(HttpDial),
}

impl From<WsDial> for AnyDial {
    fn from(d: WsDial) -> Self {
        Self::Ws(d)
    }
}

impl From<HttpDial> for AnyDial {
    fn from(d: HttpDial) -> Self {
        Self::Http(d)
    }
}

impl DialIo for AnyDial {
    type Conn = AnyConn;

    fn host(&self) -> &str {
        match self {
            Self::Ws(d) => d.host(),
            Self::Http(d) => d.host(),
        }
    }
    fn port(&self) -> u16 {
        match self {
            Self::Ws(d) => d.port(),
            Self::Http(d) => d.port(),
        }
    }
    fn addr(&self) -> Option<SocketAddr> {
        match self {
            Self::Ws(d) => d.addr(),
            Self::Http(d) => d.addr(),
        }
    }
    fn start(&mut self, addr: SocketAddr) -> Result<()> {
        match self {
            Self::Ws(d) => d.start(addr),
            Self::Http(d) => d.start(addr),
        }
    }
    fn step(&mut self) -> Result<Option<AnyConn>> {
        match self {
            Self::Ws(d) => Ok(d.step()?.map(AnyConn::Ws)),
            Self::Http(d) => Ok(d.step()?.map(AnyConn::Http)),
        }
    }
    fn gate_route(&mut self) {
        match self {
            Self::Ws(d) => d.gate_route(),
            Self::Http(d) => d.gate_route(),
        }
    }
    fn awaiting_route(&self) -> Option<std::os::fd::RawFd> {
        match self {
            Self::Ws(d) => d.awaiting_route(),
            Self::Http(d) => d.awaiting_route(),
        }
    }
    fn route_checked(&mut self, check: crate::route::RouteCheck) {
        match self {
            Self::Ws(d) => d.route_checked(check),
            Self::Http(d) => d.route_checked(check),
        }
    }
}
