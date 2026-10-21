//! The driver with every backend over loopback: delivery, sending, peer close,
//! per-connection protocol errors, stale ids and drop with I/O in flight.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod support;

use std::time::{Duration, Instant};

use ergon_io::placement::{CpuError, HelperThread};
use ergon_io::{
    Backend, Config, ConnId, Driver, Error, Handler, RxMeta, WsConn, WsDial, WsHandler, ws,
};
use support::{Lab, R};

/// Held by every test here that starts an `ergon-io-dial` helper. Tests in
/// one binary share a process and may run in parallel (`--test-threads`), and
/// a test that inspects its helper through /proc/self/task by name must not
/// find another test's.
static DIAL_HELPERS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn dial_helpers() -> std::sync::MutexGuard<'static, ()> {
    DIAL_HELPERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ANCHOR: handler
#[derive(Default)]
struct Collect {
    msgs: Vec<(ConnId, u8, Vec<u8>)>,
    closed: Vec<(ConnId, String)>,
    opened: Vec<ConnId>,
}

impl Handler for Collect {
    fn on_open(&mut self, id: ConnId) {
        self.opened.push(id);
    }

    fn on_closed(&mut self, id: ConnId, error: Error) {
        self.closed.push((id, error.to_string()));
    }
}

impl WsHandler for Collect {
    fn on_message(&mut self, id: ConnId, opcode: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.msgs.push((id, opcode, payload.to_vec()));
    }
}

fn driver(b: Backend) -> R<Driver<WsConn>> {
    // Short waits keep the sleeping backends quick in tests.
    let mut cfg = Config::new(b);
    cfg.wait_us = cfg.wait_us.min(100);
    Ok(Driver::new(cfg)?)
}

fn poll_until(d: &mut Driver<WsConn>, h: &mut Collect, done: impl Fn(&Collect) -> bool) -> R {
    let end = Instant::now() + Duration::from_secs(10);
    while !done(h) {
        d.poll(h)?;
        if Instant::now() > end {
            return Err(format!(
                "timed out: {} msgs, closed {:?}, stats {:?}",
                h.msgs.len(),
                h.closed,
                d.stats()
            )
            .into());
        }
    }
    Ok(())
}
// ANCHOR_END: handler

fn each_backend(test: impl Fn(Backend) -> R) -> R {
    let mut ran = 0;
    for b in Backend::ALL {
        eprintln!("backend {b}");
        match test(b) {
            Ok(()) => ran += 1,
            // A pod with one CPU has no core for `uring-sqpoll`'s kernel
            // thread: `Driver::new` refuses typed (asserted; covered in full
            // by `sqpoll_thread_never_shares_the_polling_cpu`). The body
            // still runs on every other backend.
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {}
            Err(e) => return Err(format!("{b}: {e}").into()),
        }
    }
    assert!(ran >= Backend::ALL.len() - 1, "only {ran} backends ran");
    Ok(())
}

#[test]
fn delivers_in_order_then_reports_peer_close() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept()?;
                for i in 0..500u32 {
                    p.send(ws::OP_TEXT, true, format!("m{i}").as_bytes())?;
                }
                p.send(ws::OP_CLOSE, true, &[])?;
                Ok(())
            });
            let id = d.add(WsConn::connect(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let mut h = Collect::default();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            srv.join().map_err(|_| "server panicked")??;
            assert_eq!(
                h.msgs.len(),
                500,
                "closed {:?} stats {:?}",
                h.closed,
                d.stats()
            );
            for (i, (cid, op, body)) in h.msgs.iter().enumerate() {
                assert_eq!((*cid, *op), (id, ws::OP_TEXT));
                assert_eq!(body, format!("m{i}").as_bytes());
            }
            assert_eq!(h.closed.len(), 1);
            assert_eq!(h.closed[0].0, id);
            assert!(d.get(id).is_none() && d.is_empty());
            assert_eq!(d.stats().closed, 1);
            Ok(())
        })
    })
}

// ANCHOR: echo
#[test]
fn sends_and_receives_echo() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept()?;
                for _ in 0..50 {
                    let (op, body) = p.recv()?;
                    p.send(op, true, &body)?;
                }
                Ok(())
            });
            let id = d.add(WsConn::connect(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let mut h = Collect::default();
            for i in 0..50usize {
                let body = vec![u8::try_from(i)?; 1 + i * 100];
                d.get_mut(id)
                    .ok_or("conn gone")?
                    .send_frame(ws::OP_BINARY, &body)?;
                d.flush()?;
                poll_until(&mut d, &mut h, |h| h.msgs.len() > i)?;
                assert_eq!(h.msgs[i].2, body);
            }
            // io_uring submits sends through io_uring_enter (or the SQPOLL
            // thread): they are sends, not syscalls of their own.
            let st = d.stats();
            assert!(st.sends >= 50, "{b}: {st:?}");
            if b.is_uring() {
                assert_eq!(st.send_calls, 0, "{b}: {st:?}");
            } else {
                assert_eq!(st.send_calls, st.sends, "{b}: {st:?}");
            }
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
    })
}
// ANCHOR_END: echo

