// Copyright (c) 2017, 2018 lumi <lumi@pew.im>
// Copyright (c) 2017, 2018, 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
// Copyright (c) 2017, 2018, 2019 Maxime “pep” Buquet <pep@bouah.net>
// Copyright (c) 2017, 2018 Astro <astro@spaceboyz.net>
// Copyright (c) 2017 Bastien Orivel <eijebong@bananium.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(missing_docs)]

//! Represents XMPP addresses, also known as JabberIDs (JIDs) for the [XMPP](https://xmpp.org/)
//! protocol. A [`Jid`] can have between one and three parts in the form `node@domain/resource`:
//! - the (optional) node part designates a specific account/service on a server, for example
//!   `username@server.com`
//! - the domain part designates a server, for example `irc.jabberfr.org`
//! - the (optional) resource part designates a more specific client, such as a participant in a
//!   groupchat (`jabberfr@chat.jabberfr.org/user`) or a specific client device associated with an
//!   account (`user@example.com/dino`)
//!
//! The [`Jid`] enum can be one of two variants, containing a more specific type:
//! - [`BareJid`] (`Jid::Bare` variant): a JID without a resource
//! - [`FullJid`] (`Jid::Full` variant): a JID with a resource
//!
//! Jids as per the XMPP protocol only ever contain valid UTF-8. However, creating any form of Jid
//! can fail in one of the following cases:
//! - wrong syntax: creating a Jid with an empty (yet declared) node or resource part, such as
//!   `@example.com` or `user@example.com/`
//! - stringprep error: some characters were invalid according to the stringprep algorithm, such as
//!   mixing left-to-write and right-to-left characters

use core::num::NonZeroU16;
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;
use stringprep::resourceprep;

#[cfg(feature = "serde")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

mod error;
pub use crate::error::Error;

mod inner;
use inner::InnerJid;

mod parts;
pub use parts::{DomainPart, NodePart, ResourcePart};

/// An enum representing a Jabber ID. It can be either a `FullJid` or a `BareJid`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Jid {
    /// Contains a [`BareJid`], without a resource part
    Bare(BareJid),

    /// Contains a [`FullJid`], with a resource part
    Full(FullJid),
}

impl FromStr for Jid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Jid, Error> {
        Jid::new(s)
    }
}

impl From<BareJid> for Jid {
    fn from(bare_jid: BareJid) -> Jid {
        Jid::Bare(bare_jid)
    }
}

impl From<FullJid> for Jid {
    fn from(full_jid: FullJid) -> Jid {
        Jid::Full(full_jid)
    }
}

impl fmt::Display for Jid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Jid::Bare(bare) => bare.fmt(fmt),
            Jid::Full(full) => full.fmt(fmt),
        }
    }
}

impl Jid {
    /// Constructs a Jabber ID from a string. This is of the form
    /// `node`@`domain`/`resource`, where node and resource parts are optional.
    /// If you want a non-fallible version, use [`Jid::from_parts`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::Jid;
    /// # use jid::Error;
    ///
    /// # fn main() -> Result<(), Error> {
    /// let jid = Jid::new("node@domain/resource")?;
    ///
    /// assert_eq!(jid.node(), Some("node"));
    /// assert_eq!(jid.domain(), "domain");
    /// assert_eq!(jid.resource(), Some("resource"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(s: &str) -> Result<Jid, Error> {
        let inner = InnerJid::new(s)?;
        if inner.slash.is_some() {
            Ok(Jid::Full(FullJid { inner }))
        } else {
            Ok(Jid::Bare(BareJid { inner }))
        }
    }

    /// Build a [`Jid`] from typed parts. This method cannot fail because it uses parts that have
    /// already been parsed and stringprepped into [`NodePart`], [`DomainPart`], and [`ResourcePart`].
    /// This method allocates and does not consume the typed parts.
    pub fn from_parts(
        node: Option<&NodePart>,
        domain: &DomainPart,
        resource: Option<&ResourcePart>,
    ) -> Jid {
        if let Some(resource) = resource {
            Jid::Full(FullJid::from_parts(node, domain, resource))
        } else {
            Jid::Bare(BareJid::from_parts(node, domain))
        }
    }

    /// The optional node part of the JID.
    pub fn node(&self) -> Option<&str> {
        match self {
            Jid::Bare(BareJid { inner }) | Jid::Full(FullJid { inner }) => inner.node(),
        }
    }

    /// The domain part of the JID.
    pub fn domain(&self) -> &str {
        match self {
            Jid::Bare(BareJid { inner }) | Jid::Full(FullJid { inner }) => inner.domain(),
        }
    }

    /// The optional resource part of the JID.
    pub fn resource(&self) -> Option<&str> {
        match self {
            Jid::Bare(BareJid { inner }) | Jid::Full(FullJid { inner }) => inner.resource(),
        }
    }

    /// Allocate a new [`BareJid`] from this JID, discarding the resource.
    pub fn to_bare(&self) -> BareJid {
        match self {
            Jid::Full(jid) => jid.to_bare(),
            Jid::Bare(jid) => jid.clone(),
        }
    }

    /// Transforms this JID into a [`BareJid`], throwing away the resource.
    pub fn into_bare(self) -> BareJid {
        match self {
            Jid::Full(jid) => jid.into_bare(),
            Jid::Bare(jid) => jid,
        }
    }
}

