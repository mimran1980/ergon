//! `HttpClient`: the native pool and (feature `curl`) libcurl behind one API.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ergon_io::{
    ClientHandler, Error, HttpClient, HttpClientConfig, HttpImpl, Response, Sent, Signer,
};
use support::{Lab, Peer, R, StopOnDrop};

// ANCHOR: client_handler
#[derive(Default)]
struct Seen {
    ok: Vec<(u64, u16, String)>,
    errors: Vec<(u64, Sent, String)>,
}

impl ClientHandler for Seen {
    fn on_response(&mut self, request: u64, r: &Response<'_>) {
        self.ok.push((
            request,
            r.status,
            String::from_utf8_lossy(r.body).into_owned(),
        ));
    }
    // `Sent::No`: the server cannot have seen it; `Sent::Maybe`: it may
    // have run (reconcile an order by its client order id).
    fn on_error(&mut self, request: u64, sent: Sent, error: Error) {
        self.errors.push((request, sent, error.to_string()));
    }
}
// ANCHOR_END: client_handler

// ANCHOR: native
/// The native implementation: `recv-busy`, connections opened in the
/// background.
fn native(lab: &Lab) -> R<HttpClient> {
    let mut cfg = HttpClientConfig::new(HttpImpl::Native);
    cfg.pool = 2; // warm connections, one request in flight each
    // A pod with one CPU has no core for the connect helper: opt in to
    // opening on the loop there (without this, `new` returns `Error::Cpu`).
    cfg.driver.inline_connect_fallback = true;
    let client = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
    Ok(client)
}
// ANCHOR_END: native

// ANCHOR: curl
/// libcurl (feature `curl`): the same API. Without the feature, `new`
/// returns `Error::Unsupported`.
fn curl(lab: &Lab) -> R<HttpClient> {
    let mut cfg = HttpClientConfig::new(HttpImpl::Curl);
    cfg.pool = 2; // the most connections libcurl may use at once
    cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string()); // libcurl's trust store
    let client = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
    Ok(client)
}
// ANCHOR_END: curl

// ANCHOR: get
/// One GET, the same code for both implementations: wait for a free
/// connection, send, then poll until the answer.
fn get(client: &mut HttpClient, path: &str) -> R<(u16, String)> {
    let mut seen = Seen::default();
    let end = Instant::now() + Duration::from_secs(10);
    while client.idle() == 0 {
        client.poll(&mut seen)?; // the pool connects in the background
        if Instant::now() > end {
            return Err("no connection".into());
        }
    }
    let id = client.request("GET", path, &[], &[])?; // returns at once
    while seen.ok.is_empty() && seen.errors.is_empty() {
        client.poll(&mut seen)?; // one non-blocking pass
        if Instant::now() > end {
            return Err("no answer".into());
        }
    }
    let (got, status, body) = seen.ok.pop().ok_or_else(|| format!("{:?}", seen.errors))?;
    assert_eq!(got, id);
    Ok((status, body))
}
// ANCHOR_END: get

#[test]
fn one_get_through_each_implementation() -> R {
    let lab = Lab::new()?;
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| lab.serve_http(&stop, None).map_err(|e| e.to_string()));
        let result = (|| -> R {
            let _stop = StopOnDrop(&stop); // a failed assertion fails, never hangs
            // The test server answers with the request's path.
            let want = (200, "/time".to_owned());
            assert_eq!(get(&mut native(&lab)?, "/time")?, want);
            if cfg!(feature = "curl") {
                assert_eq!(get(&mut curl(&lab)?, "/time")?, want);
            } else {
                assert!(curl(&lab).is_err_and(|e| e.to_string().contains("unsupported")));
            }
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        srv.join().map_err(|_| "server panicked")??;
        result
    })
}

fn poll_until(c: &mut HttpClient, h: &mut Seen, done: impl Fn(&HttpClient, &Seen) -> bool) -> R {
    let end = Instant::now() + Duration::from_secs(10);
    while !done(c, h) {
        c.poll(h)?;
        if Instant::now() > end {
            return Err(format!("timed out: ok {:?}, errors {:?}", h.ok, h.errors).into());
        }
    }
    Ok(())
}

