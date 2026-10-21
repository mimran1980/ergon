//! The WebSocket client's lifecycle over loopback: the close handshake both
//! ways, pings, timeouts, limits, subprotocols, refusals and large sends.
#![cfg(target_os = "linux")]
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod support;

use std::time::{Duration, Instant};

use ergon_io::placement::HelperThread;
use ergon_io::{
    AnyConn, Backend, Config, ConnId, Driver, Error, Handler, HttpHandler, Receive, Response,
    RxMeta, WsConn, WsHandler, WsOptions, ws,
};
use support::{Lab, Peer, R};

#[derive(Default)]
struct App {
    msgs: Vec<(ConnId, u8, Vec<u8>)>,
    closed: Vec<(ConnId, Error)>,
}

impl Handler for App {
    fn on_closed(&mut self, id: ConnId, error: Error) {
        self.closed.push((id, error));
    }
}

impl WsHandler for App {
    fn on_message(&mut self, id: ConnId, opcode: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.msgs.push((id, opcode, payload.to_vec()));
    }
}

/// For `AnyConn` drivers; these tests send no HTTP request.
impl HttpHandler for App {
    fn on_response(&mut self, _: ConnId, _: u64, _: &Response<'_>, _: &RxMeta) {}
}

fn driver(b: Backend) -> R<Driver<WsConn>> {
    let mut cfg = Config::new(b);
    cfg.wait_us = cfg.wait_us.min(100);
    Ok(Driver::new(cfg)?)
}

fn poll_until<E: Receive<App>>(d: &mut Driver<E>, h: &mut App, done: impl Fn(&App) -> bool) -> R {
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

fn each_backend(test: impl Fn(Backend) -> R) -> R {
    let mut ran = 0;
    for b in Backend::ALL {
        match test(b) {
            Ok(()) => ran += 1,
            // A pod with one CPU has no core for `uring-sqpoll`'s kernel
            // thread: `Driver::new` refuses typed (asserted; covered in full
            // by driver.rs's `sqpoll_thread_never_shares_the_polling_cpu`).
            // The body still runs on every other backend.
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {}
            Err(e) => return Err(format!("{b}: {e}").into()),
        }
    }
    assert!(ran >= Backend::ALL.len() - 1, "only {ran} backends ran");
    Ok(())
}

fn connect(lab: &Lab, d: &mut Driver<WsConn>, opts: WsOptions) -> R<ConnId> {
    Ok(d.add(WsConn::connect_with(
        &lab.target("/")?,
        lab.client_cfg.clone(),
        opts,
    )?)?)
}

/// Accepts and upgrades one client; waits and reads time out, so a test
/// whose client misbehaves fails instead of hanging.
fn peer(lab: &Lab) -> R<Peer> {
    lab.accept_within(Duration::from_secs(15))
}

/// No pings: a test that is not about them gets no surprise frames.
fn quiet() -> WsOptions {
    WsOptions {
        ping_interval_ms: 0,
        ..WsOptions::default()
    }
}

fn close_of(e: &Error) -> R<&ws::Close> {
    match e {
        Error::WsClose(c) => Ok(c),
        other => Err(format!("not a close: {other:?}").into()),
    }
}

// ANCHOR: server_close
#[test]
fn a_server_close_is_answered_with_its_code_then_reported() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                p.send(ws::OP_TEXT, true, b"last")?;
                p.send_close(Some(1001), "restart")?;
                // Data after a Close breaks the protocol: never delivered.
                p.send(ws::OP_TEXT, true, b"late")?;
                // The reply echoes the code; the server then closes TCP.
                assert_eq!(p.recv()?, (ws::OP_CLOSE, 1001u16.to_be_bytes().to_vec()));
                Ok(())
            });
            let id = connect(&lab, &mut d, quiet())?;
            let mut h = App::default();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            assert_eq!(h.msgs.len(), 1, "{:?}", h.msgs);
            assert_eq!(h.msgs[0].2, b"last");
            assert_eq!(h.closed[0].0, id);
            let c = close_of(&h.closed[0].1)?;
            assert_eq!(c.code, Some(1001));
            assert_eq!(c.reason, "restart");
            assert!(c.by_server);
            Ok(())
        })
    })
}
// ANCHOR_END: server_close

