//! Both arms of each transport put the same bytes on the wire: a capturing
//! peer written here with std sockets stands in for the far end, records
//! what `udp_lat` / `tcp_lat` send (every datagram, or the raw TCP byte
//! stream), and answers. Pings, after their two stamps are masked, are
//! byte-for-byte the core's message (`seq`, stream, kind ping, zero tail);
//! TCP's frames carry the core's u32 LE header; echoes are exactly the
//! ping with its kind turned to echo. The masked bytes hash alike across
//! mio and ergon-io.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, UdpSocket};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ergon_latency_core::frame::{self, FRAME_HEADER};
use ergon_latency_core::manifest::sha256_hex;
use ergon_latency_core::message::{Kind, MESSAGE_LEN, Message, echo_in_place};

type R<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const UDP: &str = env!("CARGO_BIN_EXE_udp_lat");
const TCP: &str = env!("CARGO_BIN_EXE_tcp_lat");
/// Streams each run uses.
const STREAMS: u32 = 2;
/// Messages kept per stream.
const KEEP: usize = 4;

fn last_cpu() -> usize {
    // SAFETY: a zeroed cpu_set_t is valid; the kernel fills at most its size.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set) != 0 {
            return 0;
        }
        (0..libc::CPU_SETSIZE as usize)
            .filter(|&c| libc::CPU_ISSET(c, &set))
            .max()
            .unwrap_or(0)
    }
}

fn out_dir(name: &str) -> R<String> {
    let d = std::env::temp_dir().join(format!("transport-wire-{}-{name}", std::process::id()));
    if d.exists() {
        std::fs::remove_dir_all(&d)?;
    }
    std::fs::create_dir_all(&d)?;
    Ok(d.display().to_string())
}

