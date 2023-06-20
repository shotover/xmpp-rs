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
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Happens when there is no domain, that is either the string is empty,
    /// starts with a /, or contains the @/ sequence.
    NoDomain,

    /// Happens when there is no resource, that is string contains no /.
    NoResource,

    /// Happens when the node is empty, that is the string starts with a @.
    EmptyNode,

    /// Happens when the resource is empty, that is the string ends with a /.
    EmptyResource,

    /// Happens when the localpart is longer than 1023 bytes.
    NodeTooLong,

    /// Happens when the domain is longer than 1023 bytes.
    DomainTooLong,

    /// Happens when the resource is longer than 1023 bytes.
    ResourceTooLong,

    /// Happens when the localpart is invalid according to nodeprep.
    NodePrep,

    /// Happens when the domain is invalid according to nameprep.
    NamePrep,

    /// Happens when the resource is invalid according to resourceprep.
    ResourcePrep,

    /// Happens when parsing a bare JID and there is a resource.
    ResourceInBareJid,
}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(match self {
            Error::NoDomain => "no domain found in this JID",
            Error::NoResource => "no resource found in this full JID",
            Error::EmptyNode => "nodepart empty despite the presence of a @",
            Error::EmptyResource => "resource empty despite the presence of a /",
            Error::NodeTooLong => "localpart longer than 1023 bytes",
            Error::DomainTooLong => "domain longer than 1023 bytes",
            Error::ResourceTooLong => "resource longer than 1023 bytes",
            Error::NodePrep => "localpart doesn’t pass nodeprep validation",
            Error::NamePrep => "domain doesn’t pass nameprep validation",
            Error::ResourcePrep => "resource doesn’t pass resourceprep validation",
            Error::ResourceInBareJid => "resource found while parsing a bare JID",
        })
    }
}