#[test]
fn a_server_that_never_closes_tcp_is_closed_at_the_close_timeout() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                p.send_close(Some(1000), "")?;
                assert_eq!(p.recv()?.0, ws::OP_CLOSE);
                // Holds the socket open: the client must close it.
                assert!(p.client_closed()?);
                Ok(())
            });
            let opts = WsOptions {
                close_timeout_ms: 100,
                ..quiet()
            };
            connect(&lab, &mut d, opts)?;
            let mut h = App::default();
            let t0 = Instant::now();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            let took = t0.elapsed();
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            let c = close_of(&h.closed[0].1)?;
            assert_eq!((c.code, c.by_server), (Some(1000), true));
            assert!(took >= Duration::from_millis(90), "closed after {took:?}");
            Ok(())
        })
    })
}

// ANCHOR: client_close
#[test]
fn a_client_close_waits_for_the_servers_close() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                let mut want = 1000u16.to_be_bytes().to_vec();
                want.extend_from_slice(b"done");
                assert_eq!(p.recv()?, (ws::OP_CLOSE, want));
                // Frames sent before the server's Close still arrive.
                p.send(ws::OP_TEXT, true, b"in flight")?;
                p.send_close(Some(1000), "")?;
                // The handshake is complete: the client closes TCP.
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = connect(&lab, &mut d, quiet())?;
            let mut c = d.get_mut(id).ok_or("gone")?;
            c.close(1000, "done")?;
            assert!(c.is_closing());
            // Nothing may follow a Close.
            assert!(c.send_text("more").is_err());
            assert!(c.close(1000, "again").is_err());
            let mut h = App::default();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            assert_eq!(h.msgs.len(), 1);
            let c = close_of(&h.closed[0].1)?;
            assert_eq!((c.code, c.by_server), (Some(1000), false));
            Ok(())
        })
    })
}
// ANCHOR_END: client_close

#[test]
fn a_client_close_without_an_answer_times_out() -> R {
    let lab = Lab::new()?;
    let mut d = driver(Backend::EpollSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            assert_eq!(p.recv()?.0, ws::OP_CLOSE);
            assert!(p.client_closed()?);
            Ok(())
        });
        let opts = WsOptions {
            close_timeout_ms: 100,
            ..quiet()
        };
        let id = connect(&lab, &mut d, opts)?;
        d.get_mut(id).ok_or("gone")?.close(4000, "")?;
        let mut h = App::default();
        let t0 = Instant::now();
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
        let took = t0.elapsed();
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        assert!(
            matches!(h.closed[0].1, Error::Timeout("websocket close")),
            "{:?}",
            h.closed
        );
        assert!(took >= Duration::from_millis(90), "closed after {took:?}");
        Ok(())
    })
}

#[test]
fn close_codes_and_reasons_are_checked_before_sending() -> R {
    let lab = Lab::new()?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            assert!(p.client_closed()?);
            Ok(())
        });
        let mut c = WsConn::connect_with(&lab.target("/")?, lab.client_cfg.clone(), quiet())?;
        assert!(c.close(1005, "").is_err(), "1005 is never sent");
        assert!(c.close(1000, &"r".repeat(124)).is_err(), "reason too long");
        assert!(!c.is_closing(), "a refused close changes nothing");
        drop(c);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

// ANCHOR: pings
#[test]
fn a_quiet_connection_is_pinged_and_closed_without_a_pong() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                assert_eq!(p.recv()?.0, ws::OP_PING);
                // Never answers.
                assert!(p.client_closed()?);
                Ok(())
            });
            let opts = WsOptions {
                ping_interval_ms: 50,
                pong_timeout_ms: 100,
                ..WsOptions::default()
            };
            let id = connect(&lab, &mut d, opts)?;
            let mut h = App::default();
            let t0 = Instant::now();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            let took = t0.elapsed();
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            assert_eq!(h.closed[0].0, id);
            assert!(
                matches!(h.closed[0].1, Error::Timeout("websocket pong")),
                "{:?}",
                h.closed
            );
            assert!(took >= Duration::from_millis(140), "closed after {took:?}");
            Ok(())
        })
    })
}
// ANCHOR_END: pings

