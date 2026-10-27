//! A rustls WebSocket server on loopback for the integration tests.
#![allow(dead_code, missing_docs)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

use ergon_io::{Target, tls, ws};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig, ServerConnection};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type R<T = ()> = Result<T, BoxError>;

/// A listening server plus the client config that trusts it.
pub struct Lab {
    pub listener: TcpListener,
    pub server_cfg: Arc<ServerConfig>,
    pub client_cfg: Arc<ClientConfig>,
    /// The self-signed server certificate, PEM (a CA file for libcurl).
    pub cert_pem: String,
}

impl Lab {
    pub fn new() -> R<Self> {
        let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let cert = CertificateDer::from(ck.cert.der().to_vec());
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let server_cfg = Arc::new(
            ServerConfig::builder_with_provider(provider)
                .with_protocol_versions(&[&rustls::version::TLS13])?
                .with_no_client_auth()
                .with_single_cert(vec![cert.clone()], key)?,
        );
        let mut roots = RootCertStore::empty();
        roots.add(cert)?;
        Ok(Self {
            listener: TcpListener::bind("127.0.0.1:0")?,
            server_cfg,
            client_cfg: tls::client_config(roots)?,
            cert_pem: ck.cert.pem(),
        })
    }

    /// Writes the certificate to a fresh file and returns its path.
    pub fn ca_file(&self) -> R<std::path::PathBuf> {
        let p = std::env::temp_dir().join(format!("ergon-io-test-{}.pem", self.addr()?.port()));
        std::fs::write(&p, &self.cert_pem)?;
        Ok(p)
    }

