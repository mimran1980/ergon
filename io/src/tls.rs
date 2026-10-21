//! TLS 1.3: rustls does the handshake; this module then owns the record
//! layer and decrypts each record **in place** in the receive buffer.
//!
//! - Why: rustls 0.23's read paths copy every record into an internal buffer
//!   (one allocation and one copy per record). In place keeps one copy per
//!   byte: kernel to our buffer.
//! - How: drive `UnbufferedClientConnection` to `WriteTraffic`, call
//!   `dangerous_into_kernel_connection` (the kTLS-style API), then run
//!   AES-GCM or ChaCha20-Poly1305 with ring.
//! - What the kernel API does *not* do: [`KernelConnection`] stores a session
//!   ticket body it is handed and computes new traffic secrets, and stops
//!   there. Parsing post-handshake messages, reassembling one that a peer
//!   split across records, dispatching it, and honouring the cipher suite's
//!   confidentiality limit are this module's, because rustls leaves those to
//!   the caller.

use std::io::{Read, Write};
use std::sync::Arc;

use ring::aead::{
    AES_128_GCM, AES_256_GCM, Aad, CHACHA20_POLY1305, LessSafeKey, Nonce, UnboundKey,
};
use rustls::client::{ClientConnectionData, UnbufferedClientConnection};
use rustls::kernel::KernelConnection;
use rustls::pki_types::ServerName;
use rustls::unbuffered::{ConnectionState, UnbufferedStatus};
use rustls::{ClientConfig, ConnectionTrafficSecrets, RootCertStore};

use crate::error::{Error, Result};

/// TLS record header length.
///
/// Hidden test lever: `io/benches` counts record headers with it. The record
/// layer itself is not part of the API.
#[doc(hidden)]
pub const HEADER_LEN: usize = 5;
/// AEAD tag length for every TLS 1.3 suite.
pub(crate) const TAG_LEN: usize = 16;
/// Largest TLS 1.3 record on the wire: 2^14 plaintext + content type + padding + tag.
pub(crate) const MAX_RECORD: usize = HEADER_LEN + (1 << 14) + 256;

/// Largest post-handshake handshake message this layer will reassemble. A
/// `NewSessionTicket` is far smaller; the bound stops a peer from making us
/// buffer without limit.
pub(crate) const MAX_HS_MSG: usize = 1 << 18;

const CT_ALERT: u8 = 21;
const CT_HANDSHAKE: u8 = 22;
/// Application-data content type.
pub(crate) const CT_APPDATA: u8 = 23;
const HS_NEW_SESSION_TICKET: u8 = 4;
const HS_KEY_UPDATE: u8 = 24;

/// A rustls client config restricted to TLS 1.3, with secret extraction enabled
/// and a session store that never blocks ([`NonBlockingSessionStore`]).
///
/// # Errors
/// The ring provider rejects the protocol selection (it does not).
pub fn client_config(roots: RootCertStore) -> Result<Arc<ClientConfig>> {
    client_config_with(
        roots,
        Arc::new(rustls::crypto::ring::default_provider()),
        true,
    )
}

/// A TLS 1.3 client config with an explicit crypto provider.
///
/// # Errors
/// The provider rejects TLS 1.3.
pub fn client_config_with(
    roots: RootCertStore,
    provider: Arc<rustls::crypto::CryptoProvider>,
    extract_secrets: bool,
) -> Result<Arc<ClientConfig>> {
    client_config_with_store(
        roots,
        provider,
        extract_secrets,
        Arc::new(NonBlockingSessionStore::new(256)),
    )
}

/// A TLS 1.3 client config whose session tickets land in `store`.
///
/// # Errors
/// The provider rejects TLS 1.3.
pub fn client_config_with_store(
    roots: RootCertStore,
    provider: Arc<rustls::crypto::CryptoProvider>,
    extract_secrets: bool,
    store: Arc<dyn rustls::client::ClientSessionStore>,
) -> Result<Arc<ClientConfig>> {
    let mut cfg = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
    cfg.enable_secret_extraction = extract_secrets;
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    cfg.resumption = rustls::client::Resumption::store(store);
    Ok(Arc::new(cfg))
}