#[test]
fn pongs_keep_a_quiet_connection_open() -> R {
    let lab = Lab::new()?;
    let mut d = driver(Backend::RecvBusy)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            for _ in 0..8 {
                let (op, body) = p.recv()?;
                assert_eq!(op, ws::OP_PING);
                p.send(ws::OP_PONG, true, &body)?;
            }
            p.send(ws::OP_TEXT, true, b"end")?;
            assert!(p.client_closed()?);
            Ok(())
        });
        let opts = WsOptions {
            ping_interval_ms: 20,
            pong_timeout_ms: 200,
            ..WsOptions::default()
        };
        let id = connect(&lab, &mut d, opts)?;
        let mut h = App::default();
        poll_until(&mut d, &mut h, |h| {
            !h.msgs.is_empty() || !h.closed.is_empty()
        })?;
        assert!(h.closed.is_empty(), "{:?}", h.closed);
        let st = d.get(id).ok_or("gone")?.stats();
        assert!(st.pings_sent >= 8 && st.pongs >= 8, "{st:?}");
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn a_busy_connection_is_never_pinged() -> R {
    let lab = Lab::new()?;
    let mut d = driver(Backend::EpollSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            for _ in 0..30 {
                p.send(ws::OP_TEXT, true, b"tick")?;
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(p.client_closed()?);
            Ok(())
        });
        let opts = WsOptions {
            ping_interval_ms: 50,
            pong_timeout_ms: 50,
            ..WsOptions::default()
        };
        let id = connect(&lab, &mut d, opts)?;
        let mut h = App::default();
        poll_until(&mut d, &mut h, |h| {
            h.msgs.len() == 30 || !h.closed.is_empty()
        })?;
        assert!(h.closed.is_empty(), "{:?}", h.closed);
        assert_eq!(d.get(id).ok_or("gone")?.stats().pings_sent, 0);
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn the_idle_timeout_closes_a_silent_connection() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                p.send(ws::OP_TEXT, true, b"one")?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let opts = WsOptions {
                idle_timeout_ms: 100,
                ..quiet()
            };
            connect(&lab, &mut d, opts)?;
            let mut h = App::default();
            poll_until(&mut d, &mut h, |h| !h.msgs.is_empty())?;
            let t0 = Instant::now();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            let took = t0.elapsed();
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            assert!(
                matches!(h.closed[0].1, Error::Timeout("websocket idle")),
                "{:?}",
                h.closed
            );
            assert!(took >= Duration::from_millis(90), "closed after {took:?}");
            Ok(())
        })
    })
}

