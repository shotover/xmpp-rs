#[cfg(feature = "scram")]
use getrandom::Error as RngError;

/// A wrapper enum for things that could go wrong in this crate.
#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "scram")]
    #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
    /// An error while initializing the Rng.
    RngError(RngError),
    /// An error in a SASL mechanism.
    SaslError(String),
}

#[cfg(feature = "scram")]
#[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
impl From<RngError> for Error {
    fn from(err: RngError) -> Error {
        Error::RngError(err)
    }
}