impl TryFrom<Jid> for FullJid {
    type Error = Error;

    fn try_from(jid: Jid) -> Result<Self, Self::Error> {
        match jid {
            Jid::Full(full) => Ok(full),
            Jid::Bare(_) => Err(Error::ResourceMissingInFullJid),
        }
    }
}

impl PartialEq<Jid> for FullJid {
    fn eq(&self, other: &Jid) -> bool {
        match other {
            Jid::Full(full) => self == full,
            Jid::Bare(_) => false,
        }
    }
}

impl PartialEq<Jid> for BareJid {
    fn eq(&self, other: &Jid) -> bool {
        match other {
            Jid::Full(_) => false,
            Jid::Bare(bare) => self == bare,
        }
    }
}

impl PartialEq<FullJid> for Jid {
    fn eq(&self, other: &FullJid) -> bool {
        match self {
            Jid::Full(full) => full == other,
            Jid::Bare(_) => false,
        }
    }
}

impl PartialEq<BareJid> for Jid {
    fn eq(&self, other: &BareJid) -> bool {
        match self {
            Jid::Full(_) => false,
            Jid::Bare(bare) => bare == other,
        }
    }
}

/// A struct representing a full Jabber ID, with a resource part.
///
/// A full JID is composed of 3 components, of which only the node is optional:
///
/// - the (optional) node part is the part before the (optional) `@`.
/// - the domain part is the mandatory part between the (optional) `@` and before the `/`.
/// - the resource part after the `/`.
///
/// Unlike a [`BareJid`], it always contains a resource, and should only be used when you are
/// certain there is no case where a resource can be missing.  Otherwise, use a [`Jid`] or
/// [`BareJid`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FullJid {
    inner: InnerJid,
}

/// A struct representing a bare Jabber ID, without a resource part.
///
/// A bare JID is composed of 2 components, of which only the node is optional:
/// - the (optional) node part is the part before the (optional) `@`.
/// - the domain part is the mandatory part between the (optional) `@` and before the `/`.
///
/// Unlike a [`FullJid`], it can’t contain a resource, and should only be used when you are certain
/// there is no case where a resource can be set.  Otherwise, use a [`Jid`] or [`FullJid`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BareJid {
    inner: InnerJid,
}

impl fmt::Debug for FullJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "FullJID({})", self)
    }
}

impl fmt::Debug for BareJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "BareJID({})", self)
    }
}

impl fmt::Display for FullJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(&self.inner.normalized)
    }
}

impl fmt::Display for BareJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(&self.inner.normalized)
    }
}

#[cfg(feature = "serde")]
impl Serialize for FullJid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(String::from(self).as_str())
    }
}

#[cfg(feature = "serde")]
impl Serialize for BareJid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(String::from(self).as_str())
    }
}

impl FromStr for FullJid {
    type Err = Error;

    fn from_str(s: &str) -> Result<FullJid, Error> {
        FullJid::new(s)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for FullJid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FullJid::from_str(&s).map_err(de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for BareJid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        BareJid::from_str(&s).map_err(de::Error::custom)
    }
}

impl FullJid {
    /// Constructs a full Jabber ID containing all three components. This is of the form
    /// `node@domain/resource`, where node part is optional.
    /// If you want a non-fallible version, use [`FullJid::from_parts`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    /// # use jid::Error;
    ///
    /// # fn main() -> Result<(), Error> {
    /// let jid = FullJid::new("node@domain/resource")?;
    ///
    /// assert_eq!(jid.node(), Some("node"));
    /// assert_eq!(jid.domain(), "domain");
    /// assert_eq!(jid.resource(), "resource");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(s: &str) -> Result<FullJid, Error> {
        let inner = InnerJid::new(s)?;
        if inner.slash.is_some() {
            Ok(FullJid { inner })
        } else {
            Err(Error::ResourceMissingInFullJid)
        }
    }