#[test]
fn a_sleeping_backend_checks_timers_on_every_poll() -> R {
    // Each idle poll sleeps `wait_us` (10 ms here). Checking timers only
    // every 64th poll would put the idle timeout about 640 ms late.
    for b in [Backend::EpollBusy, Backend::UringNapi] {
        let lab = Lab::new()?;
        let mut cfg = Config::new(b);
        cfg.wait_us = 10_000;
        let mut d: Driver<WsConn> = Driver::new(cfg)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let opts = WsOptions {
                idle_timeout_ms: 50,
                ..quiet()
            };
            connect(&lab, &mut d, opts)?;
            let mut h = App::default();
            let t0 = Instant::now();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            let took = t0.elapsed();
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            assert!(
                matches!(h.closed[0].1, Error::Timeout("websocket idle")),
                "{b}: {:?}",
                h.closed
            );
            assert!(
                took < Duration::from_millis(300),
                "{b}: closed after {took:?}"
            );
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn control_frames_between_fragments_are_handled() -> R {
    let lab = Lab::new()?;
    let mut d = driver(Backend::UringSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            p.send(ws::OP_TEXT, false, b"ab")?;
            p.send(ws::OP_PING, true, b"x")?;
            p.send(ws::OP_CONT, false, b"cd")?;
            p.send(ws::OP_PONG, true, b"y")?;
            p.send(ws::OP_CONT, true, b"ef")?;
            assert_eq!(p.recv()?, (ws::OP_PONG, b"x".to_vec()));
            assert!(p.client_closed()?);
            Ok(())
        });
        let id = connect(&lab, &mut d, quiet())?;
        let mut h = App::default();
        poll_until(&mut d, &mut h, |h| {
            !h.msgs.is_empty() || !h.closed.is_empty()
        })?;
        assert!(h.closed.is_empty(), "{:?}", h.closed);
        assert_eq!(h.msgs[0].1, ws::OP_TEXT);
        assert_eq!(h.msgs[0].2, b"abcdef");
        let st = d.get(id).ok_or("gone")?.stats();
        assert_eq!((st.pings, st.pongs), (1, 1), "{st:?}");
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn websockets_in_an_any_conn_driver_get_the_close_handshake_and_timers() -> R {
    let lab = Lab::new()?;
    let mut d: Driver<AnyConn> = Driver::new(Config::new(Backend::EpollSpin))?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            // First connection: the server closes it with a code.
            let mut p = peer(&lab)?;
            p.send_close(Some(1001), "restart")?;
            assert_eq!(p.recv()?.0, ws::OP_CLOSE);
            drop(p);
            // Second: silent, so the idle timeout closes it.
            let mut p = peer(&lab)?;
            assert!(p.client_closed()?);
            Ok(())
        });
        let any = |opts| -> R<AnyConn> {
            Ok(AnyConn::Ws(WsConn::connect_with(
                &lab.target("/")?,
                lab.client_cfg.clone(),
                opts,
            )?))
        };
        let mut h = App::default();
        d.add(any(quiet())?)?;
        poll_until(&mut d, &mut h, |h| h.closed.len() == 1)?;
        let c = close_of(&h.closed[0].1)?;
        assert_eq!((c.code, c.by_server), (Some(1001), true));
        let opts = WsOptions {
            idle_timeout_ms: 100,
            ..quiet()
        };
        d.add(any(opts)?)?;
        poll_until(&mut d, &mut h, |h| h.closed.len() == 2)?;
        assert!(
            matches!(h.closed[1].1, Error::Timeout("websocket idle")),
            "{:?}",
            h.closed
        );
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn fragments_of_two_messages_never_interleave() -> R {
    // RFC 6455 section 5.4: a new data frame inside a fragmented message, or
    // a continuation with no message started, is a protocol error.
    let cases: [(&[(u8, bool)], &str); 3] = [
        (
            &[(ws::OP_TEXT, false), (ws::OP_BINARY, true)],
            "inside a fragmented",
        ),
        (
            &[(ws::OP_TEXT, false), (ws::OP_TEXT, false)],
            "inside a fragmented",
        ),
        (&[(ws::OP_CONT, true)], "continuation without"),
    ];
    for (frames, want) in cases {
        let lab = Lab::new()?;
        let mut d = driver(Backend::EpollSpin)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                for &(op, fin) in frames {
                    p.send(op, fin, b"ab")?;
                }
                let _ = p.client_closed();
                Ok(())
            });
            connect(&lab, &mut d, quiet())?;
            let mut h = App::default();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            assert!(h.msgs.is_empty(), "{frames:?}: {:?}", h.msgs);
            assert!(
                h.closed[0].1.to_string().contains(want),
                "{frames:?}: {:?}",
                h.closed
            );
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn invalid_utf8_split_over_fragments_is_refused_when_checked() -> R {
    let lab = Lab::new()?;
    let mut d = driver(Backend::EpollSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            // Each fragment alone is valid; the message is not (a lead byte
            // with no continuation byte).
            p.send(ws::OP_TEXT, false, &[b'1', 0xE2])?;
            p.send(ws::OP_CONT, true, b"2")?;
            let _ = p.client_closed();
            Ok(())
        });
        let opts = WsOptions {
            validate_utf8: true,
            ..quiet()
        };
        connect(&lab, &mut d, opts)?;
        let mut h = App::default();
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        assert!(h.msgs.is_empty(), "{:?}", h.msgs);
        assert!(
            h.closed[0].1.to_string().contains("UTF-8"),
            "{:?}",
            h.closed
        );
        Ok(())
    })
}