#[test]
fn protocol_error_closes_only_that_connection() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut bad = lab.accept()?;
                let mut good = lab.accept()?;
                // Servers must not mask: the client treats it as a protocol error.
                let mut masked = vec![0u8; ws::header_len(3, true) + 3];
                ws::encode_frame(ws::OP_TEXT, b"bad", Some([1, 2, 3, 4]), &mut masked)?;
                bad.write(&masked)?;
                for _ in 0..20 {
                    good.send(ws::OP_TEXT, true, b"ok")?;
                }
                assert!(
                    bad.client_closed()?,
                    "the failed connection's socket is closed"
                );
                Ok(())
            });
            let t = lab.target("/")?;
            let bad = d.add(WsConn::connect(&t, lab.client_cfg.clone(), 1 << 20)?)?;
            let good = d.add(WsConn::connect(&t, lab.client_cfg.clone(), 1 << 20)?)?;
            let mut h = Collect::default();
            poll_until(&mut d, &mut h, |h| {
                h.msgs.len() == 20 && h.closed.len() == 1
            })?;
            assert_eq!(h.closed[0].0, bad);
            assert!(h.closed[0].1.contains("masked"), "{}", h.closed[0].1);
            assert!(h.msgs.iter().all(|m| m.0 == good));
            assert!(d.get(bad).is_none() && d.get(good).is_some());
            // The closing connection's buffers are released on a later poll.
            for _ in 0..1000 {
                d.poll(&mut h)?;
            }
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
    })
}

#[test]
fn stale_id_never_reaches_a_reused_slot() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut first = lab.accept()?;
                let mut second = lab.accept()?;
                assert!(first.client_closed()?);
                second.send(ws::OP_TEXT, true, b"second")?;
                assert!(second.client_closed()?);
                Ok(())
            });
            let target = lab.target("/")?;
            let old = d.add(WsConn::connect(&target, lab.client_cfg.clone(), 1 << 20)?)?;
            d.close(old);
            let mut h = Collect::default();
            // Let io_uring release the slot (the SQPOLL thread completes the
            // cancel asynchronously), then reuse it.
            let settle = Instant::now() + Duration::from_millis(100);
            while Instant::now() < settle {
                d.poll(&mut h)?;
            }
            let new = d.add(WsConn::connect(&target, lab.client_cfg.clone(), 1 << 20)?)?;
            assert_eq!(old.index(), new.index(), "the slot is reused");
            assert_ne!(old, new);
            assert!(d.get(old).is_none() && d.get_mut(old).is_none());
            d.close(old); // stale: must not close the new connection
            assert!(d.get(new).is_some());
            poll_until(&mut d, &mut h, |h| h.msgs.len() == 1)?;
            assert_eq!(h.msgs[0].0, new);
            assert!(h.closed.is_empty(), "{:?}", h.closed);
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
    })
}

#[test]
fn drop_with_io_in_flight_closes_sockets() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept()?;
                p.send(ws::OP_TEXT, true, b"one")?;
                assert!(p.client_closed()?, "dropping the driver closes the socket");
                Ok(())
            });
            let id = d.add(WsConn::connect(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let mut h = Collect::default();
            poll_until(&mut d, &mut h, |h| h.msgs.len() == 1)?;
            // A receive is armed (io_uring) and a send is queued and in flight.
            d.get_mut(id)
                .ok_or("conn gone")?
                .send_frame(ws::OP_TEXT, &[7; 12_000])?;
            d.poll(&mut h)?;
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
    })
}

#[test]
fn busy_poll_preference_reaches_the_socket_or_fails_cleanly() -> R {
    use std::os::fd::AsRawFd;
    // SO_PREFER_BUSY_POLL and a raised SO_BUSY_POLL_BUDGET need CAP_NET_ADMIN:
    // with it the options must reach the socket, without it `add` must fail
    // with an error that says so and keep nothing.
    let probe = std::net::TcpListener::bind("127.0.0.1:0")?;
    let privileged = ergon_io::sys::set_prefer_busy_poll(probe.as_raw_fd(), true).is_ok();
    for (prefer, budget) in [(true, None), (false, Some(16)), (true, Some(16))] {
        let lab = Lab::new()?;
        let mut cfg = Config::new(Backend::RecvBusy);
        cfg.prefer_busy_poll = prefer;
        cfg.busy_poll_budget = budget;
        let mut d = Driver::new(cfg)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept()?;
                let _ = p.client_closed()?;
                Ok(())
            });
            let added = d.add(WsConn::connect(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?);
            if privileged {
                let id = added?;
                let fd = d.get(id).ok_or("conn gone")?.raw_fd();
                let bp = ergon_io::sys::busy_poll_settings(fd)?;
                assert_eq!(bp.prefer, prefer, "{bp:?}");
                assert!(bp.usec >= 1, "{bp:?}");
            } else {
                let e = added.err().ok_or("added without CAP_NET_ADMIN")?;
                assert!(e.to_string().contains("CAP_NET_ADMIN"), "{e}");
                assert_eq!(d.len(), 0);
            }
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

// Which backends poll the NIC through the socket is not a documentation
// claim: it is a socket option, and it reads back. `recv-busy` (with or
// without `per_queue`) and `epoll-busy` set `SO_BUSY_POLL` on every socket,
// so an empty `recvmsg` runs one NAPI pass and the packet is taken off the
// NIC with no interrupt. `epoll-spin` and `uring-spin` set nothing and wait
// for the interrupt. `uring-sqpoll` and `uring-napi` do poll the NIC, but
// through the SQPOLL kernel thread and io_uring NAPI rather than through the
// socket, so their `SO_BUSY_POLL` is 0 as well.
#[test]
fn busy_poll_is_set_only_where_the_socket_polls_the_nic() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut cfg = Config::new(b);
        cfg.wait_us = cfg.wait_us.min(100);
        let mut d = match Driver::new(cfg) {
            Ok(d) => d,
            Err(e) if b == Backend::UringSqpoll && support::refused(&e, HelperThread::SqPoll) => {
                return Ok(());
            }
            Err(e) => return Err(format!("{b}: {e}").into()),
        };
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept()?;
                let _ = p.client_closed()?;
                Ok(())
            });
            let id = d.add(WsConn::connect(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let fd = d.get(id).ok_or("conn gone")?.raw_fd();
            let bp = ergon_io::sys::busy_poll_settings(fd)?;
            // SO_BUSY_POLL is set exactly on the two backends that poll the
            // NIC through the socket. Everything else reads back 0.
            let socket_polls = matches!(b, Backend::RecvBusy | Backend::EpollBusy);
            if socket_polls {
                assert!(bp.usec >= 1, "{b}: SO_BUSY_POLL = {}, want >= 1", bp.usec);
            } else {
                assert_eq!(bp.usec, 0, "{b}: SO_BUSY_POLL = {}, want 0", bp.usec);
            }
            // The socket-level polls-nic set is the two above; the two uring
            // pollers do it without the option; the two spin backends do not
            // poll the NIC at all.
            let polls = matches!(
                b,
                Backend::RecvBusy | Backend::EpollBusy | Backend::UringSqpoll | Backend::UringNapi
            );
            assert_eq!(b.polls_nic(), polls, "{b}: polls_nic");
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
    })
}