/// Checks that `cfg` can feed this crate's record layer (secret extraction on).
/// The protocol version is checked after the handshake: rustls does not expose
/// a config's versions.
pub(crate) fn check_client_config(cfg: &ClientConfig) -> Result<()> {
    if cfg.enable_secret_extraction {
        Ok(())
    } else {
        Err(Error::Config(
            "the rustls ClientConfig needs enable_secret_extraction = true and TLS 1.3 only: \
             build it with ergon_io::tls::client_config"
                .into(),
        ))
    }
}

/// A TLS session store that never blocks the thread using it.
///
/// Session tickets arrive after the handshake and are stored by the polling
/// thread, while a background connect may be reading the same store. rustls's
/// own cache takes a mutex, so the polling thread could wait for (or sleep
/// behind) the helper. Here every access only tries the lock: if the other
/// thread holds it, the ticket is not stored, or the lookup finds nothing and
/// that connect does a full handshake instead of resuming.
#[derive(Debug)]
pub struct NonBlockingSessionStore(std::sync::Mutex<rustls::client::ClientSessionMemoryCache>);

impl NonBlockingSessionStore {
    /// A store holding tickets for up to `size` servers.
    ///
    /// Two is the floor, even when one server is asked for. rustls's cache
    /// keeps at most eight tickets per server and, at one server slot, evicts
    /// the oldest key whenever a new key fills the ring — which drops the
    /// ticket it has just stored. Two slots keep the first ticket and make
    /// `size` mean what it says.
    #[must_use]
    pub fn new(size: usize) -> Self {
        const SESSIONS_PER_SERVER: usize = 8;
        let servers = size.max(2);
        Self(std::sync::Mutex::new(
            rustls::client::ClientSessionMemoryCache::new(servers * SESSIONS_PER_SERVER),
        ))
    }

    fn with<T>(&self, f: impl FnOnce(&rustls::client::ClientSessionMemoryCache) -> T) -> Option<T> {
        self.0.try_lock().ok().map(|c| f(&c))
    }
}

impl rustls::client::ClientSessionStore for NonBlockingSessionStore {
    fn set_kx_hint(&self, server_name: ServerName<'static>, group: rustls::NamedGroup) {
        self.with(|c| c.set_kx_hint(server_name, group));
    }

    fn kx_hint(&self, server_name: &ServerName<'_>) -> Option<rustls::NamedGroup> {
        self.with(|c| c.kx_hint(server_name)).flatten()
    }

    fn set_tls12_session(
        &self,
        server_name: ServerName<'static>,
        value: rustls::client::Tls12ClientSessionValue,
    ) {
        self.with(|c| c.set_tls12_session(server_name, value));
    }

    fn tls12_session(
        &self,
        server_name: &ServerName<'_>,
    ) -> Option<rustls::client::Tls12ClientSessionValue> {
        self.with(|c| c.tls12_session(server_name)).flatten()
    }

    fn remove_tls12_session(&self, server_name: &ServerName<'static>) {
        self.with(|c| c.remove_tls12_session(server_name));
    }

    fn insert_tls13_ticket(
        &self,
        server_name: ServerName<'static>,
        value: rustls::client::Tls13ClientSessionValue,
    ) {
        self.with(|c| c.insert_tls13_ticket(server_name, value));
    }

    fn take_tls13_ticket(
        &self,
        server_name: &ServerName<'static>,
    ) -> Option<rustls::client::Tls13ClientSessionValue> {
        self.with(|c| c.take_tls13_ticket(server_name)).flatten()
    }
}

/// Mozilla's root store (feature `webpki-roots`).
#[cfg(feature = "webpki-roots")]
#[must_use]
pub fn webpki_roots() -> RootCertStore {
    RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    }
}

