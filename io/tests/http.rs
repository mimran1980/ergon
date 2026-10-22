//! The HTTP/1.1 client over loopback: keep-alive, framing, limits, timeouts and
//! the combined WebSocket + HTTP driver.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod support;

use std::time::{Duration, Instant};

use ergon_io::placement::HelperThread;
use ergon_io::{
    AnyConn, AnyDial, Backend, Config, ConnId, Driver, Error, Handler, HttpConn, HttpDial,
    HttpHandler, HttpOptions, Response, RxMeta, WsDial, WsHandler, ws,
};
use support::{Lab, Peer, R};

/// Connection, request id, status, body and the `x-echo` header.
type Seen = (ConnId, u64, u16, Vec<u8>, Option<Vec<u8>>);

#[derive(Default)]
struct Collect {
    responses: Vec<Seen>,
    messages: Vec<(ConnId, Vec<u8>)>,
    closed: Vec<(ConnId, String)>,
    opened: Vec<ConnId>,
    /// Closes with requests in flight: the first request not handed to the
    /// kernel, and the cause.
    unanswered: Vec<(ConnId, Option<u64>, String)>,
}

impl Handler for Collect {
    fn on_open(&mut self, id: ConnId) {
        self.opened.push(id);
    }
    fn on_closed(&mut self, id: ConnId, error: Error) {
        if let Error::Unanswered(u) = &error {
            self.unanswered
                .push((id, u.first_unsent, u.cause.to_string()));
        }
        self.closed.push((id, error.to_string()));
    }
}

impl HttpHandler for Collect {
    fn on_response(&mut self, conn: ConnId, request: u64, r: &Response<'_>, _: &RxMeta) {
        self.responses.push((
            conn,
            request,
            r.status,
            r.body.to_vec(),
            r.header("x-echo").map(<[u8]>::to_vec),
        ));
    }
}

impl WsHandler for Collect {
    fn on_message(&mut self, id: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.messages.push((id, payload.to_vec()));
    }
}

fn respond(p: &mut Peer, status: &str, extra: &str, body: &[u8]) -> R {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{extra}\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    p.write(&out)
}

fn poll_until<E>(d: &mut Driver<E>, h: &mut Collect, done: impl Fn(&Collect) -> bool) -> R
where
    E: ergon_io::Endpoint + ergon_io::Receive<Collect>,
{
    let end = Instant::now() + Duration::from_secs(10);
    while !done(h) {
        d.poll(h)?;
        if Instant::now() > end {
            return Err(format!(
                "timed out: {} responses, closed {:?}, stats {:?}",
                h.responses.len(),
                h.closed,
                d.stats()
            )
            .into());
        }
    }
    Ok(())
}

fn http_driver(b: Backend) -> R<Driver<HttpConn>> {
    let mut cfg = Config::new(b);
    cfg.wait_us = cfg.wait_us.min(100);
    Ok(Driver::new(cfg)?)
}

fn dial(lab: &Lab, opts: HttpOptions) -> R<HttpDial> {
    Ok(HttpDial::new(
        lab.target("")?,
        lab.client_cfg.clone(),
        opts,
    )?)
}