// mio is the interrupt path, and so is tokio's I/O, which is built on it.
// "Interrupt path" has a socket-level meaning: nothing makes an empty receive
// run a NAPI pass, so the packet waits for the NIC's interrupt instead of
// being pulled off the queue. That flag is `SO_BUSY_POLL`, and it reads back.
// A socket mio owns must therefore report 0. The option is the whole of the
// busy-poll mechanism from userspace (`sk_busy_loop` runs only when it is
// set), so a 0 here is what "waits for the interrupt" means in the kernel.
// mio's selector is `sys/unix/selector/epoll.rs` (epoll_wait); tokio's I/O
// driver is `tokio::io::driver`, which polls a mio `Poll`.
#[test]
fn mio_registers_the_socket_without_busy_poll() -> R {
    use std::os::fd::AsRawFd;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let mut client = mio::net::TcpStream::connect(listener.local_addr()?)?;
    let (_peer, _) = listener.accept()?;
    let mut poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(8);
    poll.registry()
        .register(&mut client, mio::Token(0), mio::Interest::READABLE)?;
    let bp = ergon_io::sys::busy_poll_settings(client.as_raw_fd())?;
    assert_eq!(
        bp.usec, 0,
        "mio set SO_BUSY_POLL = {} on a socket it owns; want 0. \
         A non-zero value would mean mio polls the NIC, which it does not.",
        bp.usec
    );
    // The wait is a plain epoll_wait with a timeout: it returns at once and
    // reports nothing, it does not run a NAPI pass to go and find a packet.
    poll.poll(&mut events, Some(Duration::ZERO))?;
    assert_eq!(
        events.iter().count(),
        0,
        "mio reported an event on an idle socket"
    );
    Ok(())
}

// The "and so tokio" half of the same claim. tokio's I/O driver is a mio
// `Poll` (`tokio::io::driver`), so a socket its reactor owns is on the
// interrupt path exactly as a bare mio socket is. Prove it the same way: the
// option reads back 0. A runtime that polled the NIC would have to set
// `SO_BUSY_POLL` on the socket, and nothing in tokio does.
#[test]
fn tokio_registers_the_socket_without_busy_poll() -> R {
    use std::os::fd::AsRawFd;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;
    // A current-thread runtime with the I/O driver on: the usual shape of a
    // tokio client, and the one whose latency the book compares with epoll.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;
    rt.block_on(async {
        let client = tokio::net::TcpStream::connect(addr).await?;
        let (_peer, _) = listener.accept()?;
        let bp = ergon_io::sys::busy_poll_settings(client.as_raw_fd())?;
        assert_eq!(
            bp.usec, 0,
            "tokio set SO_BUSY_POLL = {} on a socket its reactor owns; want 0. \
             A non-zero value would mean tokio polls the NIC, which it does not.",
            bp.usec
        );
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })?;
    Ok(())
}

