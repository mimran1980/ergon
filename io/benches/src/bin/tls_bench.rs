//! TLS receive-path micro-benchmark: cost per record of turning ciphertext that is
//! already in memory into plaintext the application can read.
//!
//! ```text
//! tls_bench [--sizes 200,1000,4000,16000] [--mib 64] [--reps 3]
//! ```
//!
//! For each implementation and payload size a loopback server (rustls, TLS 1.3,
//! `TLS13_AES_128_GCM_SHA256` only) sends enough single-record messages to fill
//! `--mib` MiB. The client finishes the handshake over TCP, slurps the remaining
//! ciphertext into memory untimed, then times only the decrypt-and-deliver loop.
//! A counting global allocator reports allocations per record.
//!
//! | name | path |
//! |---|---|
//! | `inplace-ring` | rustls handshake, then our record layer decrypting in place with ring (what io-bench uses) |
//! | `inplace-awslc` | same, AEAD from aws-lc-rs (feature `aws-lc`) |
//! | `rustls-buffered-ring` | `ClientConnection` (ring): `read_tls` + `process_new_packets` + `reader().fill_buf()` (borrowed view) |
//! | `rustls-buffered-ring-copy` | same but `reader().read()` into a caller buffer (the usual way) |
//! | `rustls-unbuffered-ring` | `UnbufferedClientConnection`: `process_tls_records` + `ReadTraffic::next_record` |
//! | `rustls-buffered-awslc` | `ClientConnection` with the aws-lc-rs provider, `fill_buf` (feature `aws-lc`) |
//! | `openssl` | `openssl::ssl::SslStream::read` over the in-memory ciphertext (feature `openssl`) |

use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::io::{BufRead, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use io_bench::args::Args;
use io_bench::clock::monotonic_ns;
use io_bench::tls::{self, Opened, RecordLayer};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConnection, RootCertStore, ServerConfig, ServerConnection};

type BoxResult<T> = Result<T, Box<dyn Error>>;

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);

// SAFETY: forwards to the system allocator; only adds a relaxed counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

struct Pki {
    cert: CertificateDer<'static>,
    key_der: Vec<u8>,
    #[cfg_attr(not(feature = "openssl"), allow(dead_code))]
    pem: String,
}

fn server_config(p: &Pki) -> BoxResult<Arc<ServerConfig>> {
    let mut provider = rustls::crypto::ring::default_provider();
    provider
        .cipher_suites
        .retain(|s| s.suite() == rustls::CipherSuite::TLS13_AES_128_GCM_SHA256);
    Ok(Arc::new(
        ServerConfig::builder_with_provider(Arc::new(provider))
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(
                vec![p.cert.clone()],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(p.key_der.clone())),
            )?,
    ))
}

/// Starts a server that sends `count` messages of `size` bytes, one record each,
/// then closes. Returns the client side of the TCP connection.
fn serve(
    cfg: Arc<ServerConfig>,
    size: usize,
    count: usize,
) -> BoxResult<(TcpStream, std::thread::JoinHandle<()>)> {
    let l = TcpListener::bind("127.0.0.1:0")?;
    let addr = l.local_addr()?;
    let h = std::thread::spawn(move || {
        let Ok((mut sock, _)) = l.accept() else {
            return;
        };
        let Ok(mut conn) = ServerConnection::new(cfg) else {
            return;
        };
        while conn.is_handshaking() {
            if conn.complete_io(&mut sock).is_err() {
                return;
            }
        }
        let msg: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        for _ in 0..count {
            if conn.writer().write_all(&msg).is_err() {
                return;
            }
            while conn.wants_write() {
                if conn.write_tls(&mut sock).is_err() {
                    return;
                }
            }
        }
        conn.send_close_notify();
        while conn.wants_write() {
            if conn.write_tls(&mut sock).is_err() {
                return;
            }
        }
    });
    Ok((TcpStream::connect(addr)?, h))
}

fn slurp(sock: &mut TcpStream, mut buf: Vec<u8>) -> BoxResult<Vec<u8>> {
    sock.read_to_end(&mut buf)?;
    Ok(buf)
}

/// A stream that talks TCP during the handshake and then reads from memory.
#[derive(Debug)]
struct Swap {
    tcp: TcpStream,
    mem: Option<Cursor<Vec<u8>>>,
}

impl Read for Swap {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.mem {
            Some(c) => c.read(b),
            None => self.tcp.read(b),
        }
    }
}

impl Write for Swap {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match &self.mem {
            Some(_) => Ok(b.len()),
            None => self.tcp.write(b),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.tcp.flush()
    }
}

