//! `wstls_fstack_lat`: WebSocket-over-TLS on ergon-io's `fstack-spin`.
//!
//! One process, one F-Stack. Ping is `Driver<WsConn<FstackTcp>>`. Pong is
//! the kernel peer (`wstls::kernel_serve`): host B's NIC stays in the
//! kernel, so this process does not start F-Stack there. Linux,
//! `--features fstack-spin`.

#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::rc::Rc;
use std::sync::Arc;

use ergon_io::fstack::FstackTcp;
use ergon_io::{ConnId, Driver, Handler, RxMeta, Target, WsConn, WsDial, WsHandler, WsOptions};
use ergon_latency_core::message::MESSAGE_LEN;

use super::bypass;
use super::eio_udp::{self, Staged};
use super::point::{Common, Error, Role};
use super::sock::{self, Impl, Inspect};
use super::wstls::{self, WstlsArgs};
use super::{ip_overhead, tcp_echo_capacity};

/// Parses nothing: `wstls::run` already took the flags. The row, or `None`
/// for a pong.
///
/// # Errors
/// A flag the stack refuses, F-Stack not linked or not started, or what the
/// point met.
pub fn wstls(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    match c.role {
        Role::Pong => {
            c.check_cpus(1, "echo")?;
            wstls::kernel_serve(c, a)?;
            Ok(None)
        }
        Role::Ping => ping(c, a),
        Role::Local | Role::OnewaySend | Role::OnewayRecv => Err(Error::Usage(format!(
            "--role {}: one fstack-spin per process",
            c.role.name()
        ))),
    }
}

fn ping(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    let inp = a.fstack_in(&c.cpus)?;
    let peer = match a.peer {
        SocketAddr::V4(v) => v,
        SocketAddr::V6(v) => {
            return Err(Error::Usage(format!(
                "--peer {v}: IPv4 ({})",
                a.backend.name()
            )));
        }
    };
    std::fs::create_dir_all(&c.out)
        .map_err(|e| Error::Transport(format!("{}: {e}", c.out.display())))?;
    let path = c.out.join("fstack-spin.conf");
    let rexmit = inp.nic.rto_min_us.div_ceil(1000);
    let arm = vec![
        ("backend".to_owned(), "fstack-spin".to_owned()),
        ("stack".to_owned(), "fstack".to_owned()),
        ("rexmit_min_ms".to_owned(), rexmit.to_string()),
        ("rto_min_us".to_owned(), inp.nic.rto_min_us.to_string()),
    ];
    let streams = c.streams;
    let source = inp.nic.ip;
    sock::run(c, Impl::Ergon, "wstls", &arm, false, move |_end, _cpu| {
        let cfg = bypass::fstack_spin_config(&inp, &path)?;
        let mut driver: Driver<WsConn<FstackTcp>> =
            Driver::new(cfg).map_err(|e| eio_udp::eio("Driver::new", e))?;
        let tls = wstls::bench_client_config()?;
        let mut ids = Vec::with_capacity(usize_of(streams)?);
        for _ in 0..streams {
            let conn = connect(peer, source, Arc::clone(&tls))?;
            ids.push(
                driver
                    .add(conn)
                    .map_err(|e| eio_udp::eio("Driver::add", e))?,
            );
        }
        pair(driver, ids)
    })
}

fn connect(
    to: SocketAddrV4,
    source: std::net::Ipv4Addr,
    tls: Arc<rustls::ClientConfig>,
) -> Result<WsConn<FstackTcp>, Error> {
    let target = Target::new("localhost", to.port(), "/")
        .with_addr(SocketAddr::V4(to))
        .with_source(IpAddr::V4(source));
    let opts = WsOptions {
        ping_interval_ms: 0,
        pong_timeout_ms: 0,
        idle_timeout_ms: 0,
        close_timeout_ms: 0,
        ..WsOptions::default()
    };
    WsDial::<FstackTcp>::on_stack(target, tls, opts)
        .and_then(WsDial::connect_blocking)
        .map_err(|e| eio_udp::eio(&format!("websocket connect {to}"), e))
}

fn usize_of(n: u32) -> Result<usize, Error> {
    usize::try_from(n).map_err(|_| Error::Usage(format!("--streams {n}: does not fit")))
}

struct Shared {
    d: RefCell<Driver<WsConn<FstackTcp>>>,
    staged: RefCell<Staged>,
    ids: Vec<ConnId>,
}

struct SpinTx {
    s: Rc<Shared>,
}
struct SpinRx {
    s: Rc<Shared>,
    gone: usize,
}

