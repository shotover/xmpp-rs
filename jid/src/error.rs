// Copyright (c) 2017, 2018 lumi <lumi@pew.im>
// Copyright (c) 2017, 2018, 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
// Copyright (c) 2017, 2018, 2019 Maxime “pep” Buquet <pep@bouah.net>
// Copyright (c) 2017, 2018 Astro <astro@spaceboyz.net>
// Copyright (c) 2017 Bastien Orivel <eijebong@bananium.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::error::Error as StdError;
use std::fmt;

/// An error that signifies that a `Jid` cannot be parsed from a string.
#[derive(Debug)]
pub enum JidParseError {
    /// Happens when there is no domain, that is either the string is empty,
    /// starts with a /, or contains the @/ sequence.
    NoDomain,

    /// Happens when there is no resource, that is string contains no /.
    NoResource,

    /// Happens when the node is empty, that is the string starts with a @.
    EmptyNode,

    /// Happens when the resource is empty, that is the string ends with a /.
    EmptyResource,

    /// Happens when the JID is invalid according to stringprep. TODO: make errors
    /// meaningful.
    Stringprep(stringprep::Error),
}

impl From<stringprep::Error> for JidParseError {
    fn from(e: stringprep::Error) -> JidParseError {
        JidParseError::Stringprep(e)
    }
}

impl PartialEq for JidParseError {
    fn eq(&self, other: &JidParseError) -> bool {
        use JidParseError as E;
        match (self, other) {
            (E::NoDomain, E::NoDomain) => true,
            (E::NoResource, E::NoResource) => true,
            (E::EmptyNode, E::EmptyNode) => true,
            (E::EmptyResource, E::EmptyResource) => true,
            (E::Stringprep(_), E::Stringprep(_)) => true, // TODO: fix that upstream.
            _ => false,
        }
    }
}

impl Eq for JidParseError {}

impl StdError for JidParseError {}

impl fmt::Display for JidParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "{}",
            match self {
                JidParseError::NoDomain => "no domain found in this JID",
                JidParseError::NoResource => "no resource found in this full JID",
                JidParseError::EmptyNode => "nodepart empty despite the presence of a @",
                JidParseError::EmptyResource => "resource empty despite the presence of a /",
                JidParseError::Stringprep(_err) => "TODO",
            }
        )
    }
}