struct Outcome {
    records: u64,
    bytes: u64,
    ns: i64,
    allocs: u64,
    checksum: u64,
}

fn roots(p: &Pki) -> BoxResult<RootCertStore> {
    let mut r = RootCertStore::empty();
    r.add(p.cert.clone())?;
    Ok(r)
}

fn timed(f: impl FnOnce() -> BoxResult<(u64, u64, u64)>) -> BoxResult<Outcome> {
    let a0 = ALLOCS.load(Ordering::Relaxed);
    let t0 = monotonic_ns();
    let (records, bytes, checksum) = f()?;
    let ns = monotonic_ns() - t0;
    Ok(Outcome {
        records,
        bytes,
        ns,
        allocs: ALLOCS.load(Ordering::Relaxed) - a0,
        checksum,
    })
}

fn fold(sum: u64, p: &[u8]) -> u64 {
    sum.wrapping_add(p.len() as u64)
        .wrapping_add(u64::from(p[0]))
        .wrapping_add(u64::from(p[p.len() - 1]))
}

fn run_inplace_ring(p: &Pki, size: usize, count: usize) -> BoxResult<Outcome> {
    let (mut sock, h) = serve(server_config(p)?, size, count)?;
    let cfg = tls::client_config(roots(p)?)?;
    let (mut layer, rest) = tls::handshake(&mut sock, cfg, "localhost")?;
    let mut buf = slurp(&mut sock, rest)?;
    h.join().map_err(|_| "server panicked")?;
    let mut tx = Vec::with_capacity(4096);
    timed(|| {
        let (mut head, mut recs, mut bytes, mut sum) = (0usize, 0u64, 0u64, 0u64);
        while let Some(len) = RecordLayer::complete_record_len(&buf[head..])? {
            match layer.open_in_place(&mut buf[head..head + len], &mut tx) {
                Ok(Opened::AppData(n)) => {
                    let pl = &buf[head + tls::HEADER_LEN..head + tls::HEADER_LEN + n];
                    sum = fold(sum, std::hint::black_box(pl));
                    recs += 1;
                    bytes += n as u64;
                }
                Ok(Opened::Control) => {}
                Err(io_bench::Error::Closed) => break,
                Err(e) => return Err(e.into()),
            }
            head += len;
        }
        Ok((recs, bytes, sum))
    })
}

