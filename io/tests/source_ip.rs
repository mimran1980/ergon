//! Source-IP binding (`Target::source`): a dial binds the chosen local
//! address before it connects, so the server sees that address; a source
//! of the other address family is refused before any connect, and one the
//! host does not have fails the connect. Every platform with the blocking
//! helpers; through a `Driver` (both connect paths) on Linux, where
//! `IP_BIND_ADDRESS_NO_PORT` is read back off the socket.
#![allow(missing_docs)]

mod support;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use ergon_io::net::BindSource;
use ergon_io::{Error, HttpDial, HttpOptions, WsDial, WsOptions};
use support::{Lab, R};

/// Accepts one client on `lab`, completes TLS, and returns the address it
/// came from.
fn peer_of(lab: &Lab) -> R<SocketAddr> {
    let peer = lab.accept_tls_within(Duration::from_secs(10))?;
    Ok(peer.sock.peer_addr()?)
}

#[test]
fn a_blocking_dial_binds_its_source_ip() -> R {
    let lab = Lab::new()?;
    let loopback: IpAddr = "127.0.0.1".parse()?;
    let target = lab.target("/")?.with_source(loopback);
    assert_eq!(target.source, Some(loopback));
    std::thread::scope(|s| -> R {
        let server = s.spawn(|| peer_of(&lab).map_err(|e| e.to_string()));
        let conn = HttpDial::new(
            target.clone(),
            lab.client_cfg.clone(),
            HttpOptions::default(),
        )?
        .connect_blocking()?;
        let seen = server.join().map_err(|_| "server panicked")??;
        assert_eq!(seen.ip(), loopback);
        drop(conn);
        Ok(())
    })?;
    Ok(())
}

/// `BindSource` on a dial built from a target without a source.
#[test]
fn bind_source_sets_the_dials_source() -> R {
    let lab = Lab::new()?;
    let ip: IpAddr = "127.0.0.1".parse()?;
    let mut ws = WsDial::with_options(
        lab.target("/ws")?,
        lab.client_cfg.clone(),
        WsOptions::default(),
    )?;
    assert_eq!(ws.target().source, None);
    ws.bind_source(ip);
    assert_eq!(ws.target().source, Some(ip));
    let mut http = HttpDial::new(
        lab.target("/")?,
        lab.client_cfg.clone(),
        HttpOptions::default(),
    )?;
    http.bind_source(ip);
    assert_eq!(http.target().source, Some(ip));
    Ok(())
}

#[test]
fn a_source_of_the_other_family_is_refused_before_connecting() -> R {
    let lab = Lab::new()?;
    let target = lab.target("/")?.with_source("::1".parse()?);
    let e = HttpDial::new(target, lab.client_cfg.clone(), HttpOptions::default())?
        .connect_blocking()
        .err()
        .ok_or("connected")?;
    assert!(
        matches!(&e, Error::Config(m) if m.contains("family")),
        "{e}"
    );
    lab.listener.set_nonblocking(true)?;
    assert!(lab.listener.accept().is_err(), "a connect went out");
    Ok(())
}

#[test]
fn a_source_the_host_does_not_have_fails_the_connect() -> R {
    let lab = Lab::new()?;
    // TEST-NET-1 (RFC 5737): never a local address.
    let target = lab.target("/")?.with_source("192.0.2.1".parse()?);
    let e = HttpDial::new(target, lab.client_cfg, HttpOptions::default())?
        .connect_blocking()
        .err()
        .ok_or("connected")?;
    assert!(
        matches!(&e, Error::Io(io) if io.kind() == std::io::ErrorKind::AddrNotAvailable),
        "{e:?}"
    );
    Ok(())
}

