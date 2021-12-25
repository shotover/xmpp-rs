use crate::common::scram::DeriveError;
use crate::common::Identity;
use crate::secret::Secret;
use std::fmt;

#[macro_export]
macro_rules! impl_validator_using_provider {
    ( $validator:ty, $secret:ty ) => {
        impl $crate::server::Validator<$secret> for $validator {
            fn validate(
                &self,
                identity: &$crate::common::Identity,
                value: &$secret,
            ) -> Result<(), ValidatorError> {
                if &(self as &$crate::server::Provider<$secret>).provide(identity)? == value {
                    Ok(())
                } else {
                    Err(ValidatorError::AuthenticationFailed)
                }
            }
        }
    };
}

pub trait Provider<S: Secret>: Validator<S> {
    fn provide(&self, identity: &Identity) -> Result<S, ProviderError>;
}

pub trait Validator<S: Secret> {
    fn validate(&self, identity: &Identity, value: &S) -> Result<(), ValidatorError>;
}

#[derive(Debug, PartialEq)]
pub enum ProviderError {
    AuthenticationFailed,
    DeriveError(DeriveError),
}

#[derive(Debug, PartialEq)]
pub enum ValidatorError {
    AuthenticationFailed,
    ProviderError(ProviderError),
}

#[derive(Debug, PartialEq)]
pub enum MechanismError {
    NoUsernameSpecified,
    ErrorDecodingUsername,
    NoPasswordSpecified,
    ErrorDecodingPassword,
    ValidatorError(ValidatorError),

    FailedToDecodeMessage,
    ChannelBindingNotSupported,
    ChannelBindingIsSupported,
    ChannelBindingMechanismIncorrect,
    CannotDecodeInitialMessage,
    NoUsername,
    NoNonce,
    FailedToGenerateNonce,
    ProviderError(ProviderError),

    CannotDecodeResponse,
    InvalidKeyLength(hmac::digest::InvalidLength),
    RandomFailure(getrandom::Error),
    NoProof,
    CannotDecodeProof,
    AuthenticationFailed,
    SaslSessionAlreadyOver,
}

impl From<DeriveError> for ProviderError {
    fn from(err: DeriveError) -> ProviderError {
        ProviderError::DeriveError(err)
    }
}

impl From<ProviderError> for ValidatorError {
    fn from(err: ProviderError) -> ValidatorError {
        ValidatorError::ProviderError(err)
    }
}

impl From<ProviderError> for MechanismError {
    fn from(err: ProviderError) -> MechanismError {
        MechanismError::ProviderError(err)
    }
}

impl From<ValidatorError> for MechanismError {
    fn from(err: ValidatorError) -> MechanismError {
        MechanismError::ValidatorError(err)
    }
}

impl From<hmac::digest::InvalidLength> for MechanismError {
    fn from(err: hmac::digest::InvalidLength) -> MechanismError {
        MechanismError::InvalidKeyLength(err)
    }
}

impl From<getrandom::Error> for MechanismError {
    fn from(err: getrandom::Error) -> MechanismError {
        MechanismError::RandomFailure(err)
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "provider error")
    }
}

impl fmt::Display for ValidatorError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "validator error")
    }
}

impl fmt::Display for MechanismError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MechanismError::NoUsernameSpecified => write!(fmt, "no username specified"),
            MechanismError::ErrorDecodingUsername => write!(fmt, "error decoding username"),
            MechanismError::NoPasswordSpecified => write!(fmt, "no password specified"),
            MechanismError::ErrorDecodingPassword => write!(fmt, "error decoding password"),
            MechanismError::ValidatorError(err) => write!(fmt, "validator error: {}", err),

            MechanismError::FailedToDecodeMessage => write!(fmt, "failed to decode message"),
            MechanismError::ChannelBindingNotSupported => {
                write!(fmt, "channel binding not supported")
            }
            MechanismError::ChannelBindingIsSupported => {
                write!(fmt, "channel binding is supported")
            }
            MechanismError::ChannelBindingMechanismIncorrect => {
                write!(fmt, "channel binding mechanism is incorrect")
            }
            MechanismError::CannotDecodeInitialMessage => {
                write!(fmt, "can’t decode initial message")
            }
            MechanismError::NoUsername => write!(fmt, "no username"),
            MechanismError::NoNonce => write!(fmt, "no nonce"),
            MechanismError::FailedToGenerateNonce => write!(fmt, "failed to generate nonce"),
            MechanismError::ProviderError(err) => write!(fmt, "provider error: {}", err),

            MechanismError::CannotDecodeResponse => write!(fmt, "can’t decode response"),
            MechanismError::InvalidKeyLength(err) => write!(fmt, "invalid key length: {}", err),
            MechanismError::RandomFailure(err) => {
                write!(fmt, "failure to get random data: {}", err)
            }
            MechanismError::NoProof => write!(fmt, "no proof"),
            MechanismError::CannotDecodeProof => write!(fmt, "can’t decode proof"),
            MechanismError::AuthenticationFailed => write!(fmt, "authentication failed"),
            MechanismError::SaslSessionAlreadyOver => write!(fmt, "SASL session already over"),
        }
    }
}

impl Error for ProviderError {}

impl Error for ValidatorError {}

use std::error::Error;
impl Error for MechanismError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MechanismError::ValidatorError(err) => Some(err),
            MechanismError::ProviderError(err) => Some(err),
            // TODO: figure out how to enable the std feature on this crate.
            //MechanismError::InvalidKeyLength(err) => Some(err),
            _ => None,
        }
    }
}

pub trait Mechanism {
    fn name(&self) -> &str;
    fn respond(&mut self, payload: &[u8]) -> Result<Response, MechanismError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Success(Identity, Vec<u8>),
    Proceed(Vec<u8>),
}

pub mod mechanisms;
