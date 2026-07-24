//! Credential suppliers for cluster authentication.

/// Supplies credentials for cluster authentication.
///
/// Returning `None` from `encoded_credentials()` means no authentication
/// is attempted. Returning `None` from `on_challenge()` means the
/// challenge cannot be answered and the session will be rejected.
pub trait CredentialsSupplier: Send + Sync {
    /// Credentials to include in the SessionConnectRequest.
    /// `None` = no auth (NullCredentialsSupplier behaviour).
    fn encoded_credentials(&self) -> Option<Vec<u8>>;

    /// Credentials to send in response to an auth challenge.
    /// `None` = cannot answer; session will be rejected.
    fn on_challenge(&self, encoded_challenge: &[u8]) -> Option<Vec<u8>> {
        let _ = encoded_challenge;
        None
    }
}

/// Credentials supplier that performs no authentication.
#[derive(Clone, Debug, Default)]
pub struct NullCredentialsSupplier;

impl CredentialsSupplier for NullCredentialsSupplier {
    fn encoded_credentials(&self) -> Option<Vec<u8>> {
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
#[derive(Clone, Debug)]
pub struct StaticCredentials {
    credentials: Vec<u8>,
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
    fn encoded_credentials(&self) -> Option<Vec<u8>> {
        Some(self.credentials.clone())
    }

    fn on_challenge(&self, _encoded_challenge: &[u8]) -> Option<Vec<u8>> {
        Some(self.credentials.clone())
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
        let connect = supplier.encoded_credentials().ok_or("static creds missing on connect")?;
        let challenge = supplier
            .on_challenge(b"server-challenge")
            .ok_or("static creds missing on challenge")?;
        assert_eq!(connect, b"user:pass");
        assert_eq!(challenge, b"user:pass", "challenge must reuse the same bytes");
        assert_eq!(StaticCredentials::new(vec![1, 2, 3]).encoded_credentials(), Some(vec![1, 2, 3]));
        Ok(())
    }
}
