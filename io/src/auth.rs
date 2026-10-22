//! Query-parameter request signing: the parameters, a
//! millisecond `timestamp`, and a `signature` over exactly those bytes,
//! appended as `&signature=…`.
//!
//! | [`Signer`] | Signature | Encoding | Use it for |
//! |---|---|---|---|
//! | [`Signer::hmac_sha256`] | HMAC-SHA256 with the API secret | 64 hex digits | signed REST requests |
//! | [`Signer::ed25519_pkcs8`] | Ed25519 with a key whose public half is registered with the server | base64, percent-encoded | REST or WebSocket API log-on; the secret never leaves this machine |
//!
//! - Chosen at run time: one branch per request.
//! - No allocation: the signature is written into the request buffer.
//! - The API key is an application-defined header, set once with
//!   `Target::with_header`. Bearer or Basic authorisation is a header too.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use ring::signature::Ed25519KeyPair;

use crate::error::{Error, Result};

/// Which signature a [`Signer`] produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureKind {
    /// HMAC-SHA256, hex encoded.
    HmacSha256,
    /// Ed25519, base64 encoded and percent-encoded.
    Ed25519,
}

enum Imp {
    Hmac(ring::hmac::Key),
    Ed25519(Box<Ed25519KeyPair>),
}

/// Signs request parameters. Build it once at start-up: the HMAC key schedule
/// and the Ed25519 key expansion happen here, not per request.
pub struct Signer {
    imp: Imp,
}

impl std::fmt::Debug for Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material.
        f.debug_struct("Signer")
            .field("kind", &self.kind())
            .finish()
    }
}

/// Longest encoded signature: Ed25519 is 88 base64 characters, at most 3 bytes
/// each once percent-encoded.
const MAX_ENCODED: usize = 3 * 88;

impl Signer {
    /// HMAC-SHA256 with the API secret.
    #[must_use]
    pub fn hmac_sha256(secret: &[u8]) -> Self {
        Self {
            imp: Imp::Hmac(ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret)),
        }
    }

    /// Ed25519 from a PKCS#8 private key in DER (v1, as `openssl genpkey`
    /// writes it, or v2).
    ///
    /// # Errors
    /// The key is not a valid Ed25519 PKCS#8 document.
    pub fn ed25519_pkcs8(der: &[u8]) -> Result<Self> {
        let kp = Ed25519KeyPair::from_pkcs8_maybe_unchecked(der)
            .map_err(|e| Error::Config(format!("Ed25519 key: {e}")))?;
        Ok(Self {
            imp: Imp::Ed25519(Box::new(kp)),
        })
    }

    /// Ed25519 from a PEM `PRIVATE KEY` block.
    ///
    /// # Errors
    /// No PEM private key, or not a valid Ed25519 key.
    pub fn ed25519_pem(pem: &str) -> Result<Self> {
        use rustls::pki_types::PrivatePkcs8KeyDer;
        use rustls::pki_types::pem::PemObject;
        let der = PrivatePkcs8KeyDer::from_pem_slice(pem.as_bytes())
            .map_err(|e| Error::Config(format!("Ed25519 PEM: {e}")))?;
        Self::ed25519_pkcs8(der.secret_pkcs8_der())
    }

    /// The signature this signer produces.
    #[must_use]
    pub const fn kind(&self) -> SignatureKind {
        match self.imp {
            Imp::Hmac(_) => SignatureKind::HmacSha256,
            Imp::Ed25519(_) => SignatureKind::Ed25519,
        }
    }

    /// The Ed25519 public key (to register with the server), if any.
    #[must_use]
    pub fn ed25519_public_key(&self) -> Option<&[u8]> {
        match &self.imp {
            Imp::Ed25519(kp) => Some(ring::signature::KeyPair::public_key(kp.as_ref()).as_ref()),
            Imp::Hmac(_) => None,
        }
    }

    /// Appends the encoded signature of `payload` to `out` (hex for HMAC,
    /// percent-encoded base64 for Ed25519). Allocates nothing.
    pub fn append_signature(&self, payload: &[u8], out: &mut Vec<u8>) {
        let mut buf = [0u8; MAX_ENCODED];
        let n = self.encode_signature(payload, &mut buf);
        out.extend_from_slice(&buf[..n]);
    }

    fn encode_signature(&self, payload: &[u8], buf: &mut [u8; MAX_ENCODED]) -> usize {
        match &self.imp {
            Imp::Hmac(key) => {
                let tag = ring::hmac::sign(key, payload);
                hex_into(tag.as_ref(), buf)
            }
            Imp::Ed25519(kp) => {
                let sig = kp.sign(payload);
                let mut b64 = [0u8; 88];
                let n = B64.encode_slice(sig.as_ref(), &mut b64).unwrap_or_default();
                percent_encode_into(&b64[..n], buf)
            }
        }
    }
}

/// Appends `params`, `&timestamp=<ms>` and `&signature=<sig>` to `out`, where
/// the signature covers exactly the bytes written before `&signature`.
pub fn append_signed_params(out: &mut Vec<u8>, params: &str, timestamp_ms: u64, signer: &Signer) {
    let start = out.len();
    out.extend_from_slice(params.as_bytes());
    if !params.is_empty() {
        out.push(b'&');
    }
    out.extend_from_slice(b"timestamp=");
    push_u64(out, timestamp_ms);
    let mut buf = [0u8; MAX_ENCODED];
    let n = signer.encode_signature(&out[start..], &mut buf);
    out.extend_from_slice(b"&signature=");
    out.extend_from_slice(&buf[..n]);
}

/// Checks that `params` can go into a query string as-is.
///
/// # Errors
/// A byte that would need percent-encoding or could break the request line.
pub fn validate_params(params: &str) -> Result<()> {
    let ok = |b: u8| b.is_ascii_alphanumeric() || b"-._~!$'()*+,;=:@/?&%".contains(&b);
    if params.bytes().all(ok) {
        Ok(())
    } else {
        Err(Error::Http("parameters must be percent-encoded".into()))
    }
}

fn hex_into(bytes: &[u8], buf: &mut [u8]) -> usize {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, b) in bytes.iter().enumerate() {
        buf[2 * i] = HEX[usize::from(b >> 4)];
        buf[2 * i + 1] = HEX[usize::from(b & 0x0F)];
    }
    2 * bytes.len()
}

/// Percent-encodes the base64 characters that are not query-safe.
fn percent_encode_into(src: &[u8], buf: &mut [u8]) -> usize {
    let mut n = 0;
    for &b in src {
        let esc: &[u8] = match b {
            b'+' => b"%2B",
            b'/' => b"%2F",
            b'=' => b"%3D",
            _ => {
                buf[n] = b;
                n += 1;
                continue;
            }
        };
        buf[n..n + 3].copy_from_slice(esc);
        n += 3;
    }
    n
}

fn push_u64(out: &mut Vec<u8>, mut v: u64) {
    let mut digits = [0u8; 20];
    let mut i = digits.len();
    loop {
        i -= 1;
        #[allow(clippy::cast_possible_truncation)]
        {
            digits[i] = b'0' + (v % 10) as u8;
        }
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[i..]);
}
