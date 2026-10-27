//! A `WsConn` fed by hand: a real TLS 1.3 handshake against the loopback lab
//! server, then server records forged with the handshake's receive key and
//! handed to `WsConn::feed` as if the kernel had written them. Every
//! platform: no driver, no socket reads.
#![allow(dead_code, missing_docs, clippy::cast_possible_truncation)]

use std::net::TcpStream;

use ergon_io::tls::{self, RecordLayer};
use ergon_io::{ConnId, RxMeta, WsConn, WsHandler, WsOptions};
use ring::aead::{
    AES_128_GCM, AES_256_GCM, Aad, Algorithm, CHACHA20_POLY1305, LessSafeKey, Nonce, UnboundKey,
};
use rustls::ConnectionTrafficSecrets;

use crate::support::{Lab, R};

/// Largest plaintext in one record.
pub const RECORD: usize = 1 << 14;

pub struct WsRig {
    pub conn: WsConn,
    key: LessSafeKey,
    iv: [u8; 12],
    seq: u64,
    /// The client's sending key, to read back what it queued.
    tx_key: LessSafeKey,
    tx_iv: [u8; 12],
    tx_seq: u64,
}

fn nonce(iv: [u8; 12], seq: u64) -> Nonce {
    let mut n = iv;
    for (b, s) in n[4..].iter_mut().zip(seq.to_be_bytes()) {
        *b ^= s;
    }
    Nonce::assume_unique_for_key(n)
}

fn key_of(s: &ConnectionTrafficSecrets) -> R<(LessSafeKey, [u8; 12])> {
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
    let ivb: [u8; 12] = iv.try_into().map_err(|_| "unexpected IV length")?;
    Ok((key, ivb))
}

impl WsRig {
    /// Handshakes with `lab` (which must not be serving anything else) and
    /// builds the connection with `opts`.
    pub fn new(lab: &Lab, opts: &WsOptions) -> R<Self> {
        let addr = lab.addr()?;
        std::thread::scope(|s| -> R<Self> {
            let srv = s.spawn(|| -> R {
                let (sock, _) = lab.listener.accept()?;
                drop(crate::support::Peer::serve_tls(lab, sock)?);
                Ok(())
            });
            let mut sock = TcpStream::connect(addr)?;
            let (secrets, kconn, _rest) =
                tls::handshake_raw(&mut sock, lab.client_cfg.clone(), "localhost")?;
            let (key, iv) = key_of(&secrets.rx.1)?;
            let seq = secrets.rx.0;
            let (tx_key, tx_iv) = key_of(&secrets.tx.1)?;
            let tx_seq = secrets.tx.0;
            let layer = RecordLayer::from_parts(&secrets, kconn)?;
            srv.join().map_err(|_| "server panicked")??;
            Ok(Self {
                conn: WsConn::from_record_layer(sock, layer, opts)?,
                key,
                iv,
                seq,
                tx_key,
                tx_iv,
                tx_seq,
            })
        })
    }

    /// The records the connection queued to send, decrypted with its
    /// sending key: each record's plaintext (inner content type removed),
    /// in order. The queue is cleared.
    pub fn sent_records(&mut self) -> R<Vec<Vec<u8>>> {
        let mut q = self.conn.queued_tx().to_vec();
        self.conn.clear_tx();
        let mut out = Vec::new();
        let mut at = 0;
        while at < q.len() {
            let hdr: [u8; 5] = q.get(at..at + 5).ok_or("short header")?.try_into()?;
            if hdr[..3] != [23, 3, 3] {
                return Err(format!("not an application-data record: {hdr:?}").into());
            }
            let len = usize::from(u16::from_be_bytes([hdr[3], hdr[4]]));
            let body = q.get_mut(at + 5..at + 5 + len).ok_or("short record")?;
            let n = nonce(self.tx_iv, self.tx_seq);
            self.tx_seq += 1;
            let plain = self
                .tx_key
                .open_in_place(n, Aad::from(hdr), body)
                .map_err(|e| e.to_string())?;
            let (&ct, data) = plain.split_last().ok_or("empty record")?;
            if ct != 23 {
                return Err(format!("inner content type {ct}").into());
            }
            out.push(data.to_vec());
            at += 5 + len;
        }
        Ok(out)
    }

    /// One application-data record of `plain` (at most [`RECORD`] bytes),
    /// sealed as the server would.
    pub fn record(&mut self, plain: &[u8]) -> R<Vec<u8>> {
        if plain.len() > RECORD {
            return Err("record plaintext over 16 KiB".into());
        }
        let mut out = Vec::with_capacity(plain.len() + 22);
        out.extend_from_slice(&[23, 0x03, 0x03]);
        out.extend_from_slice(&((plain.len() + 1 + 16) as u16).to_be_bytes());
        out.extend_from_slice(plain);
        out.push(23);
        let aad: [u8; 5] = out[..5].try_into()?;
        let n = nonce(self.iv, self.seq);
        self.seq += 1;
        let tag = self
            .key
            .seal_in_place_separate_tag(n, Aad::from(aad), &mut out[5..])
            .map_err(|e| e.to_string())?;
        out.extend_from_slice(tag.as_ref());
        Ok(out)
    }

    /// `stream` cut into records at `cuts` (offsets into `stream`, any
    /// order), and further wherever a piece exceeds [`RECORD`].
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
                out.push(self.record(piece)?);
            }
        }
        Ok(out)
    }

    /// Delivers `cipher` as one read.
    pub fn feed<H: WsHandler>(&mut self, cipher: &[u8], h: &mut H) -> R<usize> {
        Ok(self
            .conn
            .feed(ConnId::from_parts(0, 0), cipher, &RxMeta::default(), h)?)
    }

    /// Seals `stream` cut at `cuts` and delivers each record as its own read.
    pub fn feed_records<H: WsHandler>(
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

    /// Seals `stream` cut at `cuts` and delivers every record in one read.
    pub fn feed_one_read<H: WsHandler>(
        &mut self,
        stream: &[u8],
        cuts: &[usize],
        h: &mut H,
    ) -> R<usize> {
        let all: Vec<u8> = self.records(stream, cuts)?.concat();
        self.feed(&all, h)
    }
}

pub use crate::support::frame;

/// `payload` as a fragmented message: `parts` frames, `opcode` first then
/// continuations, the last with FIN. `parts == 1` is one unfragmented frame.
pub fn fragmented(opcode: u8, payload: &[u8], parts: usize) -> R<Vec<u8>> {
    let parts = parts.max(1);
    let step = payload.len().div_ceil(parts).max(1);
    let mut chunks: Vec<&[u8]> = payload.chunks(step).collect();
    while chunks.len() < parts {
        chunks.push(&[]);
    }
    let mut out = Vec::new();
    for (i, c) in chunks.iter().enumerate() {
        let op = if i == 0 {
            opcode
        } else {
            ergon_io::ws::OP_CONT
        };
        out.extend_from_slice(&frame(op, i + 1 == chunks.len(), c)?);
    }
    Ok(out)
}
