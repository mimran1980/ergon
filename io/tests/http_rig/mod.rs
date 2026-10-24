//! An `HttpConn` fed by hand: a real TLS 1.3 handshake against the loopback
//! lab server, then server records forged with the handshake's receive key
//! and handed to `HttpConn::feed` as if the kernel had written them. The
//! send key seals expected requests, so queued ciphertext can be compared
//! byte for byte. Every platform: no driver, no socket reads.
#![allow(dead_code, missing_docs, clippy::cast_possible_truncation)]

use std::net::TcpStream;

use ergon_io::tls::{self, RecordLayer};
use ergon_io::{ConnId, HttpConn, HttpHandler, HttpOptions, RxMeta};
use ring::aead::{
    AES_128_GCM, AES_256_GCM, Aad, Algorithm, CHACHA20_POLY1305, LessSafeKey, Nonce, UnboundKey,
};
use rustls::ConnectionTrafficSecrets;

use crate::support::{Lab, R};

/// Largest plaintext in one record.
pub const RECORD: usize = 1 << 14;

/// One direction's AEAD key, IV and next sequence number.
pub struct Sealer {
    key: LessSafeKey,
    iv: [u8; 12],
    seq: u64,
}

impl Sealer {
    fn new(seq: u64, s: &ConnectionTrafficSecrets) -> R<Self> {
        let (alg, key, iv): (&'static Algorithm, &[u8], &[u8]) = match s {
            ConnectionTrafficSecrets::Aes128Gcm { key, iv } => {
                (&AES_128_GCM, key.as_ref(), iv.as_ref())
            }
            ConnectionTrafficSecrets::Aes256Gcm { key, iv } => {
                (&AES_256_GCM, key.as_ref(), iv.as_ref())
            }
            ConnectionTrafficSecrets::Chacha20Poly1305 { key, iv } => {
                (&CHACHA20_POLY1305, key.as_ref(), iv.as_ref())
            }
            _ => return Err("unsupported cipher suite".into()),
        };
        let key = LessSafeKey::new(UnboundKey::new(alg, key).map_err(|_| "bad key")?);
        let iv: [u8; 12] = iv.try_into().map_err(|_| "unexpected IV length")?;
        Ok(Self { key, iv, seq })
    }

    /// One application-data record of `plain`, sealed with the next
    /// sequence number.
    pub fn seal(&mut self, plain: &[u8]) -> R<Vec<u8>> {
        if plain.len() > RECORD {
            return Err("record plaintext over 16 KiB".into());
        }
        let mut out = Vec::with_capacity(plain.len() + 22);
        out.extend_from_slice(&[23, 0x03, 0x03]);
        out.extend_from_slice(&((plain.len() + 1 + 16) as u16).to_be_bytes());
        out.extend_from_slice(plain);
        out.push(23);
        let aad: [u8; 5] = out[..5].try_into()?;
        let mut n = self.iv;
        for (b, s) in n[4..].iter_mut().zip(self.seq.to_be_bytes()) {
            *b ^= s;
        }
        self.seq += 1;
        let tag = self
            .key
            .seal_in_place_separate_tag(
                Nonce::assume_unique_for_key(n),
                Aad::from(aad),
                &mut out[5..],
            )
            .map_err(|e| e.to_string())?;
        out.extend_from_slice(tag.as_ref());
        Ok(out)
    }

    /// The plaintext of every application-data record in `records`, opened
    /// with consecutive sequence numbers.
    pub fn open_all(&mut self, mut records: &[u8]) -> R<Vec<Vec<u8>>> {
        let mut out = Vec::new();
        while !records.is_empty() {
            let len = 5 + usize::from(u16::from_be_bytes([records[3], records[4]]));
            let mut rec = records[..len].to_vec();
            records = &records[len..];
            let aad: [u8; 5] = rec[..5].try_into()?;
            let mut n = self.iv;
            for (b, s) in n[4..].iter_mut().zip(self.seq.to_be_bytes()) {
                *b ^= s;
            }
            self.seq += 1;
            let plain = self
                .key
                .open_in_place(
                    Nonce::assume_unique_for_key(n),
                    Aad::from(aad),
                    &mut rec[5..],
                )
                .map_err(|e| e.to_string())?;
            let (&ct, data) = plain.split_last().ok_or("empty record")?;
            if ct != 23 {
                return Err("not application data".into());
            }
            out.push(data.to_vec());
        }
        Ok(out)
    }
}

pub struct HttpRig {
    pub conn: HttpConn,
    /// Seals what the server sends (the client's receive key).
    pub rx: Sealer,
    /// Seals what the client sends (the client's send key).
    pub tx: Sealer,
    /// Opens what the client sent (the same key, its own sequence).
    pub tx_open: Sealer,
}

impl HttpRig {
    /// Handshakes with `lab` (which must not be serving anything else) and
    /// builds the connection to `lab.target("")` with `opts`.
    pub fn new(lab: &Lab, opts: HttpOptions) -> R<Self> {
        let addr = lab.addr()?;
        let target = lab.target("")?;
        std::thread::scope(|s| -> R<Self> {
            let srv = s.spawn(|| -> R {
                let (sock, _) = lab.listener.accept()?;
                drop(crate::support::Peer::serve_tls(lab, sock)?);
                Ok(())
            });
            let mut sock = TcpStream::connect(addr)?;
            let (secrets, kconn, _rest) =
                tls::handshake_raw(&mut sock, lab.client_cfg.clone(), "localhost")?;
            let rx = Sealer::new(secrets.rx.0, &secrets.rx.1)?;
            let tx = Sealer::new(secrets.tx.0, &secrets.tx.1)?;
            let tx_open = Sealer::new(secrets.tx.0, &secrets.tx.1)?;
            let layer = RecordLayer::from_parts(&secrets, kconn)?;
            srv.join().map_err(|_| "server panicked")??;
            Ok(Self {
                conn: HttpConn::from_record_layer(sock, layer, &target, opts)?,
                rx,
                tx,
                tx_open,
            })
        })
    }

    /// `stream` cut into server records at `cuts` (offsets into `stream`,
    /// any order), and further wherever a piece exceeds [`RECORD`].
    pub fn records(&mut self, stream: &[u8], cuts: &[usize]) -> R<Vec<Vec<u8>>> {
        let mut at: Vec<usize> = cuts
            .iter()
            .copied()
            .filter(|&c| c > 0 && c < stream.len())
            .collect();
        at.push(0);
        at.push(stream.len());
        at.sort_unstable();
        at.dedup();
        let mut out = Vec::new();
        for w in at.windows(2) {
            for piece in stream[w[0]..w[1]].chunks(RECORD) {
                out.push(self.rx.seal(piece)?);
            }
        }
        Ok(out)
    }

    /// Delivers `cipher` as one read.
    pub fn feed<H: HttpHandler>(&mut self, cipher: &[u8], h: &mut H) -> R<usize> {
        Ok(self
            .conn
            .feed(ConnId::from_parts(0, 0), cipher, &RxMeta::default(), h)?)
    }

    /// Seals `stream` cut at `cuts` and delivers each record as its own read.
    pub fn feed_records<H: HttpHandler>(
        &mut self,
        stream: &[u8],
        cuts: &[usize],
        h: &mut H,
    ) -> R<usize> {
        let mut n = 0;
        for r in self.records(stream, cuts)? {
            n += self.feed(&r, h)?;
        }
        Ok(n)
    }
}
