//! Crate wrapping what we need from ICU’s C API for JIDs.
//!
//! See <http://site.icu-project.org/>

use crate::bindings::{
    icu_spoof_get_skeleton, icu_spoof_open, icu_spoof_set_checks, UErrorCode, USpoofChecker,
    U_ZERO_ERROR,
};
use crate::error::Error;

/// TODO: spoof checker.
pub struct SpoofChecker {
    inner: *mut USpoofChecker,
}

impl SpoofChecker {
    /// Create a new SpoofChecker.
    pub fn new(checks: i32) -> Result<SpoofChecker, UErrorCode> {
        let mut err: UErrorCode = U_ZERO_ERROR;
        let inner = unsafe { icu_spoof_open(&mut err) };
        if err != U_ZERO_ERROR {
            return Err(err);
        }
        unsafe { icu_spoof_set_checks(inner, checks, &mut err) };
        if err != U_ZERO_ERROR {
            return Err(err);
        }
        Ok(SpoofChecker { inner })
    }

    /// Transform a string into a skeleton for matching it with other potentially similar strings.
    pub fn get_skeleton(&self, input: &str) -> Result<String, Error> {
        let mut err: UErrorCode = U_ZERO_ERROR;
        let mut dest: Vec<u8> = vec![0u8; 256];
        let len = unsafe {
            icu_spoof_get_skeleton(
                self.inner,
                0,
                input.as_ptr(),
                input.len() as i32,
                dest.as_mut_ptr(),
                dest.len() as i32,
                &mut err,
            )
        };
        if err != U_ZERO_ERROR {
            return Err(Error::from_icu_code(err));
        }
        dest.truncate(len as usize);
        Ok(String::from_utf8(dest)?)
    }
}