#[test]
fn keep_alive_requests_on_every_backend() -> R {
    for b in Backend::ALL {
        let lab = Lab::new()?;
        let mut d = match http_driver(b) {
            Ok(d) => d,
            // A pod with one CPU has no core for SQPOLL (asserted in
            // driver.rs's `each_backend`); the other backends run the body.
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {
                continue;
            }
            Err(e) => return Err(format!("{b}: {e}").into()),
        };
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_tls()?;
                for _ in 0..100 {
                    let req = p.read_request()?;
                    assert_eq!(req.method, "POST");
                    assert_eq!(req.header("x-test"), Some("1"), "fixed header");
                    assert_eq!(
                        req.header("host"),
                        Some(format!("localhost:{}", lab.addr()?.port()).as_str())
                    );
                    let echo = format!("x-echo: {}\r\n", req.path);
                    respond(&mut p, "200 OK", &echo, &req.body)?;
                }
                Ok(())
            });
            let id = d.connect(dial(&lab, HttpOptions::default())?)?;
            let mut h = Collect::default();
            poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
            for i in 0..100u64 {
                let body = format!("{{\"order\":{i}}}");
                let path = format!("/operation?n={i}");
                let rid = d.get_mut(id).ok_or("gone")?.request(
                    "POST",
                    &path,
                    &[("Content-Type", "application/json")],
                    body.as_bytes(),
                )?;
                assert_eq!(rid, i);
                poll_until(&mut d, &mut h, |h| h.responses.len() as u64 > i)?;
                let (cid, req, status, rbody, echo) = &h.responses[usize::try_from(i)?];
                assert_eq!((*cid, *req, *status), (id, i, 200));
                assert_eq!(rbody, body.as_bytes());
                assert_eq!(echo.as_deref(), Some(path.as_bytes()));
            }
            let st = d.get(id).ok_or("gone")?.stats();
            assert_eq!((st.requests, st.responses), (100, 100));
            assert_eq!(st.chunk_copy_bytes, 0);
            assert!(h.closed.is_empty(), "{b}: {:?}", h.closed);
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })
        .map_err(|e| format!("{b}: {e}"))?;
    }
    Ok(())
}

#[test]
fn in_flight_limit_and_pipelining() -> R {
    let lab = Lab::new()?;
    let mut d = http_driver(Backend::EpollSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut one = lab.accept_tls()?;
            let req = one.read_request()?;
            respond(&mut one, "200 OK", "", req.path.as_bytes())?;
            let mut four = lab.accept_tls()?;
            let mut reqs = Vec::new();
            for _ in 0..4 {
                reqs.push(four.read_request()?);
            }
            // All four answers in one write: one record, four responses.
            let mut out = Vec::new();
            for r in &reqs {
                out.extend_from_slice(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                        r.path.len(),
                        r.path
                    )
                    .as_bytes(),
                );
            }
            four.write(&out)?;
            Ok(())
        });
        let mut h = Collect::default();
        let one = d.connect(dial(&lab, HttpOptions::default())?)?;
        poll_until(&mut d, &mut h, |h| h.opened.len() == 1)?;
        let mut c = d.get_mut(one).ok_or("gone")?;
        c.request("GET", "/a", &[], &[])?;
        assert!(matches!(
            c.request("GET", "/b", &[], &[]),
            Err(Error::Busy(_))
        ));
        poll_until(&mut d, &mut h, |h| h.responses.len() == 1)?;
        let four = d.connect(dial(
            &lab,
            HttpOptions {
                max_in_flight: 4,
                ..HttpOptions::default()
            },
        )?)?;
        poll_until(&mut d, &mut h, |h| h.opened.len() == 2)?;
        let mut c = d.get_mut(four).ok_or("gone")?;
        for p in ["/1", "/2", "/3", "/4"] {
            c.request("GET", p, &[], &[])?;
        }
        poll_until(&mut d, &mut h, |h| h.responses.len() == 5)?;
        let got: Vec<_> = h.responses[1..]
            .iter()
            .map(|r| (r.1, r.3.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                (0, b"/1".to_vec()),
                (1, b"/2".to_vec()),
                (2, b"/3".to_vec()),
                (3, b"/4".to_vec())
            ]
        );
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn framing_chunked_interim_head_and_no_content() -> R {
    let lab = Lab::new()?;
    let mut d = http_driver(Backend::UringSpin)?;
    let big: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = lab.accept_tls()?;
            p.read_request()?;
            p.write(b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n")?;
            p.read_request()?;
            p.write(b"HTTP/1.1 200 OK\r\nContent-Length: 1234\r\n\r\n")?; // HEAD: no body follows
            p.read_request()?;
            p.write(b"HTTP/1.1 204 No Content\r\n\r\n")?;
            p.read_request()?;
            respond(&mut p, "200 OK", "", &big)?;
            Ok(())
        });
        let mut h = Collect::default();
        let id = d.connect(dial(&lab, HttpOptions::default())?)?;
        poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
        for (i, method) in ["GET", "HEAD", "DELETE", "GET"].into_iter().enumerate() {
            d.get_mut(id)
                .ok_or("gone")?
                .request(method, "/x", &[], &[])?;
            poll_until(&mut d, &mut h, |h| h.responses.len() > i)?;
        }
        assert_eq!(
            (h.responses[0].2, h.responses[0].3.as_slice()),
            (200, &b"hello world"[..])
        );
        assert_eq!((h.responses[1].2, h.responses[1].3.len()), (200, 0));
        assert_eq!((h.responses[2].2, h.responses[2].3.len()), (204, 0));
        assert_eq!(h.responses[3].3, big);
        let st = d.get(id).ok_or("gone")?.stats();
        assert_eq!(st.chunk_copy_bytes, 11);
        assert!(st.span_copy_bytes > 0, "the 200 kB body spans records");
        assert!(h.closed.is_empty(), "{:?}", h.closed);
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn request_timeout_closes_the_connection() -> R {
    let lab = Lab::new()?;
    let mut d = http_driver(Backend::RecvBusy)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = lab.accept_tls()?;
            p.read_request()?; // never answered
            assert!(p.client_closed()?);
            Ok(())
        });
        let mut h = Collect::default();
        let id = d.connect(dial(
            &lab,
            HttpOptions {
                request_timeout_ms: 50,
                ..HttpOptions::default()
            },
        )?)?;
        poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
        d.get_mut(id)
            .ok_or("gone")?
            .request("GET", "/slow", &[], &[])?;
        let t0 = Instant::now();
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
        assert!(t0.elapsed() >= Duration::from_millis(50));
        assert_eq!(h.closed[0].0, id);
        assert!(h.closed[0].1.contains("timed out"), "{}", h.closed[0].1);
        // The request was handed to the kernel: it may have run.
        assert_eq!(
            h.unanswered,
            vec![(id, None, "timed out: http request".to_owned())]
        );
        assert!(d.get(id).is_none());
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn a_close_with_a_request_in_flight_says_it_was_handed() -> R {
    let lab = Lab::new()?;
    let mut d = http_driver(Backend::RecvBusy)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = lab.accept_tls()?;
            p.read_request()?;
            drop(p); // no reply
            Ok(())
        });
        let mut h = Collect::default();
        let id = d.connect(dial(&lab, HttpOptions::default())?)?;
        poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
        d.get_mut(id).ok_or("gone")?.request("GET", "/", &[], &[])?;
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
        assert_eq!(
            h.unanswered,
            vec![(id, None, "peer closed the connection".to_owned())]
        );
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