#[cfg(feature = "aws-lc")]
fn run_inplace_awslc(p: &Pki, size: usize, count: usize) -> BoxResult<Outcome> {
    use aws_lc_rs::aead::{AES_128_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
    let (mut sock, h) = serve(server_config(p)?, size, count)?;
    let cfg = tls::client_config(roots(p)?)?;
    let (secrets, _kconn, rest) = tls::handshake_raw(&mut sock, cfg, "localhost")?;
    let mut buf = slurp(&mut sock, rest)?;
    h.join().map_err(|_| "server panicked")?;
    let rustls::ConnectionTrafficSecrets::Aes128Gcm { key, iv } = &secrets.rx.1 else {
        return Err("expected AES-128-GCM".into());
    };
    let key = LessSafeKey::new(UnboundKey::new(&AES_128_GCM, key.as_ref()).map_err(|_| "key")?);
    let mut ivb = [0u8; 12];
    ivb.copy_from_slice(iv.as_ref());
    let mut seq = secrets.rx.0;
    timed(|| {
        let (mut head, mut recs, mut bytes, mut sum) = (0usize, 0u64, 0u64, 0u64);
        while let Some(len) = RecordLayer::complete_record_len(&buf[head..])? {
            let mut n = ivb;
            for (b, s) in n[4..].iter_mut().zip(seq.to_be_bytes()) {
                *b ^= s;
            }
            seq += 1;
            let mut aad = [0u8; 5];
            aad.copy_from_slice(&buf[head..head + 5]);
            let plain = key
                .open_in_place(
                    Nonce::assume_unique_for_key(n),
                    Aad::from(aad),
                    &mut buf[head + 5..head + len],
                )
                .map_err(|_| "open")?;
            let mut end = plain.len();
            while end > 0 && plain[end - 1] == 0 {
                end -= 1;
            }
            let ct = plain[end - 1];
            if ct == 21 {
                break;
            }
            if ct == 23 {
                sum = fold(sum, std::hint::black_box(&plain[..end - 1]));
                recs += 1;
                bytes += (end - 1) as u64;
            }
            head += len;
        }
        Ok((recs, bytes, sum))
    })
}

fn run_buffered(
    p: &Pki,
    provider: CryptoProvider,
    size: usize,
    count: usize,
    copy: bool,
) -> BoxResult<Outcome> {
    let (sock, h) = serve(server_config(p)?, size, count)?;
    let cfg = tls::client_config_with(roots(p)?, Arc::new(provider), false)?;
    let mut conn = ClientConnection::new(cfg, ServerName::try_from("localhost")?)?;
    let mut s = Swap {
        tcp: sock,
        mem: None,
    };
    while conn.is_handshaking() {
        conn.complete_io(&mut s)?;
    }
    let rest = slurp(&mut s.tcp, Vec::new())?;
    h.join().map_err(|_| "server panicked")?;
    s.mem = Some(Cursor::new(rest));
    let mut out = vec![0u8; 1 << 16];
    timed(|| {
        let (mut recs, mut bytes, mut sum) = (0u64, 0u64, 0u64);
        loop {
            let n = conn.read_tls(&mut s)?;
            let st = conn.process_new_packets()?;
            loop {
                if copy {
                    match conn.reader().read(&mut out) {
                        Ok(0) => break,
                        Ok(k) => {
                            sum = fold(sum, std::hint::black_box(&out[..k]));
                            recs += 1;
                            bytes += k as u64;
                        }
                        Err(_) => break,
                    }
                } else {
                    let mut r = conn.reader();
                    let k = match r.fill_buf() {
                        Ok(b) if !b.is_empty() => {
                            sum = fold(sum, std::hint::black_box(b));
                            b.len()
                        }
                        _ => break,
                    };
                    r.consume(k);
                    recs += 1;
                    bytes += k as u64;
                }
            }
            if n == 0 || st.peer_has_closed() {
                break;
            }
        }
        Ok((recs, bytes, sum))
    })
}

fn run_unbuffered(p: &Pki, size: usize, count: usize) -> BoxResult<Outcome> {
    use rustls::client::UnbufferedClientConnection;
    use rustls::unbuffered::{ConnectionState, UnbufferedStatus};
    let (mut sock, h) = serve(server_config(p)?, size, count)?;
    let cfg = tls::client_config_with(
        roots(p)?,
        Arc::new(rustls::crypto::ring::default_provider()),
        false,
    )?;
    let mut conn = UnbufferedClientConnection::new(cfg, ServerName::try_from("localhost")?)?;
    // Handshake over TCP.
    let mut inc = vec![0u8; 1 << 16];
    let mut inc_len = 0;
    let mut out = vec![0u8; 1 << 16];
    let mut out_len = 0;
    loop {
        let UnbufferedStatus { discard, state } = conn.process_tls_records(&mut inc[..inc_len]);
        let mut read = false;
        let mut done = false;
        match state? {
            ConnectionState::EncodeTlsData(mut e) => out_len += e.encode(&mut out[out_len..])?,
            ConnectionState::TransmitTlsData(t) => {
                sock.write_all(&out[..out_len])?;
                out_len = 0;
                t.done();
            }
            ConnectionState::BlockedHandshake => read = true,
            ConnectionState::WriteTraffic(_) => done = true,
            other => return Err(format!("handshake state {other:?}").into()),
        }
        inc.copy_within(discard..inc_len, 0);
        inc_len -= discard;
        if done {
            break;
        }
        if read {
            inc_len += sock.read(&mut inc[inc_len..])?;
        }
    }
    inc.truncate(inc_len);
    let mut buf = slurp(&mut sock, inc)?;
    h.join().map_err(|_| "server panicked")?;
    timed(|| {
        let (mut head, mut recs, mut bytes, mut sum) = (0usize, 0u64, 0u64, 0u64);
        loop {
            let UnbufferedStatus { mut discard, state } =
                conn.process_tls_records(&mut buf[head..]);
            let mut stop = false;
            match state? {
                ConnectionState::ReadTraffic(mut rt) => {
                    while let Some(r) = rt.next_record() {
                        let r = r?;
                        discard += r.discard;
                        sum = fold(sum, std::hint::black_box(r.payload));
                        recs += 1;
                        bytes += r.payload.len() as u64;
                    }
                }
                ConnectionState::WriteTraffic(_) => stop = head + discard >= buf.len(),
                ConnectionState::PeerClosed | ConnectionState::Closed => stop = true,
                ConnectionState::BlockedHandshake => stop = true,
                other => return Err(format!("state {other:?}").into()),
            }
            head += discard;
            if stop || head >= buf.len() {
                break;
            }
        }
        Ok((recs, bytes, sum))
    })
}

#[cfg(feature = "openssl")]
fn run_openssl(p: &Pki, size: usize, count: usize) -> BoxResult<Outcome> {
    use openssl::ssl::{SslConnector, SslMethod, SslVersion};
    use openssl::x509::X509;
    let (sock, h) = serve(server_config(p)?, size, count)?;
    let mut b = SslConnector::builder(SslMethod::tls_client())?;
    b.set_min_proto_version(Some(SslVersion::TLS1_3))?;
    b.cert_store_mut()
        .add_cert(X509::from_pem(p.pem.as_bytes())?)?;
    let conn = b.build();
    let mut ssl = conn
        .connect(
            "localhost",
            Swap {
                tcp: sock,
                mem: None,
            },
        )
        .map_err(|e| format!("openssl connect: {e}"))?;
    // OpenSSL processes session tickets lazily on the first read, which happens
    // in the timed loop for every implementation alike.
    let rest = slurp(&mut ssl.get_mut().tcp, Vec::new())?;
    h.join().map_err(|_| "server panicked")?;
    ssl.get_mut().mem = Some(Cursor::new(rest));
    let mut out = vec![0u8; 1 << 16];
    timed(|| {
        let (mut recs, mut bytes, mut sum) = (0u64, 0u64, 0u64);
        loop {
            match ssl.read(&mut out) {
                Ok(0) | Err(_) => break,
                Ok(k) => {
                    sum = fold(sum, std::hint::black_box(&out[..k]));
                    recs += 1;
                    bytes += k as u64;
                }
            }
        }
        Ok((recs, bytes, sum))
    })
}

fn main() -> BoxResult<()> {
    let a = Args::from_env();
    let sizes: Vec<usize> = a
        .list("sizes", "200,1000,4000,16000")
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()?;
    let mib: usize = a.num_or("mib", 64)?;
    let reps: usize = a.num_or("reps", 3)?;
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let pki = Pki {
        cert: CertificateDer::from(ck.cert.der().to_vec()),
        key_der: ck.signing_key.serialize_der(),
        pem: ck.cert.pem(),
    };
    #[allow(clippy::type_complexity, unused_mut)]
    let mut impls: Vec<(&str, Box<dyn Fn(&Pki, usize, usize) -> BoxResult<Outcome>>)> = vec![
        ("inplace-ring", Box::new(run_inplace_ring)),
        (
            "rustls-buffered-ring",
            Box::new(|p, s, c| {
                run_buffered(p, rustls::crypto::ring::default_provider(), s, c, false)
            }),
        ),
        (
            "rustls-buffered-ring-copy",
            Box::new(|p, s, c| {
                run_buffered(p, rustls::crypto::ring::default_provider(), s, c, true)
            }),
        ),
        ("rustls-unbuffered-ring", Box::new(run_unbuffered)),
    ];
    #[cfg(feature = "aws-lc")]
    {
        impls.push(("inplace-awslc", Box::new(run_inplace_awslc)));
        impls.push((
            "rustls-buffered-awslc",
            Box::new(|p, s, c| {
                run_buffered(
                    p,
                    rustls::crypto::aws_lc_rs::default_provider(),
                    s,
                    c,
                    false,
                )
            }),
        ));
    }
    #[cfg(feature = "openssl")]
    impls.push(("openssl", Box::new(run_openssl)));

    println!(
        "| implementation | payload B | messages | reads | ns/message (best of {reps}) | GB/s | allocs/message |"
    );
    println!("|---|---|---|---|---|---|---|");
    for &size in &sizes {
        let count = (mib << 20) / size;
        for (name, f) in &impls {
            let mut best: Option<Outcome> = None;
            for _ in 0..reps {
                let o = f(&pki, size, count)?;
                // `reader().read()` may merge records, so validate on bytes and
                // normalise by messages sent rather than by read calls.
                if o.bytes != (count * size) as u64 || o.records == 0 {
                    return Err(format!(
                        "{name}: got {} bytes in {} reads, want {}",
                        o.bytes,
                        o.records,
                        count * size
                    )
                    .into());
                }
                std::hint::black_box(o.checksum);
                if best.as_ref().is_none_or(|b| o.ns < b.ns) {
                    best = Some(o);
                }
            }
            let o = best.ok_or("no runs")?;
            println!(
                "| {name} | {size} | {count} | {} | {:.1} | {:.2} | {:.2} |",
                o.records,
                o.ns as f64 / count as f64,
                o.bytes as f64 / o.ns as f64,
                o.allocs as f64 / count as f64
            );
        }
    }
    Ok(())
}
