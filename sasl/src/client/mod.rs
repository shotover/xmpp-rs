use crate::common::Credentials;

/// A trait which defines SASL mechanisms.
pub trait Mechanism {
    /// The name of the mechanism.
    fn name(&self) -> &str;

    /// Creates this mechanism from `Credentials`.
    fn from_credentials(credentials: Credentials) -> Result<Self, String>
    where
        Self: Sized;

    /// Provides initial payload of the SASL mechanism.
    fn initial(&mut self) -> Vec<u8> {
        Vec::new()
    }

    /// Creates a response to the SASL challenge.
    fn response(&mut self, _challenge: &[u8]) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }

    /// Verifies the server success response, if there is one.
    fn success(&mut self, _data: &[u8]) -> Result<(), String> {
        Ok(())
    }
}

pub mod mechanisms;
