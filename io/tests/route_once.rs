//! The quick-ACK route check runs once per connection, never per request:
//! counted with `route::lookups`, one netlink exchange per connection a
//! driver takes or a client opens. Its own test binary, so no other test's
//! connections are counted.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod support;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ergon_io::route::{self, QuickAckCheck};
use ergon_io::{
    Backend, ClientHandler, Config, ConnId, Driver, Error, Handler, HttpClient, HttpClientConfig,
    HttpConn, HttpDial, HttpHandler, HttpImpl, HttpOptions, Response, RxMeta,
};
use support::{Lab, Peer, R};

#[derive(Default)]
struct Opened {
    opened: usize,
    closed: Vec<String>,
}

impl Handler for Opened {
    fn on_open(&mut self, _: ConnId) {
        self.opened += 1;
    }
    fn on_closed(&mut self, _: ConnId, error: Error) {
        self.closed.push(error.to_string());
    }
}

impl HttpHandler for Opened {
    fn on_response(&mut self, _: ConnId, _: u64, _: &Response<'_>, _: &RxMeta) {}
}

#[derive(Clone, Copy, Debug)]
enum How {
    Connect,
    Background,
    Add,
}

/// Opens one connection through the driver and returns how many lookups
/// that took.
fn lookups_for_one(policy: QuickAckCheck, how: How) -> R<u64> {
    let lab = Lab::new()?;
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.quickack_check = policy;
    // A pod with one CPU has no core for the connect helper: opt in to
    // connecting on the loop there (one lookup per connection either way).
    cfg.inline_connect_fallback = true;
    let mut d: Driver<HttpConn> = Driver::new(cfg)?;
    let before = route::lookups();
    std::thread::scope(|s| -> R {
        let server = s.spawn(|| {
            lab.accept_tls_within(Duration::from_secs(10))
                .map_err(|e| e.to_string())
        });
        let dial = HttpDial::new(
            lab.target("/")?,
            lab.client_cfg.clone(),
            HttpOptions::default(),
        )?;
        match how {
            How::Connect => {
                d.connect(dial)?;
            }
            How::Background => {
                d.connect_background(dial)?;
            }
            How::Add => {
                d.add(dial.connect_blocking()?)?;
            }
        }
        let mut h = Opened::default();
        let end = Instant::now() + Duration::from_secs(10);
        while !matches!(how, How::Add) && h.opened == 0 && h.closed.is_empty() {
            d.poll(&mut h)?;
            if Instant::now() > end {
                return Err("connect timed out".into());
            }
        }
        let _peer = server.join().map_err(|_| "server panicked")??;
        if let Some(e) = h.closed.first() {
            return Err(format!("{how:?} closed: {e}").into());
        }
        // Polling an open connection looks nothing up.
        for _ in 0..100 {
            d.poll(&mut h)?;
        }
        // Under `warn` the lookup runs on ergon-io's warning thread.
        if !ergon_io::warnings::flush(Duration::from_secs(10)) {
            return Err("the warning thread did not catch up".into());
        }
        Ok(())
    })?;
    Ok(route::lookups() - before)
}

#[derive(Default)]
struct Replies {
    ok: usize,
    errors: Vec<String>,
}

impl ClientHandler for Replies {
    fn on_response(&mut self, _: u64, _: &Response<'_>) {
        self.ok += 1;
    }
    fn on_error(&mut self, _: u64, _: ergon_io::Sent, error: Error) {
        self.errors.push(error.to_string());
    }
}

/// An HTTP/1.1 server that answers every request and counts the
/// connections it accepts.
fn serve(lab: &Lab, stop: &AtomicBool, accepted: &AtomicUsize) -> R {
    lab.listener.set_nonblocking(true)?;
    std::thread::scope(|s| -> R {
        while !stop.load(Ordering::Relaxed) {
            match lab.listener.accept() {
                Ok((sock, _)) => {
                    accepted.fetch_add(1, Ordering::Relaxed);
                    sock.set_nonblocking(false)?;
                    s.spawn(move || {
                        let Ok(mut p) = Peer::serve_tls(lab, sock) else {
                            return;
                        };
                        while let Ok(req) = p.read_request() {
                            let body = req.path.as_bytes();
                            let mut out = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                                body.len()
                            )
                            .into_bytes();
                            out.extend_from_slice(body);
                            if p.write(&out).is_err() {
                                return;
                            }
                        }
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    })
}

/// `rounds` rounds of `pool` concurrent requests through `imp`; returns the
/// lookups made and the connections the server accepted.
fn pool_lookups(imp: HttpImpl, pool: usize, rounds: usize) -> R<(u64, usize)> {
    let lab = Lab::new()?;
    let mut cfg = HttpClientConfig::new(imp);
    cfg.pool = pool;
    cfg.driver.quickack_check = QuickAckCheck::Warn;
    // A pod with one CPU: open on the loop rather than refuse (see
    // `lookups_for_one`).
    cfg.driver.inline_connect_fallback = true;
    cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
    let stop = AtomicBool::new(false);
    let accepted = AtomicUsize::new(0);
    let before = route::lookups();
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| serve(&lab, &stop, &accepted).map_err(|e| e.to_string()));
        let result = (|| -> R {
            let mut c = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
            let mut h = Replies::default();
            let end = Instant::now() + Duration::from_secs(30);
            // The native pool opens its connections up front.
            while c.idle() < pool {
                c.poll(&mut h)?;
                if Instant::now() > end {
                    return Err(format!("{imp:?}: pool did not open").into());
                }
            }
            for _ in 0..rounds {
                let want = h.ok + pool;
                for _ in 0..pool {
                    c.request("GET", "/q", &[], &[])?;
                }
                while h.ok < want && h.errors.is_empty() {
                    c.poll(&mut h)?;
                    if Instant::now() > end {
                        return Err(format!("{imp:?}: no response").into());
                    }
                }
                if !h.errors.is_empty() {
                    return Err(format!("{imp:?}: {:?}", h.errors).into());
                }
            }
            Ok(())
        })();
        stop.store(true, Ordering::Relaxed);
        srv.join().map_err(|_| "server panicked")??;
        result
    })?;
    Ok((route::lookups() - before, accepted.load(Ordering::Relaxed)))
}

#[test]
fn one_lookup_per_connection_and_none_per_request() -> R {
    for how in [How::Connect, How::Background, How::Add] {
        for policy in [QuickAckCheck::Warn, QuickAckCheck::Require] {
            assert_eq!(lookups_for_one(policy, how)?, 1, "{how:?} {policy}");
        }
        assert_eq!(lookups_for_one(QuickAckCheck::Off, how)?, 0, "{how:?} off");
    }
    let mut imps = vec![HttpImpl::Native];
    if cfg!(feature = "curl") {
        imps.push(HttpImpl::Curl);
    }
    for imp in imps {
        for pool in [1, 2, 4] {
            let (lookups, connections) = pool_lookups(imp, pool, 25)?;
            assert!(connections >= 1, "{imp:?} pool {pool}: no connection");
            assert_eq!(
                lookups,
                u64::try_from(connections)?,
                "{imp:?} pool {pool}: {} requests over {connections} connections",
                25 * pool
            );
        }
    }
    Ok(())
}
