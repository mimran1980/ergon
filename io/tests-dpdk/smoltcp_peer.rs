//! The kernel-side peer of the `smoltcp-spin` vdev tests
//! (`tests-dpdk/smoltcp_vdev.rs`): echo servers on the kernel's TCP/IP
//! stack, at the far end of the veth pair from the DPDK port.
//!
//! `smoltcp_peer <ip> <cert.pem>` serves, on `<ip>`:
//! - 7001: framed TCP echo (`u32` little-endian length, payload); a frame
//!   `!close` closes the connection instead of being echoed;
//! - 7443: WebSocket over TLS 1.3 echo (a self-signed certificate for
//!   `localhost`, written to `<cert.pem>`);
//! - 7002: UDP echo.
//!
//! It prints `READY` once listening, and for each framed connection that ends
//! `TCPINFO port=<client port> total_retrans=<n> rto_us=<n>` (its own
//! retransmissions, Linux). It exits when its stdin closes.
#![allow(missing_docs)]

#[path = "../tests/support/mod.rs"]
mod support;

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::Arc;

use support::{Lab, Peer, R};

fn framed(mut s: TcpStream) -> R {
    s.set_nodelay(true)?;
    let peer = s.peer_addr()?;
    let mut hdr = [0u8; 4];
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if s.read_exact(&mut hdr).is_err() {
            break;
        }
        let n = u32::from_le_bytes(hdr) as usize;
        if n > buf.len() {
            return Err(format!("frame of {n} bytes").into());
        }
        s.read_exact(&mut buf[..n])?;
        if &buf[..n] == b"!close" {
            break;
        }
        let mut out = Vec::with_capacity(n + 4);
        out.extend_from_slice(&hdr);
        out.extend_from_slice(&buf[..n]);
        s.write_all(&out)?;
    }
    report(&s, peer);
    Ok(())
}

#[cfg(target_os = "linux")]
fn report(s: &TcpStream, peer: SocketAddr) {
    use std::os::fd::AsRawFd;
    if let Ok(i) = ergon_io::sys::tcp_info(s.as_raw_fd()) {
        println!(
            "TCPINFO port={} total_retrans={} rto_us={} rtt_us={}",
            peer.port(),
            i.total_retrans,
            i.rto_us,
            i.rtt_us
        );
    }
}

#[cfg(not(target_os = "linux"))]
#[allow(clippy::missing_const_for_fn)] // as Linux's
fn report(_: &TcpStream, _: SocketAddr) {}

fn websocket(lab: &Lab, s: TcpStream) -> R {
    s.set_nodelay(true)?;
    let mut p = Peer::serve(lab, s)?;
    loop {
        let (op, body) = p.recv()?;
        match op {
            ergon_io::ws::OP_CLOSE => {
                p.send(ergon_io::ws::OP_CLOSE, true, &body)?;
                return Ok(());
            }
            ergon_io::ws::OP_PING => p.send(ergon_io::ws::OP_PONG, true, &body)?,
            _ => p.send(op, true, &body)?,
        }
    }
}

fn udp(sock: &UdpSocket) -> R {
    let mut b = vec![0u8; 65_536];
    loop {
        let (n, from) = sock.recv_from(&mut b)?;
        sock.send_to(&b[..n], from)?;
    }
}

fn main() -> R {
    let mut args = std::env::args().skip(1);
    let ip: IpAddr = args
        .next()
        .ok_or("usage: smoltcp_peer <ip> <cert.pem>")?
        .parse()?;
    let cert = args.next().ok_or("usage: smoltcp_peer <ip> <cert.pem>")?;
    let lab = Arc::new(Lab::new()?);
    std::fs::write(&cert, &lab.cert_pem)?;
    let frames = TcpListener::bind(SocketAddr::new(ip, 7001))?;
    let wss = TcpListener::bind(SocketAddr::new(ip, 7443))?;
    let dgrams = UdpSocket::bind(SocketAddr::new(ip, 7002))?;
    std::thread::spawn(move || {
        for s in frames.incoming().flatten() {
            std::thread::spawn(move || {
                if let Err(e) = framed(s) {
                    eprintln!("peer: framed: {e}");
                }
            });
        }
    });
    let l = Arc::clone(&lab);
    std::thread::spawn(move || {
        for s in wss.incoming().flatten() {
            let l = Arc::clone(&l);
            std::thread::spawn(move || {
                if let Err(e) = websocket(&l, s) {
                    eprintln!("peer: websocket: {e}");
                }
            });
        }
    });
    std::thread::spawn(move || {
        if let Err(e) = udp(&dgrams) {
            eprintln!("peer: udp: {e}");
        }
    });
    println!("READY");
    std::io::stdout().flush()?;
    // Until the test closes stdin.
    let mut sink = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut sink);
    Ok(())
}
