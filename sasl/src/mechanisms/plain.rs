//! Provides the SASL "PLAIN" mechanism.

use SaslCredentials;
use SaslMechanism;
use SaslSecret;

/// A struct for the SASL PLAIN mechanism.
pub struct Plain {
    username: String,
    password: String,
}

impl Plain {
    /// Constructs a new struct for authenticating using the SASL PLAIN mechanism.
    ///
    /// It is recommended that instead you use a `SaslCredentials` struct and turn it into the
    /// requested mechanism using `from_credentials`.
    pub fn new<N: Into<String>, P: Into<String>>(username: N, password: P) -> Plain {
        Plain {
            username: username.into(),
            password: password.into(),
        }
    }
}

impl SaslMechanism for Plain {
    fn name(&self) -> &str {
        "PLAIN"
    }

    fn from_credentials(credentials: SaslCredentials) -> Result<Plain, String> {
        if let SaslSecret::Password(password) = credentials.secret {
            if let Some(username) = credentials.username {
                Ok(Plain::new(username, password))
            } else {
                Err("PLAIN requires a username".to_owned())
            }
        } else {
            Err("PLAIN requires a password".to_owned())
        }
    }

    fn initial(&mut self) -> Result<Vec<u8>, String> {
        let mut auth = Vec::new();
        auth.push(0);
        auth.extend(self.username.bytes());
        auth.push(0);
        auth.extend(self.password.bytes());
        Ok(auth)
    }
}
