//! Crate wrapping what we need from ICU’s C API for JIDs.
//!
//! See <http://site.icu-project.org/>

use crate::bindings::{icu_error_code_to_name, UErrorCode};
use std::ffi::CStr;

/// Errors this library can produce.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// An error produced by one of the ICU functions.
    Icu(String),

    /// An error produced by one of the IDNA2008 ICU functions.
    Idna(u32),

    /// Some ICU function didn’t produce a valid UTF-8 string, should never happen.
    Utf8(std::string::FromUtf8Error),

    /// Some ICU function didn’t produce a valid UTF-8 string, should never happen.
    Utf16(std::char::DecodeUtf16Error),

    /// Some string was too long for its profile in JID.
    TooLong,
}

impl Error {
    pub(crate) fn from_icu_code(err: UErrorCode) -> Error {
        let ptr = unsafe { icu_error_code_to_name(err) };
        let c_str = unsafe { CStr::from_ptr(ptr) };
        Error::Icu(c_str.to_string_lossy().into_owned())
    }
}

impl From<UErrorCode> for Error {
    fn from(err: UErrorCode) -> Error {
        Error::from_icu_code(err)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Error {
        Error::Utf8(err)
    }
}

impl From<std::char::DecodeUtf16Error> for Error {
    fn from(err: std::char::DecodeUtf16Error) -> Error {
        Error::Utf16(err)
    }
}