fn args(imp: &str, role: &str, out: &str) -> Vec<String> {
    [
        "--impl",
        imp,
        "--role",
        role,
        "--streams",
        "2",
        "--rate",
        "200",
        "--warmup-s",
        "0",
        "--duration-s",
        "1",
        "--label",
        "t:wire",
        "--out",
        out,
        "--idle-exit-s",
        "1",
        "--wait-s",
        "20",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .chain(["--cpus".to_owned(), last_cpu().to_string()])
    .collect()
}

/// A ping's bytes with its two stamps zeroed.
fn masked(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    if v.len() >= 24 {
        v[8..24].fill(0);
    }
    v
}

/// What the core says ping `seq` of `stream` is, stamps zero.
fn expected(seq: u64, stream: u32) -> [u8; MESSAGE_LEN] {
    Message {
        seq,
        send_tsc: 0,
        intended_tsc: 0,
        stream,
        kind: Kind::Ping,
    }
    .to_bytes()
}

/// A ping as the capturing peer crafts it: every byte known.
fn crafted(seq: u64, stream: u32) -> [u8; MESSAGE_LEN] {
    Message {
        seq,
        send_tsc: 0x1111_2222_3333_0000 + seq,
        intended_tsc: 0x4444_5555_6666_0000 + seq,
        stream,
        kind: Kind::Ping,
    }
    .to_bytes()
}

fn wait_ready(child: &mut Child) -> R {
    let out = child.stdout.take().ok_or("no stdout")?;
    for l in BufReader::new(out).lines() {
        if l? == "ready" {
            return Ok(());
        }
    }
    Err("the pong never said ready".into())
}

fn ok(name: &str, child: Child) -> R {
    let o = child.wait_with_output()?;
    if o.status.success() {
        return Ok(());
    }
    Err(format!(
        "{name}: {} {}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    )
    .into())
}

/// The first datagrams `udp_lat --role ping` sends on each stream, the
/// capturing peer echoing every one.
fn udp_pings(imp: &str, base: u16) -> R<Vec<Vec<Vec<u8>>>> {
    let out = out_dir(&format!("udp-ping-{imp}"))?;
    let (local, peer) = (base, base + 100);
    let done = Arc::new(AtomicBool::new(false));
    let mut peers = Vec::new();
    for i in 0..STREAMS as u16 {
        let s = UdpSocket::bind(("127.0.0.1", peer + i))?;
        s.connect(("127.0.0.1", local + i))?;
        s.set_read_timeout(Some(Duration::from_millis(50)))?;
        let done = Arc::clone(&done);
        peers.push(std::thread::spawn(move || -> R<Vec<Vec<u8>>> {
            let mut kept = Vec::new();
            let mut b = [0u8; 2_048];
            while !done.load(Ordering::Acquire) {
                match s.recv(&mut b) {
                    Ok(n) => {
                        if kept.len() < KEEP {
                            kept.push(b[..n].to_vec());
                        }
                        echo_in_place(&mut b[..n])?;
                        s.send(&b[..n])?;
                    }
                    Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                    Err(e) if e.kind() == ErrorKind::ConnectionRefused => {}
                    Err(e) => return Err(e.into()),
                }
            }
            Ok(kept)
        }));
    }
    let ping = Command::new(UDP)
        .args(args(imp, "ping", &out))
        .args([
            "--local",
            &format!("127.0.0.1:{local}"),
            "--peer",
            &format!("127.0.0.1:{peer}"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let r = ok("udp ping", ping);
    done.store(true, Ordering::Release);
    let kept = peers
        .into_iter()
        .map(|h| h.join().map_err(|_| "peer panicked")?)
        .collect::<R<Vec<_>>>()?;
    r?;
    Ok(kept)
}

/// What `udp_lat --role pong` sends back for crafted pings.
fn udp_echoes(imp: &str, base: u16) -> R<Vec<Vec<Vec<u8>>>> {
    let out = out_dir(&format!("udp-pong-{imp}"))?;
    let (pong_at, test_at) = (base, base + 100);
    let mut pong = Command::new(UDP)
        .args(args(imp, "pong", &out))
        .args([
            "--local",
            &format!("127.0.0.1:{pong_at}"),
            "--peer",
            &format!("127.0.0.1:{test_at}"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    wait_ready(&mut pong)?;
    let mut all = Vec::new();
    for i in 0..STREAMS {
        let s = UdpSocket::bind(("127.0.0.1", test_at + i as u16))?;
        s.connect(("127.0.0.1", pong_at + i as u16))?;
        s.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut got = Vec::new();
        for seq in 0..KEEP as u64 {
            s.send(&crafted(seq, i))?;
            let mut b = [0u8; 2_048];
            let n = s.recv(&mut b)?;
            got.push(b[..n].to_vec());
        }
        all.push(got);
    }
    ok("udp pong", pong)?;
    Ok(all)
}

/// The raw byte streams `tcp_lat --role ping` writes, the capturing peer
/// echoing every frame, cut at the first `KEEP` frames.
fn tcp_pings(imp: &str, port: u16) -> R<Vec<Vec<u8>>> {
    let out = out_dir(&format!("tcp-ping-{imp}"))?;
    let l = TcpListener::bind(("127.0.0.1", port))?;
    let ping = Command::new(TCP)
        .args(args(imp, "ping", &out))
        .args(["--connect", &format!("127.0.0.1:{port}")])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut peers = Vec::new();
    for _ in 0..STREAMS {
        let (mut s, _) = l.accept()?;
        peers.push(std::thread::spawn(move || -> R<Vec<u8>> {
            let mut raw = Vec::new();
            let mut pending = Vec::new();
            let mut frames = 0usize;
            let mut b = [0u8; 4_096];
            loop {
                let n = s.read(&mut b)?;
                if n == 0 {
                    return Ok(raw);
                }
                pending.extend_from_slice(&b[..n]);
                while let Some((payload, taken)) = frame::decode(&pending, 1 << 16)? {
                    if frames < KEEP {
                        raw.extend_from_slice(&pending[..taken]);
                    }
                    frames += 1;
                    let mut m = payload.to_vec();
                    echo_in_place(&mut m)?;
                    let mut f = vec![0u8; FRAME_HEADER + m.len()];
                    frame::encode(&m, &mut f)?;
                    s.write_all(&f)?;
                    pending.drain(..taken);
                }
            }
        }));
    }
    let r = ok("tcp ping", ping);
    let raw = peers
        .into_iter()
        .map(|h| h.join().map_err(|_| "peer panicked")?)
        .collect::<R<Vec<_>>>()?;
    r?;
    Ok(raw)
}

/// What `tcp_lat --role pong` writes back for crafted frames, raw.
fn tcp_echoes(imp: &str, port: u16) -> R<Vec<Vec<u8>>> {
    let out = out_dir(&format!("tcp-pong-{imp}"))?;
    let mut pong = Command::new(TCP)
        .args(args(imp, "pong", &out))
        .args(["--listen", &format!("127.0.0.1:{port}")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    wait_ready(&mut pong)?;
    let mut conns = Vec::new();
    let until = Instant::now() + Duration::from_secs(10);
    for _ in 0..STREAMS {
        loop {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(s) => {
                    conns.push(s);
                    break;
                }
                Err(_) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
                Err(e) => return Err(e.into()),
            }
        }
    }
    let mut all = Vec::new();
    for (i, s) in (0u32..).zip(&mut conns) {
        s.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut want = 0;
        for seq in 0..KEEP as u64 {
            let mut f = [0u8; FRAME_HEADER + MESSAGE_LEN];
            frame::encode(&crafted(seq, i), &mut f)?;
            s.write_all(&f)?;
            want += f.len();
        }
        let mut back = vec![0u8; want];
        s.read_exact(&mut back)?;
        all.push(back);
    }
    for s in &conns {
        s.shutdown(Shutdown::Both)?;
    }
    ok("tcp pong", pong)?;
    Ok(all)
}

#[test]
fn both_udp_arms_send_the_same_pings() -> R {
    let mut hashes = Vec::new();
    for (imp, base) in [("mio", 29_100), ("ergon", 29_300)] {
        let kept = udp_pings(imp, base)?;
        let mut all = Vec::new();
        for (stream, msgs) in (0u32..).zip(&kept) {
            assert_eq!(msgs.len(), KEEP, "{imp} stream {stream}");
            for (seq, m) in (0u64..).zip(msgs) {
                assert_eq!(
                    m.len(),
                    MESSAGE_LEN,
                    "{imp}: one 64-byte datagram per message"
                );
                assert_eq!(
                    masked(m),
                    expected(seq, stream),
                    "{imp} stream {stream} seq {seq}"
                );
                all.extend_from_slice(&masked(m));
            }
        }
        hashes.push(sha256_hex(&all));
    }
    assert_eq!(hashes[0], hashes[1]);
    Ok(())
}

#[test]
fn both_udp_arms_echo_the_same_bytes() -> R {
    let mut got = Vec::new();
    for (imp, base) in [("mio", 29_500), ("ergon", 29_700)] {
        let echoes = udp_echoes(imp, base)?;
        for (stream, msgs) in (0u32..).zip(&echoes) {
            for (seq, m) in (0u64..).zip(msgs) {
                let mut want = crafted(seq, stream);
                echo_in_place(&mut want)?;
                assert_eq!(
                    m.as_slice(),
                    want.as_slice(),
                    "{imp} stream {stream} seq {seq}"
                );
            }
        }
        got.push(echoes);
    }
    assert_eq!(got[0], got[1]);
    Ok(())
}

#[test]
fn both_tcp_arms_send_the_same_frames() -> R {
    let mut hashes = Vec::new();
    for (imp, port) in [("mio", 29_901), ("ergon", 29_902)] {
        let raw = tcp_pings(imp, port)?;
        let mut all = Vec::new();
        for (stream, bytes) in (0u32..).zip(&raw) {
            assert_eq!(
                bytes.len(),
                KEEP * (FRAME_HEADER + MESSAGE_LEN),
                "{imp} stream {stream}"
            );
            for (seq, f) in (0u64..).zip(bytes.chunks(FRAME_HEADER + MESSAGE_LEN)) {
                assert_eq!(
                    f[..FRAME_HEADER],
                    [64, 0, 0, 0],
                    "{imp}: the core's u32 LE header"
                );
                assert_eq!(
                    masked(&f[FRAME_HEADER..]),
                    expected(seq, stream),
                    "{imp} {stream}/{seq}"
                );
                all.extend_from_slice(&f[..FRAME_HEADER]);
                all.extend_from_slice(&masked(&f[FRAME_HEADER..]));
            }
        }
        hashes.push(sha256_hex(&all));
    }
    assert_eq!(hashes[0], hashes[1]);
    Ok(())
}

#[test]
fn both_tcp_arms_echo_the_same_frames() -> R {
    let mut got = Vec::new();
    for (imp, port) in [("mio", 29_903), ("ergon", 29_904)] {
        let echoes = tcp_echoes(imp, port)?;
        for (stream, bytes) in (0u32..).zip(&echoes) {
            let mut want = Vec::new();
            for seq in 0..KEEP as u64 {
                let mut m = crafted(seq, stream);
                echo_in_place(&mut m)?;
                let mut f = [0u8; FRAME_HEADER + MESSAGE_LEN];
                frame::encode(&m, &mut f)?;
                want.extend_from_slice(&f);
            }
            assert_eq!(bytes, &want, "{imp} stream {stream}");
        }
        got.push(echoes);
    }
    assert_eq!(got[0], got[1]);
    Ok(())
}
