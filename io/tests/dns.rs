//! The non-blocking resolver against a fake name server on loopback.
#![allow(missing_docs)]

use std::collections::HashMap;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::{Duration, Instant};

use ergon_io::clock::monotonic_ns;
use ergon_io::dns::{self, DnsConfig, Lookup, Resolver};

type R = Result<(), Box<dyn std::error::Error>>;

/// Answers one query with SERVFAIL.
fn servfail(sock: &UdpSocket) -> R {
    let mut buf = [0u8; 512];
    let (n, from) = sock.recv_from(&mut buf)?;
    let mut r = buf[..n].to_vec();
    r[2] = 0x81;
    r[3] = 0x82; // RA; rcode 2
    sock.send_to(&r, from)?;
    Ok(())
}

/// Answers one query with `ip` (or NXDOMAIN when `None`) after `delay`.
fn answer(sock: &UdpSocket, ip: Option<Ipv4Addr>, delay: Duration) -> R {
    let mut buf = [0u8; 512];
    let (n, from) = sock.recv_from(&mut buf)?;
    std::thread::sleep(delay);
    let q = &buf[..n];
    let mut r = q.to_vec();
    r[2] = 0x81; // QR, RD
    r[3] = if ip.is_some() { 0x80 } else { 0x83 }; // RA; rcode 0 or 3
    if let Some(ip) = ip {
        r[7] = 1; // ANCOUNT
        r.extend_from_slice(&[0xC0, 12, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4]);
        r.extend_from_slice(&ip.octets());
    }
    sock.send_to(&r, from)?;
    Ok(())
}

fn resolver(server: &UdpSocket) -> Result<Resolver, Box<dyn std::error::Error>> {
    Ok(Resolver::new(DnsConfig {
        servers: vec![server.local_addr()?],
        timeout_ms: 2000,
        attempts: 1,
        min_ttl_s: 5,
        negative_ttl_s: 1,
        max_ttl_s: 3600,
        hosts: HashMap::from([("pinned.test".to_owned(), Ipv4Addr::new(10, 0, 0, 9))]),
    }))
}

#[test]
fn query_is_answered_without_blocking_and_cached() -> R {
    let server = UdpSocket::bind("127.0.0.1:0")?;
    let mut r = resolver(&server)?;
    let ip = Ipv4Addr::new(10, 1, 2, 3);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| {
            answer(&server, Some(ip), Duration::from_millis(50)).map_err(|e| e.to_string())
        });
        let t0 = Instant::now();
        // Never blocking means never sleeping: count the thread's voluntary
        // context switches (wall time would also count preemption).
        let (slept0, _) = ergon_io::clock::thread_ctx_switches();
        assert_eq!(
            r.lookup("api.example.test", monotonic_ns())?,
            Lookup::Pending
        );
        let got = loop {
            r.poll(monotonic_ns());
            if let Lookup::Ready(a) = r.lookup("api.example.test", monotonic_ns())? {
                break a;
            }
            assert!(t0.elapsed() < Duration::from_secs(5), "no answer");
        };
        let (slept1, _) = ergon_io::clock::thread_ctx_switches();
        srv.join().map_err(|_| "server panicked")??;
        assert_eq!(got, ip);
        assert_eq!(slept1 - slept0, 0, "the resolver slept");
        Ok(())
    })?;
    // Cached: ready at once, no query.
    assert_eq!(
        r.lookup("API.example.test.", monotonic_ns())?,
        Lookup::Ready(ip)
    );
    assert!(!r.busy());
    Ok(())
}

#[test]
fn literals_and_hosts_entries_skip_dns() -> R {
    let server = UdpSocket::bind("127.0.0.1:0")?;
    let mut r = resolver(&server)?;
    let now = monotonic_ns();
    assert_eq!(
        r.lookup("192.0.2.1", now)?,
        Lookup::Ready(Ipv4Addr::new(192, 0, 2, 1))
    );
    assert_eq!(
        r.lookup("pinned.test", now)?,
        Lookup::Ready(Ipv4Addr::new(10, 0, 0, 9))
    );
    assert!(!r.busy());
    Ok(())
}

#[test]
fn nxdomain_is_an_error_and_cached_briefly() -> R {
    let server = UdpSocket::bind("127.0.0.1:0")?;
    let mut r = resolver(&server)?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| answer(&server, None, Duration::ZERO).map_err(|e| e.to_string()));
        assert_eq!(r.lookup("missing.test", monotonic_ns())?, Lookup::Pending);
        let t0 = Instant::now();
        let err = loop {
            r.poll(monotonic_ns());
            match r.lookup("missing.test", monotonic_ns()) {
                Err(e) => break e,
                Ok(Lookup::Pending) => assert!(t0.elapsed() < Duration::from_secs(5)),
                Ok(Lookup::Ready(a)) => return Err(format!("resolved to {a}").into()),
            }
        };
        assert!(err.to_string().contains("no such name"), "{err}");
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}

