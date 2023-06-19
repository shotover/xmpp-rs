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

//! Provides a type for Jabber IDs.
//!
//! For usage, check the documentation on the `Jid` struct.

use std::convert::{Into, TryFrom};
use std::error::Error as StdError;
use std::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "icu")]
use icu::{Icu, Strict};

/// An error that signifies that a `Jid` cannot be parsed from a string.
#[derive(Debug, PartialEq, Eq)]
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

    #[cfg(feature = "icu")]
    /// Happens when the JID is invalid according to stringprep. TODO: make errors
    /// meaningful.
    Stringprep(icu::Error),
}

#[cfg(feature = "icu")]
impl From<icu::Error> for JidParseError {
    fn from(e: icu::Error) -> JidParseError {
        JidParseError::Stringprep(e)
    }
}

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
                #[cfg(feature = "icu")]
                JidParseError::Stringprep(_err) => "TODO",
            }
        )
    }
}

/// An enum representing a Jabber ID. It can be either a `FullJid` or a `BareJid`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Jid {
    /// Bare Jid
    Bare(BareJid),

    /// Full Jid
    Full(FullJid),
}

impl FromStr for Jid {
    type Err = JidParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (ns, ds, rs): StringJid = _from_str(s)?;
        Ok(match rs {
            Some(rs) => Jid::Full(FullJid {
                node: ns,
                domain: ds,
                resource: rs,
            }),
            None => Jid::Bare(BareJid {
                node: ns,
                domain: ds,
            }),
        })
    }
}

impl From<Jid> for String {
    fn from(jid: Jid) -> String {
        match jid {
            Jid::Bare(bare) => String::from(bare),
            Jid::Full(full) => String::from(full),
        }
    }
}

impl From<&Jid> for String {
    fn from(jid: &Jid) -> String {
        match jid {
            Jid::Bare(bare) => String::from(bare),
            Jid::Full(full) => String::from(full),
        }
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
        fmt.write_str(String::from(self.clone()).as_ref())
    }
}

impl Jid {
    /// The node part of the Jabber ID, if it exists, else None.
    pub fn node(self) -> Option<String> {
        match self {
            Jid::Bare(BareJid { node, .. }) | Jid::Full(FullJid { node, .. }) => node,
        }
    }

    /// The domain of the Jabber ID.
    pub fn domain(self) -> String {
        match self {
            Jid::Bare(BareJid { domain, .. }) | Jid::Full(FullJid { domain, .. }) => domain,
        }
    }
}

impl From<Jid> for BareJid {
    fn from(jid: Jid) -> BareJid {
        match jid {
            Jid::Full(full) => full.into(),
            Jid::Bare(bare) => bare,
        }
    }
}

impl TryFrom<Jid> for FullJid {
    type Error = JidParseError;

    fn try_from(jid: Jid) -> Result<Self, Self::Error> {
        match jid {
            Jid::Full(full) => Ok(full),
            Jid::Bare(_) => Err(JidParseError::NoResource),
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

/// A struct representing a full Jabber ID.
///
/// A full Jabber ID is composed of 3 components, of which one is optional:
///
///  - A node/name, `node`, which is the optional part before the @.
///  - A domain, `domain`, which is the mandatory part after the @ but before the /.
///  - A resource, `resource`, which is the part after the /.
///
/// Unlike a `BareJid`, it always contains a resource, and should only be used when you are certain
/// there is no case where a resource can be missing.  Otherwise, use a `Jid` enum.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FullJid {
    /// The node part of the Jabber ID, if it exists, else None.
    pub node: Option<String>,
    /// The domain of the Jabber ID.
    pub domain: String,
    /// The resource of the Jabber ID.
    pub resource: String,
}

/// A struct representing a bare Jabber ID.
///
/// A bare Jabber ID is composed of 2 components, of which one is optional:
///
///  - A node/name, `node`, which is the optional part before the @.
///  - A domain, `domain`, which is the mandatory part after the @.
///
/// Unlike a `FullJid`, it can’t contain a resource, and should only be used when you are certain
/// there is no case where a resource can be set.  Otherwise, use a `Jid` enum.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BareJid {
    /// The node part of the Jabber ID, if it exists, else None.
    pub node: Option<String>,
    /// The domain of the Jabber ID.
    pub domain: String,
}

impl From<FullJid> for String {
    fn from(jid: FullJid) -> String {
        String::from(&jid)
    }
}

impl From<&FullJid> for String {
    fn from(jid: &FullJid) -> String {
        let mut string = String::new();
        if let Some(ref node) = jid.node {
            string.push_str(node);
            string.push('@');
        }
        string.push_str(&jid.domain);
        string.push('/');
        string.push_str(&jid.resource);
        string
    }
}

impl From<BareJid> for String {
    fn from(jid: BareJid) -> String {
        String::from(&jid)
    }
}

impl From<&BareJid> for String {
    fn from(jid: &BareJid) -> String {
        let mut string = String::new();
        if let Some(ref node) = jid.node {
            string.push_str(node);
            string.push('@');
        }
        string.push_str(&jid.domain);
        string
    }
}

impl From<FullJid> for BareJid {
    fn from(full: FullJid) -> BareJid {
        BareJid {
            node: full.node,
            domain: full.domain,
        }
    }
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
        fmt.write_str(String::from(self.clone()).as_ref())
    }
}