#[test]
fn send_failures_found_by_flush_reach_the_next_poll() -> R {
    for b in Backend::ALL.into_iter().filter(|b| !b.may_sleep()) {
        let lab = Lab::new()?;
        let mut d = match driver(b) {
            Ok(d) => d,
            // A pod with one CPU has no core for SQPOLL (asserted in
            // `each_backend`); the other spin backends run the body.
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {
                continue;
            }
            Err(e) => return Err(format!("{b}: {e}").into()),
        };
        let id = std::thread::scope(|s| -> R<ConnId> {
            let srv = s.spawn(|| -> R {
                // Once the client is connected, reset the connection: its next
                // sends fail.
                let mut p = lab.accept()?;
                p.recv()?;
                socket2::SockRef::from(&p.sock).set_linger(Some(Duration::ZERO))?;
                drop(p);
                Ok(())
            });
            let id = d.add(WsConn::connect(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            d.get_mut(id)
                .ok_or("conn gone")?
                .send_frame(ws::OP_BINARY, b"connected")?;
            d.flush()?;
            srv.join().map_err(|_| "server panicked")??;
            Ok(id)
        })?;
        // Send and flush without polling until the connection is gone (epoll
        // backends close it inside `flush`) or the send is stuck in flight.
        for _ in 0..200 {
            let Some(mut c) = d.get_mut(id) else { break };
            c.send_frame(ws::OP_BINARY, b"x")?;
            d.flush()?;
            std::thread::sleep(Duration::from_millis(1));
        }
        let mut h = Collect::default();
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())
            .map_err(|e| format!("{b}: close never reported: {e}"))?;
        assert_eq!(h.closed.len(), 1, "{b}: {:?}", h.closed);
        assert_eq!(h.closed[0].0, id, "{b}");
    }
    Ok(())
}

/// Opens `n` echoing connections on `d` (each server thread echoes until the
/// client closes) and returns their ids.
fn echo_conns<'s>(
    s: &'s std::thread::Scope<'s, '_>,
    lab: &'s Lab,
    d: &mut Driver<WsConn>,
    n: usize,
) -> R<Vec<ConnId>> {
    let mut ids = Vec::new();
    for _ in 0..n {
        let srv = s.spawn(|| -> R<support::Peer> { lab.accept() });
        let conn = WsConn::connect(&lab.target("/")?, lab.client_cfg.clone(), 1 << 20);
        let mut p = srv.join().map_err(|_| "server panicked")??;
        let id = d.add(conn?)?;
        // A driver that cannot reap its requests leaks its sockets on drop:
        // time out instead of waiting for a close that never comes.
        p.sock.set_read_timeout(Some(Duration::from_secs(20)))?;
        s.spawn(move || {
            while let Ok((op, body)) = p.recv() {
                if p.send(op, true, &body).is_err() {
                    break;
                }
            }
        });
        ids.push(id);
    }
    Ok(ids)
}

#[test]
fn recv_busy_per_queue_reads_every_socket_that_has_data() -> R {
    // Twenty connections, each echoing its own numbered messages: with one
    // socket read per queue and the rest found by epoll, every reply arrives,
    // in order on each connection, and nothing is closed.
    let lab = Lab::new()?;
    let mut cfg = Config::new(Backend::RecvBusy);
    cfg.per_queue = true;
    let mut d: Driver<WsConn> = Driver::new(cfg)?;
    std::thread::scope(|s| -> R {
        let ids = echo_conns(s, &lab, &mut d, 20)?;
        let mut h = Collect::default();
        for round in 0..50u32 {
            for &id in &ids {
                d.get_mut(id)
                    .ok_or("connection gone")?
                    .send_frame(ws::OP_TEXT, format!("{round}").as_bytes())?;
            }
            let want = (round as usize + 1) * ids.len();
            poll_until(&mut d, &mut h, |h| {
                h.msgs.len() >= want || !h.closed.is_empty()
            })?;
        }
        assert!(h.closed.is_empty(), "closed {:?}", h.closed);
        for &id in &ids {
            let got: Vec<&[u8]> = h
                .msgs
                .iter()
                .filter(|m| m.0 == id)
                .map(|m| m.2.as_slice())
                .collect();
            let want: Vec<String> = (0..50).map(|r| r.to_string()).collect();
            assert_eq!(got, want.iter().map(String::as_bytes).collect::<Vec<_>>());
        }
        assert!(d.stats().epoll_wait_calls > 0, "{:?}", d.stats());
        drop(d);
        Ok(())
    })
}

#[test]
fn uring_full_submission_queue_keeps_connections_open() -> R {
    // 16 connections on a 4-entry ring: every round needs 32 submissions and
    // overflows the completion queue too. Nothing may be closed.
    for b in [Backend::UringSpin, Backend::UringSqpoll] {
        let lab = Lab::new()?;
        let mut cfg = Config::new(b);
        cfg.ring_entries = 4;
        let mut d = match Driver::new(cfg) {
            Ok(d) => d,
            // A pod with one CPU has no core for SQPOLL: typed refusal
            // (asserted); `UringSpin` above runs the body everywhere.
            Err(e) if b == Backend::UringSqpoll => {
                assert!(support::refused(&e, HelperThread::SqPoll), "{e}");
                continue;
            }
            Err(e) => return Err(format!("{b}: {e}").into()),
        };
        std::thread::scope(|s| -> R {
            let res = (|| -> R {
                let ids = echo_conns(s, &lab, &mut d, 16)?;
                let mut h = Collect::default();
                for round in 0..3usize {
                    for &id in &ids {
                        d.get_mut(id)
                            .ok_or("conn gone")?
                            .send_frame(ws::OP_BINARY, &[u8::try_from(round)?; 32])?;
                    }
                    poll_until(&mut d, &mut h, |h| {
                        h.msgs.len() == 16 * (round + 1) || !h.closed.is_empty()
                    })?;
                    if !h.closed.is_empty() {
                        return Err(format!("round {round}: closed {:?}", h.closed).into());
                    }
                }
                Ok(())
            })();
            // Closes the clients, so the echo threads end.
            drop(d);
            res
        })
        .map_err(|e| format!("{b}: {e}"))?;
    }
    Ok(())
}

