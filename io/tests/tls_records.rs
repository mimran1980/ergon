//! Post-handshake record-layer obligations the kernel API leaves to the caller:
//! reassembly of a handshake message that spans records, and the cipher suite's
//! confidentiality limit.
//!
//! TLS 1.3 lets a handshake message be split across records (RFC 8446 §5.1),
//! and a real server does this with session tickets. rustls's
//! `KernelConnection` only stores a ticket body it is handed and computes new
//! traffic secrets, so reassembly and the record-count policy live here.
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod support;

use std::net::TcpStream;
use std::sync::Arc;

use ergon_io::tls::{self, Opened, RecordLayer};
use ring::aead::{
    AES_128_GCM, AES_256_GCM, Aad, Algorithm, CHACHA20_POLY1305, LessSafeKey, Nonce, UnboundKey,
};
use rustls::client::ClientSessionStore;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ConnectionTrafficSecrets, RootCertStore};
use support::{Lab, R};

/// Outer content type of an encrypted record (TLS 1.3 puts everything there).
const CT_APPDATA: u8 = 23;
/// Inner content type of a handshake message inside the record.
const CT_HANDSHAKE: u8 = 22;

const HS_NEW_SESSION_TICKET: u8 = 4;
const HS_KEY_UPDATE: u8 = 24;

/// A completed handshake plus enough of the receive key to write records the
/// peer could have sent, so a test can split a post-handshake message across
/// records at an exact byte boundary.
struct Rig {
    layer: RecordLayer,
    rx_key: LessSafeKey,
    rx_iv: [u8; 12],
    /// Sequence number used for the next forged record. The key never changes:
    /// it is the key the handshake produced, so a record forged after the
    /// layer has rotated is deliberately under the old key.
    rx_seq: u64,
}

impl Rig {
    /// Writes one record of `inner_type` + `payload`, sealed the way the peer
    /// would seal it at that sequence number.
    fn forge(&mut self, inner_type: u8, payload: &[u8]) -> R<Vec<u8>> {
        let mut out = Vec::new();
        out.extend_from_slice(&[CT_APPDATA, 0x03, 0x03]);
        out.extend_from_slice(&((payload.len() + 1 + 16) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out.push(inner_type);
        let mut aad = [0u8; 5];
        aad.copy_from_slice(&out[..5]);
        let mut n = self.rx_iv;
        for (b, s) in n[4..].iter_mut().zip(self.rx_seq.to_be_bytes()) {
            *b ^= s;
        }
        self.rx_seq += 1;
        let tag = self
            .rx_key
            .seal_in_place_separate_tag(
                Nonce::assume_unique_for_key(n),
                Aad::from(aad),
                &mut out[5..],
            )
            .map_err(|e| e.to_string())?;
        out.extend_from_slice(tag.as_ref());
        Ok(out)
    }
}

/// Counts the TLS records in a sealed buffer.
fn count_records(buf: &[u8]) -> R<usize> {
    let (mut head, mut n) = (0usize, 0usize);
    while head < buf.len() {
        let len = RecordLayer::complete_record_len(&buf[head..])?
            .ok_or("a sealed record is always complete")?;
        head += len;
        n += 1;
    }
    Ok(n)
}

/// Completes a handshake against the lab server and hands back the record
/// layer together with the client's receive key.
/// Builds a ring key from whatever the handshake negotiated.
fn rx_key_of(s: &ConnectionTrafficSecrets) -> R<(LessSafeKey, [u8; 12])> {
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
    let mut ivb = [0u8; 12];
    if iv.len() != 12 {
        return Err("unexpected IV length".into());
    }
    ivb.copy_from_slice(iv);
    Ok((key, ivb))
}

fn rig_with(lab: &Lab, cfg: Arc<ClientConfig>) -> R<Rig> {
    let addr = lab.addr()?;
    std::thread::scope(|s| -> R<Rig> {
        let srv = s.spawn(|| -> R {
            let (sock, _) = lab.listener.accept()?;
            let p = support::Peer::serve_tls(lab, sock)?;
            drop(p);
            Ok(())
        });
        let mut sock = TcpStream::connect(addr)?;
        let (secrets, kconn, _rest) = tls::handshake_raw(&mut sock, cfg, "localhost")?;
        let (rx_key, rx_iv) = rx_key_of(&secrets.rx.1)?;
        let rx_seq = secrets.rx.0;
        let layer = RecordLayer::from_parts(&secrets, kconn)?;
        drop(sock);
        srv.join().map_err(|_| "server panicked")??;
        Ok(Rig {
            layer,
            rx_key,
            rx_iv,
            rx_seq,
        })
    })
}

fn rig(lab: &Lab) -> R<Rig> {
    rig_with(lab, lab.client_cfg.clone())
}

/// A client config whose session tickets land in `store`, so a test can see
/// whether a ticket was accepted.
fn config_with_store(lab: &Lab, store: Arc<tls::NonBlockingSessionStore>) -> R<Arc<ClientConfig>> {
    use rustls::pki_types::pem::PemObject;
    let mut roots = RootCertStore::empty();
    roots.add(rustls::pki_types::CertificateDer::from_pem_slice(
        lab.cert_pem.as_bytes(),
    )?)?;
    Ok(tls::client_config_with_store(
        roots,
        Arc::new(rustls::crypto::ring::default_provider()),
        true,
        store,
    )?)
}

/// A TLS 1.3 `NewSessionTicket` body: lifetime, age add, a one-byte nonce, a
/// three-byte ticket and no extensions.
fn nst_body() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&3600u32.to_be_bytes());
    b.extend_from_slice(&0u32.to_be_bytes());
    b.extend_from_slice(&[1, 0x5a]);
    b.extend_from_slice(&[0, 3, b'a', b'b', b'c']);
    b.extend_from_slice(&[0, 0]);
    b
}