struct Direction {
    key: LessSafeKey,
    iv: [u8; 12],
    seq: u64,
}

/// Records under one key before a proactive `KeyUpdate`: half the cipher's
/// confidentiality limit, so a peer that is slow to answer still has a wide
/// margin. ChaCha20-Poly1305 has no practical limit; it gets a long period so
/// the record counter never approaches the nonce.
fn default_key_update_at(limit: u64) -> u64 {
    if limit == u64::MAX {
        1 << 62
    } else {
        (limit / 2).max(1)
    }
}

/// One handshake message split off the front of a buffer: its type, its body
/// and whatever follows.
type HsMessage<'a> = Option<(u8, &'a [u8], &'a [u8])>;

/// Splits one handshake message off the front of `buf`.
///
/// `Ok(None)` means the message runs past the end of `buf`: a fragment of a
/// message that continues in a later record (RFC 8446 §5.1).
fn split_hs_message(buf: &[u8]) -> Result<HsMessage<'_>> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let len = usize::from(buf[1]) << 16 | usize::from(buf[2]) << 8 | usize::from(buf[3]);
    if len > MAX_HS_MSG - 4 {
        return Err(Error::Record("post-handshake message too large"));
    }
    let total = 4 + len;
    if buf.len() < total {
        return Ok(None);
    }
    Ok(Some((buf[0], &buf[4..total], &buf[total..])))
}

impl Direction {
    fn new(seq: u64, secrets: &ConnectionTrafficSecrets) -> Result<Self> {
        let (alg, key, iv) = match secrets {
            ConnectionTrafficSecrets::Aes128Gcm { key, iv } => {
                (&AES_128_GCM, key.as_ref(), iv.as_ref())
            }
            ConnectionTrafficSecrets::Aes256Gcm { key, iv } => {
                (&AES_256_GCM, key.as_ref(), iv.as_ref())
            }
            ConnectionTrafficSecrets::Chacha20Poly1305 { key, iv } => {
                (&CHACHA20_POLY1305, key.as_ref(), iv.as_ref())
            }
            _ => return Err(Error::Record("unsupported cipher suite")),
        };
        let key =
            LessSafeKey::new(UnboundKey::new(alg, key).map_err(|_| Error::Record("bad key"))?);
        let mut ivb = [0u8; 12];
        if iv.len() != 12 {
            return Err(Error::Record("unexpected IV length"));
        }
        ivb.copy_from_slice(iv);
        Ok(Self { key, iv: ivb, seq })
    }

    fn next_nonce(&mut self) -> Nonce {
        let mut n = self.iv;
        for (b, s) in n[4..].iter_mut().zip(self.seq.to_be_bytes()) {
            *b ^= s;
        }
        self.seq += 1;
        Nonce::assume_unique_for_key(n)
    }
}

/// Result of opening one record in place.
///
/// Hidden test lever: the post-handshake record layer is not part of the API;
/// `io/tests/tls_records.rs` and `io/benches` drive it directly.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// Application data at `record[HEADER_LEN..HEADER_LEN + len]`.
    AppData(usize),
    /// A post-handshake message was consumed; `reply` bytes (if any) were queued.
    Control,
}

/// The TLS 1.3 record layer after the handshake.
///
/// rustls's [`KernelConnection`] stores session tickets and computes new
/// traffic secrets. Everything else about post-handshake messages — parsing,
/// reassembly across records, dispatch — and the cipher suite's
/// confidentiality limit is this type's, because the kernel API leaves those
/// to the caller.
///
/// Hidden test lever: a post-handshake record layer is an implementation
/// detail of [`crate::WsConn`] and [`crate::HttpConn`]. `io/tests/tls_records.rs`
/// and `io/benches` exercise it directly.
#[doc(hidden)]
pub struct RecordLayer {
    rx: Direction,
    tx: Direction,
    kconn: KernelConnection<ClientConnectionData>,
    /// Number of records opened.
    pub records_in: u64,
    /// Bytes of a post-handshake handshake message that arrived split across
    /// records. Empty on the common path, where a message sits whole inside
    /// one record and never touches this buffer.
    hs: Vec<u8>,
    /// Records opened under one key before the layer refuses: the negotiated
    /// suite's confidentiality limit, or an operator's tighter policy.
    limit: u64,
    /// Records under one key before a proactive `KeyUpdate` goes out.
    key_update_at: u64,
    /// Key updates applied, in either direction.
    key_updates: u64,
    /// A `KeyUpdate(update_requested)` is in flight. Do not send another
    /// until the peer's reply has rotated the receive key.
    rx_rekey_requested: bool,
}

