// Copyright (c) 2023 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(missing_docs)]

//! Provides a type for Jabber IDs.
//!
//! For usage, check the documentation on the `Jid` struct.

use crate::Error;
use core::num::NonZeroU16;
use memchr::memchr;
use std::str::FromStr;
use stringprep::{nameprep, nodeprep, resourceprep};

fn length_check(len: usize, error_empty: Error, error_too_long: Error) -> Result<(), Error> {
    if len == 0 {
        Err(error_empty)
    } else if len > 1023 {
        Err(error_too_long)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct InnerJid {
    pub(crate) normalized: String,
    pub(crate) at: Option<NonZeroU16>,
    pub(crate) slash: Option<NonZeroU16>,
}

impl InnerJid {
    pub(crate) fn new(unnormalized: &str) -> Result<InnerJid, Error> {
        let bytes = unnormalized.as_bytes();
        let mut orig_at = memchr(b'@', bytes);
        let mut orig_slash = memchr(b'/', bytes);
        if orig_at.is_some() && orig_slash.is_some() && orig_at > orig_slash {
            // This is part of the resource, not a node@domain separator.
            orig_at = None;
        }

        let normalized = match (orig_at, orig_slash) {
            (Some(at), Some(slash)) => {
                let node = nodeprep(&unnormalized[..at]).map_err(|_| Error::NodePrep)?;
                length_check(node.len(), Error::NodeEmpty, Error::NodeTooLong)?;

                let domain = nameprep(&unnormalized[at + 1..slash]).map_err(|_| Error::NamePrep)?;
                length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;

                let resource =
                    resourceprep(&unnormalized[slash + 1..]).map_err(|_| Error::ResourcePrep)?;
                length_check(resource.len(), Error::ResourceEmpty, Error::ResourceTooLong)?;

                orig_at = Some(node.len());
                orig_slash = Some(node.len() + domain.len() + 1);
                format!("{node}@{domain}/{resource}")
            }
            (Some(at), None) => {
                let node = nodeprep(&unnormalized[..at]).map_err(|_| Error::NodePrep)?;
                length_check(node.len(), Error::NodeEmpty, Error::NodeTooLong)?;

                let domain = nameprep(&unnormalized[at + 1..]).map_err(|_| Error::NamePrep)?;
                length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;

                orig_at = Some(node.len());
                format!("{node}@{domain}")
            }
            (None, Some(slash)) => {
                let domain = nameprep(&unnormalized[..slash]).map_err(|_| Error::NamePrep)?;
                length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;

                let resource =
                    resourceprep(&unnormalized[slash + 1..]).map_err(|_| Error::ResourcePrep)?;
                length_check(resource.len(), Error::ResourceEmpty, Error::ResourceTooLong)?;

                orig_slash = Some(domain.len());
                format!("{domain}/{resource}")
            }
            (None, None) => {
                let domain = nameprep(unnormalized).map_err(|_| Error::NamePrep)?;
                length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;

                domain.into_owned()
            }
        };

        Ok(InnerJid {
            normalized,
            at: orig_at.and_then(|x| NonZeroU16::new(x as u16)),
            slash: orig_slash.and_then(|x| NonZeroU16::new(x as u16)),
        })
    }

    pub(crate) fn node(&self) -> Option<&str> {
        self.at.and_then(|at| {
            let at = u16::from(at) as usize;
            Some(&self.normalized[..at])
        })
    }

    pub(crate) fn domain(&self) -> &str {
        match (self.at, self.slash) {
            (Some(at), Some(slash)) => {
                let at = u16::from(at) as usize;
                let slash = u16::from(slash) as usize;
                &self.normalized[at + 1..slash]
            }
            (Some(at), None) => {
                let at = u16::from(at) as usize;
                &self.normalized[at + 1..]
            }
            (None, Some(slash)) => {
                let slash = u16::from(slash) as usize;
                &self.normalized[..slash]
            }
            (None, None) => &self.normalized,
        }
    }

    pub(crate) fn resource(&self) -> Option<&str> {
        self.slash.and_then(|slash| {
            let slash = u16::from(slash) as usize;
            Some(&self.normalized[slash + 1..])
        })
    }
}

impl FromStr for InnerJid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        InnerJid::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_size (
        ($t:ty, $sz:expr) => (
            assert_eq!(::std::mem::size_of::<$t>(), $sz);
        );
    );

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(InnerJid, 16);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(InnerJid, 32);
    }
}