/// Wraps a handshake message: type, 24-bit length, body.
fn hs_msg(typ: u8, body: &[u8]) -> Vec<u8> {
    let len = body.len();
    let mut m = vec![
        typ,
        ((len >> 16) & 0xff) as u8,
        ((len >> 8) & 0xff) as u8,
        (len & 0xff) as u8,
    ];
    m.extend_from_slice(body);
    m
}

fn open(rig: &mut Rig, record: &mut [u8]) -> R<Opened> {
    let mut tx = Vec::new();
    Ok(rig.layer.open_in_place(record, &mut tx)?)
}

/// A post-handshake handshake message split across two records is a legal
/// TLS 1.3 stream (RFC 8446 §5.1). Every split point must work, including one
/// in the middle of the four-byte handshake header.
#[test]
fn a_handshake_message_split_across_records_is_reassembled() -> R {
    // KeyUpdate(update_not_requested): type 24, length 1, body 0.
    let msg = hs_msg(HS_KEY_UPDATE, &[0]);
    assert_eq!(msg, vec![24, 0, 0, 1, 0]);
    for at in 1..msg.len() {
        let lab = Lab::new()?;
        let mut r = rig(&lab)?;
        assert_eq!(r.layer.key_updates(), 0, "split at {at}");
        let mut first = r.forge(CT_HANDSHAKE, &msg[..at])?;
        let mut second = r.forge(CT_HANDSHAKE, &msg[at..])?;
        let got = open(&mut r, &mut first);
        assert!(
            matches!(got, Ok(Opened::Control)),
            "first half at split {at} must be held, got {got:?}"
        );
        let got = open(&mut r, &mut second);
        assert!(
            matches!(got, Ok(Opened::Control)),
            "second half at split {at} must complete the message, got {got:?}"
        );
        assert_eq!(
            r.layer.key_updates(),
            1,
            "split at {at} must update the key"
        );
        // The key really rotated: a record sealed under the handshake key is
        // now refused.
        let mut stale = r.forge(CT_HANDSHAKE, &hs_msg(HS_KEY_UPDATE, &[0]))?;
        let got = open(&mut r, &mut stale);
        assert!(
            got.is_err(),
            "a record under the old key must be refused after the update \
             (split {at}), got {got:?}"
        );
    }
    Ok(())
}

