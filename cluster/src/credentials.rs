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
    /// Build from opaque wire bytes.
    ///
    /// Accepts any `Into<Vec<u8>>` so a moved `Vec<u8>` is kept without a copy,
    /// while slices and arrays allocate once. The same bytes answer both the
    /// connect request and any challenge.
    ///
    /// ```
    /// use ergo_aeron_cluster::StaticCredentials;
    ///
    /// // Non-UTF-8 and embedded NUL are valid credential octets.
    /// let raw = [0x00, 0xff, 0x00, b'x'];
    /// let from_vec = StaticCredentials::new(raw.to_vec());
    /// let from_slice = StaticCredentials::new(&raw[..]);
    /// let from_array = StaticCredentials::new(raw);
    /// assert_eq!(format!("{from_vec:?}"), format!("{from_slice:?}"));
    /// assert_eq!(format!("{from_vec:?}"), format!("{from_array:?}"));
    /// ```
    #[must_use = "the credentials supplier is unused; ignoring it skips authentication"]
    pub fn new(credentials: impl Into<Vec<u8>>) -> Self {
        Self {
            credentials: credentials.into(),
        }
    }

    /// Build from a UTF-8 string (encoded as its bytes).
    ///
    /// Prefer [`Self::new`] when the secret is already opaque octets.
    #[must_use = "the credentials supplier is unused; ignoring it skips authentication"]
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

    fn assert_same_connect_and_challenge(
        supplier: &StaticCredentials,
        expected: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let connect = supplier
            .encoded_credentials()
            .ok_or("static creds missing on connect")?;
        let challenge = supplier
            .on_challenge(b"server-challenge")
            .ok_or("static creds missing on challenge")?;
        assert_eq!(&*connect, expected);
        assert_eq!(&*challenge, expected, "challenge must reuse the same bytes");
        Ok(())
    }

    #[test]
    fn test_static_credentials_answer_connect_and_challenge() -> Result<(), Box<dyn std::error::Error>> {
        assert_same_connect_and_challenge(&StaticCredentials::from_utf8("user:pass"), b"user:pass")?;
        let moved = vec![1u8, 2, 3];
        assert_same_connect_and_challenge(&StaticCredentials::new(moved), &[1, 2, 3])?;
        let slice: &[u8] = &[1, 2, 3];
        assert_same_connect_and_challenge(&StaticCredentials::new(slice), &[1, 2, 3])?;
        assert_same_connect_and_challenge(&StaticCredentials::new([1u8, 2, 3]), &[1, 2, 3])?;
        let with_nul = b"user\0pass";
        assert_same_connect_and_challenge(&StaticCredentials::new(with_nul.as_slice()), with_nul)?;
        let invalid_utf8 = [0xffu8, 0x00, 0xfe];
        assert_same_connect_and_challenge(&StaticCredentials::new(invalid_utf8), &invalid_utf8)?;
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