/// Requests, a signed request and the busy limit, through either implementation.
fn exercise(lab: &Lab, cfg: &HttpClientConfig) -> R {
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| lab.serve_http(&stop, None).map_err(|e| e.to_string()));
        let result = (|| -> R {
            let _stop = StopOnDrop(&stop);
            let target = lab.target("")?.with_header("X-API-Key", "k");
            let mut c = HttpClient::new(cfg, target, lab.client_cfg.clone())?;
            assert_eq!(c.implementation(), cfg.implementation);
            let mut h = Seen::default();
            poll_until(&mut c, &mut h, |c, _| c.idle() == cfg.pool)?;
            let mut ids = Vec::new();
            for i in 0..cfg.pool {
                ids.push(c.request("GET", &format!("/r{i}"), &[], &[])?);
            }
            assert!(matches!(
                c.request("GET", "/x", &[], &[]),
                Err(Error::Busy(_))
            ));
            poll_until(&mut c, &mut h, |_, h| h.ok.len() == cfg.pool)?;
            for (i, id) in ids.iter().enumerate() {
                let got = h.ok.iter().find(|r| r.0 == *id).ok_or("missing response")?;
                assert_eq!((got.1, got.2.as_str()), (200, format!("/r{i}").as_str()));
            }
            let signer = Signer::hmac_sha256(b"secret");
            let sid = c.signed_request("POST", "/operation", "resource=example", &[], &signer)?;
            poll_until(&mut c, &mut h, |_, h| h.ok.iter().any(|r| r.0 == sid))?;
            let body = &h.ok.iter().find(|r| r.0 == sid).ok_or("missing")?.2;
            assert!(
                body.starts_with("/operation?resource=example&timestamp="),
                "{body}"
            );
            assert!(body.contains("&signature="));
            // HEAD: the length is announced, no body follows; both
            // implementations must complete it at once.
            let hid = c.request("HEAD", "/head", &[], &[])?;
            poll_until(&mut c, &mut h, |_, h| {
                h.ok.iter().any(|r| r.0 == hid) || !h.errors.is_empty()
            })?;
            let head = h.ok.iter().find(|r| r.0 == hid).ok_or("HEAD failed")?;
            assert_eq!((head.1, head.2.as_str()), (200, ""));
            assert!(h.errors.is_empty(), "{:?}", h.errors);
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        srv.join().map_err(|_| "server panicked")??;
        result
    })
}

#[test]
fn native_pool_requests_signs_and_limits() -> R {
    let lab = Lab::new()?;
    let mut cfg = native_cfg();
    cfg.pool = 3;
    exercise(&lab, &cfg)
}