    /// Serves HTTP on every accepted connection until `stop` is set: each
    /// request is answered with its target (path and query) as the body.
    /// `close_after` makes the first connection close after that many
    /// responses.
    pub fn serve_http(
        &self,
        stop: &std::sync::atomic::AtomicBool,
        close_after: Option<usize>,
    ) -> R {
        use std::sync::atomic::Ordering;
        self.listener.set_nonblocking(true)?;
        std::thread::scope(|s| -> R {
            let mut first = true;
            while !stop.load(Ordering::Relaxed) {
                match self.listener.accept() {
                    Ok((sock, _)) => {
                        sock.set_nonblocking(false)?;
                        let limit = if first { close_after } else { None };
                        first = false;
                        s.spawn(move || {
                            let Ok(mut p) = Peer::serve_tls(self, sock) else {
                                return;
                            };
                            let mut n = 0;
                            while let Ok(req) = p.read_request() {
                                n += 1;
                                let close = limit == Some(n);
                                let extra = if close { "Connection: close\r\n" } else { "" };
                                let body = req.path.as_bytes();
                                let head = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n{extra}\r\n",
                                    body.len()
                                );
                                let mut out = head.into_bytes();
                                // A HEAD response announces the length only.
                                if req.method != "HEAD" {
                                    out.extend_from_slice(body);
                                }
                                if p.write(&out).is_err() || close {
                                    return;
                                }
                            }
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            Ok(())
        })
    }

    pub fn addr(&self) -> R<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    /// Accepts one client and completes only the TLS handshake.
    pub fn accept_tls(&self) -> R<Peer> {
        let (sock, _) = self.listener.accept()?;
        Peer::serve_tls(self, sock)
    }

    /// Like [`Lab::accept_tls`], but fails after `wait` with no client, and
    /// the peer's reads fail after `wait` with no data: a test whose client
    /// never connects (or refuses, as `connect_background` does on a pod
    /// with one CPU) fails instead of hanging.
    pub fn accept_tls_within(&self, wait: std::time::Duration) -> R<Peer> {
        self.listener.set_nonblocking(true)?;
        let end = std::time::Instant::now() + wait;
        let sock = loop {
            match self.listener.accept() {
                Ok((sock, _)) => break sock,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() > end {
                        return Err("no client connected".into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(e) => return Err(e.into()),
            }
        };
        self.listener.set_nonblocking(false)?;
        sock.set_nonblocking(false)?;
        sock.set_read_timeout(Some(wait))?;
        Peer::serve_tls(self, sock)
    }

    pub fn target(&self, path: &str) -> R<Target> {
        let addr = self.addr()?;
        Ok(Target::new("localhost", addr.port(), path)
            .with_addr(addr)
            .with_header("X-Test", "1"))
    }

    /// Like [`Lab::accept`], but fails after `wait` with no client, and the
    /// peer's reads fail after `wait` with no data: a test whose client
    /// misbehaves fails instead of hanging.
    pub fn accept_within(&self, wait: std::time::Duration) -> R<Peer> {
        self.listener.set_nonblocking(true)?;
        let end = std::time::Instant::now() + wait;
        let sock = loop {
            match self.listener.accept() {
                Ok((sock, _)) => break sock,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() > end {
                        return Err("no client connected".into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(e) => return Err(e.into()),
            }
        };
        self.listener.set_nonblocking(false)?;
        sock.set_nonblocking(false)?;
        sock.set_read_timeout(Some(wait))?;
        Peer::serve(self, sock)
    }

    /// Accepts one client and completes the TLS handshake and the upgrade.
    pub fn accept(&self) -> R<Peer> {
        let (sock, _) = self.listener.accept()?;
        Peer::serve(self, sock)
    }
}

impl Peer {
    /// Completes the TLS handshake only (HTTP tests).
    pub fn serve_tls(lab: &Lab, sock: TcpStream) -> R<Self> {
        let mut p = Self {
            tls: ServerConnection::new(lab.server_cfg.clone())?,
            sock,
            plain: Vec::new(),
            path: String::new(),
        };
        while p.tls.is_handshaking() {
            p.tls.complete_io(&mut p.sock)?;
        }
        Ok(p)
    }

    /// Reads one HTTP request: method, path, headers and body.
    pub fn read_request(&mut self) -> R<HttpRequest> {
        loop {
            let mut hdrs = [httparse::EMPTY_HEADER; 32];
            let mut req = httparse::Request::new(&mut hdrs);
            if let httparse::Status::Complete(n) = req.parse(&self.plain)? {
                let len = req
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("content-length"))
                    .map(|h| std::str::from_utf8(h.value).map(str::parse::<usize>))
                    .transpose()?
                    .transpose()?
                    .unwrap_or(0);
                if self.plain.len() >= n + len {
                    let r = HttpRequest {
                        method: req.method.unwrap_or_default().to_owned(),
                        path: req.path.unwrap_or_default().to_owned(),
                        headers: req
                            .headers
                            .iter()
                            .map(|h| {
                                (
                                    h.name.to_owned(),
                                    String::from_utf8_lossy(h.value).into_owned(),
                                )
                            })
                            .collect(),
                        body: self.plain[n..n + len].to_vec(),
                    };
                    self.plain.drain(..n + len);
                    return Ok(r);
                }
            }
            self.read_more()?;
        }
    }

    /// Like [`Peer::serve`], but the first `frames` go out in the same TLS
    /// record as the 101 response.
    pub fn serve_with(lab: &Lab, sock: TcpStream, frames: &[&[u8]]) -> R<Self> {
        let mut p = Self::serve_tls(lab, sock)?;
        let (n, key, path) = loop {
            if let Some(r) = ws::parse_server_request(&p.plain)? {
                break r;
            }
            p.read_more()?;
        };
        p.plain.drain(..n);
        p.path = path;
        let mut first = ws::server_response(&key);
        for f in frames {
            first.extend_from_slice(&frame(ws::OP_TEXT, true, f)?);
        }
        p.write(&first)?;
        Ok(p)
    }

    /// Completes the TLS handshake, reads the upgrade request and writes
    /// `respond(request, Sec-WebSocket-Accept value)`: for upgrade answers
    /// the tests shape themselves (subprotocols, refusals).
    pub fn serve_custom(
        lab: &Lab,
        sock: TcpStream,
        respond: impl FnOnce(&HttpRequest, &str) -> String,
    ) -> R<(Self, HttpRequest)> {
        let mut p = Self::serve_tls(lab, sock)?;
        let req = p.read_request()?;
        let key = req.header("sec-websocket-key").ok_or("no key")?.to_owned();
        let answer = respond(&req, &ws::accept_key(&key));
        p.write(answer.as_bytes())?;
        p.path.clone_from(&req.path);
        Ok((p, req))
    }

    /// Completes the TLS handshake and the upgrade on an accepted socket.
    pub fn serve(lab: &Lab, sock: TcpStream) -> R<Self> {
        let mut p = Self {
            tls: ServerConnection::new(lab.server_cfg.clone())?,
            sock,
            plain: Vec::new(),
            path: String::new(),
        };
        while p.tls.is_handshaking() {
            p.tls.complete_io(&mut p.sock)?;
        }
        let (n, key, path) = loop {
            if let Some(r) = ws::parse_server_request(&p.plain)? {
                break r;
            }
            p.read_more()?;
        };
        p.plain.drain(..n);
        p.path = path;
        p.write(&ws::server_response(&key))?;
        Ok(p)
    }
}

/// A request as the test server saw it.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// The server side of one connection.
pub struct Peer {
    pub tls: ServerConnection,
    pub sock: TcpStream,
    plain: Vec<u8>,
    pub path: String,
}

impl Peer {
    /// Writes `data`, flushing as it goes (rustls buffers at most 64 KiB of
    /// plaintext).
    pub fn write(&mut self, mut data: &[u8]) -> R {
        while !data.is_empty() {
            let n = self.tls.writer().write(data)?;
            data = &data[n..];
            while self.tls.wants_write() {
                self.tls.write_tls(&mut self.sock)?;
            }
        }
        Ok(())
    }

    pub fn send(&mut self, opcode: u8, fin: bool, payload: &[u8]) -> R {
        self.write(&frame(opcode, fin, payload)?)
    }

    /// Sends a Close frame: `code` then `reason`, or an empty body.
    pub fn send_close(&mut self, code: Option<u16>, reason: &str) -> R {
        let mut body = Vec::new();
        if let Some(c) = code {
            body.extend_from_slice(&c.to_be_bytes());
            body.extend_from_slice(reason.as_bytes());
        }
        self.send(ws::OP_CLOSE, true, &body)
    }

    /// Moves plaintext into `plain`, reading the socket only when rustls has
    /// none buffered (it may already hold data that arrived with the handshake).
    fn read_more(&mut self) -> R {
        if self.drain()? > 0 {
            return Ok(());
        }
        if self.tls.read_tls(&mut self.sock)? == 0 {
            return Err("client closed".into());
        }
        self.tls.process_new_packets()?;
        self.drain()?;
        Ok(())
    }

    fn drain(&mut self) -> R<usize> {
        let mut b = [0u8; 16_384];
        let mut total = 0;
        loop {
            match self.tls.reader().read(&mut b) {
                Ok(0) => return Ok(total),
                Ok(n) => {
                    self.plain.extend_from_slice(&b[..n]);
                    total += n;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(total),
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Reads one client frame and unmasks it.
    pub fn recv(&mut self) -> R<(u8, Vec<u8>)> {
        loop {
            if let Some(h) = ws::parse_header(&self.plain)?
                && self.plain.len() >= h.frame_len()
            {
                let mut body = self.plain[h.header_len..h.frame_len()].to_vec();
                ws::apply_mask(&mut body, h.mask.ok_or("client frame not masked")?);
                self.plain.drain(..h.frame_len());
                return Ok((h.opcode, body));
            }
            self.read_more()?;
        }
    }

    /// Whether the client has closed its end (read returns EOF).
    pub fn client_closed(&mut self) -> R<bool> {
        let mut b = [0u8; 4096];
        loop {
            match self.sock.read(&mut b) {
                Ok(0) => return Ok(true),
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => return Ok(true),
                Err(e) => return Err(e.into()),
            }
        }
    }
}

/// An unmasked server frame; `fin = false` clears the FIN bit.
pub fn frame(opcode: u8, fin: bool, payload: &[u8]) -> R<Vec<u8>> {
    let mut out = vec![0u8; ws::header_len(payload.len(), false) + payload.len()];
    let n = ws::encode_frame(opcode, payload, None, &mut out)?;
    out.truncate(n);
    if !fin {
        out[0] &= 0x7F;
    }
    Ok(out)
}

/// Sets `stop` when dropped, on a panic too: a failed assertion next to
/// [`Lab::serve_http`] then fails the test instead of hanging it.
pub struct StopOnDrop<'a>(pub &'a std::sync::atomic::AtomicBool);

impl Drop for StopOnDrop<'_> {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The CPUs this process may run on: its cpuset (Docker's `--cpuset-cpus`,
/// a Kubernetes pod's CPU set), as the test thread's mask shows it. The
/// host-wide online list also holds CPUs a pod cannot use. Linux only.
#[cfg(target_os = "linux")]
pub fn cpuset() -> R<Vec<usize>> {
    std::fs::read_to_string("/proc/thread-self/status")?
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .map(ergon_io::sys::parse_cpu_list)
        .ok_or_else(|| "no Cpus_allowed_list".into())
}

/// A pod with one CPU: helper threads (the dial helper, `SQPOLL`) have no
/// core of their own, so `connect_background` and `uring-sqpoll` refuse
/// with [`ergon_io::Error::Cpu`] rather than time-slice the polling thread.
/// Linux only (no `/proc` elsewhere); its callers are Linux test files.
#[cfg(target_os = "linux")]
pub fn one_cpu() -> R<bool> {
    Ok(cpuset()?.len() == 1)
}

/// The `io_uring` backend where this process has a CPU for its `SQPOLL`
/// thread: `uring-sqpoll`, else `epoll-spin` (a pod with one CPU, where
/// `uring-sqpoll` is refused typed; driver.rs's `each_backend` asserts
/// that). Linux only.
#[cfg(target_os = "linux")]
pub fn uring_backend() -> R<ergon_io::Backend> {
    Ok(if one_cpu()? {
        ergon_io::Backend::EpollSpin
    } else {
        ergon_io::Backend::UringSqpoll
    })
}

/// Whether `e` is the typed refusal of `want` for want of a second CPU
/// ([`ergon_io::placement::CpuError::NoSpareCpu`]), when a test's error
/// came back boxed.
pub fn refused(
    e: &(dyn std::error::Error + 'static),
    want: ergon_io::placement::HelperThread,
) -> bool {
    use ergon_io::placement::CpuError;
    match e.downcast_ref::<ergon_io::Error>() {
        Some(ergon_io::Error::Cpu(e)) => {
            matches!(**e, CpuError::NoSpareCpu { helper, .. } if helper == want)
        }
        _ => false,
    }
}