#[test]
fn uring_close_and_reopen_under_a_full_queue_leaves_new_connections_alone() -> R {
    // Closing many connections at once on a 2-entry ring defers their
    // cancels; the slots and descriptors are then reused at once. A late
    // cancel must never reach the newer connections.
    for b in [Backend::UringSpin, Backend::UringSqpoll] {
        let lab = Lab::new()?;
        let mut cfg = Config::new(b);
        cfg.ring_entries = 2;
        let mut d = match Driver::new(cfg) {
            Ok(d) => d,
            // A pod with one CPU has no core for SQPOLL: typed refusal
            // (asserted); `UringSpin` above runs the body everywhere.
            Err(e) if b == Backend::UringSqpoll => {
                assert!(support::refused(&e, HelperThread::SqPoll), "{e}");
                continue;
            }
            Err(e) => return Err(format!("{b}: {e}").into()),
        };
        std::thread::scope(|s| -> R {
            let res = (|| -> R {
                let mut h = Collect::default();
                for cycle in 0..4usize {
                    let ids = echo_conns(s, &lab, &mut d, 8)?;
                    for &id in &ids {
                        d.get_mut(id)
                            .ok_or("conn gone")?
                            .send_frame(ws::OP_BINARY, &[u8::try_from(cycle)?; 16])?;
                    }
                    let want = h.msgs.len() + 8;
                    poll_until(&mut d, &mut h, |h| {
                        h.msgs.len() == want || !h.closed.is_empty()
                    })?;
                    if !h.closed.is_empty() {
                        return Err(format!("cycle {cycle}: closed {:?}", h.closed).into());
                    }
                    for &id in &ids {
                        d.close(id);
                    }
                    // A few polls only: the cancels may still be queued when
                    // the next cycle reuses the slots and descriptors.
                    for _ in 0..3 {
                        d.poll(&mut h)?;
                    }
                }
                Ok(())
            })();
            drop(d);
            res
        })
        .map_err(|e| format!("{b}: {e}"))?;
    }
    Ok(())
}

#[test]
fn pinned_threads_start_on_their_cpus() -> R {
    // The process's CPUs (its cpuset), not the host's online list: the
    // kernel refuses a mask with none of the cpuset's CPUs.
    let cpus = cpuset()?;
    let (&[here, there, ..], true) = (cpus.as_slice(), cpus.len() >= 2) else {
        // One CPU (`--cpuset-cpus=0`): a thread cannot be put outside it.
        let online = ergon_io::sys::parse_cpu_list(&std::fs::read_to_string(
            "/sys/devices/system/cpu/online",
        )?);
        if let Some(&outside) = online.iter().find(|c| !cpus.contains(c)) {
            let e = ergon_io::sys::spawn_pinned("ergon-io-test", &[outside], || {})
                .err()
                .ok_or("a thread started outside the cpuset")?;
            assert_eq!(e.raw_os_error(), Some(libc::EINVAL), "{e}");
        }
        return Ok(());
    };
    // The creator sits on one CPU; the new thread must never run there, not
    // even for its first instructions (an inherited mask would put it there).
    ergon_io::sys::set_affinity(&[here])?;
    let (tx, rx) = std::sync::mpsc::channel();
    ergon_io::sys::spawn_pinned("ergon-io-test", &[there], move || {
        let _ = tx.send(ergon_io::sys::current_cpu());
    })?;
    let first = rx.recv_timeout(Duration::from_secs(5))?;
    ergon_io::sys::set_affinity(&cpus)?;
    assert_eq!(first, Some(there));
    Ok(())
}