#[test]
fn native_pool_replaces_a_dropped_connection() -> R {
    let lab = Lab::new()?;
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| -> R {
        // The first connection closes after one response.
        let srv = s.spawn(|| lab.serve_http(&stop, Some(1)).map_err(|e| e.to_string()));
        let result = (|| -> R {
            let _stop = StopOnDrop(&stop);
            let mut cfg = native_cfg();
            cfg.pool = 1;
            let mut c = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
            let mut h = Seen::default();
            poll_until(&mut c, &mut h, |c, _| c.idle() == 1)?;
            c.request("GET", "/one", &[], &[])?;
            poll_until(&mut c, &mut h, |_, h| h.ok.len() == 1)?;
            // The server closed it; the pool reopens a connection by itself.
            poll_until(&mut c, &mut h, |c, _| c.idle() == 1)?;
            let id = c.request("GET", "/two", &[], &[])?;
            poll_until(&mut c, &mut h, |_, h| h.ok.iter().any(|r| r.0 == id))?;
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        srv.join().map_err(|_| "server panicked")??;
        result
    })
}

#[derive(Default)]
struct Counting {
    seen: Seen,
    connect_errors: usize,
}

impl ClientHandler for Counting {
    fn on_response(&mut self, request: u64, r: &Response<'_>) {
        self.seen.on_response(request, r);
    }
    fn on_error(&mut self, request: u64, sent: Sent, error: Error) {
        self.seen.on_error(request, sent, error);
    }
    fn on_connect_error(&mut self, _error: Error) {
        self.connect_errors += 1;
    }
}

/// A handler whose [`ClientHandler::connect_gate`] runs `gate`: the gate
/// lives on the handler so it can share the handler's `Budget`.
struct Gated<F> {
    seen: Seen,
    gate: F,
}

impl<F: FnMut() -> std::result::Result<(), u64>> ClientHandler for Gated<F> {
    fn on_response(&mut self, request: u64, r: &Response<'_>) {
        self.seen.on_response(request, r);
    }
    fn on_error(&mut self, request: u64, sent: Sent, error: Error) {
        self.seen.on_error(request, sent, error);
    }
    fn connect_gate(&mut self) -> std::result::Result<(), u64> {
        (self.gate)()
    }
}

#[test]
fn connect_failures_are_reported_not_returned() -> R {
    let lab = Lab::new()?;
    // TCP refuses a multicast destination inside connect() itself
    // (ENETUNREACH): every attempt fails synchronously, on the polling thread.
    let addr = std::net::SocketAddr::from(([224, 0, 0, 1], 443));
    let target = ergon_io::Target::new("localhost", 443, "").with_addr(addr);
    for background in [false, true] {
        let mut cfg = native_cfg();
        cfg.pool = 2;
        cfg.background_connect = background;
        let mut c = HttpClient::new(&cfg, target.clone(), lab.client_cfg.clone())?;
        let mut h = Counting::default();
        let end = Instant::now() + Duration::from_secs(10);
        while h.connect_errors < 4 {
            c.poll(&mut h)?;
            assert!(
                Instant::now() < end,
                "background={background}: connect errors not reported"
            );
        }
        assert_eq!(c.idle(), 0);
    }
    Ok(())
}

#[test]
fn unhandled_connect_failures_are_counted_and_warned() -> R {
    // `Seen` does not implement on_connect_error: the default warns (stderr,
    // from a background thread, rate-limited) and the client counts them.
    let lab = Lab::new()?;
    let addr = std::net::SocketAddr::from(([224, 0, 0, 1], 443));
    let target = ergon_io::Target::new("localhost", 443, "").with_addr(addr);
    let mut cfg = native_cfg();
    cfg.pool = 1;
    let mut c = HttpClient::new(&cfg, target, lab.client_cfg)?;
    let mut h = Seen::default();
    let end = Instant::now() + Duration::from_secs(10);
    while c.connect_failures() < 2 {
        c.poll(&mut h)?;
        assert!(Instant::now() < end, "connect failures not counted");
    }
    assert!(h.errors.is_empty(), "{:?}", h.errors);
    Ok(())
}

#[test]
fn curl_is_unsupported_without_the_feature_or_works_with_it() -> R {
    let lab = Lab::new()?;
    let mut cfg = HttpClientConfig::new(HttpImpl::Curl);
    cfg.pool = 2;
    cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
    // `exercise` sends a signed POST: the test server answers it again
    // harmlessly if libcurl re-sends it.
    cfg.curl_allow_non_idempotent_methods = true;
    if cfg!(feature = "curl") {
        for drive in [
            ergon_io::CurlDrive::Perform,
            ergon_io::CurlDrive::SocketAction,
        ] {
            cfg.curl_drive = drive;
            let lab = Lab::new()?;
            cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
            exercise(&lab, &cfg).map_err(|e| format!("{drive:?}: {e}"))?;
        }
    } else {
        let r = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone());
        assert!(matches!(r, Err(Error::Unsupported(_))));
    }
    Ok(())
}

// ---- What became of a failed request: `Sent`, and one error per cause ----

