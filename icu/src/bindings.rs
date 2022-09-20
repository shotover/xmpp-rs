//! Crate wrapping what we need from ICU’s C API for JIDs.
//!
//! See <http://site.icu-project.org/>

use std::os::raw::c_char;

// From unicode/umachine.h
pub(crate) type UChar = u16;

// From unicode/utypes.h
pub(crate) type UErrorCode = u32;
pub(crate) const U_ZERO_ERROR: UErrorCode = 0;

pub(crate) type UStringPrepProfile = u32;
type UParseError = u32;

// From unicode/usprep.h
pub(crate) const USPREP_DEFAULT: i32 = 0;
pub(crate) const USPREP_ALLOW_UNASSIGNED: i32 = 1;

pub(crate) type UStringPrepProfileType = u32;
pub(crate) const USPREP_RFC3491_NAMEPREP: UStringPrepProfileType = 0;
pub(crate) const USPREP_RFC3920_NODEPREP: UStringPrepProfileType = 7;
pub(crate) const USPREP_RFC3920_RESOURCEPREP: UStringPrepProfileType = 8;
pub(crate) const USPREP_RFC4013_SASLPREP: UStringPrepProfileType = 10;

// From unicode/utrace.h
type UTraceLevel = i32;
pub(crate) const UTRACE_VERBOSE: UTraceLevel = 9;

// From unicode/uidna.h
#[repr(C)]
pub(crate) struct UIDNA {
    _unused: [u8; 0],
}
type UBool = i8;

#[repr(C)]
pub(crate) struct UIDNAInfo {
    size: i16,
    is_transitional_different: UBool,
    reserved_b3: UBool,
    errors: u32,
    reserved_i2: i32,
    reserved_i3: i32,
}

impl UIDNAInfo {
    pub(crate) fn new() -> UIDNAInfo {
        assert_eq!(std::mem::size_of::<UIDNAInfo>(), 16);
        UIDNAInfo {
            size: std::mem::size_of::<UIDNAInfo>() as i16,
            is_transitional_different: false as UBool,
            reserved_b3: false as UBool,
            errors: 0,
            reserved_i2: 0,
            reserved_i3: 0,
        }
    }

    // TODO: Return a String instead, or a custom error type, this is a bitflag (defined in
    // uidna.h) where multiple errors can be accumulated.
    pub(crate) fn get_errors(&self) -> u32 {
        self.errors
    }
}

pub(crate) const UIDNA_DEFAULT: u32 = 0;
pub(crate) const UIDNA_USE_STD3_RULES: u32 = 2;

pub(crate) type UIdnaFunction = unsafe extern "C" fn(
    *const UIDNA,
    *const u8,
    i32,
    *mut u8,
    i32,
    *mut UIDNAInfo,
    *mut u32,
) -> i32;

// From unicode/uspoof.h
#[repr(C)]
pub(crate) struct USpoofChecker {
    _unused: [u8; 0],
}
pub(crate) const USPOOF_CONFUSABLE: i32 = 7;

#[link(name = "bindings")]
extern "C" {
    // From unicode/ustring.h
    pub(crate) fn icu_error_code_to_name(code: UErrorCode) -> *const c_char;

    // From unicode/usprep.h
    pub(crate) fn icu_stringprep_open(
        type_: UStringPrepProfileType,
        status: *mut UErrorCode,
    ) -> *mut UStringPrepProfile;
    pub(crate) fn icu_stringprep_prepare(
        prep: *const UStringPrepProfile,
        src: *const UChar,
        srcLength: i32,
        dest: *mut UChar,
        destCapacity: i32,
        options: i32,
        parseError: *mut UParseError,
        status: *mut UErrorCode,
    ) -> i32;

    // From unicode/utrace.h
    pub(crate) fn icu_trace_set_level(traceLevel: UTraceLevel);

    // From unicode/uidna.h
    pub(crate) fn icu_idna_open(options: u32, pErrorCode: *mut UErrorCode) -> *mut UIDNA;
    pub(crate) fn icu_idna_name_to_ascii(
        idna: *const UIDNA,
        name: *const u8,
        length: i32,
        dest: *mut u8,
        capacity: i32,
        pInfo: *mut UIDNAInfo,
        pErrorCode: *mut UErrorCode,
    ) -> i32;
    pub(crate) fn icu_idna_name_to_unicode(
        idna: *const UIDNA,
        name: *const u8,
        length: i32,
        dest: *mut u8,
        capacity: i32,
        pInfo: *mut UIDNAInfo,
        pErrorCode: *mut UErrorCode,
    ) -> i32;

    // From unicode/uspoof.h
    pub(crate) fn icu_spoof_open(status: *mut UErrorCode) -> *mut USpoofChecker;
    pub(crate) fn icu_spoof_set_checks(
        sc: *mut USpoofChecker,
        checks: i32,
        status: *mut UErrorCode,
    );
    pub(crate) fn icu_spoof_get_skeleton(
        sc: *const USpoofChecker,
        type_: u32,
        id: *const u8,
        length: i32,
        dest: *mut u8,
        destCapacity: i32,
        status: *mut UErrorCode,
    ) -> i32;
}
