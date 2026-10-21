//! Non-blocking DNS: `/etc/hosts`, a TTL cache, and A-record queries over
//! UDP, advanced by [`Resolver::poll`] without waiting.
//!
//! - IPv4 (A records) only: what exchange endpoints publish.
//! - Names are fully qualified: no search domains.
//! - A truncated answer is an error, not retried over TCP (A answers for one
//!   name fit in one datagram).
//! - Random query ids; a fresh socket (random source port) per query.
//! - Lowest connect latency: resolve early ([`Resolver::prefetch`]) so
//!   connects start from the cache.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use crate::error::{Error, Result};

const DNS_PORT: u16 = 53;
const TYPE_A: u16 = 1;
const CLASS_IN: u16 = 1;
const MAX_NAME: usize = 253;

/// Resolver settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsConfig {
    /// Name servers, tried in order.
    pub servers: Vec<SocketAddr>,
    /// Time to wait for one answer before retrying, in milliseconds.
    pub timeout_ms: u32,
    /// Attempts per query (rotating through the servers).
    pub attempts: u32,
    /// Shortest time an answer is cached, in seconds (guards against TTL 0).
    pub min_ttl_s: u32,
    /// How long a failure is cached, in seconds.
    pub negative_ttl_s: u32,
    /// Longest time an answer is cached, in seconds.
    pub max_ttl_s: u32,
    /// Static entries (`/etc/hosts`).
    pub hosts: HashMap<String, Ipv4Addr>,
}

impl DnsConfig {
    /// Reads `/etc/resolv.conf` and `/etc/hosts` (blocking file reads: call this
    /// at start-up). Missing files give no servers or no static entries.
    #[must_use]
    pub fn system() -> Self {
        let servers = std::fs::read_to_string("/etc/resolv.conf")
            .map(|s| parse_resolv_conf(&s))
            .unwrap_or_default();
        let hosts = std::fs::read_to_string("/etc/hosts")
            .map(|s| parse_hosts(&s))
            .unwrap_or_default();
        Self {
            servers,
            timeout_ms: 1000,
            attempts: 3,
            min_ttl_s: 5,
            negative_ttl_s: 1,
            max_ttl_s: 3600,
            hosts,
        }
    }
}

/// Name servers from `resolv.conf` text (IPv4 only).
///
/// Hidden test lever: [`DnsConfig::system`] parses the system files; a caller
/// builds [`DnsConfig`] from its fields.
#[doc(hidden)]
#[must_use]
pub fn parse_resolv_conf(text: &str) -> Vec<SocketAddr> {
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            (it.next() == Some("nameserver"))
                .then(|| it.next())
                .flatten()
                .and_then(|a| a.parse::<Ipv4Addr>().ok())
                .map(|ip| SocketAddr::new(IpAddr::V4(ip), DNS_PORT))
        })
        .collect()
}

/// IPv4 entries from `/etc/hosts` text (first address wins per name).
///
/// Hidden test lever: see [`parse_resolv_conf`].
#[doc(hidden)]
#[must_use]
pub fn parse_hosts(text: &str) -> HashMap<String, Ipv4Addr> {
    let mut m = HashMap::new();
    for l in text.lines() {
        let l = l.split('#').next().unwrap_or_default();
        let mut it = l.split_whitespace();
        let Some(Ok(ip)) = it.next().map(str::parse::<Ipv4Addr>) else {
            continue;
        };
        for name in it {
            m.entry(name.to_ascii_lowercase()).or_insert(ip);
        }
    }
    m
}

enum Entry {
    Found {
        addrs: Vec<Ipv4Addr>,
        expires_ns: i64,
    },
    Failed {
        error: String,
        expires_ns: i64,
    },
}

struct Query {
    host: String,
    sock: UdpSocket,
    id: u16,
    packet: Vec<u8>,
    sent_ns: i64,
    attempt: u32,
}

/// What [`Resolver::lookup`] knows about a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// An address (literal, static entry or cached answer).
    Ready(Ipv4Addr),
    /// A query is in flight; ask again after a later [`Resolver::poll`].
    Pending,
}

/// A non-blocking stub resolver with a cache.
pub struct Resolver {
    cfg: DnsConfig,
    cache: HashMap<String, Entry>,
    queries: Vec<Query>,
    next: u32,
}

impl Resolver {
    /// A resolver with `cfg`.
    #[must_use]
    pub fn new(cfg: DnsConfig) -> Self {
        Self {
            cfg,
            cache: HashMap::new(),
            queries: Vec::new(),
            next: 0,
        }
    }

    /// Whether queries are in flight.
    #[must_use]
    pub const fn busy(&self) -> bool {
        !self.queries.is_empty()
    }