fn pair(driver: Driver<WsConn<FstackTcp>>, ids: Vec<ConnId>) -> Result<(SpinTx, SpinRx), Error> {
    if ids.is_empty() {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    let room = tcp_echo_capacity(ids.len(), ids.len(), false);
    let s = Rc::new(Shared {
        d: RefCell::new(driver),
        staged: RefCell::new(Staged::new(room)),
        ids,
    });
    Ok((SpinTx { s: Rc::clone(&s) }, SpinRx { s, gone: 0 }))
}

fn send_now(
    d: &mut Driver<WsConn<FstackTcp>>,
    ids: &[ConnId],
    stream: u32,
    msg: &[u8; MESSAGE_LEN],
) -> Result<bool, Error> {
    let id = ids
        .get(usize_of(stream)?)
        .copied()
        .ok_or_else(|| Error::Transport(format!("stream {stream} of {}", ids.len())))?;
    let result = {
        let mut c = d
            .get_mut(id)
            .ok_or_else(|| Error::Transport(format!("stream {stream}: its connection closed")))?;
        c.send_binary(msg)
    };
    match result {
        Ok(()) => {
            d.flush().map_err(|e| eio_udp::eio("flush", e))?;
            Ok(true)
        }
        Err(ergon_io::Error::Busy(_)) => Ok(false),
        Err(e) => Err(eio_udp::eio("send_binary", e)),
    }
}

impl super::point::Tx for SpinTx {
    fn try_send(
        &mut self,
        stream: u32,
        msg: &[u8; MESSAGE_LEN],
    ) -> Result<super::point::Sent, Error> {
        let Ok(mut d) = self.s.d.try_borrow_mut() else {
            return Ok(if self.s.staged.borrow_mut().push(stream, msg) {
                super::point::Sent::Sent
            } else {
                super::point::Sent::Busy
            });
        };
        Ok(if send_now(&mut d, &self.s.ids, stream, msg)? {
            super::point::Sent::Sent
        } else {
            super::point::Sent::Busy
        })
    }
}

struct Deliver<'a, F> {
    on: &'a mut F,
    gone: usize,
    err: Option<Error>,
}

impl<F> Handler for Deliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        match error {
            ergon_io::Error::Closed => self.gone += 1,
            ergon_io::Error::Io(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                self.gone += 1;
            }
            e => {
                if self.err.is_none() {
                    self.err = Some(Error::Transport(format!("websocket {id:?} closed: {e}")));
                }
            }
        }
    }
}

impl<F: FnMut(&[u8])> WsHandler for Deliver<'_, F> {
    fn on_message(&mut self, _conn: ConnId, opcode: u8, payload: &[u8], _meta: &RxMeta, _pos: u16) {
        if opcode == 0x2 {
            (self.on)(payload);
        }
    }
}

impl super::point::Rx for SpinRx {
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let s = &*self.s;
        let mut d = s.d.borrow_mut();
        {
            let mut staged = s.staged.borrow_mut();
            if staged.len > 0 {
                d.flush().map_err(|e| eio_udp::eio("flush", e))?;
                staged.drain(|stream, m| send_now(&mut d, &s.ids, stream, m))?;
                if staged.len > 0 {
                    return Ok(0);
                }
            }
        }
        let mut h = Deliver {
            on,
            gone: 0,
            err: None,
        };
        let n = d.poll(&mut h).map_err(|e| eio_udp::eio("poll", e))?;
        self.gone += h.gone;
        if let Some(e) = h.err {
            return Err(e);
        }
        let mut staged = s.staged.borrow_mut();
        if staged.len > 0 {
            staged.drain(|stream, m| send_now(&mut d, &s.ids, stream, m))?;
        }
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.gone >= self.s.ids.len()
    }
}

impl Inspect for SpinRx {
    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(ip_overhead(false) + 20 + 5 + 6 + MESSAGE_LEN as u64)
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let d = self.s.d.borrow();
        let cfg = d.config();
        let mut messages = 0u64;
        for c in self.s.ids.iter().filter_map(|&id| d.get(id)) {
            messages += c.stats().messages;
        }
        let rexmit = cfg
            .fstack
            .sysctl
            .iter()
            .find(|(k, _)| k == "net.inet.tcp.rexmit_min")
            .map_or_else(|| "-".to_owned(), |(_, v)| v.clone());
        for (k, v) in [
            ("ergon.backend", cfg.backend.to_string()),
            ("fstack.rexmit_min_ms", rexmit),
            ("fstack.delayed_ack", cfg.fstack.delayed_ack.to_string()),
            ("stat.messages", messages.to_string()),
            ("stat.polls", d.stats().polls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        Ok(())
    }
}