/// Accepts one client within 10 s and completes only the TLS handshake; the
/// server's reads then fail after 10 s with no data, so a test whose client
/// misbehaves fails instead of hanging.
fn accept_tls(lab: &Lab) -> R<Peer> {
    lab.listener.set_nonblocking(true)?;
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        match lab.listener.accept() {
            Ok((sock, _)) => {
                sock.set_nonblocking(false)?;
                sock.set_read_timeout(Some(Duration::from_secs(10)))?;
                return Peer::serve_tls(lab, sock);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > end {
                    return Err("no client connected".into());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Closes the server's end with a TCP reset.
fn reset(p: Peer) -> R {
    socket2::SockRef::from(&p.sock).set_linger(Some(Duration::ZERO))?;
    drop(p);
    Ok(())
}

#[cfg(feature = "curl")]
fn respond(p: &mut Peer, head: &str, body: &str) -> R {
    p.write(
        format!(
            "HTTP/1.1 {head}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    )
}

/// A native client's settings for tests that need its pool on any pod:
/// background connects where the pod has a second CPU, and the documented
/// `inline_connect_fallback` on a pod with one (where `connect_background`
/// would otherwise refuse — `one_cpu_refuses_background_connects_unless_inline_is_allowed`).
const fn native_cfg() -> HttpClientConfig {
    let mut cfg = HttpClientConfig::new(HttpImpl::Native);
    cfg.driver.inline_connect_fallback = true;
    cfg
}

/// A pool of `pool` connections (native) or handles (libcurl) with
/// `timeout_ms` per request; libcurl takes POST, PUT and PATCH only with
/// `allow_writes`.
fn client(
    lab: &Lab,
    imp: HttpImpl,
    pool: usize,
    timeout_ms: u32,
    allow_writes: bool,
) -> R<HttpClient> {
    let mut cfg = HttpClientConfig::new(imp);
    cfg.pool = pool;
    cfg.driver.inline_connect_fallback = true; // see `native_cfg`
    cfg.request_timeout_ms = timeout_ms;
    cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
    cfg.curl_allow_non_idempotent_methods = allow_writes;
    Ok(HttpClient::new(
        &cfg,
        lab.target("")?,
        lab.client_cfg.clone(),
    )?)
}

fn implementations() -> Vec<HttpImpl> {
    let mut v = vec![HttpImpl::Native];
    if cfg!(feature = "curl") {
        v.push(HttpImpl::Curl);
    }
    v
}

/// Sends `method path` once a connection is free, then polls until it ends.
fn one(c: &mut HttpClient, method: &str, path: &str) -> R<(u64, Seen)> {
    let mut h = Seen::default();
    poll_until(c, &mut h, |c, _| c.idle() > 0)?;
    let id = c.request(method, path, &[], &[])?;
    poll_until(c, &mut h, |_, h| !h.ok.is_empty() || !h.errors.is_empty())?;
    Ok((id, h))
}

#[test]
fn a_request_the_kernel_never_took_is_sent_no() -> R {
    let lab = Lab::new()?;
    let (go, wait_go) = std::sync::mpsc::channel::<()>();
    let (gone, wait_gone) = std::sync::mpsc::channel::<()>();
    std::thread::scope(|s| -> R {
        let l = &lab;
        let srv = s.spawn(move || -> R {
            let p = accept_tls(l)?;
            wait_go.recv_timeout(Duration::from_secs(10))?;
            reset(p)?;
            gone.send(())?;
            Ok(())
        });
        let result = (|| -> R {
            let mut c = client(&lab, HttpImpl::Native, 1, 5_000, false)?;
            let mut h = Seen::default();
            poll_until(&mut c, &mut h, |c, _| c.idle() == 1)?;
            // The server resets the connection before the request is sent:
            // `request` only queues it, and the next poll finds the reset
            // before sending.
            go.send(())?;
            wait_gone.recv_timeout(Duration::from_secs(10))?;
            std::thread::sleep(Duration::from_millis(50));
            let id = c.request("POST", "/operation", &[], b"x")?;
            poll_until(&mut c, &mut h, |_, h| !h.errors.is_empty())?;
            assert_eq!(
                (h.errors[0].0, h.errors[0].1),
                (id, Sent::No),
                "{:?}",
                h.errors
            );
            assert_eq!(h.ok, vec![]);
            Ok(())
        })();
        srv.join().map_err(|_| "server panicked")??;
        result
    })
}

#[test]
fn a_request_the_server_read_but_never_answered_is_sent_maybe() -> R {
    for imp in implementations() {
        let lab = Lab::new()?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R<String> {
                let mut p = accept_tls(&lab)?;
                let req = p.read_request()?;
                drop(p); // closed without a reply
                Ok(req.path)
            });
            let r = one(
                &mut client(&lab, imp, 1, 5_000, true)?,
                "POST",
                "/operation",
            );
            let path = srv.join().map_err(|_| "server panicked")??;
            let (id, h) = r?;
            assert_eq!(path, "/operation");
            assert_eq!(h.errors.len(), 1, "{imp:?}: {:?}", h.errors);
            assert_eq!((h.errors[0].0, h.errors[0].1), (id, Sent::Maybe), "{imp:?}");
            assert_eq!(h.ok, vec![]);
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn a_timeout_is_one_error_on_both_implementations() -> R {
    for imp in implementations() {
        let lab = Lab::new()?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = accept_tls(&lab)?;
                p.read_request()?; // never answered
                let _ = p.client_closed();
                Ok(())
            });
            let mut c = client(&lab, imp, 1, 200, false)?;
            let r = one(&mut c, "GET", "/slow");
            drop(c); // libcurl keeps the connection; closing it ends the server
            srv.join().map_err(|_| "server panicked")??;
            let (id, h) = r?;
            assert_eq!(
                h.errors,
                vec![(id, Sent::Maybe, "timed out: http request".to_owned())],
                "{imp:?}"
            );
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn unreadable_response_headers_are_an_error_not_an_empty_list() -> R {
    // 70 headers: more than either implementation parses (64).
    let many = "X-H: 1\r\n".repeat(70);
    for imp in implementations() {
        let lab = Lab::new()?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = accept_tls(&lab)?;
                p.read_request()?;
                p.write(format!("HTTP/1.1 200 OK\r\n{many}Content-Length: 0\r\n\r\n").as_bytes())?;
                let _ = p.client_closed();
                Ok(())
            });
            let mut c = client(&lab, imp, 1, 5_000, false)?;
            let r = one(&mut c, "GET", "/many");
            drop(c);
            srv.join().map_err(|_| "server panicked")??;
            let (id, h) = r?;
            assert!(h.ok.is_empty(), "{imp:?}: delivered {:?}", h.ok);
            assert_eq!(h.errors.len(), 1, "{imp:?}");
            assert_eq!((h.errors[0].0, h.errors[0].1), (id, Sent::Maybe), "{imp:?}");
            assert!(
                h.errors[0].2.starts_with("http:"),
                "{imp:?}: {}",
                h.errors[0].2
            );
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn signed_request_at_signs_the_callers_timestamp() -> R {
    for imp in implementations() {
        let lab = Lab::new()?;
        let stop = AtomicBool::new(false);
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| lab.serve_http(&stop, None).map_err(|e| e.to_string()));
            let result = (|| -> R {
                let _stop = StopOnDrop(&stop);
                let mut c = client(&lab, imp, 1, 5_000, false)?;
                let mut h = Seen::default();
                poll_until(&mut c, &mut h, |c, _| c.idle() == 1)?;
                let signer = Signer::hmac_sha256(b"secret");
                let id = c.signed_request_at(
                    "GET",
                    "/operation",
                    "resource=X",
                    &[],
                    &signer,
                    1_234_567,
                )?;
                poll_until(&mut c, &mut h, |_, h| {
                    !h.ok.is_empty() || !h.errors.is_empty()
                })?;
                let got =
                    h.ok.iter()
                        .find(|r| r.0 == id)
                        .ok_or_else(|| format!("{:?}", h.errors))?;
                assert!(
                    got.2
                        .starts_with("/operation?resource=X&timestamp=1234567&signature="),
                    "{imp:?}: {}",
                    got.2
                );
                Ok(())
            })();
            stop.store(true, Ordering::Relaxed);
            srv.join().map_err(|_| "server panicked")??;
            result
        })?;
    }
    Ok(())
}

#[cfg(feature = "curl")]
#[test]
fn curl_refuses_methods_it_could_send_twice_unless_allowed() -> R {
    let lab = Lab::new()?;
    let mut c = client(&lab, HttpImpl::Curl, 8, 5_000, false)?;
    for m in ["POST", "PUT", "PATCH", "post", "PURGE"] {
        match c.request(m, "/operation", &[], b"x") {
            Err(Error::CurlResend { ref method }) if method == m => {}
            other => return Err(format!("{m}: {other:?}").into()),
        }
    }
    let signer = Signer::hmac_sha256(b"secret");
    assert!(matches!(
        c.signed_request("POST", "/operation", "resource=X", &[], &signer),
        Err(Error::CurlResend { .. })
    ));
    // Methods a second copy cannot harm go out without opting in.
    for m in ["GET", "HEAD", "OPTIONS", "DELETE"] {
        c.request(m, "/x", &[], &[])?;
    }
    // Opted in: the caller accepts the risk.
    let mut c = client(&lab, HttpImpl::Curl, 1, 5_000, true)?;
    c.request("POST", "/operation", &[], b"x")?;
    Ok(())
}

/// Pins the hazard behind the refusal: libcurl sends a POST again when the
/// reused connection it went out on dies before any reply byte. The native
/// client reports `Sent::Maybe` instead (see above) and never re-sends.
#[cfg(feature = "curl")]
#[test]
fn libcurl_sends_a_post_again_when_its_reused_connection_dies() -> R {
    let lab = Lab::new()?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R<Vec<(usize, String, String)>> {
            let mut seen = Vec::new();
            let mut p = accept_tls(&lab)?;
            let r = p.read_request()?;
            seen.push((1, r.method, r.path));
            respond(&mut p, "200 OK", "warm")?;
            let r = p.read_request()?;
            seen.push((1, r.method, r.path));
            drop(p); // the order may have run; no reply
            let mut p = accept_tls(&lab)?;
            let r = p.read_request()?;
            seen.push((2, r.method, r.path));
            respond(&mut p, "200 OK", "placed")?;
            let _ = p.client_closed();
            Ok(seen)
        });
        let result = (|| -> R<Vec<(u16, String)>> {
            let mut c = client(&lab, HttpImpl::Curl, 1, 5_000, true)?;
            let (_, warm) = one(&mut c, "GET", "/warm")?;
            let (_, order) = one(&mut c, "POST", "/operation")?;
            assert!(
                warm.errors.is_empty() && order.errors.is_empty(),
                "{:?}",
                order.errors
            );
            Ok(warm
                .ok
                .into_iter()
                .chain(order.ok)
                .map(|r| (r.1, r.2))
                .collect())
        })();
        let seen = srv.join().map_err(|_| "server panicked")??;
        assert_eq!(
            result?,
            vec![(200, "warm".to_owned()), (200, "placed".to_owned())]
        );
        let post = |c| (c, "POST".to_owned(), "/operation".to_owned());
        assert_eq!(
            seen,
            vec![(1, "GET".to_owned(), "/warm".to_owned()), post(1), post(2)]
        );
        Ok(())
    })
}

#[cfg(feature = "curl")]
#[test]
fn curl_reports_a_refused_connect_as_sent_no() -> R {
    let lab = Lab::new()?;
    let closed = std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?;
    let mut cfg = HttpClientConfig::new(HttpImpl::Curl);
    cfg.pool = 1;
    cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
    cfg.curl_allow_non_idempotent_methods = true;
    let target = ergon_io::Target::new("localhost", closed.port(), "").with_addr(closed);
    let mut c = HttpClient::new(&cfg, target, lab.client_cfg)?;
    let (id, h) = one(&mut c, "POST", "/operation")?;
    assert_eq!(h.errors.len(), 1, "{:?}", h.errors);
    assert_eq!(
        (h.errors[0].0, h.errors[0].1),
        (id, Sent::No),
        "{:?}",
        h.errors
    );
    Ok(())
}

// ---- Pool reconnects: jitter and the connect gate ----

/// Accepts `n` clients within 10 s; no TLS.
fn accept_n(lab: &Lab, n: usize) -> R<Vec<std::net::TcpStream>> {
    lab.listener.set_nonblocking(true)?;
    let end = Instant::now() + Duration::from_secs(10);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        match lab.listener.accept() {
            Ok((sock, _)) => out.push(sock),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > end {
                    return Err(format!("{} of {n} clients connected", out.len()).into());
                }
                std::thread::sleep(Duration::from_micros(100));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(out)
}

#[test]
fn dropped_pool_connections_reconnect_spread_out() -> R {
    const POOL: usize = 20;
    let lab = Lab::new()?;
    let (all_open, wait_open) = std::sync::mpsc::channel::<()>();
    let (dropped, wait_dropped) = std::sync::mpsc::channel::<Instant>();
    std::thread::scope(|s| -> R {
        let l = &lab;
        let srv = s.spawn(move || -> R {
            let mut peers = Vec::new();
            for sock in accept_n(l, POOL)? {
                sock.set_nonblocking(false)?;
                peers.push(Peer::serve_tls(l, sock)?);
            }
            wait_open.recv_timeout(Duration::from_secs(10))?;
            let t0 = Instant::now();
            drop(peers); // every connection closes at once
            dropped.send(t0)?;
            Ok(())
        });
        let result = (|| -> R<(Instant, Vec<Instant>)> {
            let mut cfg = native_cfg();
            cfg.pool = POOL;
            cfg.reconnect_seed = 7; // the same delays every run
            let mut c = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
            let mut h = Seen::default();
            poll_until(&mut c, &mut h, |c, _| c.idle() == POOL)?;
            all_open.send(())?;
            let t0 = wait_dropped.recv_timeout(Duration::from_secs(10))?;
            // A gate that passes every connect and notes when it starts: the
            // reconnect schedule itself, whatever the connects then take.
            let starts = std::cell::RefCell::new(Vec::new());
            let mut g = Gated {
                seen: Seen::default(),
                gate: || {
                    starts.borrow_mut().push(Instant::now());
                    Ok(())
                },
            };
            let end = Instant::now() + Duration::from_secs(10);
            while starts.borrow().len() < POOL {
                c.poll_gated(&mut g)?;
                if Instant::now() > end {
                    return Err(format!("{} of {POOL} reconnects", starts.borrow().len()).into());
                }
            }
            Ok((t0, starts.into_inner()))
        })();
        srv.join().map_err(|_| "server panicked")??;
        let (t0, starts) = result?;
        let at: Vec<Duration> = starts
            .iter()
            .map(|s| s.saturating_duration_since(t0))
            .collect();
        // Not all at once: spread over the first delay (below 100 ms).
        let spread = at[POOL - 1].saturating_sub(at[0]);
        assert!(
            spread >= Duration::from_millis(40),
            "within {spread:?}: {at:?}"
        );
        for (i, &t) in at.iter().enumerate() {
            let n = at[i..]
                .iter()
                .take_while(|&&u| u < t + Duration::from_millis(10))
                .count();
            assert!(n <= POOL / 2, "{n} reconnects within 10 ms: {at:?}");
        }
        assert!(at[POOL - 1] < Duration::from_millis(200), "{at:?}");
        Ok(())
    })
}

#[test]
fn a_gated_pool_opens_only_through_its_gate() -> R {
    use ergon_io::limit::{BucketSpec, Budget, Charge, Class};
    // ANCHOR: pool_gate
    // The pool's own bucket: venues' connect limits count WebSocket
    // connects, not HTTP ones. The gate lives on the handler, so it shares
    // the handler's budget with `on_response`.
    struct BudgetGate {
        seen: Seen,
        budget: Budget,
        connects: ergon_io::limit::BucketId,
        now: i64,
    }
    impl ClientHandler for BudgetGate {
        fn on_response(&mut self, request: u64, r: &Response<'_>) {
            self.seen.on_response(request, r);
        }
        fn on_error(&mut self, request: u64, sent: Sent, error: Error) {
            self.seen.on_error(request, sent, error);
        }
        fn connect_gate(&mut self) -> std::result::Result<(), u64> {
            self.budget
                .try_charge(&[Charge::new(self.connects, 1)], Class::Normal, self.now)
                .map(drop)
                .map_err(|l| l.wait_ns(self.now))
        }
    }
    let lab = Lab::new()?;
    let budget = Budget::new(vec![BucketSpec::rolling("http-connects", 2, 1_000, 10)])?;
    let connects = budget.id("http-connects").ok_or("no bucket")?;
    let mut cfg = native_cfg();
    cfg.pool = 4;
    cfg.connect_gate = true; // `new` opens nothing: every open asks the gate
    let mut c = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
    // ANCHOR_END: pool_gate
    let mut h = BudgetGate {
        seen: Seen::default(),
        budget,
        connects,
        now: 0,
    };
    assert!(
        matches!(c.poll(&mut h), Err(Error::Config(_))),
        "poll cannot ask the gate"
    );
    // A gate that refuses for 1 s: nothing connects, and each connection
    // asks once, then waits.
    {
        let mut refused = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
        let asked = std::cell::Cell::new(0);
        let mut g = Gated {
            seen: Seen::default(),
            gate: || {
                asked.set(asked.get() + 1);
                Err(1_000_000_000)
            },
        };
        let end = Instant::now() + Duration::from_millis(300);
        while Instant::now() < end {
            refused.poll_gated(&mut g)?;
        }
        assert_eq!(asked.get(), 4, "each connection asks once, then waits");
        lab.listener.set_nonblocking(true)?;
        assert!(
            matches!(lab.listener.accept(), Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock),
            "a connection opened without the gate"
        );
    }
    let stop = AtomicBool::new(false);
    let mut client = Some(c);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| lab.serve_http(&stop, None).map_err(|e| e.to_string()));
        let result = (|| -> R {
            let _stop = StopOnDrop(&stop);
            // Dropped on the way out, even on a failed assertion: the server
            // ends once its connections close.
            let mut c = client.take().ok_or("no client")?;
            let t0 = Instant::now();
            while c.idle() < 4 {
                h.now = i64::try_from(t0.elapsed().as_nanos())?; // the budget's clock
                // ANCHOR: pool_gate_poll
                c.poll_gated(&mut h)?;
                // ANCHOR_END: pool_gate_poll
                if t0.elapsed() > Duration::from_secs(10) {
                    return Err(format!("{} of 4 open", c.idle()).into());
                }
            }
            // Two per second: the last two wait for the window (1 s, plus a
            // 100 ms guard), and no longer: a refusal does not grow the
            // backoff.
            let took = t0.elapsed();
            assert!(took >= Duration::from_millis(1_100), "{took:?}");
            assert!(took < Duration::from_millis(1_600), "{took:?}");
            let st = h.budget.stats(h.connects, h.now)?;
            assert_eq!(st.charged, 4);
            assert!(st.limited > 0);
            Ok(())
        })();
        srv.join().map_err(|_| "server panicked")??;
        result
    })
}