    /// Build a [`FullJid`] from typed parts. This method cannot fail because it uses parts that have
    /// already been parsed and stringprepped into [`NodePart`], [`DomainPart`], and [`ResourcePart`].
    /// This method allocates and does not consume the typed parts.
    pub fn from_parts(
        node: Option<&NodePart>,
        domain: &DomainPart,
        resource: &ResourcePart,
    ) -> FullJid {
        let (at, slash, normalized) = if let Some(node) = node {
            // Parts are never empty so len > 0 for NonZeroU16::new is always Some
            (
                NonZeroU16::new(node.0.len() as u16),
                NonZeroU16::new((node.0.len() + 1 + domain.0.len()) as u16),
                format!("{}@{}/{}", node.0, domain.0, resource.0),
            )
        } else {
            (
                None,
                NonZeroU16::new(domain.0.len() as u16),
                format!("{}/{}", domain.0, resource.0),
            )
        };

        let inner = InnerJid {
            normalized,
            at,
            slash,
        };

        FullJid { inner }
    }

    /// The optional node part of the JID.
    pub fn node(&self) -> Option<&str> {
        self.inner.node()
    }

    /// The domain part of the JID.
    pub fn domain(&self) -> &str {
        self.inner.domain()
    }

    /// The optional resource of the Jabber ID.  Since this is a full JID it is always present.
    pub fn resource(&self) -> &str {
        self.inner.resource().unwrap()
    }

    /// Allocate a new [`BareJid`] from this full JID, discarding the resource.
    pub fn to_bare(&self) -> BareJid {
        let slash = self.inner.slash.unwrap().get() as usize;
        let normalized = self.inner.normalized[..slash].to_string();
        let inner = InnerJid {
            normalized,
            at: self.inner.at,
            slash: None,
        };
        BareJid { inner }
    }

    /// Transforms this full JID into a [`BareJid`], discarding the resource.
    pub fn into_bare(mut self) -> BareJid {
        let slash = self.inner.slash.unwrap().get() as usize;
        self.inner.normalized.truncate(slash);
        self.inner.normalized.shrink_to_fit();
        self.inner.slash = None;
        BareJid { inner: self.inner }
    }
}

impl FromStr for BareJid {
    type Err = Error;

    fn from_str(s: &str) -> Result<BareJid, Error> {
        BareJid::new(s)
    }
}

impl BareJid {
    /// Constructs a bare Jabber ID, containing two components. This is of the form
    /// `node`@`domain`, where node part is optional.
    /// If you want a non-fallible version, use [`BareJid::from_parts`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    /// # use jid::Error;
    ///
    /// # fn main() -> Result<(), Error> {
    /// let jid = BareJid::new("node@domain")?;
    ///
    /// assert_eq!(jid.node(), Some("node"));
    /// assert_eq!(jid.domain(), "domain");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(s: &str) -> Result<BareJid, Error> {
        let inner = InnerJid::new(s)?;
        if inner.slash.is_none() {
            Ok(BareJid { inner })
        } else {
            Err(Error::ResourceInBareJid)
        }
    }

    /// Build a [`BareJid`] from typed parts. This method cannot fail because it uses parts that have
    /// already been parsed and stringprepped into [`NodePart`] and [`DomainPart`]. This method allocates
    /// and does not consume the typed parts.
    pub fn from_parts(node: Option<&NodePart>, domain: &DomainPart) -> BareJid {
        let (at, normalized) = if let Some(node) = node {
            // Parts are never empty so len > 0 for NonZeroU16::new is always Some
            (
                NonZeroU16::new(node.0.len() as u16),
                format!("{}@{}", node.0, domain.0),
            )
        } else {
            (None, domain.0.clone())
        };

        let inner = InnerJid {
            normalized,
            at,
            slash: None,
        };

        BareJid { inner }
    }

    /// The optional node part of the JID.
    pub fn node(&self) -> Option<&str> {
        self.inner.node()
    }

    /// The domain part of the JID.
    pub fn domain(&self) -> &str {
        self.inner.domain()
    }

    /// Constructs a [`FullJid`] from the bare JID, by specifying a `resource`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let bare = BareJid::new("node@domain").unwrap();
    /// let full = bare.with_resource("resource").unwrap();
    ///
    /// assert_eq!(full.node(), Some("node"));
    /// assert_eq!(full.domain(), "domain");
    /// assert_eq!(full.resource(), "resource");
    /// ```
    pub fn with_resource(&self, resource: &str) -> Result<FullJid, Error> {
        let resource = resourceprep(resource).map_err(|_| Error::ResourcePrep)?;
        let slash = NonZeroU16::new(self.inner.normalized.len() as u16);
        let normalized = format!("{}/{resource}", self.inner.normalized);
        let inner = InnerJid {
            normalized,
            at: self.inner.at,
            slash,
        };
        Ok(FullJid { inner })
    }
}

