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
    fn on_challenge(&self, _encoded_challenge: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

/// Credentials supplier that performs no authentication.
#[derive(Clone, Debug)]
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

    struct SimpleCredentialsSupplier {
        creds: Vec<u8>,
        response: Vec<u8>,
    }

    impl CredentialsSupplier for SimpleCredentialsSupplier {
        fn encoded_credentials(&self) -> Option<Vec<u8>> {
            Some(self.creds.clone())
        }

        fn on_challenge(&self, _challenge: &[u8]) -> Option<Vec<u8>> {
            Some(self.response.clone())
        }
    }

    #[test]
    fn test_simple_credentials_supplier() -> Result<(), Box<dyn std::error::Error>> {
        let supplier = SimpleCredentialsSupplier {
            creds: b"user:pass".to_vec(),
            response: b"response".to_vec(),
        };
        assert_eq!(supplier.encoded_credentials().unwrap(), b"user:pass".to_vec());
        assert_eq!(supplier.on_challenge(b"challenge").unwrap(), b"response".to_vec());
    
        Ok(())
    }
}