impl fmt::Display for BareJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(String::from(self.clone()).as_ref())
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

enum ParserState {
    Node,
    Domain,
    Resource,
}

type StringJid = (Option<String>, String, Option<String>);
fn _from_str(s: &str) -> Result<StringJid, JidParseError> {
    // TODO: very naive, may need to do it differently
    let iter = s.chars();
    let mut buf = String::with_capacity(s.len());
    let mut state = ParserState::Node;
    let mut node = None;
    let mut domain = None;
    let mut resource = None;
    for c in iter {
        match state {
            ParserState::Node => {
                match c {
                    '@' => {
                        if buf.is_empty() {
                            return Err(JidParseError::EmptyNode);
                        }
                        state = ParserState::Domain;
                        node = Some(buf.clone()); // TODO: performance tweaks, do not need to copy it
                        buf.clear();
                    }
                    '/' => {
                        if buf.is_empty() {
                            return Err(JidParseError::NoDomain);
                        }
                        state = ParserState::Resource;
                        domain = Some(buf.clone()); // TODO: performance tweaks
                        buf.clear();
                    }
                    c => {
                        buf.push(c);
                    }
                }
            }
            ParserState::Domain => {
                match c {
                    '/' => {
                        if buf.is_empty() {
                            return Err(JidParseError::NoDomain);
                        }
                        state = ParserState::Resource;
                        domain = Some(buf.clone()); // TODO: performance tweaks
                        buf.clear();
                    }
                    c => {
                        buf.push(c);
                    }
                }
            }
            ParserState::Resource => {
                buf.push(c);
            }
        }
    }
    if !buf.is_empty() {
        match state {
            ParserState::Node => {
                domain = Some(buf);
            }
            ParserState::Domain => {
                domain = Some(buf);
            }
            ParserState::Resource => {
                resource = Some(buf);
            }
        }
    } else if let ParserState::Resource = state {
        return Err(JidParseError::EmptyResource);
    }
    let domain = domain.ok_or(JidParseError::NoDomain)?;
    #[cfg(feature = "icu")]
    let (node, domain, resource) = {
        let icu = Icu::new()?;
        let node = if let Some(node) = node {
            Some(icu.nodeprep(&node, Strict::AllowUnassigned)?)
        } else {
            None
        };
        let domain = icu.idna2008.to_unicode(&domain)?;
        let resource = if let Some(resource) = resource {
            Some(icu.resourceprep(&resource, Strict::AllowUnassigned)?)
        } else {
            None
        };

        (node, domain, resource)
    };
    Ok((node, domain, resource))
}

impl FromStr for FullJid {
    type Err = JidParseError;

