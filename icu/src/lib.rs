//! Crate wrapping what we need from ICU’s C API for JIDs.
//!
//! See <http://site.icu-project.org/>

#![deny(missing_docs)]

mod bindings;
mod error;
mod idna2008;
mod spoof;
mod stringprep;

use crate::bindings::{
    icu_trace_set_level, UIDNA_DEFAULT, UIDNA_USE_STD3_RULES, USPOOF_CONFUSABLE,
    USPREP_RFC3491_NAMEPREP, USPREP_RFC3920_NODEPREP, USPREP_RFC3920_RESOURCEPREP,
    USPREP_RFC4013_SASLPREP, UTRACE_VERBOSE,
};
pub use crate::error::Error;
pub use crate::idna2008::Idna;
pub use crate::spoof::SpoofChecker;
use crate::stringprep::Stringprep;

/// How unassigned codepoints should be handled.
pub enum Strict {
    /// All codepoints should be assigned, otherwise an error will be emitted.
    True,

    /// Codepoints can be unassigned.
    AllowUnassigned,
}

/// Main struct of this module, exposing the needed ICU functions to JID.
pub struct Icu {
    nameprep: Stringprep,
    nodeprep: Stringprep,
    resourceprep: Stringprep,
    saslprep: Stringprep,

    /// IDNA2008 support.
    ///
    /// See [RFC5891](https://tools.ietf.org/html/rfc5891).
    pub idna2008: Idna,

    /// Spoof checker TODO: better doc.
    pub spoofchecker: SpoofChecker,
}

impl Icu {
    /// Create a new ICU struct, initialising stringprep profiles, IDNA2008, as well as a spoof
    /// checker.
    pub fn new() -> Result<Icu, Error> {
        unsafe { icu_trace_set_level(UTRACE_VERBOSE) };

        let nameprep = Stringprep::new(USPREP_RFC3491_NAMEPREP)?;
        let nodeprep = Stringprep::new(USPREP_RFC3920_NODEPREP)?;
        let resourceprep = Stringprep::new(USPREP_RFC3920_RESOURCEPREP)?;
        let saslprep = Stringprep::new(USPREP_RFC4013_SASLPREP)?;

        let mut options = UIDNA_DEFAULT;
        options |= UIDNA_USE_STD3_RULES;
        let idna2008 = Idna::new(options)?;

        let spoofchecker = SpoofChecker::new(USPOOF_CONFUSABLE)?;

        Ok(Icu {
            nameprep,
            nodeprep,
            resourceprep,
            saslprep,
            idna2008,
            spoofchecker,
        })
    }

    /// Perform stringprep using the Nameprep profile.
    ///
    /// See [RFC3491](https://tools.ietf.org/html/rfc3491).
    pub fn nameprep(&self, string: &str, strict: Strict) -> Result<String, Error> {
        self.nameprep.stringprep(string, strict)
    }

    /// Perform stringprep using the Nodeprep profile.
    ///
    /// See [RFC6122 appendix A](https://tools.ietf.org/html/rfc6122#appendix-A).
    pub fn nodeprep(&self, string: &str, strict: Strict) -> Result<String, Error> {
        self.nodeprep.stringprep(string, strict)
    }

    /// Perform stringprep using the Resourceprep profile.
    ///
    /// See [RFC6122 appendix A](https://tools.ietf.org/html/rfc6122#appendix-A).
    pub fn resourceprep(&self, string: &str, strict: Strict) -> Result<String, Error> {
        self.resourceprep.stringprep(string, strict)
    }

    /// Perform stringprep using the Saslprep profile.
    ///
    /// See [RFC4013](https://tools.ietf.org/html/rfc4013).
    pub fn saslprep(&self, string: &str, strict: Strict) -> Result<String, Error> {
        self.saslprep.stringprep(string, strict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nameprep() {
        let name = "Link";
        let icu = Icu::new().unwrap();
        let name = icu.nodeprep.stringprep(name, Strict::True).unwrap();
        assert_eq!(name, "link");
    }

    #[test]
    fn resourceprep() {
        let name = "Test™";
        let icu = Icu::new().unwrap();
        let name = icu
            .resourceprep
            .stringprep(name, Strict::AllowUnassigned)
            .unwrap();
        assert_eq!(name, "TestTM");
    }

    #[test]
    fn idna() {
        let name = "☃.coM";
        let icu = Icu::new().unwrap();
        let name = icu.idna2008.to_ascii(name).unwrap();
        assert_eq!(name, "xn--n3h.com");

        let name = "xn--N3H.com";
        let icu = Icu::new().unwrap();
        let name = icu.idna2008.to_unicode(name).unwrap();
        assert_eq!(name, "☃.com");
    }

    #[test]
    fn spoof() {
        // Non-breakable and narrow non-breakable spaces spoofing.
        let name = "foo bar baz";
        let icu = Icu::new().unwrap();
        let name = icu.spoofchecker.get_skeleton(name).unwrap();
        assert_eq!(name, "foo bar baz");

        // Cyrillic spoofing.
        let name = "Неllо wоrld";
        let icu = Icu::new().unwrap();
        let name = icu.spoofchecker.get_skeleton(name).unwrap();
        assert_eq!(name, "Hello world");
    }
}