impl RecordLayer {
    /// Builds the record layer from the secrets of a completed handshake.
    /// This is the kTLS-style entry point: drive the handshake yourself and
    /// hand over what rustls extracted.
    ///
    /// # Errors
    /// The secrets name a cipher suite this layer cannot run.
    pub fn from_parts(
        secrets: &rustls::ExtractedSecrets,
        kconn: KernelConnection<ClientConnectionData>,
    ) -> Result<Self> {
        let limit = kconn
            .negotiated_cipher_suite()
            .tls13()
            .map_or(u64::MAX, |s| s.common.confidentiality_limit);
        let key_update_at = default_key_update_at(limit);
        Ok(Self {
            rx: Direction::new(secrets.rx.0, &secrets.rx.1)?,
            tx: Direction::new(secrets.tx.0, &secrets.tx.1)?,
            kconn,
            records_in: 0,
            hs: Vec::with_capacity(MAX_HS_MSG),
            limit,
            key_update_at,
            key_updates: 0,
            rx_rekey_requested: false,
        })
    }

    /// Records that may be sealed or opened under one key before the layer
    /// refuses: the negotiated suite's confidentiality limit.
    #[must_use]
    pub const fn record_limit(&self) -> u64 {
        self.limit
    }

    /// Records under one key before a proactive `KeyUpdate` goes out. Always
    /// below [`RecordLayer::record_limit`].
    #[must_use]
    pub const fn key_update_at(&self) -> u64 {
        self.key_update_at
    }

    /// Sets the proactive `KeyUpdate` threshold. Clamped below
    /// [`RecordLayer::record_limit`]; `0` means "rotate before the first
    /// record". `u64::MAX` turns the proactive update off, so the hard limit
    /// is what stops the key.
    pub fn set_key_update_at(&mut self, n: u64) -> &mut Self {
        self.key_update_at = if n == u64::MAX {
            u64::MAX
        } else {
            n.min(self.limit.saturating_sub(1))
        };
        self
    }

    /// Sets the hard record limit for one key, replacing the suite's
    /// confidentiality limit. Clamped to at least 1. Use it to rekey earlier
    /// than the cipher requires, never later.
    pub fn set_record_limit(&mut self, n: u64) -> &mut Self {
        self.limit = n.max(1);
        self.set_key_update_at(self.key_update_at);
        self
    }

    /// Key updates applied, in either direction.
    #[must_use]
    pub const fn key_updates(&self) -> u64 {
        self.key_updates
    }

    /// Length of the complete record at the front of `buf`, if it has fully arrived.
    ///
    /// # Errors
    /// The header is not a TLS 1.3 application-data record header.
    pub fn complete_record_len(buf: &[u8]) -> Result<Option<usize>> {
        if buf.len() < HEADER_LEN {
            return Ok(None);
        }
        if buf[0] != CT_APPDATA && buf[0] != CT_ALERT {
            return Err(Error::Record("unexpected outer content type"));
        }
        let len = usize::from(u16::from_be_bytes([buf[3], buf[4]]));
        if len > MAX_RECORD - HEADER_LEN {
            return Err(Error::Record("record too long"));
        }
        let total = HEADER_LEN + len;
        Ok((buf.len() >= total).then_some(total))
    }

