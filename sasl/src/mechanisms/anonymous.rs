//! Provides the SASL "ANONYMOUS" mechanism.

use SaslCredentials;
use SaslMechanism;
use SaslSecret;

pub struct Anonymous;

impl Anonymous {
    pub fn new() -> Anonymous {
        Anonymous
    }
}

impl SaslMechanism for Anonymous {
    fn name(&self) -> &str {
        "ANONYMOUS"
    }

    fn from_credentials(credentials: SaslCredentials) -> Result<Anonymous, String> {
        if let SaslSecret::None = credentials.secret {
            Ok(Anonymous)
        } else {
            Err("the anonymous sasl mechanism requires no credentials".to_owned())
        }
    }
}
