#![deny(missing_docs)]

//! This crate provides a framework for SASL authentication and a few authentication mechanisms.
//!
//! # Examples
//!
//! ```rust
//! use sasl::{SaslCredentials, SaslSecret, SaslMechanism, Error};
//! use sasl::mechanisms::Plain;
//!
//! let creds = SaslCredentials {
//!     username: "user".to_owned(),
//!     secret: SaslSecret::Password("pencil".to_owned()),
//!     channel_binding: None,
//! };
//!
//! let mut mechanism = Plain::from_credentials(creds).unwrap();
//!
//! let initial_data = mechanism.initial().unwrap();
//!
//! assert_eq!(initial_data, b"\0user\0pencil");
//! ```
//!
//! You may look at the tests of `mechanisms/scram.rs` for examples of more advanced usage.
//!
//! # Usage
//!
//! You can use this in your crate by adding this under `dependencies` in your `Cargo.toml`:
//!
//! ```toml,ignore
//! sasl = "*"
//! ```

extern crate base64;
extern crate openssl;

mod error;

pub use error::Error;

/// A struct containing SASL credentials.
pub struct SaslCredentials {
    /// The requested username.
    pub username: String, // TODO: change this since some mechanisms do not use it
    /// The secret used to authenticate.
    pub secret: SaslSecret,
    /// Optionally, channel binding data, for *-PLUS mechanisms.
    pub channel_binding: Option<Vec<u8>>,
}

/// Represents a SASL secret, like a password.
pub enum SaslSecret {
    /// No extra data needed.
    None,
    /// Password required.
    Password(String),
}

/// A trait which defines SASL mechanisms.
pub trait SaslMechanism {
    /// The name of the mechanism.
    fn name(&self) -> &str;

    /// Creates this mechanism from `SaslCredentials`.
    fn from_credentials(credentials: SaslCredentials) -> Result<Self, String>
    where
        Self: Sized;

    /// Provides initial payload of the SASL mechanism.
    fn initial(&mut self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
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