    /// Resolves `host` if the answer is known; otherwise starts a query (once)
    /// and returns [`Lookup::Pending`].
    ///
    /// # Errors
    /// The name is invalid, the last query failed (cached for `negative_ttl_s`)
    /// or no name server is configured.
    pub fn lookup(&mut self, host: &str, now_ns: i64) -> Result<Lookup> {
        if let Ok(ip) = host.parse::<Ipv4Addr>() {
            return Ok(Lookup::Ready(ip));
        }
        let key = host.trim_end_matches('.').to_ascii_lowercase();
        if let Some(ip) = self.cfg.hosts.get(&key) {
            return Ok(Lookup::Ready(*ip));
        }
        let refreshing = self.queries.iter().any(|q| q.host == key);
        let (answer, fresh) = match self.cache.get(&key) {
            Some(Entry::Found { addrs, expires_ns }) => {
                // Rotate through the answers so connections spread over them.
                let i = self.next as usize % addrs.len().max(1);
                self.next = self.next.wrapping_add(1);
                (addrs.get(i).copied(), *expires_ns > now_ns)
            }
            Some(Entry::Failed { error, expires_ns }) if *expires_ns > now_ns => {
                return Err(Error::Dns(error.clone()));
            }
            _ => (None, false),
        };
        // Expired or missing: (re)query; an expired answer is still served
        // while the refresh is in flight.
        if !fresh && !refreshing {
            self.start(key, now_ns)?;
        }
        Ok(answer.map_or(Lookup::Pending, Lookup::Ready))
    }

    /// Starts a query for `host` unless it is cached or in flight, so later
    /// connections find it ready.
    ///
    /// # Errors
    /// As [`Resolver::lookup`].
    pub fn prefetch(&mut self, host: &str, now_ns: i64) -> Result<()> {
        self.lookup(host, now_ns).map(|_| ())
    }

    fn start(&mut self, host: String, now_ns: i64) -> Result<()> {
        if self.cfg.servers.is_empty() {
            return Err(Error::Dns("no name server configured".into()));
        }
        let id = random_u16()?;
        let packet = encode_query(&host, id)?;
        let sock = self.socket(0)?;
        sock.send(&packet)?;
        self.queries.push(Query {
            host,
            sock,
            id,
            packet,
            sent_ns: now_ns,
            attempt: 0,
        });
        Ok(())
    }

    fn socket(&self, attempt: u32) -> Result<UdpSocket> {
        let n = self.cfg.servers.len().max(1);
        let server = self.cfg.servers[attempt as usize % n];
        let sock = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        sock.set_nonblocking(true)?;
        sock.connect(server)?;
        Ok(sock)
    }

    /// Advances in-flight queries without blocking: reads answers, retries on
    /// timeout, and caches results. Cheap when nothing is in flight.
    pub fn poll(&mut self, now_ns: i64) {
        let mut i = 0;
        while i < self.queries.len() {
            match self.advance(i, now_ns) {
                Some(entry) => {
                    let q = self.queries.swap_remove(i);
                    // A failed refresh keeps the last good answer.
                    let keep = matches!(entry, Entry::Failed { .. })
                        && matches!(self.cache.get(&q.host), Some(Entry::Found { .. }));
                    if !keep {
                        self.cache.insert(q.host, entry);
                    }
                }
                None => i += 1,
            }
        }
    }

