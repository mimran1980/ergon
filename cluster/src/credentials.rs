//! Credential suppliers for cluster authentication.

use std::borrow::Cow;

/// Supplies credentials for cluster authentication.
///
/// Methods return `Option<Cow<'_, [u8]>>` so that [`StaticCredentials`] (which
/// owns its bytes) can lend them by reference with no clone, while bespoke
/// suppliers that derive credentials on the fly can return `Cow::Owned`.
/// Returning `None` from [`Self::encoded_credentials`] means no authentication
/// is attempted; returning `None` from [`Self::on_challenge`] means the
/// challenge cannot be answered and the session will be rejected.
pub trait CredentialsSupplier: Send + Sync {
    /// Credentials to include in the SessionConnectRequest.
    /// `None` = no auth (NullCredentialsSupplier behaviour).
    fn encoded_credentials(&self) -> Option<Cow<'_, [u8]>>;

    /// Credentials to send in response to an auth challenge.
    /// `None` = cannot answer; session will be rejected.
    fn on_challenge(&self, encoded_challenge: &[u8]) -> Option<Cow<'_, [u8]>> {
        let _ = encoded_challenge;
        None
    }
}

/// Credentials supplier that performs no authentication.
#[derive(Clone, Debug, Default)]
pub struct NullCredentialsSupplier;

impl CredentialsSupplier for NullCredentialsSupplier {
    fn encoded_credentials(&self) -> Option<Cow<'_, [u8]>> {
        None
    }
}

/// Fixed credentials returned for both the connect request and any challenge.
///
/// Mirrors Java `StaticCredentialsSupplier`: the same bytes are supplied on
/// connect and in response to a challenge, so challenge-response auth is
/// answerable without a custom supplier. Prefer a bespoke
/// [`CredentialsSupplier`] implementation when the challenge must be derived
/// from the encoded challenge bytes.
#[derive(Clone)]
pub struct StaticCredentials {
    credentials: Vec<u8>,
}

impl core::fmt::Debug for StaticCredentials {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StaticCredentials")
            .field(
                "credentials",
                &format_args!("<redacted {} bytes>", self.credentials.len()),
            )
            .finish()
    }
}

impl StaticCredentials {
    /// Build from raw credential bytes.
    pub fn new(credentials: Vec<u8>) -> Self {
        Self { credentials }
    }

    /// Build from a UTF-8 string (encoded as its bytes).
    pub fn from_utf8(text: &str) -> Self {
        Self {
            credentials: text.as_bytes().to_vec(),
        }
    }
}

impl CredentialsSupplier for StaticCredentials {
    fn encoded_credentials(&self) -> Option<Cow<'_, [u8]>> {
        Some(Cow::Borrowed(&self.credentials))
    }

    fn on_challenge(&self, _encoded_challenge: &[u8]) -> Option<Cow<'_, [u8]>> {
        Some(Cow::Borrowed(&self.credentials))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_credentials_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let supplier = NullCredentialsSupplier;
        assert!(supplier.encoded_credentials().is_none());
        assert!(supplier.on_challenge(b"challenge").is_none());
        Ok(())
    }

    #[test]
    fn test_static_credentials_answer_connect_and_challenge() -> Result<(), Box<dyn std::error::Error>> {
        let supplier = StaticCredentials::from_utf8("user:pass");
        let connect = supplier
            .encoded_credentials()
            .ok_or("static creds missing on connect")?;
        let challenge = supplier
            .on_challenge(b"server-challenge")
            .ok_or("static creds missing on challenge")?;
        assert_eq!(&*connect, b"user:pass");
        assert_eq!(&*challenge, b"user:pass", "challenge must reuse the same bytes");
        assert_eq!(
            &*StaticCredentials::new(vec![1, 2, 3]).encoded_credentials().unwrap(),
            &[1u8, 2, 3][..]
        );
        Ok(())
    }

    #[test]
    fn debug_redacts_credential_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let secret = b"leaked-credential-material-abc";
        let supplier = StaticCredentials::new(secret.to_vec());
        let dbg = format!("{supplier:?}");
        assert!(
            !dbg.contains("leaked-credential"),
            "Debug must not include credential text: {dbg}"
        );
        assert!(
            !dbg.as_bytes().windows(secret.len()).any(|w| w == secret),
            "Debug must not include credential bytes"
        );
        assert!(dbg.contains("redacted"), "Debug should say redacted: {dbg}");
        assert!(
            dbg.contains(&format!("{} bytes", secret.len())),
            "Debug may show length: {dbg}"
        );
        Ok(())
    }
}