    /// Decrypts one complete record in place. On `AppData(n)` the plaintext is at
    /// `record[HEADER_LEN..HEADER_LEN + n]`. Post-handshake handshake messages are
    /// handled here; any reply (a key update) is appended to `tx_out` already
    /// encrypted.
    ///
    /// # Errors
    /// AEAD failure, an alert, or a malformed post-handshake message.
    pub fn open_in_place(&mut self, record: &mut [u8], tx_out: &mut Vec<u8>) -> Result<Opened> {
        if self.rx.seq >= self.limit {
            return Err(Error::Record("confidentiality limit reached"));
        }
        let mut aad = [0u8; HEADER_LEN];
        aad.copy_from_slice(&record[..HEADER_LEN]);
        let nonce = self.rx.next_nonce();
        let plain = self
            .rx
            .key
            .open_in_place(nonce, Aad::from(aad), &mut record[HEADER_LEN..])
            .map_err(|_| Error::Record("AEAD open failed"))?;
        self.records_in += 1;
        // Strip zero padding; the last non-zero byte is the real content type.
        let mut end = plain.len();
        while end > 0 && plain[end - 1] == 0 {
            end -= 1;
        }
        if end == 0 {
            return Err(Error::Record("record has no content type"));
        }
        let inner = plain[end - 1];
        let n = end - 1;
        match inner {
            CT_APPDATA => Ok(Opened::AppData(n)),
            CT_HANDSHAKE => {
                let body = &plain[..n];
                self.handle_handshake(body, tx_out)?;
                Ok(Opened::Control)
            }
            CT_ALERT => Err(Error::Closed),
            _ => Err(Error::Record("unexpected inner content type")),
        }
    }

    /// Dispatches the handshake bytes of one record. TLS 1.3 allows a
    /// handshake message to span records (RFC 8446 §5.1) and a server does
    /// exactly that with a session ticket, so a runt is a fragment to hold,
    /// not an error.
    fn handle_handshake(&mut self, msgs: &[u8], tx_out: &mut Vec<u8>) -> Result<()> {
        if self.hs.is_empty() {
            let mut rest = msgs;
            while !rest.is_empty() {
                if let Some((typ, body, next)) = split_hs_message(rest)? {
                    self.dispatch_hs(typ, body, tx_out)?;
                    rest = next;
                } else {
                    self.append_hs(rest)?;
                    return Ok(());
                }
            }
            return Ok(());
        }
        self.append_hs(msgs)?;
        self.drain_hs(tx_out)
    }

    /// Copies a fragment into the reassembly buffer. The buffer is reserved
    /// once at construction, so this does not allocate.
    fn append_hs(&mut self, more: &[u8]) -> Result<()> {
        if self.hs.len() + more.len() > MAX_HS_MSG {
            return Err(Error::Record("post-handshake message too large"));
        }
        self.hs.extend_from_slice(more);
        Ok(())
    }

    /// Dispatches every message now complete in the reassembly buffer.
    fn drain_hs(&mut self, tx_out: &mut Vec<u8>) -> Result<()> {
        let mut buf = std::mem::take(&mut self.hs);
        let r = self.drain_hs_from(&mut buf, tx_out);
        self.hs = buf;
        r
    }

    fn drain_hs_from(&mut self, buf: &mut Vec<u8>, tx_out: &mut Vec<u8>) -> Result<()> {
        while !buf.is_empty() {
            let Some((typ, body, next)) = split_hs_message(buf)? else {
                return Ok(());
            };
            let consumed = buf.len() - next.len();
            self.dispatch_hs(typ, body, tx_out)?;
            buf.drain(..consumed);
        }
        Ok(())
    }