/// A `NewSessionTicket` split across two records is the case real servers
/// produce. Both halves must be accepted and the ticket stored.
#[test]
fn a_split_new_session_ticket_is_accepted_and_stored() -> R {
    let lab = Lab::new()?;
    // The smallest store, so this also guards the size floor: a one-server
    // store must still keep the first ticket.
    let store = Arc::new(tls::NonBlockingSessionStore::new(1));
    let mut r = rig_with(&lab, config_with_store(&lab, store.clone())?)?;
    let msg = hs_msg(HS_NEW_SESSION_TICKET, &nst_body());
    // Split in the body, past the four-byte handshake header.
    let at = 7;
    let mut first = r.forge(CT_HANDSHAKE, &msg[..at])?;
    let mut second = r.forge(CT_HANDSHAKE, &msg[at..])?;
    assert!(
        matches!(open(&mut r, &mut first)?, Opened::Control),
        "the first half must be held"
    );
    assert!(
        matches!(open(&mut r, &mut second)?, Opened::Control),
        "the second half must complete the ticket"
    );
    assert_eq!(r.layer.key_updates(), 0, "a ticket is not a key update");
    let name = ServerName::try_from("localhost".to_owned()).map_err(|e| e.to_string())?;
    assert!(
        store.take_tls13_ticket(&name).is_some(),
        "the reassembled ticket must reach the session store"
    );
    // The layer is still healthy: an ordinary application-data record opens.
    let mut app = r.forge(CT_APPDATA, b"still here")?;
    let got = open(&mut r, &mut app)?;
    assert!(
        matches!(got, Opened::AppData(10)),
        "expected application data, got {got:?}"
    );
    Ok(())
}

/// Whole messages and a fragment in one record must frame correctly: the two
/// tickets are dispatched, the partial `KeyUpdate` is held, and it completes in
/// the next record.
#[test]
fn whole_messages_and_a_fragment_in_one_record_are_framed_correctly() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab)?;
    let a = hs_msg(HS_NEW_SESSION_TICKET, &nst_body());
    let b = hs_msg(HS_NEW_SESSION_TICKET, &nst_body());
    let c = hs_msg(HS_KEY_UPDATE, &[0]);
    let mut stream = a.clone();
    stream.extend_from_slice(&b);
    stream.extend_from_slice(&c);
    let at = a.len() + b.len() + 2;
    let mut first = r.forge(CT_HANDSHAKE, &stream[..at])?;
    assert!(matches!(open(&mut r, &mut first)?, Opened::Control));
    assert_eq!(
        r.layer.key_updates(),
        0,
        "the KeyUpdate is still a fragment"
    );
    let mut second = r.forge(CT_HANDSHAKE, &stream[at..])?;
    assert!(matches!(open(&mut r, &mut second)?, Opened::Control));
    assert_eq!(r.layer.key_updates(), 1);
    Ok(())
}

/// The cipher suite's confidentiality limit is enforced: a proactive `KeyUpdate`
/// goes out before it, and the layer refuses rather than use a key past it.
#[test]
fn the_confidentiality_limit_is_enforced_with_a_proactive_key_update() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab)?;
    // The suite's limit, as rustls sets it: 2^24 for AES-GCM, none for
    // ChaCha20-Poly1305.
    let limit = r.layer.record_limit();
    assert!(
        limit == 1 << 24 || limit == u64::MAX,
        "unexpected confidentiality limit {limit}"
    );
    assert!(r.layer.key_update_at() < limit);

    r.layer.set_key_update_at(3).set_record_limit(8);
    let mut out = Vec::new();
    for i in 0..5u8 {
        r.layer.seal_app(&[i], &mut out)?;
    }
    assert!(
        r.layer.key_updates() >= 1,
        "a KeyUpdate must go out before three records are sealed"
    );
    assert_eq!(
        count_records(&out)?,
        5 + r.layer.key_updates() as usize,
        "every sealed record is on the wire once"
    );

    // With the proactive update off, the hard limit is what stops the key.
    r.layer.set_key_update_at(u64::MAX);
    let updates = r.layer.key_updates();
    let mut refused = false;
    for _ in 0..20u32 {
        if r.layer.seal_app(b"x", &mut out).is_err() {
            refused = true;
            break;
        }
    }
    assert!(
        refused,
        "sealing past the confidentiality limit must be refused"
    );
    assert_eq!(
        r.layer.key_updates(),
        updates,
        "no KeyUpdate goes out once the proactive threshold is off"
    );
    Ok(())
}

/// Filling the receive side past the limit is refused too.
#[test]
fn opening_past_the_confidentiality_limit_is_refused() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab)?;
    r.layer.set_key_update_at(u64::MAX).set_record_limit(3);
    for i in 0..3u8 {
        let mut rec = r.forge(CT_APPDATA, &[i])?;
        assert!(
            matches!(open(&mut r, &mut rec)?, Opened::AppData(1)),
            "record {i} is inside the limit"
        );
    }
    let mut rec = r.forge(CT_APPDATA, b"one too many")?;
    let got = open(&mut r, &mut rec);
    assert!(
        got.is_err(),
        "the record past the limit must be refused: {got:?}"
    );
    Ok(())
}