#[test]
fn background_dns_waits_never_hold_up_other_connects() -> R {
    let _helpers = dial_helpers();
    use ergon_io::dns::DnsConfig;
    // The property is the helper's: one dial waits on DNS there while the
    // other opens. A pod with one CPU has no core for the helper and refuses
    // typed instead (asserted; the helper half runs on a pod with two CPUs).
    if support::one_cpu()? {
        return background_refuses_on_one_cpu();
    }
    // A name server that never answers: one connect waits on DNS while a
    // second, with a pinned address, must open regardless; the first must
    // still time out on schedule.
    let silent = std::net::UdpSocket::bind("127.0.0.1:0")?;
    let dns = DnsConfig {
        servers: vec![silent.local_addr()?],
        timeout_ms: 10_000,
        attempts: 1,
        ..DnsConfig::system()
    };
    let lab = Lab::new()?;
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.connect_timeout_ms = 1_500;
    let mut d = Driver::with_dns(cfg, dns)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            // Bounded: with no CPU for the helper (one CPU) nothing connects.
            let mut p = lab.accept_within(Duration::from_secs(10))?;
            assert!(p.client_closed()?);
            Ok(())
        });
        let t0 = Instant::now();
        let stuck = d.connect_background(WsDial::new(
            ergon_io::Target::new("never-answers.example", 443, "/"),
            lab.client_cfg.clone(),
            1 << 20,
        )?)?;
        let fine = d.connect_background(WsDial::new(
            lab.target("/")?,
            lab.client_cfg.clone(),
            1 << 20,
        )?)?;
        let mut h = Collect::default();
        poll_until(&mut d, &mut h, |h| h.opened.contains(&fine))?;
        let opened_after = t0.elapsed();
        poll_until(&mut d, &mut h, |h| h.closed.iter().any(|c| c.0 == stuck))?;
        let failed_after = t0.elapsed();
        // The helper must use the driver's resolver (and its servers), so the
        // connect ends at its own deadline, not with a system resolver error.
        let why = &h.closed.iter().find(|c| c.0 == stuck).ok_or("no close")?.1;
        assert!(why.contains("timed out"), "{why}");
        assert!(
            opened_after < Duration::from_secs(1),
            "the pinned connect waited {opened_after:?}"
        );
        assert!(
            failed_after < Duration::from_secs(3),
            "the deadline fired after {failed_after:?}"
        );
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn spin_backends_never_sleep_when_idle() -> R {
    for b in Backend::ALL.into_iter().filter(|b| !b.may_sleep()) {
        let lab = Lab::new()?;
        let mut d = match driver(b) {
            Ok(d) => d,
            // A pod with one CPU has no core for SQPOLL (asserted in
            // `each_backend`); the other spin backends run the body.
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {
                continue;
            }
            Err(e) => return Err(format!("{b}: {e}").into()),
        };
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept()?;
                assert!(p.client_closed()?);
                Ok(())
            });
            d.add(WsConn::connect(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let mut h = Collect::default();
            let (v0, _) = ergon_io::clock::thread_ctx_switches();
            for _ in 0..200_000 {
                d.poll(&mut h)?;
            }
            let (v1, _) = ergon_io::clock::thread_ctx_switches();
            assert_eq!(v1 - v0, 0, "{b} slept while idle");
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn connect_never_blocks_the_poll_loop() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                // A slow server: 150 ms before the handshake even starts.
                let (sock, _) = lab.listener.accept()?;
                std::thread::sleep(Duration::from_millis(150));
                let mut p = support::Peer::serve(&lab, sock)?;
                p.send(ws::OP_TEXT, true, b"hi")?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let t0 = Instant::now();
            let id = d.connect(WsDial::new(
                lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            assert!(
                t0.elapsed() < Duration::from_millis(5),
                "connect returned late"
            );
            assert!(d.get(id).is_none(), "not open yet");
            let mut h = Collect::default();
            let mut slowest = Duration::ZERO;
            let end = Instant::now() + Duration::from_secs(10);
            let (slept0, _) = ergon_io::clock::thread_ctx_switches();
            while h.msgs.is_empty() {
                let t = Instant::now();
                d.poll(&mut h)?;
                slowest = slowest.max(t.elapsed());
                assert!(Instant::now() < end, "never opened: {:?}", h.closed);
            }
            let (slept1, _) = ergon_io::clock::thread_ctx_switches();
            assert_eq!(h.opened, vec![id]);
            assert_eq!(h.msgs[0].0, id);
            assert!(d.get(id).is_some());
            // Never waiting for the network: the thread never slept, and no
            // poll came near the server's 150 ms delay (the worst is one
            // handshake step's CPU, slow in a debug build on a shared host).
            if !b.may_sleep() {
                assert_eq!(slept1 - slept0, 0, "{b}: the polling thread slept");
            }
            assert!(
                slowest < Duration::from_millis(100),
                "{b}: a poll took {slowest:?}"
            );
            assert_eq!(d.stats().connects, 1);
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
    })
}

#[test]
fn connect_times_out_and_cleans_up() -> R {
    let lab = Lab::new()?;
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.connect_timeout_ms = 200;
    let mut d: Driver<WsConn> = Driver::new(cfg)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            // Accepts but never speaks TLS.
            let (mut sock, _) = lab.listener.accept()?;
            let mut b = [0u8; 4096];
            loop {
                match std::io::Read::read(&mut sock, &mut b) {
                    // The client closed its socket (a reset if it left data unread).
                    Ok(0) => return Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => return Ok(()),
                    Ok(_) => {}
                    Err(e) => return Err(e.into()),
                }
            }
        });
        let id = d.connect(WsDial::new(
            lab.target("/")?,
            lab.client_cfg.clone(),
            1 << 20,
        )?)?;
        let mut h = Collect::default();
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
        assert_eq!(h.closed[0].0, id);
        assert!(h.closed[0].1.contains("timed out"), "{}", h.closed[0].1);
        assert!(h.opened.is_empty() && d.connecting() == 0);
        assert_eq!(d.stats().connect_failures, 1);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn closing_while_connecting_drops_the_socket() -> R {
    let lab = Lab::new()?;
    let mut d = driver(Backend::RecvBusy)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let (mut sock, _) = lab.listener.accept()?;
            let mut b = [0u8; 4096];
            loop {
                match std::io::Read::read(&mut sock, &mut b) {
                    Ok(0) => return Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => return Ok(()),
                    Ok(_) => {}
                    Err(e) => return Err(e.into()),
                }
            }
        });
        let id = d.connect(WsDial::new(
            lab.target("/")?,
            lab.client_cfg.clone(),
            1 << 20,
        )?)?;
        let mut h = Collect::default();
        for _ in 0..100 {
            d.poll(&mut h)?;
        }
        d.close(id);
        assert_eq!(d.connecting(), 0);
        srv.join().map_err(|_| "server panicked")??;
        // A new connection does not inherit the closed id.
        assert!(d.get(id).is_none());
        assert!(h.opened.is_empty() && h.closed.is_empty());
        Ok(())
    })
}

#[test]
fn frames_sent_with_the_upgrade_then_more_on_every_backend() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let (sock, _) = lab.listener.accept()?;
                let mut p = support::Peer::serve_with(&lab, sock, &[b"first", b"second"])?;
                // Later traffic lands where the armed receive points.
                std::thread::sleep(Duration::from_millis(20));
                for i in 0..50u32 {
                    p.send(ws::OP_TEXT, true, format!("m{i}").as_bytes())?;
                }
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = d.connect(WsDial::new(
                lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let mut h = Collect::default();
            poll_until(&mut d, &mut h, |h| h.msgs.len() == 52)?;
            assert_eq!(h.msgs[0].2, b"first");
            assert_eq!(h.msgs[1].2, b"second");
            assert_eq!(h.msgs[51].2, b"m49");
            assert!(h.closed.is_empty(), "{:?}", h.closed);
            assert!(d.get(id).is_some());
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
    })
}