    fn dispatch_hs(&mut self, typ: u8, body: &[u8], tx_out: &mut Vec<u8>) -> Result<()> {
        match typ {
            HS_NEW_SESSION_TICKET => Ok(self.kconn.handle_new_session_ticket(body)?),
            HS_KEY_UPDATE => {
                if body.len() != 1 {
                    return Err(Error::Record("malformed key update"));
                }
                let (seq, secrets) = self.kconn.update_rx_secret()?;
                self.rx = Direction::new(seq, &secrets)?;
                self.key_updates += 1;
                self.rx_rekey_requested = false;
                if body[0] == 1 {
                    // update_requested: answer with our own KeyUpdate under the
                    // old key, then switch our sending key.
                    self.seal_inner(CT_HANDSHAKE, &[HS_KEY_UPDATE, 0, 0, 1, 0], tx_out)?;
                    let (seq, secrets) = self.kconn.update_tx_secret()?;
                    self.tx = Direction::new(seq, &secrets)?;
                    self.key_updates += 1;
                }
                Ok(())
            }
            _ => Err(Error::Record("unexpected post-handshake message")),
        }
    }

    /// Rotates the sending key before it gets old. One message covers both
    /// directions: `update_requested` is set when the *receive* key is the one
    /// that needs the peer to move, and the sender of a `KeyUpdate` always
    /// switches its own sending key.
    fn maybe_rekey(&mut self, out: &mut Vec<u8>) -> Result<()> {
        let tx_due = self.tx.seq >= self.key_update_at;
        let rx_due = self.rx.seq >= self.key_update_at && !self.rx_rekey_requested;
        if !tx_due && !rx_due {
            return Ok(());
        }
        self.seal_inner(
            CT_HANDSHAKE,
            &[HS_KEY_UPDATE, 0, 0, 1, u8::from(rx_due)],
            out,
        )?;
        let (seq, secrets) = self.kconn.update_tx_secret()?;
        self.tx = Direction::new(seq, &secrets)?;
        self.key_updates += 1;
        self.rx_rekey_requested |= rx_due;
        Ok(())
    }

    /// Encrypts `payload` as one record of `inner_type` and appends it to `out`.
    ///
    /// # Errors
    /// The payload is larger than one record, or the sending key has reached
    /// its confidentiality limit.
    pub fn seal(&mut self, inner_type: u8, payload: &[u8], out: &mut Vec<u8>) -> Result<()> {
        self.maybe_rekey(out)?;
        self.seal_inner(inner_type, payload, out)
    }

    fn seal_inner(&mut self, inner_type: u8, payload: &[u8], out: &mut Vec<u8>) -> Result<()> {
        if payload.len() > 1 << 14 {
            return Err(Error::Record("payload larger than one record"));
        }
        if self.tx.seq >= self.limit {
            return Err(Error::Record("confidentiality limit reached"));
        }
        let start = out.len();
        let ct_len = payload.len() + 1 + TAG_LEN;
        let len = u16::try_from(ct_len).map_err(|_| Error::Record("record too long"))?;
        out.extend_from_slice(&[CT_APPDATA, 0x03, 0x03]);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(payload);
        out.push(inner_type);
        let mut aad = [0u8; HEADER_LEN];
        aad.copy_from_slice(&out[start..start + HEADER_LEN]);
        let nonce = self.tx.next_nonce();
        let tag = self
            .tx
            .key
            .seal_in_place_separate_tag(nonce, Aad::from(aad), &mut out[start + HEADER_LEN..])
            .map_err(|_| Error::Record("AEAD seal failed"))?;
        out.extend_from_slice(tag.as_ref());
        Ok(())
    }

    /// Builds a record in place at the end of `out`: `build` appends the plaintext
    /// directly after the header, so the payload is written exactly once.
    ///
    /// # Errors
    /// `build` fails, produces more than one record of plaintext, or the
    /// sending key has reached its confidentiality limit.
    pub fn seal_build(
        &mut self,
        out: &mut Vec<u8>,
        build: impl FnOnce(&mut Vec<u8>) -> Result<()>,
    ) -> Result<()> {
        self.maybe_rekey(out)?;
        self.seal_build_inner(out, build)
    }

