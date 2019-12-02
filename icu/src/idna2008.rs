//! Crate wrapping what we need from ICU’s C API for JIDs.
//!
//! See http://site.icu-project.org/

use crate::bindings::{
    icu_idna_name_to_ascii, icu_idna_name_to_unicode, icu_idna_open, UErrorCode, UIDNAInfo,
    UIdnaFunction, UIDNA, U_ZERO_ERROR,
};
use crate::error::Error;

/// TODO: IDNA2008 support.
pub struct Idna {
    inner: *mut UIDNA,
}

impl Idna {
    /// Create a new Idna struct.
    pub fn new(options: u32) -> Result<Idna, UErrorCode> {
        let mut err: UErrorCode = U_ZERO_ERROR;
        let inner = unsafe { icu_idna_open(options, &mut err) };
        match err {
            U_ZERO_ERROR => Ok(Idna { inner }),
            err => Err(err),
        }
    }

    /// Converts a whole domain name into its ASCII form for DNS lookup.
    pub fn to_ascii(&self, input: &str) -> Result<String, Error> {
        self.idna(input, icu_idna_name_to_ascii)
    }

    /// Converts a whole domain name into its Unicode form for human-readable display.
    pub fn to_unicode(&self, input: &str) -> Result<String, Error> {
        self.idna(input, icu_idna_name_to_unicode)
    }

    fn idna(&self, input: &str, function: UIdnaFunction) -> Result<String, Error> {
        if input.len() > 255 {
            return Err(Error::TooLong);
        }

        let mut err: UErrorCode = U_ZERO_ERROR;
        let mut dest: Vec<u8> = vec![0u8; 256];
        let mut info = UIDNAInfo::new();
        let len = unsafe {
            function(
                self.inner,
                input.as_ptr(),
                input.len() as i32,
                dest.as_mut_ptr(),
                dest.len() as i32,
                &mut info,
                &mut err,
            )
        };
        if err != U_ZERO_ERROR {
            return Err(Error::from_icu_code(err));
        }
        let errors = info.get_errors();
        if errors != 0 {
            return Err(Error::Idna(errors));
        }
        if len > 255 {
            return Err(Error::TooLong);
        }
        dest.truncate(len as usize);
        Ok(String::from_utf8(dest)?)
    }
}
