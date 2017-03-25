#[cfg(feature = "scram")]
use openssl::error::ErrorStack;

/// A wrapper enum for things that could go wrong in this crate.
#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "scram")]
    /// An error in OpenSSL.
    OpenSslErrorStack(ErrorStack),
    /// An error in a SASL mechanism.
    SaslError(String),
}

#[cfg(feature = "scram")]
impl From<ErrorStack> for Error {
    fn from(err: ErrorStack) -> Error {
        Error::OpenSslErrorStack(err)
    }
}
