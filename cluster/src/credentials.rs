//! Credential suppliers for cluster authentication.

use std::sync::Arc;

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

/// Fixed connect credentials and optional challenge response.
///
/// Suitable for simple `user:pass` style authenticators used in samples.
#[derive(Clone, Debug)]
pub struct StaticCredentials {
    connect: Vec<u8>,
    challenge_response: Option<Vec<u8>>,
}

impl StaticCredentials {
    /// Connect credentials only (challenge returns the same bytes if set via
    /// [`Self::with_challenge_response`], else `None`).
    pub fn new(connect: impl Into<Vec<u8>>) -> Self {
        Self {
            connect: connect.into(),
            challenge_response: None,
        }
    }

    /// Challenge response payload (cloned on each challenge).
    pub fn with_challenge_response(mut self, response: impl Into<Vec<u8>>) -> Self {
        self.challenge_response = Some(response.into());
        self
    }

    /// Wrap as `Arc<dyn CredentialsSupplier>` for [`crate::SessionBuilder::credentials`].
    pub fn arc(self) -> Arc<dyn CredentialsSupplier> {
        Arc::new(self)
    }
}

impl CredentialsSupplier for StaticCredentials {
    fn encoded_credentials(&self) -> Option<Vec<u8>> {
        Some(self.connect.clone())
    }

    fn on_challenge(&self, _encoded_challenge: &[u8]) -> Option<Vec<u8>> {
        self.challenge_response.clone()
    }
}

/// Echoes the challenge payload back as the response (test/demo authenticators).
#[derive(Clone, Debug, Default)]
pub struct EchoChallengeCredentials {
    connect: Vec<u8>,
}

impl EchoChallengeCredentials {
    pub fn new(connect: impl Into<Vec<u8>>) -> Self {
        Self {
            connect: connect.into(),
        }
    }

    pub fn arc(self) -> Arc<dyn CredentialsSupplier> {
        Arc::new(self)
    }
}

impl CredentialsSupplier for EchoChallengeCredentials {
    fn encoded_credentials(&self) -> Option<Vec<u8>> {
        Some(self.connect.clone())
    }

    fn on_challenge(&self, encoded_challenge: &[u8]) -> Option<Vec<u8>> {
        Some(encoded_challenge.to_vec())
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
    fn test_static_credentials() -> Result<(), Box<dyn std::error::Error>> {
        let supplier = StaticCredentials::new(b"user:pass".as_slice()).with_challenge_response(b"response".as_slice());
        assert_eq!(supplier.encoded_credentials().unwrap(), b"user:pass");
        assert_eq!(supplier.on_challenge(b"challenge").unwrap(), b"response");
        Ok(())
    }

    #[test]
    fn test_echo_challenge() -> Result<(), Box<dyn std::error::Error>> {
        let supplier = EchoChallengeCredentials::new(b"u:p".as_slice());
        assert_eq!(supplier.on_challenge(b"xyz").unwrap(), b"xyz");
        Ok(())
    }
}