    fn advance(&mut self, i: usize, now_ns: i64) -> Option<Entry> {
        let mut buf = [0u8; 1500];
        let mut retry_now = false;
        let mut last_error = None;
        let q = &mut self.queries[i];
        loop {
            match q.sock.recv(&mut buf) {
                Ok(n) => match parse_answer(&buf[..n], q.id) {
                    Ok(None) => {} // not ours (stale or spoofed): keep reading
                    Ok(Some((addrs, ttl))) => {
                        // RFC 2181 §8: a TTL with the top bit set means 0.
                        let ttl = if ttl & 0x8000_0000 != 0 { 0 } else { ttl };
                        let ttl = ttl.clamp(
                            self.cfg.min_ttl_s,
                            self.cfg.max_ttl_s.max(self.cfg.min_ttl_s),
                        );
                        return Some(Entry::Found {
                            addrs,
                            expires_ns: now_ns.saturating_add(i64::from(ttl) * 1_000_000_000),
                        });
                    }
                    // Authoritative negatives end the query; server errors and
                    // malformed answers try the next attempt at once.
                    Err(e) if is_final(&e) => return Some(self.failed(e.to_string(), now_ns)),
                    Err(e) => {
                        last_error = Some(e.to_string());
                        retry_now = true;
                        break;
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                // ICMP port unreachable and the like: try the next attempt.
                Err(e) => {
                    last_error = Some(e.to_string());
                    retry_now = true;
                    break;
                }
            }
        }
        let q = &self.queries[i];
        let waited = now_ns.saturating_sub(q.sent_ns);
        if !retry_now && waited < i64::from(self.cfg.timeout_ms) * 1_000_000 {
            return None;
        }
        let attempt = q.attempt + 1;
        if attempt >= self.cfg.attempts {
            let why = last_error.unwrap_or_else(|| "no answer".to_owned());
            return Some(self.failed(format!("{}: {why}", q.host), now_ns));
        }
        match self.socket(attempt).and_then(|s| {
            s.send(&self.queries[i].packet)?;
            Ok(s)
        }) {
            Ok(s) => {
                let q = &mut self.queries[i];
                q.sock = s;
                q.attempt = attempt;
                q.sent_ns = now_ns;
                None
            }
            Err(e) => Some(self.failed(e.to_string(), now_ns)),
        }
    }

    fn failed(&self, error: String, now_ns: i64) -> Entry {
        Entry::Failed {
            error,
            expires_ns: now_ns.saturating_add(i64::from(self.cfg.negative_ttl_s) * 1_000_000_000),
        }
    }
}

/// Answers that retrying another server would not change.
fn is_final(e: &Error) -> bool {
    matches!(e, Error::Dns(m) if m == "no such name" || m == "no A record")
}

fn random_u16() -> Result<u16> {
    let mut b = [0u8; 2];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut b)
        .map_err(|_| Error::Config("no system randomness".into()))?;
    Ok(u16::from_ne_bytes(b))
}

/// Encodes a recursive A query for `host`.
///
/// Hidden test lever: the wire codec is an implementation detail of
/// [`Resolver`]; `io/tests/dns.rs` drives it directly.
///
/// # Errors
/// The name is empty, too long, or has an empty or over-long label.
#[doc(hidden)]
pub fn encode_query(host: &str, id: u16) -> Result<Vec<u8>> {
    let host = host.trim_end_matches('.');
    if host.is_empty() || host.len() > MAX_NAME {
        return Err(Error::Dns(format!("invalid name {host:?}")));
    }
    let mut p = Vec::with_capacity(18 + host.len());
    p.extend_from_slice(&id.to_be_bytes());
    p.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]); // RD; QDCOUNT 1
    for label in host.split('.') {
        let len = u8::try_from(label.len())
            .ok()
            .filter(|&l| (1..=63).contains(&l))
            .ok_or_else(|| Error::Dns(format!("invalid name {host:?}")))?;
        p.push(len);
        p.extend_from_slice(label.as_bytes());
    }
    p.push(0);
    p.extend_from_slice(&TYPE_A.to_be_bytes());
    p.extend_from_slice(&CLASS_IN.to_be_bytes());
    Ok(p)
}

/// Parses an answer to query `id`: `Ok(None)` if it is not an answer to it,
/// otherwise the A records and the smallest TTL.
///
/// Hidden test lever: see [`encode_query`].
///
/// # Errors
/// The server reported an error, the answer was truncated or malformed, or it
/// held no A record.
#[doc(hidden)]
pub fn parse_answer(msg: &[u8], id: u16) -> Result<Option<(Vec<Ipv4Addr>, u32)>> {
    let bad = || Error::Dns("malformed answer".into());
    let u16_at = |i: usize| -> Result<u16> {
        msg.get(i..i + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
            .ok_or_else(bad)
    };
    if msg.len() < 12 || u16_at(0)? != id || msg[2] & 0x80 == 0 {
        return Ok(None);
    }
    if msg[2] & 0x02 != 0 {
        return Err(Error::Dns("answer truncated".into()));
    }
    match msg[3] & 0x0F {
        0 => {}
        3 => return Err(Error::Dns("no such name".into())),
        rcode => return Err(Error::Dns(format!("server error (rcode {rcode})"))),
    }
    let qd = u16_at(4)?;
    let an = u16_at(6)?;
    let mut pos = 12;
    for _ in 0..qd {
        pos = skip_name(msg, pos).ok_or_else(bad)? + 4;
    }
    let mut addrs = Vec::new();
    let mut ttl = u32::MAX;
    for _ in 0..an {
        pos = skip_name(msg, pos).ok_or_else(bad)?;
        let rtype = u16_at(pos)?;
        let class = u16_at(pos + 2)?;
        let rttl = msg
            .get(pos + 4..pos + 8)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(bad)?;
        let rdlen = usize::from(u16_at(pos + 8)?);
        let rdata = msg.get(pos + 10..pos + 10 + rdlen).ok_or_else(bad)?;
        if rtype == TYPE_A && class == CLASS_IN && rdlen == 4 {
            addrs.push(Ipv4Addr::new(rdata[0], rdata[1], rdata[2], rdata[3]));
            ttl = ttl.min(rttl);
        }
        pos += 10 + rdlen;
    }
    if addrs.is_empty() {
        return Err(Error::Dns("no A record".into()));
    }
    Ok(Some((addrs, ttl)))
}

/// Skips a (possibly compressed) name; returns the position after it.
fn skip_name(msg: &[u8], mut pos: usize) -> Option<usize> {
    loop {
        let len = *msg.get(pos)?;
        match len {
            0 => return Some(pos + 1),
            l if l & 0xC0 == 0xC0 => return Some(pos + 2),
            l if l & 0xC0 == 0 => pos += 1 + usize::from(l),
            _ => return None,
        }
    }
}