#[cfg(feature = "minidom")]
use minidom::{IntoAttributeValue, Node};

#[cfg(feature = "minidom")]
impl IntoAttributeValue for Jid {
    fn into_attribute_value(self) -> Option<String> {
        Some(format!("{}", self))
    }
}

#[cfg(feature = "minidom")]
impl From<Jid> for Node {
    fn from(jid: Jid) -> Node {
        Node::Text(format!("{}", jid))
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for FullJid {
    fn into_attribute_value(self) -> Option<String> {
        Some(format!("{}", self))
    }
}

#[cfg(feature = "minidom")]
impl From<FullJid> for Node {
    fn from(jid: FullJid) -> Node {
        Node::Text(format!("{}", jid))
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for BareJid {
    fn into_attribute_value(self) -> Option<String> {
        Some(format!("{}", self))
    }
}

#[cfg(feature = "minidom")]
impl From<BareJid> for Node {
    fn from(jid: BareJid) -> Node {
        Node::Text(format!("{}", jid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    macro_rules! assert_size (
        ($t:ty, $sz:expr) => (
            assert_eq!(::std::mem::size_of::<$t>(), $sz);
        );
    );

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(BareJid, 16);
        assert_size!(FullJid, 16);
        assert_size!(Jid, 20);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(BareJid, 32);
        assert_size!(FullJid, 32);
        assert_size!(Jid, 40);
    }

    #[test]
    fn can_parse_full_jids() {
        assert_eq!(
            FullJid::from_str("a@b.c/d"),
            Ok(FullJid::new("a@b.c/d").unwrap())
        );
        assert_eq!(
            FullJid::from_str("b.c/d"),
            Ok(FullJid::new("b.c/d").unwrap())
        );

        assert_eq!(
            FullJid::from_str("a@b.c"),
            Err(Error::ResourceMissingInFullJid)
        );
        assert_eq!(
            FullJid::from_str("b.c"),
            Err(Error::ResourceMissingInFullJid)
        );
    }

    #[test]
    fn can_parse_bare_jids() {
        assert_eq!(
            BareJid::from_str("a@b.c"),
            Ok(BareJid::new("a@b.c").unwrap())
        );
        assert_eq!(BareJid::from_str("b.c"), Ok(BareJid::new("b.c").unwrap()));
    }

    #[test]
    fn can_parse_jids() {
        let full = FullJid::from_str("a@b.c/d").unwrap();
        let bare = BareJid::from_str("e@f.g").unwrap();

        assert_eq!(Jid::from_str("a@b.c/d"), Ok(Jid::Full(full)));
        assert_eq!(Jid::from_str("e@f.g"), Ok(Jid::Bare(bare)));
    }

    #[test]
    fn full_to_bare_jid() {
        let bare: BareJid = FullJid::new("a@b.c/d").unwrap().to_bare();
        assert_eq!(bare, BareJid::new("a@b.c").unwrap());
    }

    #[test]
    fn bare_to_full_jid() {
        assert_eq!(
            BareJid::new("a@b.c").unwrap().with_resource("d").unwrap(),
            FullJid::new("a@b.c/d").unwrap()
        );
    }

    #[test]
    fn node_from_jid() {
        assert_eq!(
            Jid::Full(FullJid::new("a@b.c/d").unwrap()).node(),
            Some("a"),
        );
    }

    #[test]
    fn domain_from_jid() {
        assert_eq!(Jid::Bare(BareJid::new("a@b.c").unwrap()).domain(), "b.c");
    }

    #[test]
    fn jid_to_full_bare() {
        let full = FullJid::new("a@b.c/d").unwrap();
        let bare = BareJid::new("a@b.c").unwrap();

        assert_eq!(FullJid::try_from(Jid::Full(full.clone())), Ok(full.clone()));
        assert_eq!(
            FullJid::try_from(Jid::Bare(bare.clone())),
            Err(Error::ResourceMissingInFullJid),
        );
        assert_eq!(Jid::Bare(full.clone().to_bare()), bare.clone());
        assert_eq!(Jid::Bare(bare.clone()), bare);
    }

    #[test]
    fn serialise() {
        assert_eq!(
            format!("{}", FullJid::new("a@b/c").unwrap()),
            String::from("a@b/c")
        );
        assert_eq!(
            format!("{}", BareJid::new("a@b").unwrap()),
            String::from("a@b")
        );
    }

    #[test]
    fn hash() {
        let _map: HashMap<Jid, String> = HashMap::new();
    }

    #[test]
    fn invalid_jids() {
        assert_eq!(BareJid::from_str(""), Err(Error::DomainEmpty));
        assert_eq!(BareJid::from_str("/c"), Err(Error::DomainEmpty));
        assert_eq!(BareJid::from_str("a@/c"), Err(Error::DomainEmpty));
        assert_eq!(BareJid::from_str("@b"), Err(Error::NodeEmpty));
        assert_eq!(BareJid::from_str("b/"), Err(Error::ResourceEmpty));

        assert_eq!(FullJid::from_str(""), Err(Error::DomainEmpty));
        assert_eq!(FullJid::from_str("/c"), Err(Error::DomainEmpty));
        assert_eq!(FullJid::from_str("a@/c"), Err(Error::DomainEmpty));
        assert_eq!(FullJid::from_str("@b"), Err(Error::NodeEmpty));
        assert_eq!(FullJid::from_str("b/"), Err(Error::ResourceEmpty));
        assert_eq!(
            FullJid::from_str("a@b"),
            Err(Error::ResourceMissingInFullJid)
        );
    }

    #[test]
    fn display_jids() {
        assert_eq!(
            format!("{}", FullJid::new("a@b/c").unwrap()),
            String::from("a@b/c")
        );
        assert_eq!(
            format!("{}", BareJid::new("a@b").unwrap()),
            String::from("a@b")
        );
        assert_eq!(
            format!("{}", Jid::Full(FullJid::new("a@b/c").unwrap())),
            String::from("a@b/c")
        );
        assert_eq!(
            format!("{}", Jid::Bare(BareJid::new("a@b").unwrap())),
            String::from("a@b")
        );
    }

    #[cfg(feature = "minidom")]
    #[test]
    fn minidom() {
        let elem: minidom::Element = "<message xmlns='ns1' from='a@b/c'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::Full(FullJid::new("a@b/c").unwrap()));

        let elem: minidom::Element = "<message xmlns='ns1' from='a@b'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::Bare(BareJid::new("a@b").unwrap()));

        let elem: minidom::Element = "<message xmlns='ns1' from='a@b/c'/>".parse().unwrap();
        let to: FullJid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, FullJid::new("a@b/c").unwrap());

        let elem: minidom::Element = "<message xmlns='ns1' from='a@b'/>".parse().unwrap();
        let to: BareJid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, BareJid::new("a@b").unwrap());
    }

    #[cfg(feature = "minidom")]
    #[test]
    fn minidom_into_attr() {
        let full = FullJid::new("a@b/c").unwrap();
        let elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", full.clone())
            .build();
        assert_eq!(elem.attr("from"), Some(format!("{}", full).as_str()));

        let bare = BareJid::new("a@b").unwrap();
        let elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", bare.clone())
            .build();
        assert_eq!(elem.attr("from"), Some(format!("{}", bare).as_str()));

        let jid = Jid::Bare(bare.clone());
        let _elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", jid)
            .build();
        assert_eq!(elem.attr("from"), Some(format!("{}", bare).as_str()));
    }

    #[test]
    fn stringprep() {
        let full = FullJid::from_str("Test@☃.coM/Test™").unwrap();
        let equiv = FullJid::new("test@☃.com/TestTM").unwrap();
        assert_eq!(full, equiv);
    }
}