#[test]
fn invalid_utf8_is_refused_only_when_checked() -> R {
    for validate in [false, true] {
        let lab = Lab::new()?;
        let mut d = driver(Backend::EpollSpin)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                // A code point split across two fragments is valid text.
                p.send(ws::OP_TEXT, false, &[b'1', 0xE2, 0x82])?;
                p.send(ws::OP_CONT, true, &[0xAC])?;
                p.send(ws::OP_BINARY, true, &[0xFF])?;
                p.send(ws::OP_TEXT, true, &[b'2', 0xFF, 0xFE])?;
                let _ = p.client_closed();
                Ok(())
            });
            let opts = WsOptions {
                validate_utf8: validate,
                ..quiet()
            };
            let id = connect(&lab, &mut d, opts)?;
            let mut h = App::default();
            if validate {
                poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
                assert_eq!(h.msgs.len(), 2, "text then binary: {:?}", h.msgs);
                assert!(
                    h.closed[0].1.to_string().contains("UTF-8"),
                    "{:?}",
                    h.closed
                );
            } else {
                poll_until(&mut d, &mut h, |h| h.msgs.len() == 3)?;
                assert!(h.closed.is_empty() && d.get(id).is_some());
            }
            assert_eq!(h.msgs[0].2, "1\u{20AC}".as_bytes());
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn messages_over_max_message_are_refused() -> R {
    for fragmented in [false, true] {
        let lab = Lab::new()?;
        let mut d = driver(Backend::EpollSpin)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = peer(&lab)?;
                p.send(ws::OP_BINARY, true, &[1; 1000])?;
                if fragmented {
                    p.send(ws::OP_BINARY, false, &[2; 600])?;
                    p.send(ws::OP_CONT, true, &[2; 401])?;
                } else {
                    p.send(ws::OP_BINARY, true, &[2; 1001])?;
                }
                let _ = p.client_closed();
                Ok(())
            });
            let opts = WsOptions {
                max_message: 1000,
                ..quiet()
            };
            connect(&lab, &mut d, opts)?;
            let mut h = App::default();
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            assert_eq!(h.msgs.len(), 1, "exactly the limit is accepted");
            assert!(
                h.closed[0].1.to_string().contains("too large"),
                "{:?}",
                h.closed
            );
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn large_messages_go_out_across_records() -> R {
    each_backend(|b| {
        let lab = Lab::new()?;
        let mut d = driver(b)?;
        let big: Vec<u8> = (0..100_003u32).map(|i| (i % 251) as u8).collect();
        let text = "0123456789".repeat(2_000);
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R<Vec<(u8, Vec<u8>)>> {
                let mut p = peer(&lab)?;
                let got = vec![p.recv()?, p.recv()?, p.recv()?];
                p.send(ws::OP_TEXT, true, b"got")?;
                assert!(p.client_closed()?);
                Ok(got)
            });
            let id = connect(&lab, &mut d, quiet())?;
            let mut c = d.get_mut(id).ok_or("gone")?;
            c.send_binary(&big)?;
            c.send_text(&text)?;
            c.send_binary(b"small")?;
            let mut h = App::default();
            poll_until(&mut d, &mut h, |h| {
                !h.msgs.is_empty() || !h.closed.is_empty()
            })?;
            assert!(h.closed.is_empty(), "{:?}", h.closed);
            drop(d);
            let got = srv.join().map_err(|_| "server panicked")??;
            assert_eq!(got[0], (ws::OP_BINARY, big.clone()));
            assert_eq!(got[1], (ws::OP_TEXT, text.as_bytes().to_vec()));
            assert_eq!(got[2], (ws::OP_BINARY, b"small".to_vec()));
            Ok(())
        })
    })
}

