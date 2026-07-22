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
}