    fn from_str(s: &str) -> Result<FullJid, JidParseError> {
        let (ns, ds, rs): StringJid = _from_str(s)?;
        Ok(FullJid {
            node: ns,
            domain: ds,
            resource: rs.ok_or(JidParseError::NoResource)?,
        })
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
    /// Constructs a full Jabber ID containing all three components.
    ///
    /// This is of the form `node`@`domain`/`resource`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.node, Some("node".to_owned()));
    /// assert_eq!(jid.domain, "domain".to_owned());
    /// assert_eq!(jid.resource, "resource".to_owned());
    /// ```
    pub fn new<NS, DS, RS>(node: NS, domain: DS, resource: RS) -> FullJid
    where
        NS: Into<String>,
        DS: Into<String>,
        RS: Into<String>,
    {
        FullJid {
            node: Some(node.into()),
            domain: domain.into(),
            resource: resource.into(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the node swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.node, Some("node".to_owned()));
    ///
    /// let new_jid = jid.with_node("new_node");
    ///
    /// assert_eq!(new_jid.node, Some("new_node".to_owned()));
    /// ```
    pub fn with_node<NS>(&self, node: NS) -> FullJid
    where
        NS: Into<String>,
    {
        FullJid {
            node: Some(node.into()),
            domain: self.domain.clone(),
            resource: self.resource.clone(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the domain swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.domain, "domain".to_owned());
    ///
    /// let new_jid = jid.with_domain("new_domain");
    ///
    /// assert_eq!(new_jid.domain, "new_domain");
    /// ```
    pub fn with_domain<DS>(&self, domain: DS) -> FullJid
    where
        DS: Into<String>,
    {
        FullJid {
            node: self.node.clone(),
            domain: domain.into(),
            resource: self.resource.clone(),
        }
    }

    /// Constructs a full Jabber ID from a bare Jabber ID, specifying a `resource`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.resource, "resource".to_owned());
    ///
    /// let new_jid = jid.with_resource("new_resource");
    ///
    /// assert_eq!(new_jid.resource, "new_resource");
    /// ```
    pub fn with_resource<RS>(&self, resource: RS) -> FullJid
    where
        RS: Into<String>,
    {
        FullJid {
            node: self.node.clone(),
            domain: self.domain.clone(),
            resource: resource.into(),
        }
    }
}

impl FromStr for BareJid {
    type Err = JidParseError;

    fn from_str(s: &str) -> Result<BareJid, JidParseError> {
        let (ns, ds, _rs): StringJid = _from_str(s)?;
        Ok(BareJid {
            node: ns,
            domain: ds,
        })
    }
}

impl BareJid {
    /// Constructs a bare Jabber ID, containing two components.
    ///
    /// This is of the form `node`@`domain`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::new("node", "domain");
    ///
    /// assert_eq!(jid.node, Some("node".to_owned()));
    /// assert_eq!(jid.domain, "domain".to_owned());
    /// ```
    pub fn new<NS, DS>(node: NS, domain: DS) -> BareJid
    where
        NS: Into<String>,
        DS: Into<String>,
    {
        BareJid {
            node: Some(node.into()),
            domain: domain.into(),
        }
    }

    /// Constructs a bare Jabber ID containing only a `domain`.
    ///
    /// This is of the form `domain`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::domain("domain");
    ///
    /// assert_eq!(jid.node, None);
    /// assert_eq!(jid.domain, "domain".to_owned());
    /// ```
    pub fn domain<DS>(domain: DS) -> BareJid
    where
        DS: Into<String>,
    {
        BareJid {
            node: None,
            domain: domain.into(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the node swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::domain("domain");
    ///
    /// assert_eq!(jid.node, None);
    ///
    /// let new_jid = jid.with_node("node");
    ///
    /// assert_eq!(new_jid.node, Some("node".to_owned()));
    /// ```
    pub fn with_node<NS>(&self, node: NS) -> BareJid
    where
        NS: Into<String>,
    {
        BareJid {
            node: Some(node.into()),
            domain: self.domain.clone(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the domain swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::domain("domain");
    ///
    /// assert_eq!(jid.domain, "domain");
    ///
    /// let new_jid = jid.with_domain("new_domain");
    ///
    /// assert_eq!(new_jid.domain, "new_domain");
    /// ```
    pub fn with_domain<DS>(&self, domain: DS) -> BareJid
    where
        DS: Into<String>,
    {
        BareJid {
            node: self.node.clone(),
            domain: domain.into(),
        }
    }

    /// Constructs a full Jabber ID from a bare Jabber ID, specifying a `resource`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let bare = BareJid::new("node", "domain");
    /// let full = bare.with_resource("resource");
    ///
    /// assert_eq!(full.node, Some("node".to_owned()));
    /// assert_eq!(full.domain, "domain".to_owned());
    /// assert_eq!(full.resource, "resource".to_owned());
    /// ```
    pub fn with_resource<RS>(self, resource: RS) -> FullJid
    where
        RS: Into<String>,
    {
        FullJid {
            node: self.node,
            domain: self.domain,
            resource: resource.into(),
        }
    }
}

#[cfg(feature = "minidom")]
use minidom::{IntoAttributeValue, Node};

#[cfg(feature = "minidom")]
impl IntoAttributeValue for Jid {
    fn into_attribute_value(self) -> Option<String> {
        Some(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl From<Jid> for Node {
    fn from(jid: Jid) -> Node {
        Node::Text(String::from(jid))
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for FullJid {
    fn into_attribute_value(self) -> Option<String> {
        Some(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl From<FullJid> for Node {
    fn from(jid: FullJid) -> Node {
        Node::Text(String::from(jid))
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for BareJid {
    fn into_attribute_value(self) -> Option<String> {
        Some(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl From<BareJid> for Node {
    fn from(jid: BareJid) -> Node {
        Node::Text(String::from(jid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::str::FromStr;

    macro_rules! assert_size (
        ($t:ty, $sz:expr) => (
            assert_eq!(::std::mem::size_of::<$t>(), $sz);
        );
    );

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(BareJid, 24);
        assert_size!(FullJid, 36);
        assert_size!(Jid, 36);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(BareJid, 48);
        assert_size!(FullJid, 72);
        assert_size!(Jid, 72);
    }

    #[test]
    fn can_parse_full_jids() {
        assert_eq!(
            FullJid::from_str("a@b.c/d"),
            Ok(FullJid::new("a", "b.c", "d"))
        );
        assert_eq!(
            FullJid::from_str("b.c/d"),
            Ok(FullJid {
                node: None,
                domain: "b.c".to_owned(),
                resource: "d".to_owned(),
            })
        );

        assert_eq!(FullJid::from_str("a@b.c"), Err(JidParseError::NoResource));
        assert_eq!(FullJid::from_str("b.c"), Err(JidParseError::NoResource));
    }

    #[test]
    fn can_parse_bare_jids() {
        assert_eq!(BareJid::from_str("a@b.c/d"), Ok(BareJid::new("a", "b.c")));
        assert_eq!(
            BareJid::from_str("b.c/d"),
            Ok(BareJid {
                node: None,
                domain: "b.c".to_owned(),
            })
        );

        assert_eq!(BareJid::from_str("a@b.c"), Ok(BareJid::new("a", "b.c")));
        assert_eq!(
            BareJid::from_str("b.c"),
            Ok(BareJid {
                node: None,
                domain: "b.c".to_owned(),
            })
        );
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
        let bare: BareJid = FullJid::new("a", "b.c", "d").into();
        assert_eq!(bare, BareJid::new("a", "b.c"));
    }

    #[test]
    fn bare_to_full_jid() {
        assert_eq!(
            BareJid::new("a", "b.c").with_resource("d"),
            FullJid::new("a", "b.c", "d")
        );
    }

    #[test]
    fn node_from_jid() {
        assert_eq!(
            Jid::Full(FullJid::new("a", "b.c", "d")).node(),
            Some(String::from("a")),
        );
    }

    #[test]
    fn domain_from_jid() {
        assert_eq!(
            Jid::Bare(BareJid::new("a", "b.c")).domain(),
            String::from("b.c"),
        );
    }

    #[test]
    fn jid_to_full_bare() {
        let full = FullJid::new("a", "b.c", "d");
        let bare = BareJid::new("a", "b.c");

        assert_eq!(FullJid::try_from(Jid::Full(full.clone())), Ok(full.clone()),);
        assert_eq!(
            FullJid::try_from(Jid::Bare(bare.clone())),
            Err(JidParseError::NoResource),
        );
        assert_eq!(BareJid::from(Jid::Full(full.clone())), bare.clone(),);
        assert_eq!(BareJid::from(Jid::Bare(bare.clone())), bare,);
    }

    #[test]
    fn serialise() {
        assert_eq!(
            String::from(FullJid::new("a", "b", "c")),
            String::from("a@b/c")
        );
        assert_eq!(String::from(BareJid::new("a", "b")), String::from("a@b"));
    }

    #[test]
    fn hash() {
        let _map: HashMap<Jid, String> = HashMap::new();
    }

    #[test]
    fn invalid_jids() {
        assert_eq!(BareJid::from_str(""), Err(JidParseError::NoDomain));
        assert_eq!(BareJid::from_str("/c"), Err(JidParseError::NoDomain));
        assert_eq!(BareJid::from_str("a@/c"), Err(JidParseError::NoDomain));
        assert_eq!(BareJid::from_str("@b"), Err(JidParseError::EmptyNode));
        assert_eq!(BareJid::from_str("b/"), Err(JidParseError::EmptyResource));

        assert_eq!(FullJid::from_str(""), Err(JidParseError::NoDomain));
        assert_eq!(FullJid::from_str("/c"), Err(JidParseError::NoDomain));
        assert_eq!(FullJid::from_str("a@/c"), Err(JidParseError::NoDomain));
        assert_eq!(FullJid::from_str("@b"), Err(JidParseError::EmptyNode));
        assert_eq!(FullJid::from_str("b/"), Err(JidParseError::EmptyResource));
        assert_eq!(FullJid::from_str("a@b"), Err(JidParseError::NoResource));
    }

    #[test]
    fn display_jids() {
        assert_eq!(
            format!("{}", FullJid::new("a", "b", "c")),
            String::from("a@b/c")
        );
        assert_eq!(format!("{}", BareJid::new("a", "b")), String::from("a@b"));
        assert_eq!(
            format!("{}", Jid::Full(FullJid::new("a", "b", "c"))),
            String::from("a@b/c")
        );
        assert_eq!(
            format!("{}", Jid::Bare(BareJid::new("a", "b"))),
            String::from("a@b")
        );
    }

    #[cfg(feature = "minidom")]
    #[test]
    fn minidom() {
        let elem: minidom::Element = "<message xmlns='ns1' from='a@b/c'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::Full(FullJid::new("a", "b", "c")));

        let elem: minidom::Element = "<message xmlns='ns1' from='a@b'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::Bare(BareJid::new("a", "b")));

        let elem: minidom::Element = "<message xmlns='ns1' from='a@b/c'/>".parse().unwrap();
        let to: FullJid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, FullJid::new("a", "b", "c"));

        let elem: minidom::Element = "<message xmlns='ns1' from='a@b'/>".parse().unwrap();
        let to: BareJid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, BareJid::new("a", "b"));
    }

    #[cfg(feature = "minidom")]
    #[test]
    fn minidom_into_attr() {
        let full = FullJid::new("a", "b", "c");
        let elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", full.clone())
            .build();
        assert_eq!(elem.attr("from"), Some(String::from(full).as_ref()));

        let bare = BareJid::new("a", "b");
        let elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", bare.clone())
            .build();
        assert_eq!(elem.attr("from"), Some(String::from(bare.clone()).as_ref()));

        let jid = Jid::Bare(bare.clone());
        let _elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", jid)
            .build();
        assert_eq!(elem.attr("from"), Some(String::from(bare).as_ref()));
    }

    #[cfg(feature = "icu")]
    #[test]
    fn icu_jid() {
        let full = FullJid::from_str("Test@☃.coM/Test™").unwrap();
        let equiv = FullJid::new("test", "☃.com", "TestTM");
        assert_eq!(full, equiv);
    }
}