#[test]
fn send_frame_takes_data_and_short_control_frames_only() -> R {
    let lab = Lab::new()?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            assert_eq!(p.recv()?, (ws::OP_PING, vec![1; 125]));
            assert!(p.client_closed()?);
            Ok(())
        });
        let mut c = WsConn::connect_with(&lab.target("/")?, lab.client_cfg.clone(), quiet())?;
        assert!(
            c.send_frame(ws::OP_CLOSE, &[]).is_err(),
            "close goes through close()"
        );
        assert!(c.send_frame(ws::OP_CONT, b"x").is_err());
        assert!(c.send_frame(0x3, b"x").is_err(), "reserved opcode");
        assert!(
            c.send_frame(ws::OP_PING, &[1; 126]).is_err(),
            "control over 125"
        );
        c.ping(&[1; 125])?;
        let mut d = driver(Backend::EpollSpin)?;
        d.add(c)?;
        d.flush()?;
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn subprotocols_are_offered_and_the_choice_reported() -> R {
    let lab = Lab::new()?;
    let opts = WsOptions {
        protocols: vec!["v2.json".to_owned(), "v1.json".to_owned()],
        ..quiet()
    };
    for (pick, ok) in [(Some("v1.json"), true), (None, true), (Some("v3"), false)] {
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R<String> {
                let (sock, _) = lab.listener.accept()?;
                let (mut p, req) = Peer::serve_custom(&lab, sock, |_, accept| {
                    let proto = pick
                        .map(|p| format!("Sec-WebSocket-Protocol: {p}\r\n"))
                        .unwrap_or_default();
                    format!(
                        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
                         Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n{proto}\r\n"
                    )
                })?;
                let _ = p.client_closed();
                Ok(req
                    .header("sec-websocket-protocol")
                    .unwrap_or_default()
                    .to_owned())
            });
            let c = WsConn::connect_with(&lab.target("/")?, lab.client_cfg.clone(), opts.clone());
            match (&c, ok) {
                (Ok(c), true) => assert_eq!(c.protocol(), pick),
                (Err(e), false) => assert!(e.to_string().contains("subprotocol"), "{e}"),
                _ => return Err(format!("{pick:?}: {:?}", c.err()).into()),
            }
            drop(c);
            let offered = srv.join().map_err(|_| "server panicked")??;
            assert_eq!(offered, "v2.json, v1.json");
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn a_refused_upgrade_reports_the_status() -> R {
    let lab = Lab::new()?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let (sock, _) = lab.listener.accept()?;
            let (mut p, _) = Peer::serve_custom(&lab, sock, |_, _| {
                "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 30\r\nContent-Length: 4\r\n\r\nslow"
                    .to_owned()
            })?;
            let _ = p.client_closed();
            Ok(())
        });
        let c = WsConn::connect_with(&lab.target("/")?, lab.client_cfg.clone(), quiet());
        match c {
            Err(Error::Upgrade {
                status: 429,
                ref body,
                retry_after_ms: Some(30_000),
                retry_after_invalid: false,
            }) if body == "slow" => {}
            other => return Err(format!("{:?}", other.err()).into()),
        }
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

/// `auto_frames` counts the frames the connection sends by itself (the
/// Pong to a server Ping, the timer's Ping, the echo of a server Close), so
/// an application can charge them to a per-connection message limit. The
/// application's own frames are not in it.
#[test]
fn auto_frames_counts_what_the_connection_sends_by_itself() -> R {
    use std::sync::atomic::{AtomicBool, Ordering};
    let lab = Lab::new()?;
    let mut d = driver(Backend::EpollSpin)?;
    let echoed = AtomicBool::new(false);
    let counted = AtomicBool::new(false);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = peer(&lab)?;
            // The application's own frames.
            assert_eq!(p.recv()?, (ws::OP_TEXT, b"order".to_vec()));
            assert_eq!(p.recv()?, (ws::OP_PING, b"app".to_vec()));
            // A server Ping: answered by the connection.
            p.send(ws::OP_PING, true, b"hi")?;
            assert_eq!(p.recv()?, (ws::OP_PONG, b"hi".to_vec()));
            // Quiet: the timer's Ping.
            let (op, body) = p.recv()?;
            assert_eq!(op, ws::OP_PING);
            p.send(ws::OP_PONG, true, &body)?;
            // A server Close: echoed.
            p.send_close(Some(1000), "")?;
            assert_eq!(p.recv()?.0, ws::OP_CLOSE);
            echoed.store(true, Ordering::Release);
            // TCP stays open until the client has read its counters.
            let end = Instant::now() + Duration::from_secs(10);
            while !counted.load(Ordering::Acquire) && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(())
        });
        let opts = WsOptions {
            ping_interval_ms: 50,
            pong_timeout_ms: 0,
            close_timeout_ms: 10_000,
            ..WsOptions::default()
        };
        let id = connect(&lab, &mut d, opts)?;
        {
            let mut c = d.get_mut(id).ok_or("gone")?;
            c.send_text("order")?;
            c.ping(b"app")?;
        }
        let mut h = App::default();
        let end = Instant::now() + Duration::from_secs(10);
        while !echoed.load(Ordering::Acquire) {
            d.poll(&mut h)?;
            if Instant::now() > end || !h.closed.is_empty() {
                counted.store(true, Ordering::Release);
                return Err(format!("no Close echo: {:?}", h.closed).into());
            }
        }
        let st = d.get(id).ok_or("gone")?.stats();
        counted.store(true, Ordering::Release);
        assert_eq!(st.pings, 1, "{st:?}");
        assert!(st.pings_sent >= 1, "{st:?}");
        // The Pong, the timer's Pings and the Close echo; not "order" or the
        // application's Ping.
        assert_eq!(st.auto_frames, st.pings + st.pings_sent + 1, "{st:?}");
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}
