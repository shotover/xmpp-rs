//! Crate wrapping what we need from ICU’s C API for JIDs.
//!
//! See http://site.icu-project.org/

use crate::bindings::{
    icu_stringprep_open, icu_stringprep_prepare, UChar, UErrorCode, UStringPrepProfile,
    UStringPrepProfileType, USPREP_ALLOW_UNASSIGNED, USPREP_DEFAULT, U_ZERO_ERROR,
};
use crate::error::Error;
use crate::Strict;
use std::ptr::null_mut;

/// Struct representing a given stringprep profile.
pub struct Stringprep {
    inner: *mut UStringPrepProfile,
}

impl Stringprep {
    /// Create a new Stringprep struct for the given profile.
    pub(crate) fn new(profile: UStringPrepProfileType) -> Result<Stringprep, UErrorCode> {
        let mut err: UErrorCode = U_ZERO_ERROR;
        let inner = unsafe { icu_stringprep_open(profile, &mut err) };
        match err {
            U_ZERO_ERROR => Ok(Stringprep { inner }),
            err => Err(err),
        }
    }

    /// Perform a stringprep operation using this profile.
    ///
    /// # Panics
    /// Panics if ICU doesn’t return a valid UTF-16 string, which should never happen.
    pub fn stringprep(&self, input: &str, strict: Strict) -> Result<String, Error> {
        if input.len() > 1023 {
            return Err(Error::TooLong);
        }

        // ICU works on UTF-16 data, so convert it first.
        let unprepped: Vec<UChar> = input.encode_utf16().collect();

        // Now do the actual stringprep operation.
        let mut prepped: Vec<UChar> = vec![0u16; 1024];
        let flags = match strict {
            Strict::True => USPREP_DEFAULT,
            Strict::AllowUnassigned => USPREP_ALLOW_UNASSIGNED,
        };
        self.prepare(&unprepped, &mut prepped, flags)?;

        // And then convert it back to UTF-8.
        let output = std::char::decode_utf16(prepped.into_iter())
            //.map(Result::unwrap)
            .try_fold(Vec::new(), |mut acc, c| match c {
                Ok(c) => {
                    acc.push(c);
                    Ok(acc)
                }
                Err(err) => Err(err),
            })?;
        let output: String = output.into_iter().collect();

        if output.len() > 1023 {
            return Err(Error::TooLong);
        }

        Ok(output)
    }

    fn prepare(&self, input: &[UChar], buf: &mut Vec<UChar>, flags: i32) -> Result<(), UErrorCode> {
        let mut err: UErrorCode = U_ZERO_ERROR;
        let prepped_len = unsafe {
            icu_stringprep_prepare(
                self.inner,
                input.as_ptr(),
                input.len() as i32,
                buf.as_mut_ptr(),
                buf.len() as i32,
                flags,
                null_mut(),
                &mut err,
            )
        };
        if err != U_ZERO_ERROR {
            return Err(err);
        }
        buf.truncate(prepped_len as usize);
        Ok(())
    }
}