/// Pipelined requests the kernel could not take before the server reset
/// the connection: the first of them, and every later one, cannot have
/// reached the server.
#[test]
fn requests_never_handed_to_the_kernel_are_named() -> R {
    const N: usize = 1_000;
    for b in Backend::ALL {
        let lab = Lab::new()?;
        let mut d = match http_driver(b) {
            Ok(d) => d,
            // A pod with one CPU has no core for SQPOLL (asserted in
            // driver.rs's `each_backend`); the other backends run the body.
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {
                continue;
            }
            Err(e) => return Err(format!("{b}: {e}").into()),
        };
        let (go, wait_go) = std::sync::mpsc::channel::<()>();
        let (gone, wait_gone) = std::sync::mpsc::channel::<()>();
        std::thread::scope(|s| -> R {
            let l = &lab;
            let srv = s.spawn(move || -> R<usize> {
                let mut p = l.accept_tls()?;
                // Once the client has queued everything and sent what the
                // kernel took (and stopped polling), read what arrived.
                wait_go.recv_timeout(Duration::from_secs(10))?;
                p.sock.set_read_timeout(Some(Duration::from_millis(300)))?;
                let mut seen = 0;
                while p.read_request().is_ok() {
                    seen += 1;
                }
                socket2::SockRef::from(&p.sock).set_linger(Some(Duration::ZERO))?;
                drop(p); // reset
                gone.send(())?;
                Ok(seen)
            });
            let result = (|| -> R<Option<u64>> {
                let mut h = Collect::default();
                let opts = HttpOptions {
                    max_in_flight: N,
                    ..HttpOptions::default()
                };
                let id = d.connect(dial(&lab, opts)?)?;
                poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
                let body = vec![b'x'; 15_000];
                for _ in 0..N {
                    d.get_mut(id)
                        .ok_or("gone")?
                        .request("POST", "/", &[], &body)?;
                }
                for _ in 0..100 {
                    d.poll(&mut h)?;
                }
                go.send(())?;
                wait_gone.recv_timeout(Duration::from_secs(30))?;
                poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
                assert_eq!(h.unanswered.len(), 1, "{b}: {:?}", h.closed);
                Ok(h.unanswered[0].1)
            })();
            let seen = srv.join().map_err(|_| "server panicked")??;
            let first_unsent = result?;
            // 15 MB cannot all be in the two kernels' buffers: on epoll the
            // driver counts exactly what `send` took. io_uring counts what
            // it submitted, an upper bound, so it may name none.
            if !b.is_uring() {
                let k = first_unsent.ok_or_else(|| format!("{b}: every request handed"))?;
                assert!(usize::try_from(k)? < N, "{b}: {k}");
            }
            let handed = first_unsent.map_or(N, |k| usize::try_from(k).unwrap_or(N));
            assert!(seen <= handed, "{b}: server read {seen}, handed {handed}");
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn connection_close_delivers_then_closes() -> R {
    let lab = Lab::new()?;
    let mut d = http_driver(Backend::EpollSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = lab.accept_tls()?;
            p.read_request()?;
            respond(
                &mut p,
                "503 Service Unavailable",
                "Connection: close\r\n",
                b"later",
            )?;
            Ok(())
        });
        let mut h = Collect::default();
        let id = d.connect(dial(&lab, HttpOptions::default())?)?;
        poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
        d.get_mut(id).ok_or("gone")?.request("GET", "/", &[], &[])?;
        poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
        assert_eq!(
            (h.responses[0].2, h.responses[0].3.as_slice()),
            (503, &b"later"[..])
        );
        assert!(d.get(id).is_none());
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn invalid_requests_are_rejected_before_sending() -> R {
    let lab = Lab::new()?;
    let mut d = http_driver(Backend::EpollSpin)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = lab.accept_tls()?;
            let req = p.read_request()?;
            assert_eq!(req.path, "/ok", "only the valid request reached the server");
            respond(&mut p, "200 OK", "", b"")?;
            Ok(())
        });
        let mut h = Collect::default();
        let id = d.connect(dial(&lab, HttpOptions::default())?)?;
        poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
        let mut c = d.get_mut(id).ok_or("gone")?;
        assert!(c.request("GET", "/a\r\nX-Evil: 1", &[], &[]).is_err());
        assert!(c.request("GET", "/", &[("X", "a\nb")], &[]).is_err());
        assert!(c.request("POST", "/", &[], &vec![0; 20_000]).is_err());
        assert!(
            c.request("GET", "/", &[("Content-Length", "0")], &[])
                .is_err()
        );
        assert!(c.request("GET /x", "/", &[], &[]).is_err());
        assert!(c.request("GET", "/a b", &[], &[]).is_err());
        assert_eq!(c.in_flight(), 0);
        c.request("GET", "/ok", &[], &[])?;
        poll_until(&mut d, &mut h, |h| h.responses.len() == 1)?;
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn websocket_and_http_share_one_driver() -> R {
    let lab = Lab::new()?;
    let mut d: Driver<AnyConn> = Driver::new(Config::new(Backend::UringSpin))?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut wsp = lab.accept()?;
            let mut hp = lab.accept_tls()?;
            for i in 0..10 {
                wsp.send(ws::OP_TEXT, true, format!("tick{i}").as_bytes())?;
                let r = hp.read_request()?;
                respond(&mut hp, "200 OK", "", r.path.as_bytes())?;
            }
            Ok(())
        });
        let mut h = Collect::default();
        let w = d.connect(AnyDial::from(WsDial::new(
            lab.target("/")?,
            lab.client_cfg.clone(),
            1 << 20,
        )?))?;
        poll_until(&mut d, &mut h, |h| h.opened.len() == 1)?;
        let http = d.connect(AnyDial::from(dial(&lab, HttpOptions::default())?))?;
        poll_until(&mut d, &mut h, |h| h.opened.len() == 2)?;
        for i in 0..10 {
            let mut any = d.get_mut(http).ok_or("gone")?;
            any.http()
                .ok_or("not http")?
                .request("GET", &format!("/o{i}"), &[], &[])?;
            poll_until(&mut d, &mut h, |h| h.responses.len() > i)?;
        }
        poll_until(&mut d, &mut h, |h| h.messages.len() == 10)?;
        assert!(h.messages.iter().all(|m| m.0 == w));
        assert!(h.responses.iter().all(|r| r.0 == http));
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn hostile_lengths_close_the_connection_without_panicking() -> R {
    for reply in [
        &b"HTTP/1.1 200 OK\r\nContent-Length: 18446744073709551615\r\n\r\nx"[..],
        &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nffffffffffffffff\r\nx"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 6\r\n\r\nhello!"[..],
    ] {
        let lab = Lab::new()?;
        let mut d = http_driver(Backend::EpollSpin)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_tls()?;
                p.read_request()?;
                p.write(reply)?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let mut h = Collect::default();
            let id = d.connect(dial(&lab, HttpOptions::default())?)?;
            poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
            d.get_mut(id).ok_or("gone")?.request("GET", "/", &[], &[])?;
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            assert_eq!(h.responses.len(), 0, "{:?}", h.responses);
            assert!(h.closed[0].1.starts_with("http:"), "{}", h.closed[0].1);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn reserved_and_malformed_headers_are_rejected() -> R {
    let lab = Lab::new()?;
    for (k, v) in [
        ("Content-Length", "5"),
        ("transfer-encoding", "chunked"),
        ("Host", "x"),
        ("Bad Name", "1"),
        ("X", "a\0b"),
    ] {
        let t = lab.target("")?.with_header(k, v);
        assert!(
            HttpDial::new(t, lab.client_cfg.clone(), HttpOptions::default()).is_err(),
            "{k}"
        );
    }
    assert!(
        HttpDial::new(
            lab.target("")?,
            lab.client_cfg.clone(),
            HttpOptions {
                max_in_flight: 1 << 30,
                ..HttpOptions::default()
            }
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn signed_request_reaches_the_server_with_a_valid_signature() -> R {
    let lab = Lab::new()?;
    let mut d = http_driver(Backend::EpollSpin)?;
    let secret = b"test-secret";
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = lab.accept_tls()?;
            let req = p.read_request()?;
            assert_eq!(req.method, "POST");
            assert_eq!(req.header("x-api-key"), Some("key-1"));
            let (path, query) = req.path.split_once('?').ok_or("no query")?;
            assert_eq!(path, "/operation");
            let (payload, sig) = query.split_once("&signature=").ok_or("no signature")?;
            assert!(payload.starts_with("resource=example&action=create&timestamp="));
            let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret);
            let tag = ring::hmac::sign(&key, payload.as_bytes());
            let hex = tag.as_ref().iter().fold(String::new(), |mut h, b| {
                use std::fmt::Write as _;
                let _ = write!(h, "{b:02x}");
                h
            });
            assert_eq!(sig, hex);
            respond(&mut p, "200 OK", "", b"{}")?;
            Ok(())
        });
        let mut h = Collect::default();
        let target = lab.target("")?.with_header("X-API-Key", "key-1");
        let id = d.connect(HttpDial::new(
            target,
            lab.client_cfg.clone(),
            HttpOptions::default(),
        )?)?;
        poll_until(&mut d, &mut h, |h| !h.opened.is_empty())?;
        let signer = ergon_io::Signer::hmac_sha256(secret);
        let mut c = d.get_mut(id).ok_or("gone")?;
        assert!(
            c.signed_request("POST", "/operation?x=1", "a=1", &[], &signer)
                .is_err()
        );
        assert!(
            c.signed_request("POST", "/operation", "a=b c", &[], &signer)
                .is_err()
        );
        c.signed_request(
            "POST",
            "/operation",
            "resource=example&action=create",
            &[],
            &signer,
        )?;
        poll_until(&mut d, &mut h, |h| h.responses.len() == 1)?;
        assert_eq!(h.responses[0].2, 200);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}
