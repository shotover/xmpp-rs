#[cfg(feature = "scram")]
use rand_os::rand_core::Error as RngError;

/// A wrapper enum for things that could go wrong in this crate.
#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "scram")]
    /// An error while initializing the Rng.
    RngError(RngError),
    /// An error in a SASL mechanism.
    SaslError(String),
}

#[cfg(feature = "scram")]
impl From<RngError> for Error {
    fn from(err: RngError) -> Error {
        Error::RngError(err)
    }
}
