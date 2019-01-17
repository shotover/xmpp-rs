//! Provides the SASL "ANONYMOUS" mechanism.

use crate::client::Mechanism;
use crate::common::{Credentials, Secret};

/// A struct for the SASL ANONYMOUS mechanism.
pub struct Anonymous;

impl Anonymous {
    /// Constructs a new struct for authenticating using the SASL ANONYMOUS mechanism.
    ///
    /// It is recommended that instead you use a `Credentials` struct and turn it into the
    /// requested mechanism using `from_credentials`.
    pub fn new() -> Anonymous {
        Anonymous
    }
}

impl Mechanism for Anonymous {
    fn name(&self) -> &str {
        "ANONYMOUS"
    }

    fn from_credentials(credentials: Credentials) -> Result<Anonymous, String> {
        if let Secret::None = credentials.secret {
            Ok(Anonymous)
        } else {
            Err("the anonymous sasl mechanism requires no credentials".to_owned())
        }
    }
}
