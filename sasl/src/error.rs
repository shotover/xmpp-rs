use openssl::error::ErrorStack;

#[derive(Debug)]
pub enum Error {
    OpenSslErrorStack(ErrorStack),
    SaslError(String),
}

impl From<ErrorStack> for Error {
    fn from(err: ErrorStack) -> Error {
        Error::OpenSslErrorStack(err)
    }
}