    fn seal_build_inner(
        &mut self,
        out: &mut Vec<u8>,
        build: impl FnOnce(&mut Vec<u8>) -> Result<()>,
    ) -> Result<()> {
        if self.tx.seq >= self.limit {
            return Err(Error::Record("confidentiality limit reached"));
        }
        let start = out.len();
        out.extend_from_slice(&[CT_APPDATA, 0x03, 0x03, 0, 0]);
        if let Err(e) = build(out) {
            // Nothing half-built may reach the wire.
            out.truncate(start);
            return Err(e);
        }
        let plain_len = out.len() - start - HEADER_LEN;
        if plain_len > 1 << 14 {
            out.truncate(start);
            return Err(Error::Record("payload larger than one record"));
        }
        out.push(CT_APPDATA);
        let len =
            u16::try_from(plain_len + 1 + TAG_LEN).map_err(|_| Error::Record("record too long"))?;
        out[start + 3..start + 5].copy_from_slice(&len.to_be_bytes());
        let mut aad = [0u8; HEADER_LEN];
        aad.copy_from_slice(&out[start..start + HEADER_LEN]);
        let nonce = self.tx.next_nonce();
        let tag = self
            .tx
            .key
            .seal_in_place_separate_tag(nonce, Aad::from(aad), &mut out[start + HEADER_LEN..])
            .map_err(|_| Error::Record("AEAD seal failed"))?;
        out.extend_from_slice(tag.as_ref());
        Ok(())
    }

    /// Encrypts application data.
    ///
    /// # Errors
    /// See [`RecordLayer::seal`].
    pub fn seal_app(&mut self, payload: &[u8], out: &mut Vec<u8>) -> Result<()> {
        self.seal(CT_APPDATA, payload, out)
    }
}

/// Runs the TLS 1.3 handshake over a blocking stream and returns the record layer
/// plus any bytes already read that belong to records after the handshake.
///
/// Hidden test lever: see [`RecordLayer`].
///
/// # Errors
/// I/O failure, a TLS error, or a peer that closes during the handshake.
#[doc(hidden)]
pub fn handshake<S: Read + Write>(
    stream: &mut S,
    config: Arc<ClientConfig>,
    server_name: &str,
) -> Result<(RecordLayer, Vec<u8>)> {
    let mut hs = TlsHandshake::new(config, server_name)?;
    while !hs.step(stream)? {}
    hs.finish()
}

/// Like [`handshake`] but returns the raw traffic secrets, so callers (the TLS
/// micro-benchmark) can drive a different AEAD implementation.
///
/// Hidden test lever: see [`RecordLayer`].
///
/// # Errors
/// I/O failure, a TLS error, or a peer that closes during the handshake.
#[doc(hidden)]
pub fn handshake_raw<S: Read + Write>(
    stream: &mut S,
    config: Arc<ClientConfig>,
    server_name: &str,
) -> Result<(
    rustls::ExtractedSecrets,
    KernelConnection<ClientConnectionData>,
    Vec<u8>,
)> {
    let mut hs = TlsHandshake::new(config, server_name)?;
    while !hs.step(stream)? {}
    hs.finish_raw()
}

/// A client handshake advanced without blocking: [`TlsHandshake::step`] reads
/// and writes what the socket accepts and returns at the first `WouldBlock`.
pub(crate) struct TlsHandshake {
    conn: UnbufferedClientConnection,
    inc: Vec<u8>,
    inc_len: usize,
    out: Vec<u8>,
    out_len: usize,
    out_sent: usize,
    done: bool,
}

enum Next {
    Read,
    Done,
    Continue,
}

impl TlsHandshake {
    /// Starts a handshake for `server_name`.
    ///
    /// # Errors
    /// Invalid server name or configuration.
    pub fn new(config: Arc<ClientConfig>, server_name: &str) -> Result<Self> {
        check_client_config(&config)?;
        let name = ServerName::try_from(server_name.to_owned())
            .map_err(|e| Error::Config(e.to_string()))?;
        Ok(Self {
            conn: UnbufferedClientConnection::new(config, name)?,
            inc: vec![0u8; 1 << 16],
            inc_len: 0,
            out: vec![0u8; 1 << 16],
            out_len: 0,
            out_sent: 0,
            done: false,
        })
    }