#[cfg(feature = "curl")]
#[test]
fn the_connect_gate_is_native_only() -> R {
    let lab = Lab::new()?;
    let mut cfg = HttpClientConfig::new(HttpImpl::Curl);
    cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
    cfg.connect_gate = true;
    let r = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone());
    assert!(
        matches!(r, Err(Error::Config(_))),
        "libcurl opens its own connections"
    );
    let mut c = curl(&lab)?;
    let mut h = Gated {
        seen: Seen::default(),
        gate: || Ok(()),
    };
    assert!(matches!(c.poll_gated(&mut h), Err(Error::Unsupported(_))));
    Ok(())
}

/// A pod with one CPU (`docker run --cpuset-cpus=0`): background connects
/// have no CPU for the driver's helper, so `HttpClient::new` fails at once
/// with `Error::Cpu` rather than retry unseen; with
/// `driver.inline_connect_fallback` the pool opens its connections on the
/// loop and serves requests. With more CPUs both clients work.
#[test]
fn one_cpu_refuses_background_connects_unless_inline_is_allowed() -> R {
    let cpus = std::fs::read_to_string("/proc/thread-self/status")?
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .map(ergon_io::sys::parse_cpu_list)
        .ok_or("no Cpus_allowed_list")?;
    let lab = Lab::new()?;
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| lab.serve_http(&stop, None).map_err(|e| e.to_string()));
        let result = (|| -> R {
            let _stop = StopOnDrop(&stop);
            let mut cfg = HttpClientConfig::new(HttpImpl::Native);
            cfg.pool = 1;
            let made = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone());
            assert_eq!(
                matches!(made, Err(Error::Cpu(_))),
                cpus.len() == 1,
                "cpuset {cpus:?}: {:?}",
                made.as_ref().err()
            );
            drop(made);
            cfg.driver.inline_connect_fallback = true;
            let mut c = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
            assert_eq!(get(&mut c, "/x")?, (200, "/x".to_owned()));
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        srv.join().map_err(|_| "server panicked")??;
        result
    })
}
