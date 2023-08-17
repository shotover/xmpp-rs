use std::fmt;

use crate::common::Credentials;

#[cfg(feature = "scram")]
use crate::common::scram::DeriveError;
#[cfg(feature = "scram")]
use hmac::digest::InvalidLength;

#[derive(Debug, PartialEq)]
pub enum MechanismError {
    AnonymousRequiresNoCredentials,

    PlainRequiresUsername,
    PlainRequiresPlaintextPassword,

    CannotGenerateNonce,
    ScramRequiresUsername,
    ScramRequiresPassword,

    CannotDecodeChallenge,
    NoServerNonce,
    NoServerSalt,
    NoServerIterations,
    #[cfg(feature = "scram")]
    DeriveError(DeriveError),
    #[cfg(feature = "scram")]
    InvalidKeyLength(InvalidLength),
    InvalidState,

    CannotDecodeSuccessResponse,
    InvalidSignatureInSuccessResponse,
    NoSignatureInSuccessResponse,
}

#[cfg(feature = "scram")]
impl From<DeriveError> for MechanismError {
    fn from(err: DeriveError) -> MechanismError {
        MechanismError::DeriveError(err)
    }
}

#[cfg(feature = "scram")]
impl From<InvalidLength> for MechanismError {
    fn from(err: InvalidLength) -> MechanismError {
        MechanismError::InvalidKeyLength(err)
    }
}

impl fmt::Display for MechanismError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "{}",
            match self {
                MechanismError::AnonymousRequiresNoCredentials =>
                    "ANONYMOUS mechanism requires no credentials",

                MechanismError::PlainRequiresUsername => "PLAIN requires a username",
                MechanismError::PlainRequiresPlaintextPassword =>
                    "PLAIN requires a plaintext password",

                MechanismError::CannotGenerateNonce => "can't generate nonce",
                MechanismError::ScramRequiresUsername => "SCRAM requires a username",
                MechanismError::ScramRequiresPassword => "SCRAM requires a password",

                MechanismError::CannotDecodeChallenge => "can't decode challenge",
                MechanismError::NoServerNonce => "no server nonce",
                MechanismError::NoServerSalt => "no server salt",
                MechanismError::NoServerIterations => "no server iterations",
                #[cfg(feature = "scram")]
                MechanismError::DeriveError(err) => return write!(fmt, "derive error: {}", err),
                #[cfg(feature = "scram")]
                MechanismError::InvalidKeyLength(err) =>
                    return write!(fmt, "invalid key length: {}", err),
                MechanismError::InvalidState => "not in the right state to receive this response",

                MechanismError::CannotDecodeSuccessResponse => "can't decode success response",
                MechanismError::InvalidSignatureInSuccessResponse =>
                    "invalid signature in success response",
                MechanismError::NoSignatureInSuccessResponse => "no signature in success response",
            }
        )
    }
}

impl std::error::Error for MechanismError {}

/// A trait which defines SASL mechanisms.
pub trait Mechanism {
    /// The name of the mechanism.
    fn name(&self) -> &str;

    /// Creates this mechanism from `Credentials`.
    fn from_credentials(credentials: Credentials) -> Result<Self, MechanismError>
    where
        Self: Sized;

    /// Provides initial payload of the SASL mechanism.
    fn initial(&mut self) -> Vec<u8> {
        Vec::new()
    }

    /// Creates a response to the SASL challenge.
    fn response(&mut self, _challenge: &[u8]) -> Result<Vec<u8>, MechanismError> {
        Ok(Vec::new())
    }

    /// Verifies the server success response, if there is one.
    fn success(&mut self, _data: &[u8]) -> Result<(), MechanismError> {
        Ok(())
    }
}

pub mod mechanisms;
