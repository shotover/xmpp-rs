use std::collections::HashMap;

use std::convert::From;

use std::string::FromUtf8Error;

#[cfg(feature = "scram")]
pub mod scram;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Identity {
    None,
    Username(String),
}

impl From<String> for Identity {
    fn from(s: String) -> Identity {
        Identity::Username(s)
    }
}

impl<'a> From<&'a str> for Identity {
    fn from(s: &'a str) -> Identity {
        Identity::Username(s.to_owned())
    }
}

/// A struct containing SASL credentials.
#[derive(Clone, Debug)]
pub struct Credentials {
    /// The requested identity.
    pub identity: Identity,
    /// The secret used to authenticate.
    pub secret: Secret,
    /// Channel binding data, for *-PLUS mechanisms.
    pub channel_binding: ChannelBinding,
}

impl Default for Credentials {
    fn default() -> Credentials {
        Credentials {
            identity: Identity::None,
            secret: Secret::None,
            channel_binding: ChannelBinding::Unsupported,
        }
    }
}

impl Credentials {
    /// Creates a new Credentials with the specified username.
    pub fn with_username<N: Into<String>>(mut self, username: N) -> Credentials {
        self.identity = Identity::Username(username.into());
        self
    }

    /// Creates a new Credentials with the specified plaintext password.
    pub fn with_password<P: Into<String>>(mut self, password: P) -> Credentials {
        self.secret = Secret::password_plain(password);
        self
    }

    /// Creates a new Credentials with the specified chanel binding.
    pub fn with_channel_binding(mut self, channel_binding: ChannelBinding) -> Credentials {
        self.channel_binding = channel_binding;
        self
    }
}

/// Represents a SASL secret, like a password.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Secret {
    /// No extra data needed.
    None,
    /// Password required.
    Password(Password),
}

impl Secret {
    pub fn password_plain<S: Into<String>>(password: S) -> Secret {
        Secret::Password(Password::Plain(password.into()))
    }

    pub fn password_pbkdf2<S: Into<String>>(
        method: S,
        salt: Vec<u8>,
        iterations: u32,
        data: Vec<u8>,
    ) -> Secret {
        Secret::Password(Password::Pbkdf2 {
            method: method.into(),
            salt: salt,
            iterations: iterations,
            data: data,
        })
    }
}

/// Represents a password.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Password {
    /// A plaintext password.
    Plain(String),
    /// A password digest derived using PBKDF2.
    Pbkdf2 {
        method: String,
        salt: Vec<u8>,
        iterations: u32,
        data: Vec<u8>,
    },
}

impl From<String> for Password {
    fn from(s: String) -> Password {
        Password::Plain(s)
    }
}

impl<'a> From<&'a str> for Password {
    fn from(s: &'a str) -> Password {
        Password::Plain(s.to_owned())
    }
}

#[cfg(test)]
#[test]
fn xor_works() {
    assert_eq!(
        xor(
            &[135, 94, 53, 134, 73, 233, 140, 221, 150, 12, 96, 111, 54, 66, 11, 76],
            &[163, 9, 122, 180, 107, 44, 22, 252, 248, 134, 112, 82, 84, 122, 56, 209]
        ),
        &[36, 87, 79, 50, 34, 197, 154, 33, 110, 138, 16, 61, 98, 56, 51, 157]
    );
}

#[doc(hidden)]
pub fn xor(a: &[u8], b: &[u8]) -> Vec<u8> {
    assert_eq!(a.len(), b.len());
    let mut ret = Vec::with_capacity(a.len());
    for (a, b) in a.into_iter().zip(b) {
        ret.push(a ^ b);
    }
    ret
}

#[doc(hidden)]
pub fn parse_frame(frame: &[u8]) -> Result<HashMap<String, String>, FromUtf8Error> {
    let inner = String::from_utf8(frame.to_owned())?;
    let mut ret = HashMap::new();
    for s in inner.split(',') {
        let mut tmp = s.splitn(2, '=');
        let key = tmp.next();
        let val = tmp.next();
        match (key, val) {
            (Some(k), Some(v)) => {
                ret.insert(k.to_owned(), v.to_owned());
            }
            _ => (),
        }
    }
    Ok(ret)
}

/// Channel binding configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelBinding {
    /// No channel binding data.
    None,
    /// Advertise that the client does not think the server supports channel binding.
    Unsupported,
    /// p=tls-unique channel binding data (for TLS 1.2).
    TlsUnique(Vec<u8>),
    /// p=tls-exporter channel binding data (for TLS 1.3).
    TlsExporter(Vec<u8>),
}

impl ChannelBinding {
    /// Return the gs2 header for this channel binding mechanism.
    pub fn header(&self) -> &[u8] {
        match *self {
            ChannelBinding::None => b"n,,",
            ChannelBinding::Unsupported => b"y,,",
            ChannelBinding::TlsUnique(_) => b"p=tls-unique,,",
            ChannelBinding::TlsExporter(_) => b"p=tls-exporter,,",
        }
    }

    /// Return the channel binding data for this channel binding mechanism.
    pub fn data(&self) -> &[u8] {
        match *self {
            ChannelBinding::None => &[],
            ChannelBinding::Unsupported => &[],
            ChannelBinding::TlsUnique(ref data) => data,
            ChannelBinding::TlsExporter(ref data) => data,
        }
    }

    /// Checks whether this channel binding mechanism is supported.
    pub fn supports(&self, mechanism: &str) -> bool {
        match *self {
            ChannelBinding::None => false,
            ChannelBinding::Unsupported => false,
            ChannelBinding::TlsUnique(_) => mechanism == "tls-unique",
            ChannelBinding::TlsExporter(_) => mechanism == "tls-exporter",
        }
    }
}