#[test]
fn background_connect_keeps_handshake_cpu_off_the_loop() -> R {
    let _helpers = dial_helpers();
    // `connect_step_max_ns == 0` is the helper's: the loop never sets a
    // connection up. A pod with one CPU has no core for the helper and
    // refuses typed instead (asserted; the helper half runs on a pod with
    // two CPUs).
    if support::one_cpu()? {
        return background_refuses_on_one_cpu();
    }
    for b in [Backend::RecvBusy, Backend::UringSpin] {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(10))?;
                p.send(ws::OP_TEXT, true, b"hi")?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = d.connect_background(WsDial::new(
                lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let mut h = Collect::default();
            let mut slowest = Duration::ZERO;
            let end = Instant::now() + Duration::from_secs(10);
            while h.msgs.is_empty() {
                let t = Instant::now();
                d.poll(&mut h)?;
                slowest = slowest.max(t.elapsed());
                assert!(Instant::now() < end, "never opened: {:?}", h.closed);
            }
            assert_eq!(h.opened, vec![id]);
            assert_eq!(h.msgs[0].0, id);
            assert_eq!(
                d.stats().connect_step_max_ns,
                0,
                "no set-up step ran on the loop"
            );
            assert!(
                slowest < Duration::from_millis(5),
                "{b}: a poll took {slowest:?}"
            );
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
        .map_err(|e| format!("{b}: {e}"))?;
    }
    Ok(())
}

/// The next TCP client within `wait`, not yet served: a test whose client
/// never connects fails instead of hanging.
fn accept_raw_within(lab: &Lab, wait: Duration) -> R<std::net::TcpStream> {
    lab.listener.set_nonblocking(true)?;
    let end = Instant::now() + wait;
    let sock = loop {
        match lab.listener.accept() {
            Ok((sock, _)) => break sock,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < end => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => return Err(e.into()),
        }
    };
    lab.listener.set_nonblocking(false)?;
    sock.set_nonblocking(false)?;
    Ok(sock)
}

#[test]
fn background_connect_closed_early_is_discarded() -> R {
    let _helpers = dial_helpers();
    // The race is the helper's: the dial is in flight there when the loop
    // closes it. A pod with one CPU has no core for the helper and refuses
    // typed instead (asserted; the helper half runs on a pod with two CPUs).
    if support::one_cpu()? {
        return background_refuses_on_one_cpu();
    }
    let lab = Lab::new()?;
    let mut d = driver(Backend::EpollSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let sock = accept_raw_within(&lab, Duration::from_secs(10))?;
            std::thread::sleep(Duration::from_millis(100));
            let mut p = support::Peer::serve(&lab, sock)?;
            assert!(p.client_closed()?, "the discarded connection is closed");
            Ok(())
        });
        let id = d.connect_background(WsDial::new(
            lab.target("/")?,
            lab.client_cfg.clone(),
            1 << 20,
        )?)?;
        d.close(id);
        let mut h = Collect::default();
        let end = Instant::now() + Duration::from_millis(500);
        while Instant::now() < end {
            d.poll(&mut h)?;
        }
        assert!(h.opened.is_empty() && h.closed.is_empty());
        assert!(d.get(id).is_none() && d.is_empty());
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn cpu_lists_parse() {
    use ergon_io::sys::parse_cpu_list;
    assert_eq!(parse_cpu_list("0-3,6,8-9\n"), vec![0, 1, 2, 3, 6, 8, 9]);
    assert_eq!(parse_cpu_list(""), Vec::<usize>::new());
    assert_eq!(parse_cpu_list("5"), vec![5]);
}

/// The CPUs this process may run on: its cpuset (a Kubernetes pod's CPU
/// set, Docker's `--cpuset-cpus`), as the test thread's mask shows it. The
/// host-wide online list also holds CPUs a pod cannot use.
fn cpuset() -> R<Vec<usize>> {
    std::fs::read_to_string("/proc/thread-self/status")?
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .map(ergon_io::sys::parse_cpu_list)
        .ok_or_else(|| "no Cpus_allowed_list".into())
}

/// The helper's allowed CPUs, read from its `status`.
fn allowed_cpus(task: &std::path::Path) -> R<Vec<usize>> {
    std::fs::read_to_string(task.join("status"))?
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .map(ergon_io::sys::parse_cpu_list)
        .ok_or_else(|| "no Cpus_allowed_list".into())
}

/// A pod with one CPU has no CPU for the dial helper: `connect_background`
/// refuses typed at once (nothing queued, `connecting()` stays 0) rather
/// than start a helper that would time-slice the polling thread. A test
/// whose property needs the helper asserts this refusal and returns.
fn background_refuses_on_one_cpu() -> R {
    let cpus = cpuset()?;
    let poller = *cpus.first().ok_or("an empty cpuset")?;
    std::thread::scope(|s| {
        s.spawn(|| -> R {
            ergon_io::clock::pin_current_thread(poller)?;
            let mut d = driver(Backend::EpollSpin)?;
            let lab = Lab::new()?;
            let r = d.connect_background(WsDial::new(
                lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?);
            match r {
                Err(Error::Cpu(e)) => assert!(
                    matches!(
                        *e,
                        CpuError::NoSpareCpu {
                            helper: HelperThread::Dial,
                            ..
                        }
                    ),
                    "{e}"
                ),
                other => return Err(format!("in cpuset {cpus:?}: {other:?}").into()),
            }
            assert_eq!(d.connecting(), 0);
            Ok(())
        })
        .join()
        .map_err(|_| "panicked")?
    })
}

// The two tests below also run in a pod-like cpuset, the loop on its first
// CPU: `docker run --cpuset-cpus=2,3 --security-opt seccomp=unconfined ...`,
// and `--cpuset-cpus=0`, a pod with one CPU, where they check the refusal.
#[test]
fn background_helper_leaves_the_polling_cpu() -> R {
    let _helpers = dial_helpers();
    let cpus = cpuset()?;
    let poller = *cpus.first().ok_or("an empty cpuset")?;
    if cpus.len() == 1 {
        // One CPU, the poller's: connect_background refuses at once, typed,
        // rather than start a helper that would time-slice the loop.
        return background_refuses_on_one_cpu();
    }
    let lab = Lab::new()?;
    let lab_ref = &lab;
    std::thread::scope(|s| -> R {
        // The poller pins itself, as a busy-spin runtime would: the helper
        // inherits that single-CPU affinity unless it moves.
        let client = s.spawn(move || -> R {
            let lab = lab_ref;
            ergon_io::clock::pin_current_thread(poller)?;
            let mut d = driver(Backend::EpollSpin)?;
            d.connect_background(WsDial::new(
                lab.target("/")?,
                lab.client_cfg.clone(),
                1 << 20,
            )?)?;
            let mut h = Collect::default();
            poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
            let helper = std::fs::read_dir("/proc/self/task")?
                .filter_map(std::result::Result::ok)
                .map(|e| e.path())
                .find(|p| {
                    std::fs::read_to_string(p.join("comm"))
                        .is_ok_and(|c| c.trim() == "ergon-io-dial")
                })
                .ok_or("helper thread not found")?;
            let allowed = allowed_cpus(&helper)?;
            assert!(
                !allowed.is_empty() && !allowed.contains(&poller),
                "helper may run on the polling CPU: {allowed:?}"
            );
            assert!(
                allowed.iter().all(|c| cpus.contains(c)),
                "helper mask {allowed:?} outside the cpuset {cpus:?}"
            );
            Ok(())
        });
        // Bounded: a connect that never starts fails the test, not hangs it,
        // and the client's own error is the one reported.
        let served = lab
            .accept_within(Duration::from_secs(10))
            .and_then(|mut p| {
                p.send(ws::OP_TEXT, true, b"x")?;
                Ok(p)
            });
        client.join().map_err(|_| "client panicked")??;
        served.map(drop)
    })
}

#[test]
fn sqpoll_thread_never_shares_the_polling_cpu() -> R {
    let cpus = cpuset()?;
    let poller = *cpus.first().ok_or("an empty cpuset")?;
    std::thread::scope(|s| -> R {
        s.spawn(move || -> R {
            ergon_io::clock::pin_current_thread(poller)?;
            // SQPOLL threads of earlier tests may still be exiting: only the
            // one this driver creates counts.
            let sqpoll_threads = || -> Vec<std::path::PathBuf> {
                std::fs::read_dir("/proc/self/task")
                    .into_iter()
                    .flatten()
                    .filter_map(std::result::Result::ok)
                    .map(|e| e.path())
                    .filter(|p| {
                        std::fs::read_to_string(p.join("comm"))
                            .is_ok_and(|c| c.starts_with("iou-sqp"))
                    })
                    .collect()
            };
            let before = sqpoll_threads();
            // No sqpoll_cpu configured: the default must avoid the poller's
            // CPU and lie inside the cpuset (io_uring_setup refuses one
            // outside it with EINVAL). With one CPU there is none: refused.
            let made: ergon_io::Result<Driver<WsConn>> =
                Driver::new(Config::new(Backend::UringSqpoll));
            if cpus.len() == 1 {
                match made {
                    Err(Error::Cpu(e)) => assert!(
                        matches!(
                            *e,
                            CpuError::NoSpareCpu {
                                helper: HelperThread::SqPoll,
                                ..
                            }
                        ),
                        "{e}"
                    ),
                    Err(e) => return Err(format!("in cpuset {cpus:?}: {e}").into()),
                    Ok(_) => return Err(format!("uring-sqpoll started in cpuset {cpus:?}").into()),
                }
                return Ok(());
            }
            let _d = made.map_err(|e| format!("uring-sqpoll in cpuset {cpus:?}: {e}"))?;
            // The kernel thread names itself once it runs: look for up to 1 s.
            let deadline = Instant::now() + Duration::from_secs(1);
            let sq = loop {
                if let Some(p) = sqpoll_threads().into_iter().find(|p| !before.contains(p)) {
                    break p;
                }
                if Instant::now() > deadline {
                    return Err("SQPOLL thread not found".into());
                }
                std::thread::sleep(Duration::from_millis(5));
            };
            let allowed = allowed_cpus(&sq)?;
            assert!(
                !allowed.is_empty() && !allowed.contains(&poller),
                "SQPOLL thread may run on the polling CPU: {allowed:?}"
            );
            assert!(
                allowed.iter().all(|c| cpus.contains(c)),
                "SQPOLL mask {allowed:?} outside the cpuset {cpus:?}"
            );
            Ok(())
        })
        .join()
        .map_err(|_| "panicked")?
    })
}
