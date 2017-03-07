#![deny(missing_docs)]

//! This crate provides a framework for SASL authentication and a few authentication mechanisms.
//!
//! # Examples
//!
//! ```rust
//! use sasl::{Credentials, Mechanism, Error};
//! use sasl::mechanisms::Plain;
//!
//! let creds = Credentials::default()
//!                         .with_username("user")
//!                         .with_password("pencil");
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
#[derive(Clone, Debug)]
pub struct Credentials {
    /// The requested username.
    pub username: Option<String>,
    /// The secret used to authenticate.
    pub secret: Secret,
    /// Channel binding data, for *-PLUS mechanisms.
    pub channel_binding: ChannelBinding,
}

impl Default for Credentials {
    fn default() -> Credentials {
        Credentials {
            username: None,
            secret: Secret::None,
            channel_binding: ChannelBinding::None,
        }
    }
}

impl Credentials {
    /// Creates a new Credentials with the specified username.
    pub fn with_username<N: Into<String>>(mut self, username: N) -> Credentials {
        self.username = Some(username.into());
        self
    }

    /// Creates a new Credentials with the specified password.
    pub fn with_password<P: Into<String>>(mut self, password: P) -> Credentials {
        self.secret = Secret::Password(password.into());
        self
    }

    /// Creates a new Credentials with the specified chanel binding.
    pub fn with_channel_binding(mut self, channel_binding: ChannelBinding) -> Credentials {
        self.channel_binding = channel_binding;
        self
    }
}

/// Channel binding configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelBinding {
    /// No channel binding data.
    None,
    /// Advertise that the client does not think the server supports channel binding.
    Unsupported,
    /// p=tls-unique channel binding data.
    TlsUnique(Vec<u8>),
}

impl ChannelBinding {
    /// Return the gs2 header for this channel binding mechanism.
    pub fn header(&self) -> &[u8] {
        match *self {
            ChannelBinding::None => b"n,,",
            ChannelBinding::Unsupported => b"y,,",
            ChannelBinding::TlsUnique(_) => b"p=tls-unique,,",
        }
    }

    /// Return the channel binding data for this channel binding mechanism.
    pub fn data(&self) -> &[u8] {
        match *self {
            ChannelBinding::None => &[],
            ChannelBinding::Unsupported => &[],
            ChannelBinding::TlsUnique(ref data) => data,
        }
    }
}

/// Represents a SASL secret, like a password.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Secret {
    /// No extra data needed.
    None,
    /// Password required.
    Password(String),
}

/// A trait which defines SASL mechanisms.
pub trait Mechanism {
    /// The name of the mechanism.
    fn name(&self) -> &str;

    /// Creates this mechanism from `Credentials`.
    fn from_credentials(credentials: Credentials) -> Result<Self, String>
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