    /// Advances as far as the socket allows. Returns `true` once the handshake
    /// is complete and every handshake byte has been sent.
    ///
    /// # Errors
    /// I/O failure (other than `WouldBlock`), a TLS error, or a peer close.
    pub fn step<S: Read + Write>(&mut self, stream: &mut S) -> Result<bool> {
        loop {
            if !self.flush(stream)? {
                return Ok(false);
            }
            if self.done {
                return Ok(true);
            }
            let UnbufferedStatus { discard, state } =
                self.conn.process_tls_records(&mut self.inc[..self.inc_len]);
            let next = match state? {
                ConnectionState::EncodeTlsData(mut e) => {
                    self.out_len += e
                        .encode(&mut self.out[self.out_len..])
                        .map_err(|e| Error::Http(format!("encode: {e}")))?;
                    Next::Continue
                }
                ConnectionState::TransmitTlsData(t) => {
                    // The bytes are ours now; `flush` sends them.
                    t.done();
                    Next::Continue
                }
                ConnectionState::BlockedHandshake => Next::Read,
                ConnectionState::WriteTraffic(_) => Next::Done,
                ConnectionState::PeerClosed | ConnectionState::Closed => {
                    return Err(Error::Closed);
                }
                other => {
                    return Err(Error::Http(format!("unexpected handshake state {other:?}")));
                }
            };
            if discard > 0 {
                self.inc.copy_within(discard..self.inc_len, 0);
                self.inc_len -= discard;
            }
            match next {
                Next::Continue => {}
                Next::Done => self.done = true,
                Next::Read => {
                    if self.inc_len == self.inc.len() {
                        return Err(Error::Record("handshake buffer full"));
                    }
                    match stream.read(&mut self.inc[self.inc_len..]) {
                        Ok(0) => return Err(Error::Closed),
                        Ok(n) => self.inc_len += n,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }
    }

    /// Sends pending handshake bytes; `false` if the socket would block first.
    fn flush<S: Write>(&mut self, stream: &mut S) -> Result<bool> {
        while self.out_sent < self.out_len {
            match stream.write(&self.out[self.out_sent..self.out_len]) {
                Ok(0) => return Err(Error::Closed),
                Ok(n) => self.out_sent += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
                Err(e) => return Err(e.into()),
            }
        }
        self.out_len = 0;
        self.out_sent = 0;
        Ok(true)
    }

    /// The record layer and any bytes received after the handshake, once
    /// [`TlsHandshake::step`] has returned `true`.
    ///
    /// # Errors
    /// The handshake is not complete or negotiated an unsupported suite.
    pub fn finish(self) -> Result<(RecordLayer, Vec<u8>)> {
        let (secrets, kconn, rest) = self.finish_raw()?;
        Ok((RecordLayer::from_parts(&secrets, kconn)?, rest))
    }

    /// Like [`TlsHandshake::finish`] with the raw secrets.
    ///
    /// # Errors
    /// The handshake is not complete.
    pub fn finish_raw(
        mut self,
    ) -> Result<(
        rustls::ExtractedSecrets,
        KernelConnection<ClientConnectionData>,
        Vec<u8>,
    )> {
        if !self.done {
            return Err(Error::Record("handshake not complete"));
        }
        if self.conn.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3) {
            return Err(Error::Config(
                "the server negotiated TLS 1.2; the record layer needs TLS 1.3 \
                 (restrict the ClientConfig to TLS 1.3, as ergon_io::tls::client_config does)"
                    .into(),
            ));
        }
        let (secrets, kconn) = self.conn.dangerous_into_kernel_connection()?;
        self.inc.truncate(self.inc_len);
        Ok((secrets, kconn, self.inc))
    }
}