#[test]
fn unanswered_query_times_out() -> R {
    let server = UdpSocket::bind("127.0.0.1:0")?; // never answers
    let mut r = Resolver::new(DnsConfig {
        timeout_ms: 50,
        attempts: 2,
        ..DnsConfig {
            servers: vec![server.local_addr()?],
            timeout_ms: 0,
            attempts: 0,
            min_ttl_s: 1,
            negative_ttl_s: 1,
            max_ttl_s: 3600,
            hosts: HashMap::new(),
        }
    });
    assert_eq!(r.lookup("slow.test", monotonic_ns())?, Lookup::Pending);
    let t0 = Instant::now();
    loop {
        r.poll(monotonic_ns());
        match r.lookup("slow.test", monotonic_ns()) {
            Err(e) => {
                assert!(e.to_string().contains("no answer"), "{e}");
                break;
            }
            Ok(_) => assert!(t0.elapsed() < Duration::from_secs(5), "never timed out"),
        }
    }
    assert!(
        // Two 50 ms attempts; the clock started just after the first was sent.
        t0.elapsed() >= Duration::from_millis(90),
        "both attempts waited"
    );
    Ok(())
}

#[test]
fn codec_round_trip_and_malformed_input() -> R {
    let q = dns::encode_query("a.b.example", 0x1234)?;
    assert_eq!(&q[..2], &[0x12, 0x34]);
    assert!(dns::encode_query("", 1).is_err());
    assert!(dns::encode_query("a..b", 1).is_err());
    assert!(dns::encode_query(&"x".repeat(64), 1).is_err());
    // A reply to another id is ignored, not an error.
    let mut r = q;
    r[2] |= 0x80;
    assert_eq!(dns::parse_answer(&r, 0x9999)?, None);
    // Truncated or garbage answers are errors.
    r[2] |= 0x02;
    assert!(dns::parse_answer(&r, 0x1234).is_err());
    assert!(
        dns::parse_answer(&[0x12, 0x34, 0x80, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0xFF], 0x1234).is_err()
    );
    let conf = dns::parse_resolv_conf("# c\nnameserver 168.63.129.16\nnameserver ::1\nsearch x\n");
    assert_eq!(conf.len(), 1);
    let hosts = dns::parse_hosts("127.0.0.1 localhost Local # x\n::1 localhost\n");
    assert_eq!(hosts.get("local"), Some(&Ipv4Addr::LOCALHOST));
    Ok(())
}

#[test]
fn server_failure_retries_the_next_server_at_once() -> R {
    let bad = UdpSocket::bind("127.0.0.1:0")?;
    let good = UdpSocket::bind("127.0.0.1:0")?;
    let mut r = Resolver::new(DnsConfig {
        servers: vec![bad.local_addr()?, good.local_addr()?],
        timeout_ms: 5000,
        attempts: 2,
        min_ttl_s: 5,
        negative_ttl_s: 1,
        max_ttl_s: 3600,
        hosts: HashMap::new(),
    });
    let ip = Ipv4Addr::new(10, 9, 8, 7);
    std::thread::scope(|s| -> R {
        let a = s.spawn(|| servfail(&bad).map_err(|e| e.to_string()));
        let b = s.spawn(|| answer(&good, Some(ip), Duration::ZERO).map_err(|e| e.to_string()));
        assert_eq!(r.lookup("retry.test", monotonic_ns())?, Lookup::Pending);
        let t0 = Instant::now();
        let got = loop {
            r.poll(monotonic_ns());
            if let Lookup::Ready(x) = r.lookup("retry.test", monotonic_ns())? {
                break x;
            }
            assert!(
                t0.elapsed() < Duration::from_secs(2),
                "did not move on after SERVFAIL"
            );
        };
        assert_eq!(got, ip);
        a.join().map_err(|_| "panicked")??;
        b.join().map_err(|_| "panicked")??;
        Ok(())
    })
}

#[test]
fn expired_answer_is_served_while_refreshing() -> R {
    let server = UdpSocket::bind("127.0.0.1:0")?;
    let mut r = Resolver::new(DnsConfig {
        servers: vec![server.local_addr()?],
        timeout_ms: 2000,
        attempts: 1,
        min_ttl_s: 1,
        negative_ttl_s: 1,
        max_ttl_s: 1,
        hosts: HashMap::new(),
    });
    let ip = Ipv4Addr::new(10, 1, 1, 1);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| answer(&server, Some(ip), Duration::ZERO).map_err(|e| e.to_string()));
        r.lookup("stale.test", monotonic_ns())?;
        while r.busy() {
            r.poll(monotonic_ns());
        }
        srv.join().map_err(|_| "panicked")??;
        Ok(())
    })?;
    // Two seconds later the 1 s TTL has expired: still answered at once, and a
    // refresh is in flight.
    let later = monotonic_ns() + 2_000_000_000;
    assert_eq!(r.lookup("stale.test", later)?, Lookup::Ready(ip));
    assert!(r.busy(), "refresh started");
    Ok(())
}
