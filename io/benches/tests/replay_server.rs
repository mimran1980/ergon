//! The lab server's single-thread modes, on this host over loopback: HTTP
//! with `--threads one` serves a pool of 20 connections from one thread (so a
//! 20-connection run measures the client, not the server's threads sharing a
//! CPU), and `--mode mixed` echoes an order connection between its feed sends
//! (one order connection and N-1 feeds on one receiving loop), or alone.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use io_bench::ws;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, StreamOwned};

type R = Result<(), Box<dyn std::error::Error>>;
type Tls = StreamOwned<ClientConnection, TcpStream>;

/// A running `replay_server`: its address, its certificate and its stderr.
struct Server {
    child: Child,
    addr: SocketAddr,
    cfg: Arc<ClientConfig>,
    lines: std::sync::mpsc::Receiver<String>,
    dir: PathBuf,
}

impl Server {
    fn start(tag: &str, args: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("replay-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let cert = dir.join("cert.pem");
        let mut child = Command::new(env!("CARGO_BIN_EXE_replay_server"))
            .args(["--listen", "127.0.0.1:0", "--cert-out"])
            .arg(&cert)
            .args(args)
            .stderr(Stdio::piped())
            .spawn()?;
        let stderr = child.stderr.take().ok_or("no stderr")?;
        let (tx, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for l in BufReader::new(stderr).lines().map_while(Result::ok) {
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        let first = lines.recv_timeout(Duration::from_secs(20))?;
        let addr: SocketAddr = first
            .strip_prefix("listening on ")
            .and_then(|r| r.split_whitespace().next())
            .ok_or_else(|| format!("unexpected first line {first:?}"))?
            .parse()?;
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from_pem_file(&cert)?)?;
        let cfg = io_bench::tls::client_config(roots)?;
        Ok(Self {
            child,
            addr,
            cfg,
            lines,
            dir,
        })
    }

    /// A TLS connection, handshake done, reads timing out after 10 s.
    fn connect(&self) -> Result<Tls, Box<dyn std::error::Error>> {
        let sock = TcpStream::connect(self.addr)?;
        sock.set_nodelay(true)?;
        sock.set_read_timeout(Some(Duration::from_secs(10)))?;
        let conn = ClientConnection::new(self.cfg.clone(), ServerName::try_from("lab.local")?)?;
        let mut s = StreamOwned::new(conn, sock);
        while s.conn.is_handshaking() {
            s.conn.complete_io(&mut s.sock)?;
        }
        Ok(s)
    }

    /// Every stderr line until the server exits (or 20 s pass), or with
    /// `exits` false, half a second (a server that serves until killed).
    fn finish(mut self, exits: bool) -> Result<String, Box<dyn std::error::Error>> {
        let mut out = String::new();
        for _ in 0..if exits { 200 } else { 5 } {
            if self.child.try_wait()?.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        while let Ok(l) = self.lines.recv_timeout(Duration::from_millis(500)) {
            out.push_str(&l);
            out.push('\n');
        }
        std::fs::remove_dir_all(&self.dir)?;
        Ok(out)
    }
}

/// Threads of process `pid`.
fn threads(pid: u32) -> Result<usize, Box<dyn std::error::Error>> {
    if cfg!(target_os = "linux") {
        Ok(std::fs::read_dir(format!("/proc/{pid}/task"))?.count())
    } else {
        // macOS: `ps -M` prints a header, then a line per thread.
        let o = Command::new("ps")
            .args(["-M", "-p", &pid.to_string()])
            .output()?;
        Ok(String::from_utf8_lossy(&o.stdout)
            .lines()
            .count()
            .saturating_sub(1))
    }
}

/// One HTTP request and its whole response: (status, body).
fn http(s: &mut Tls, seq: u64) -> Result<(u16, Vec<u8>), Box<dyn std::error::Error>> {
    let body = io_bench::order::order_body(seq);
    write!(
        s,
        "POST /api/v3/order HTTP/1.1\r\nHost: lab.local\r\nContent-Type: application/x-www-form-urlencoded\r\n\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    )?;
    s.flush()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = s.read(&mut chunk)?;
        if n == 0 {
            return Err("closed".into());
        }
        buf.extend_from_slice(&chunk[..n]);
        let mut hdrs = [httparse::EMPTY_HEADER; 16];
        let mut r = httparse::Response::new(&mut hdrs);
        if let httparse::Status::Complete(h) = r.parse(&buf)? {
            let len: usize = r
                .headers
                .iter()
                .find(|x| x.name.eq_ignore_ascii_case("content-length"))
                .and_then(|x| std::str::from_utf8(x.value).ok()?.parse().ok())
                .ok_or("no content length")?;
            if buf.len() >= h + len {
                return Ok((r.code.unwrap_or(0), buf[h..h + len].to_vec()));
            }
        }
    }
}

#[test]
fn twenty_http_connections_are_served_by_one_thread() -> R {
    let srv = Server::start(
        "http",
        &["--mode", "http", "--threads", "one", "--duration", "60"],
    )?;
    assert_eq!(
        srv.lines.recv_timeout(Duration::from_secs(10))?,
        "serving every connection from one thread"
    );
    let mut pool: Vec<Tls> = (0..20).map(|_| srv.connect()).collect::<Result<_, _>>()?;
    // Requests in turn over the pool, as the pooled client sends them.
    for seq in 0..100u64 {
        let i = usize::try_from(seq % 20)?;
        let (status, body) = http(&mut pool[i], seq)?;
        assert_eq!(status, 200);
        assert!(String::from_utf8_lossy(&body).contains("\"orderId\""));
    }
    assert_eq!(
        threads(srv.child.id())?,
        1,
        "the server must serve all 20 connections on one thread"
    );
    for mut s in pool {
        s.conn.send_close_notify();
        let _ = s.flush();
    }
    let err = srv.finish(false)?;
    assert!(
        err.contains("http: session of 20 connection(s) over, 100 answered"),
        "{err}"
    );
    Ok(())
}

/// Opens a WebSocket at `path` on `s`.
fn upgrade(s: &mut Tls, path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let key = ws::client_key([7u8; 16]);
    s.write_all(&ws::client_request("lab.local", path, &key, &[]))?;
    s.flush()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(n) = ws::parse_client_response(&buf, &key)? {
            buf.drain(..n);
            return Ok(buf);
        }
        let n = s.read(&mut chunk)?;
        if n == 0 {
            return Err("closed during the upgrade".into());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// The next complete frame's (opcode, payload).
fn frame(s: &mut Tls, buf: &mut Vec<u8>) -> Result<(u8, Vec<u8>), Box<dyn std::error::Error>> {
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(h) = ws::parse_header(buf)?
            && buf.len() >= h.frame_len()
        {
            let p = buf[h.header_len..h.frame_len()].to_vec();
            buf.drain(..h.frame_len());
            return Ok((h.opcode, p));
        }
        let n = s.read(&mut chunk)?;
        if n == 0 {
            return Err("closed".into());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn send_frame(s: &mut Tls, payload: &[u8]) -> R {
    let mut out = vec![0u8; ws::header_len(payload.len(), true) + payload.len()];
    let n = ws::encode_frame(ws::OP_BINARY, payload, Some([1, 2, 3, 4]), &mut out)?;
    s.write_all(&out[..n])?;
    s.flush()?;
    Ok(())
}

/// `n` echoed round trips on `order`.
fn round_trips(order: &mut Tls, buf: &mut Vec<u8>, n: u64) -> R {
    for seq in 0..n {
        let mut payload = [0u8; 64];
        payload[..8].copy_from_slice(&seq.to_le_bytes());
        send_frame(order, &payload)?;
        let (op, p) = frame(order, buf)?;
        assert_eq!(op, ws::OP_BINARY);
        assert_eq!(p, payload);
    }
    Ok(())
}

#[test]
fn the_order_connection_is_echoed_between_feed_sends() -> R {
    let srv = Server::start(
        "mixed",
        &[
            "--mode",
            "mixed",
            "--clients",
            "3",
            "--duration",
            "30",
            "--trickle-min-us",
            "500",
            "--trickle-max-us",
            "1000",
            "--burst-every-us",
            "1000",
            "--burst-slots",
            "2",
            "--burst",
            "5",
        ],
    )?;
    let mut order = srv.connect()?;
    let mut order_buf = upgrade(&mut order, "/")?;
    let mut feeds = Vec::new();
    for j in 0..2 {
        let mut f = srv.connect()?;
        let mut buf = upgrade(&mut f, &format!("/slot/{j}"))?;
        // Each feed drains its stream on a thread of its own.
        feeds.push(std::thread::spawn(move || -> u64 {
            let mut n = 0;
            while let Ok((op, _)) = frame(&mut f, &mut buf) {
                if op == ws::OP_BINARY {
                    n += 1;
                }
            }
            n
        }));
    }
    round_trips(&mut order, &mut order_buf, 500)?;
    // The order connection closes: the server ends the run.
    order.conn.send_close_notify();
    let _ = order.flush();
    drop(order);
    let feed_messages: Vec<u64> = feeds.into_iter().map(|h| h.join().unwrap_or(0)).collect();
    assert!(feed_messages.iter().all(|&n| n > 0), "{feed_messages:?}");
    let err = srv.finish(true)?;
    assert!(
        err.contains("echoed 500 frames on 1 order connection(s)"),
        "{err}"
    );
    Ok(())
}

#[test]
fn an_order_connection_alone_is_echoed() -> R {
    let srv = Server::start(
        "alone",
        &["--mode", "mixed", "--clients", "1", "--duration", "30"],
    )?;
    let mut order = srv.connect()?;
    let mut buf = upgrade(&mut order, "/")?;
    round_trips(&mut order, &mut buf, 200)?;
    order.conn.send_close_notify();
    let _ = order.flush();
    drop(order);
    let err = srv.finish(true)?;
    assert!(
        err.contains("echoed 200 frames on 1 order connection(s)"),
        "{err}"
    );
    assert!(!err.contains("no client named a slot"), "{err}");
    Ok(())
}
