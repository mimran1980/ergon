//! Request signing against Binance's documented example and round-trip checks.
#![allow(missing_docs)]

use base64::Engine as _;
use ergon_io::auth::{self, SignatureKind, Signer};
use ring::signature::{ED25519, Ed25519KeyPair, UnparsedPublicKey};

type R = Result<(), Box<dyn std::error::Error>>;

// ANCHOR: signing
/// Binance "SIGNED endpoint" example (HMAC-SHA256, parameters in the query).
const SECRET: &str = "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
const PARAMS: &str =
    "symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000";
const TS: u64 = 1_499_827_319_559;
const SIGNATURE: &str = "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71";

#[test]
fn hmac_matches_binance_documented_example() -> R {
    let signer = Signer::hmac_sha256(SECRET.as_bytes());
    assert_eq!(signer.kind(), SignatureKind::HmacSha256);
    let mut out = Vec::new();
    auth::append_signed_params(&mut out, PARAMS, TS, &signer);
    let expected = format!("{PARAMS}&timestamp={TS}&signature={SIGNATURE}");
    assert_eq!(String::from_utf8(out)?, expected);
    Ok(())
}
// ANCHOR_END: signing

#[test]
fn ed25519_signature_verifies() -> R {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|e| e.to_string())?;
    let signer = Signer::ed25519_pkcs8(pkcs8.as_ref())?;
    assert_eq!(signer.kind(), SignatureKind::Ed25519);
    let mut out = Vec::new();
    auth::append_signed_params(&mut out, PARAMS, TS, &signer);
    let text = String::from_utf8(out)?;
    let (payload, sig) = text.split_once("&signature=").ok_or("no signature")?;
    assert!(
        !sig.contains(['+', '/', '=']),
        "base64 must be percent-encoded: {sig}"
    );
    let sig = sig
        .replace("%2B", "+")
        .replace("%2F", "/")
        .replace("%3D", "=");
    let raw = base64::engine::general_purpose::STANDARD.decode(sig)?;
    let public = signer.ed25519_public_key().ok_or("no public key")?;
    UnparsedPublicKey::new(&ED25519, public)
        .verify(payload.as_bytes(), &raw)
        .map_err(|_| "signature does not verify")?;
    Ok(())
}

#[test]
fn ed25519_pem_and_bad_keys() -> R {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|e| e.to_string())?;
    let pem = format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(pkcs8.as_ref())
    );
    assert!(Signer::ed25519_pem(&pem).is_ok());
    assert!(Signer::ed25519_pkcs8(b"not a key").is_err());
    assert!(Signer::ed25519_pem("no pem here").is_err());
    Ok(())
}

#[test]
fn debug_never_shows_key_material() {
    let s = format!("{:?}", Signer::hmac_sha256(SECRET.as_bytes()));
    assert!(!s.contains(&SECRET[..8]), "{s}");
}

#[test]
fn params_must_be_query_safe() {
    assert!(auth::validate_params(PARAMS).is_ok());
    assert!(auth::validate_params("a=b%20c").is_ok());
    for bad in ["a=b c", "a=b\r\nX: y", "a=\"q\"", "a=b#c"] {
        assert!(auth::validate_params(bad).is_err(), "{bad:?}");
    }
}