/// Linux: every 127.0.0.0/8 address is local, so a connection can come
/// from 127.0.0.2 and 127.0.0.3. Both `Driver::connect` and
/// `connect_background` bind the source, and the socket has
/// `IP_BIND_ADDRESS_NO_PORT` set: the port is picked at connect, per
/// destination, so binding does not use up the source's ephemeral ports.
#[cfg(target_os = "linux")]
#[test]
fn a_driver_connect_binds_the_source_without_reserving_a_port() -> R {
    use std::time::Instant;

    use ergon_io::{
        Backend, Config, ConnId, Driver, Handler, HttpConn, HttpHandler, Response, RxMeta,
    };

    #[derive(Default)]
    struct App {
        open: Vec<ConnId>,
        closed: Vec<String>,
    }
    impl Handler for App {
        fn on_open(&mut self, id: ConnId) {
            self.open.push(id);
        }
        fn on_closed(&mut self, _: ConnId, e: Error) {
            self.closed.push(e.to_string());
        }
    }
    impl HttpHandler for App {
        fn on_response(&mut self, _: ConnId, _: u64, _: &Response<'_>, _: &RxMeta) {}
    }

    for (background, ip) in [(false, "127.0.0.2"), (true, "127.0.0.3")] {
        let ip: IpAddr = ip.parse()?;
        let lab = Lab::new()?;
        let mut cfg = Config::new(Backend::EpollSpin);
        cfg.inline_connect_fallback = true;
        let mut d: Driver<HttpConn> = Driver::new(cfg)?;
        let dial = HttpDial::new(
            lab.target("/")?.with_source(ip),
            lab.client_cfg.clone(),
            HttpOptions::default(),
        )?;
        let id = if background {
            d.connect_background(dial)?
        } else {
            d.connect(dial)?
        };
        let mut app = App::default();
        std::thread::scope(|s| -> R {
            let server = s.spawn(|| peer_of(&lab).map_err(|e| e.to_string()));
            let end = Instant::now() + Duration::from_secs(10);
            while app.open.is_empty() {
                assert!(app.closed.is_empty(), "{:?}", app.closed);
                assert!(Instant::now() < end, "no open");
                d.poll(&mut app)?;
            }
            let seen = server.join().map_err(|_| "server panicked")??;
            assert_eq!(seen.ip(), ip, "background {background}");
            Ok(())
        })?;
        let fd = d.get(id).ok_or("no connection")?.raw_fd();
        assert!(
            ergon_io::sys::bind_address_no_port(fd)?,
            "background {background}"
        );
    }
    Ok(())
}

/// Linux: `HttpClient` connects from its target's source, on each
/// implementation built in (libcurl through `CURLOPT_INTERFACE`).
#[cfg(target_os = "linux")]
#[test]
fn an_http_client_connects_from_its_targets_source() -> R {
    use std::time::Instant;

    use ergon_io::{ClientHandler, HttpClient, HttpClientConfig, HttpImpl, Response};

    #[derive(Default)]
    struct Got(Vec<u16>, Vec<String>);
    impl ClientHandler for Got {
        fn on_response(&mut self, _: u64, r: &Response<'_>) {
            self.0.push(r.status);
        }
        fn on_error(&mut self, _: u64, _: ergon_io::Sent, e: Error) {
            self.1.push(e.to_string());
        }
    }

    let mut imps = vec![(HttpImpl::Native, "127.0.0.4")];
    if cfg!(feature = "curl") {
        imps.push((HttpImpl::Curl, "127.0.0.5"));
    }
    for (imp, ip) in imps {
        let ip: IpAddr = ip.parse()?;
        let lab = Lab::new()?;
        let mut cfg = HttpClientConfig::new(imp);
        cfg.pool = 1;
        cfg.driver.inline_connect_fallback = true;
        cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
        let target = lab.target("")?.with_source(ip);
        std::thread::scope(|s| -> R {
            let server = s.spawn(|| -> Result<IpAddr, String> {
                let mut p = lab
                    .accept_tls_within(Duration::from_secs(10))
                    .map_err(|e| e.to_string())?;
                let from = p.sock.peer_addr().map_err(|e| e.to_string())?.ip();
                p.read_request().map_err(|e| e.to_string())?;
                p.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .map_err(|e| e.to_string())?;
                Ok(from)
            });
            let mut client = HttpClient::new(&cfg, target, lab.client_cfg.clone())?;
            let mut got = Got::default();
            let end = Instant::now() + Duration::from_secs(10);
            while client.idle() == 0 {
                assert!(Instant::now() < end, "{imp:?}: no connection");
                client.poll(&mut got)?;
            }
            client.request("GET", "/", &[], &[])?;
            while got.0.is_empty() && got.1.is_empty() {
                assert!(Instant::now() < end, "{imp:?}: no answer");
                client.poll(&mut got)?;
            }
            assert_eq!(got.0, [200], "{imp:?}: {:?}", got.1);
            let from = server.join().map_err(|_| "server panicked")??;
            assert_eq!(from, ip, "{imp:?}");
            Ok(())
        })?;
    }
    Ok(())
}
