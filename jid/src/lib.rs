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
use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;

use memchr::memchr;

use stringprep::{nameprep, nodeprep, resourceprep};

#[cfg(feature = "serde")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "quote")]
use proc_macro2::TokenStream;
#[cfg(feature = "quote")]
use quote::{quote, ToTokens};

#[cfg(feature = "minidom")]
use minidom::{IntoAttributeValue, Node};

mod error;
pub use crate::error::Error;

mod parts;
pub use parts::{DomainPart, DomainRef, NodePart, NodeRef, ResourcePart, ResourceRef};

fn length_check(len: usize, error_empty: Error, error_too_long: Error) -> Result<(), Error> {
    if len == 0 {
        Err(error_empty)
    } else if len > 1023 {
        Err(error_too_long)
    } else {
        Ok(())
    }
}

/// A struct representing a Jabber ID (JID).
///
/// This JID can either be "bare" (without a `/resource` suffix) or full (with
/// a resource suffix).
///
/// In many APIs, it is appropriate to use the more specific types
/// ([`BareJid`] or [`FullJid`]) instead, as these two JID types are generally
/// used in different contexts within XMPP.
///
/// This dynamic type on the other hand can be used in contexts where it is
/// not known, at compile-time, whether a JID is full or bare.
#[derive(Debug, Clone, Eq)]
pub struct Jid {
    normalized: String,
    at: Option<NonZeroU16>,
    slash: Option<NonZeroU16>,
}

impl PartialEq for Jid {
    fn eq(&self, other: &Jid) -> bool {
        self.normalized == other.normalized
    }
}

impl PartialOrd for Jid {
    fn partial_cmp(&self, other: &Jid) -> Option<Ordering> {
        self.normalized.partial_cmp(&other.normalized)
    }
}

impl Ord for Jid {
    fn cmp(&self, other: &Jid) -> Ordering {
        self.normalized.cmp(&other.normalized)
    }
}

impl Hash for Jid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.normalized.hash(state)
    }
}

impl FromStr for Jid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl From<BareJid> for Jid {
    fn from(other: BareJid) -> Self {
        other.inner
    }
}

impl From<FullJid> for Jid {
    fn from(other: FullJid) -> Self {
        other.inner
    }
}

impl fmt::Display for Jid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.normalized)
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
    /// assert_eq!(jid.node().map(|x| x.as_str()), Some("node"));
    /// assert_eq!(jid.domain().as_str(), "domain");
    /// assert_eq!(jid.resource().map(|x| x.as_str()), Some("resource"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(unnormalized: &str) -> Result<Jid, Error> {
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
                match (node, domain, resource) {
                    (Cow::Borrowed(_), Cow::Borrowed(_), Cow::Borrowed(_)) => {
                        unnormalized.to_string()
                    }
                    (node, domain, resource) => format!("{node}@{domain}/{resource}"),
                }
            }
            (Some(at), None) => {
                let node = nodeprep(&unnormalized[..at]).map_err(|_| Error::NodePrep)?;
                length_check(node.len(), Error::NodeEmpty, Error::NodeTooLong)?;

                let domain = nameprep(&unnormalized[at + 1..]).map_err(|_| Error::NamePrep)?;
                length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;

                orig_at = Some(node.len());
                match (node, domain) {
                    (Cow::Borrowed(_), Cow::Borrowed(_)) => unnormalized.to_string(),
                    (node, domain) => format!("{node}@{domain}"),
                }
            }
            (None, Some(slash)) => {
                let domain = nameprep(&unnormalized[..slash]).map_err(|_| Error::NamePrep)?;
                length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;

                let resource =
                    resourceprep(&unnormalized[slash + 1..]).map_err(|_| Error::ResourcePrep)?;
                length_check(resource.len(), Error::ResourceEmpty, Error::ResourceTooLong)?;

                orig_slash = Some(domain.len());
                match (domain, resource) {
                    (Cow::Borrowed(_), Cow::Borrowed(_)) => unnormalized.to_string(),
                    (domain, resource) => format!("{domain}/{resource}"),
                }
            }
            (None, None) => {
                let domain = nameprep(unnormalized).map_err(|_| Error::NamePrep)?;
                length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;

                domain.into_owned()
            }
        };

        Ok(Self {
            normalized,
            at: orig_at.and_then(|x| NonZeroU16::new(x as u16)),
            slash: orig_slash.and_then(|x| NonZeroU16::new(x as u16)),
        })
    }

    /// Returns the inner String of this JID.
    pub fn into_inner(self) -> String {
        self.normalized
    }

    /// Build a [`Jid`] from typed parts. This method cannot fail because it uses parts that have
    /// already been parsed and stringprepped into [`NodePart`], [`DomainPart`], and [`ResourcePart`].
    ///
    /// This method allocates and does not consume the typed parts. To avoid
    /// allocation if both `node` and `resource` are known to be `None` and
    /// `domain` is owned, you can use `domain.into()`.
    pub fn from_parts(
        node: Option<&NodeRef>,
        domain: &DomainRef,
        resource: Option<&ResourceRef>,
    ) -> Self {
        match resource {
            Some(resource) => FullJid::from_parts(node, domain, resource).into(),
            None => BareJid::from_parts(node, domain).into(),
        }
    }

    /// The optional node part of the JID as reference.
    pub fn node(&self) -> Option<&NodeRef> {
        self.at.map(|at| {
            let at = u16::from(at) as usize;
            NodeRef::from_str_unchecked(&self.normalized[..at])
        })
    }

    /// The domain part of the JID as reference
    pub fn domain(&self) -> &DomainRef {
        match (self.at, self.slash) {
            (Some(at), Some(slash)) => {
                let at = u16::from(at) as usize;
                let slash = u16::from(slash) as usize;
                DomainRef::from_str_unchecked(&self.normalized[at + 1..slash])
            }
            (Some(at), None) => {
                let at = u16::from(at) as usize;
                DomainRef::from_str_unchecked(&self.normalized[at + 1..])
            }
            (None, Some(slash)) => {
                let slash = u16::from(slash) as usize;
                DomainRef::from_str_unchecked(&self.normalized[..slash])
            }
            (None, None) => DomainRef::from_str_unchecked(&self.normalized),
        }
    }

    /// The optional resource of the Jabber ID. It is guaranteed to be present when the JID is
    /// a Full variant, which you can check with [`Jid::is_full`].
    pub fn resource(&self) -> Option<&ResourceRef> {
        self.slash.map(|slash| {
            let slash = u16::from(slash) as usize;
            ResourceRef::from_str_unchecked(&self.normalized[slash + 1..])
        })
    }

    /// Allocate a new [`BareJid`] from this JID, discarding the resource.
    pub fn to_bare(&self) -> BareJid {
        BareJid::from_parts(self.node(), self.domain())
    }

    /// Transforms this JID into a [`BareJid`], throwing away the resource.
    ///
    /// ```
    /// # use jid::{BareJid, Jid};
    /// let jid: Jid = "foo@bar/baz".parse().unwrap();
    /// let bare = jid.into_bare();
    /// assert_eq!(bare.to_string(), "foo@bar");
    /// ```
    pub fn into_bare(mut self) -> BareJid {
        if let Some(slash) = self.slash {
            // truncate the string
            self.normalized.truncate(slash.get() as usize);
            self.slash = None;
        }
        BareJid { inner: self }
    }

    /// Checks if the JID is a full JID.
    pub fn is_full(&self) -> bool {
        self.slash.is_some()
    }

    /// Checks if the JID is a bare JID.
    pub fn is_bare(&self) -> bool {
        self.slash.is_none()
    }

    /// Return a reference to the canonical string representation of the JID.
    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    /// Try to convert this Jid to a [`FullJid`] if it contains a resource
    /// and return a [`BareJid`] otherwise.
    ///
    /// This is useful for match blocks:
    ///
    /// ```
    /// # use jid::Jid;
    /// let jid: Jid = "foo@bar".parse().unwrap();
    /// match jid.try_into_full() {
    ///     Ok(full) => println!("it is full: {:?}", full),
    ///     Err(bare) => println!("it is bare: {:?}", bare),
    /// }
    /// ```
    pub fn try_into_full(self) -> Result<FullJid, BareJid> {
        if self.slash.is_some() {
            Ok(FullJid { inner: self })
        } else {
            Err(BareJid { inner: self })
        }
    }

    /// Try to convert this Jid reference to a [`&FullJid`][`FullJid`] if it
    /// contains a resource and return a [`&BareJid`][`BareJid`] otherwise.
    ///
    /// This is useful for match blocks:
    ///
    /// ```
    /// # use jid::Jid;
    /// let jid: Jid = "foo@bar".parse().unwrap();
    /// match jid.try_as_full() {
    ///     Ok(full) => println!("it is full: {:?}", full),
    ///     Err(bare) => println!("it is bare: {:?}", bare),
    /// }
    /// ```
    pub fn try_as_full(&self) -> Result<&FullJid, &BareJid> {
        if self.slash.is_some() {
            Ok(unsafe {
                // SAFETY: FullJid is #[repr(transparent)] of Jid
                // SOUNDNESS: we asserted that self.slash is set above
                std::mem::transmute::<&Jid, &FullJid>(self)
            })
        } else {
            Err(unsafe {
                // SAFETY: BareJid is #[repr(transparent)] of Jid
                // SOUNDNESS: we asserted that self.slash is unset above
                std::mem::transmute::<&Jid, &BareJid>(self)
            })
        }
    }

    /// Try to convert this mutable Jid reference to a
    /// [`&mut FullJid`][`FullJid`] if it contains a resource and return a
    /// [`&mut BareJid`][`BareJid`] otherwise.
    pub fn try_as_full_mut(&mut self) -> Result<&mut FullJid, &mut BareJid> {
        if self.slash.is_some() {
            Ok(unsafe {
                // SAFETY: FullJid is #[repr(transparent)] of Jid
                // SOUNDNESS: we asserted that self.slash is set above
                std::mem::transmute::<&mut Jid, &mut FullJid>(self)
            })
        } else {
            Err(unsafe {
                // SAFETY: BareJid is #[repr(transparent)] of Jid
                // SOUNDNESS: we asserted that self.slash is unset above
                std::mem::transmute::<&mut Jid, &mut BareJid>(self)
            })
        }
    }

    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[deprecated(
        since = "0.11.0",
        note = "use Jid::from (for construction of Jid values) or Jid::try_into_full/Jid::try_as_full (for match blocks) instead"
    )]
    pub fn Bare(other: BareJid) -> Self {
        Self::from(other)
    }

    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[deprecated(
        since = "0.11.0",
        note = "use Jid::from (for construction of Jid values) or Jid::try_into_full/Jid::try_as_full (for match blocks) instead"
    )]
    pub fn Full(other: BareJid) -> Self {
        Self::from(other)
    }
}

impl TryFrom<Jid> for FullJid {
    type Error = Error;

    fn try_from(inner: Jid) -> Result<Self, Self::Error> {
        if inner.slash.is_none() {
            return Err(Error::ResourceMissingInFullJid);
        }
        Ok(Self { inner })
    }
}

impl TryFrom<Jid> for BareJid {
    type Error = Error;

    fn try_from(inner: Jid) -> Result<Self, Self::Error> {
        if inner.slash.is_some() {
            return Err(Error::ResourceInBareJid);
        }
        Ok(Self { inner })
    }
}

impl PartialEq<Jid> for FullJid {
    fn eq(&self, other: &Jid) -> bool {
        &self.inner == other
    }
}

impl PartialEq<Jid> for BareJid {
    fn eq(&self, other: &Jid) -> bool {
        &self.inner == other
    }
}

impl PartialEq<FullJid> for Jid {
    fn eq(&self, other: &FullJid) -> bool {
        self == &other.inner
    }
}

impl PartialEq<BareJid> for Jid {
    fn eq(&self, other: &BareJid) -> bool {
        self == &other.inner
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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)] // WARNING: Jid::try_as_* relies on this for safety!
pub struct FullJid {
    inner: Jid,
}

/// A struct representing a bare Jabber ID, without a resource part.
///
/// A bare JID is composed of 2 components, of which only the node is optional:
/// - the (optional) node part is the part before the (optional) `@`.
/// - the domain part is the mandatory part between the (optional) `@` and before the `/`.
///
/// Unlike a [`FullJid`], it can’t contain a resource, and should only be used when you are certain
/// there is no case where a resource can be set.  Otherwise, use a [`Jid`] or [`FullJid`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)] // WARNING: Jid::try_as_* relies on this for safety!
pub struct BareJid {
    inner: Jid,
}

impl Deref for FullJid {
    type Target = Jid;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Deref for BareJid {
    type Target = Jid;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Borrow<Jid> for FullJid {
    fn borrow(&self) -> &Jid {
        &self.inner
    }
}

impl Borrow<Jid> for BareJid {
    fn borrow(&self) -> &Jid {
        &self.inner
    }
}

impl fmt::Debug for FullJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("FullJid").field(&self.inner).finish()
    }
}

impl fmt::Debug for BareJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("BareJid").field(&self.inner).finish()
    }
}

impl fmt::Display for FullJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, fmt)
    }
}

impl fmt::Display for BareJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, fmt)
    }
}

#[cfg(feature = "serde")]
impl Serialize for Jid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.normalized)
    }
}

#[cfg(feature = "serde")]
impl Serialize for FullJid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl Serialize for BareJid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl FromStr for FullJid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Jid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Jid::new(&s).map_err(de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for FullJid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let jid = Jid::deserialize(deserializer)?;
        jid.try_into().map_err(de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for BareJid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let jid = Jid::deserialize(deserializer)?;
        jid.try_into().map_err(de::Error::custom)
    }
}

#[cfg(feature = "quote")]
impl ToTokens for Jid {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let s = &self.normalized;
        tokens.extend(quote! {
            ::jid::Jid::new(#s).unwrap()
        });
    }
}

#[cfg(feature = "quote")]
impl ToTokens for FullJid {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let s = &self.inner.normalized;
        tokens.extend(quote! {
            ::jid::FullJid::new(#s).unwrap()
        });
    }
}

#[cfg(feature = "quote")]
impl ToTokens for BareJid {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let s = &self.inner.normalized;
        tokens.extend(quote! {
            ::jid::BareJid::new(#s).unwrap()
        });
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
    /// assert_eq!(jid.node().map(|x| x.as_str()), Some("node"));
    /// assert_eq!(jid.domain().as_str(), "domain");
    /// assert_eq!(jid.resource().as_str(), "resource");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(unnormalized: &str) -> Result<Self, Error> {
        Jid::new(unnormalized)?.try_into()
    }

    /// Build a [`FullJid`] from typed parts. This method cannot fail because it uses parts that have
    /// already been parsed and stringprepped into [`NodePart`], [`DomainPart`], and [`ResourcePart`].
    /// This method allocates and does not consume the typed parts.
    pub fn from_parts(
        node: Option<&NodeRef>,
        domain: &DomainRef,
        resource: &ResourceRef,
    ) -> FullJid {
        let (at, slash, normalized) = if let Some(node) = node {
            // Parts are never empty so len > 0 for NonZeroU16::new is always Some
            (
                NonZeroU16::new(node.len() as u16),
                NonZeroU16::new((node.len() + 1 + domain.len()) as u16),
                format!(
                    "{}@{}/{}",
                    node.as_str(),
                    domain.as_str(),
                    resource.as_str()
                ),
            )
        } else {
            (
                None,
                NonZeroU16::new(domain.len() as u16),
                format!("{}/{}", domain.as_str(), resource.as_str()),
            )
        };

        let inner = Jid {
            normalized,
            at,
            slash,
        };

        Self { inner }
    }

    /// The optional resource of the Jabber ID.  Since this is a full JID it is always present.
    pub fn resource(&self) -> &ResourceRef {
        self.inner.resource().unwrap()
    }
}

impl FromStr for BareJid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
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
    /// assert_eq!(jid.node().map(|x| x.as_str()), Some("node"));
    /// assert_eq!(jid.domain().as_str(), "domain");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(unnormalized: &str) -> Result<Self, Error> {
        Jid::new(unnormalized)?.try_into()
    }

    /// Build a [`BareJid`] from typed parts. This method cannot fail because it uses parts that have
    /// already been parsed and stringprepped into [`NodePart`] and [`DomainPart`].
    ///
    /// This method allocates and does not consume the typed parts. To avoid
    /// allocation if `node` is known to be `None` and `domain` is owned, you
    /// can use `domain.into()`.
    pub fn from_parts(node: Option<&NodeRef>, domain: &DomainRef) -> Self {
        let (at, normalized) = if let Some(node) = node {
            // Parts are never empty so len > 0 for NonZeroU16::new is always Some
            (
                NonZeroU16::new(node.len() as u16),
                format!("{}@{}", node.as_str(), domain.as_str()),
            )
        } else {
            (None, domain.to_string())
        };

        let inner = Jid {
            normalized,
            at,
            slash: None,
        };

        Self { inner }
    }

    /// Constructs a [`BareJid`] from the bare JID, by specifying a [`ResourcePart`].
    /// If you'd like to specify a stringy resource, use [`BareJid::with_resource_str`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::{BareJid, ResourcePart};
    ///
    /// let resource = ResourcePart::new("resource").unwrap();
    /// let bare = BareJid::new("node@domain").unwrap();
    /// let full = bare.with_resource(&resource);
    ///
    /// assert_eq!(full.node().map(|x| x.as_str()), Some("node"));
    /// assert_eq!(full.domain().as_str(), "domain");
    /// assert_eq!(full.resource().as_str(), "resource");
    /// ```
    pub fn with_resource(&self, resource: &ResourceRef) -> FullJid {
        let slash = NonZeroU16::new(self.inner.normalized.len() as u16);
        let normalized = format!("{}/{resource}", self.inner.normalized);
        let inner = Jid {
            normalized,
            at: self.inner.at,
            slash,
        };

        FullJid { inner }
    }

    /// Constructs a [`FullJid`] from the bare JID, by specifying a stringy `resource`.
    /// If your resource has already been parsed into a [`ResourcePart`], use [`BareJid::with_resource`].
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let bare = BareJid::new("node@domain").unwrap();
    /// let full = bare.with_resource_str("resource").unwrap();
    ///
    /// assert_eq!(full.node().map(|x| x.as_str()), Some("node"));
    /// assert_eq!(full.domain().as_str(), "domain");
    /// assert_eq!(full.resource().as_str(), "resource");
    /// ```
    pub fn with_resource_str(&self, resource: &str) -> Result<FullJid, Error> {
        let resource = ResourcePart::new(resource)?;
        Ok(self.with_resource(&resource))
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for Jid {
    fn into_attribute_value(self) -> Option<String> {
        Some(self.to_string())
    }
}

#[cfg(feature = "minidom")]
impl From<Jid> for Node {
    fn from(jid: Jid) -> Self {
        Node::Text(jid.to_string())
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for FullJid {
    fn into_attribute_value(self) -> Option<String> {
        self.inner.into_attribute_value()
    }
}

#[cfg(feature = "minidom")]
impl From<FullJid> for Node {
    fn from(jid: FullJid) -> Self {
        jid.inner.into()
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for BareJid {
    fn into_attribute_value(self) -> Option<String> {
        self.inner.into_attribute_value()
    }
}

#[cfg(feature = "minidom")]
impl From<BareJid> for Node {
    fn from(other: BareJid) -> Self {
        other.inner.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::{HashMap, HashSet};

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
        assert_size!(Jid, 16);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(BareJid, 32);
        assert_size!(FullJid, 32);
        assert_size!(Jid, 32);
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

        assert_eq!(Jid::from_str("a@b.c/d").unwrap(), full);
        assert_eq!(Jid::from_str("e@f.g").unwrap(), bare);
    }

    #[test]
    fn full_to_bare_jid() {
        let bare: BareJid = FullJid::new("a@b.c/d").unwrap().to_bare();
        assert_eq!(bare, BareJid::new("a@b.c").unwrap());
    }

    #[test]
    fn bare_to_full_jid_str() {
        assert_eq!(
            BareJid::new("a@b.c")
                .unwrap()
                .with_resource_str("d")
                .unwrap(),
            FullJid::new("a@b.c/d").unwrap()
        );
    }

    #[test]
    fn bare_to_full_jid() {
        assert_eq!(
            BareJid::new("a@b.c")
                .unwrap()
                .with_resource(&ResourcePart::new("d").unwrap()),
            FullJid::new("a@b.c/d").unwrap()
        )
    }

    #[test]
    fn node_from_jid() {
        let jid = Jid::new("a@b.c/d").unwrap();

        assert_eq!(jid.node().map(|x| x.as_str()), Some("a"),);
    }

    #[test]
    fn domain_from_jid() {
        let jid = Jid::new("a@b.c").unwrap();

        assert_eq!(jid.domain().as_str(), "b.c");
    }

    #[test]
    fn resource_from_jid() {
        let jid = Jid::new("a@b.c/d").unwrap();

        assert_eq!(jid.resource().map(|x| x.as_str()), Some("d"),);
    }

    #[test]
    fn jid_to_full_bare() {
        let full = FullJid::new("a@b.c/d").unwrap();
        let bare = BareJid::new("a@b.c").unwrap();

        assert_eq!(FullJid::try_from(Jid::from(full.clone())), Ok(full.clone()));
        assert_eq!(
            FullJid::try_from(Jid::from(bare.clone())),
            Err(Error::ResourceMissingInFullJid),
        );
        assert_eq!(Jid::from(full.clone().to_bare()), bare.clone());
        assert_eq!(Jid::from(bare.clone()), bare);
    }

    #[test]
    fn serialise() {
        assert_eq!(FullJid::new("a@b/c").unwrap().to_string(), "a@b/c");
        assert_eq!(BareJid::new("a@b").unwrap().to_string(), "a@b");
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
        assert_eq!(FullJid::new("a@b/c").unwrap().to_string(), "a@b/c");
        assert_eq!(BareJid::new("a@b").unwrap().to_string(), "a@b");
        assert_eq!(
            Jid::from(FullJid::new("a@b/c").unwrap()).to_string(),
            "a@b/c"
        );
        assert_eq!(Jid::from(BareJid::new("a@b").unwrap()).to_string(), "a@b");
    }

    #[cfg(feature = "minidom")]
    #[test]
    fn minidom() {
        let elem: minidom::Element = "<message xmlns='ns1' from='a@b/c'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::from(FullJid::new("a@b/c").unwrap()));

        let elem: minidom::Element = "<message xmlns='ns1' from='a@b'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::from(BareJid::new("a@b").unwrap()));

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
        assert_eq!(elem.attr("from"), Some(full.to_string().as_str()));

        let bare = BareJid::new("a@b").unwrap();
        let elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", bare.clone())
            .build();
        assert_eq!(elem.attr("from"), Some(bare.to_string().as_str()));

        let jid = Jid::from(bare.clone());
        let _elem = minidom::Element::builder("message", "jabber:client")
            .attr("from", jid)
            .build();
        assert_eq!(elem.attr("from"), Some(bare.to_string().as_str()));
    }

    #[test]
    fn stringprep() {
        let full = FullJid::from_str("Test@☃.coM/Test™").unwrap();
        let equiv = FullJid::new("test@☃.com/TestTM").unwrap();
        assert_eq!(full, equiv);
    }

    #[test]
    fn invalid_stringprep() {
        FullJid::from_str("a@b/🎉").unwrap_err();
    }

    #[test]
    fn jid_from_parts() {
        let node = NodePart::new("node").unwrap();
        let domain = DomainPart::new("domain").unwrap();
        let resource = ResourcePart::new("resource").unwrap();

        let jid = Jid::from_parts(Some(&node), &domain, Some(&resource));
        assert_eq!(jid, Jid::new("node@domain/resource").unwrap());

        let barejid = BareJid::from_parts(Some(&node), &domain);
        assert_eq!(barejid, BareJid::new("node@domain").unwrap());

        let fulljid = FullJid::from_parts(Some(&node), &domain, &resource);
        assert_eq!(fulljid, FullJid::new("node@domain/resource").unwrap());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn jid_ser_de() {
        let jid: Jid = Jid::new("node@domain").unwrap();
        serde_test::assert_tokens(&jid, &[serde_test::Token::Str("node@domain")]);

        let jid: Jid = Jid::new("node@domain/resource").unwrap();
        serde_test::assert_tokens(&jid, &[serde_test::Token::Str("node@domain/resource")]);

        let jid: BareJid = BareJid::new("node@domain").unwrap();
        serde_test::assert_tokens(&jid, &[serde_test::Token::Str("node@domain")]);

        let jid: FullJid = FullJid::new("node@domain/resource").unwrap();
        serde_test::assert_tokens(&jid, &[serde_test::Token::Str("node@domain/resource")]);
    }

    #[test]
    fn jid_into_parts_and_from_parts() {
        let node = NodePart::new("node").unwrap();
        let domain = DomainPart::new("domain").unwrap();

        let jid1 = domain.with_node(&node);
        let jid2 = node.with_domain(&domain);
        let jid3 = BareJid::new("node@domain").unwrap();
        assert_eq!(jid1, jid2);
        assert_eq!(jid2, jid3);
    }

    #[test]
    fn jid_match_replacement_try_as() {
        let jid1 = Jid::new("foo@bar").unwrap();
        let jid2 = Jid::new("foo@bar/baz").unwrap();

        match jid1.try_as_full() {
            Err(_) => (),
            other => panic!("unexpected result: {:?}", other),
        };

        match jid2.try_as_full() {
            Ok(_) => (),
            other => panic!("unexpected result: {:?}", other),
        };
    }

    #[test]
    fn jid_match_replacement_try_as_mut() {
        let mut jid1 = Jid::new("foo@bar").unwrap();
        let mut jid2 = Jid::new("foo@bar/baz").unwrap();

        match jid1.try_as_full_mut() {
            Err(_) => (),
            other => panic!("unexpected result: {:?}", other),
        };

        match jid2.try_as_full_mut() {
            Ok(_) => (),
            other => panic!("unexpected result: {:?}", other),
        };
    }

    #[test]
    fn jid_match_replacement_try_into() {
        let jid1 = Jid::new("foo@bar").unwrap();
        let jid2 = Jid::new("foo@bar/baz").unwrap();

        match jid1.try_as_full() {
            Err(_) => (),
            other => panic!("unexpected result: {:?}", other),
        };

        match jid2.try_as_full() {
            Ok(_) => (),
            other => panic!("unexpected result: {:?}", other),
        };
    }

    #[test]
    fn lookup_jid_by_full_jid() {
        let mut map: HashSet<Jid> = HashSet::new();
        let jid1 = Jid::new("foo@bar").unwrap();
        let jid2 = Jid::new("foo@bar/baz").unwrap();
        let jid3 = FullJid::new("foo@bar/baz").unwrap();

        map.insert(jid1);
        assert!(!map.contains(&jid2));
        assert!(!map.contains(&jid3));
        map.insert(jid2);
        assert!(map.contains(&jid3));
    }

    #[test]
    fn lookup_full_jid_by_jid() {
        let mut map: HashSet<FullJid> = HashSet::new();
        let jid1 = FullJid::new("foo@bar/baz").unwrap();
        let jid2 = FullJid::new("foo@bar/fnord").unwrap();
        let jid3 = Jid::new("foo@bar/fnord").unwrap();

        map.insert(jid1);
        assert!(!map.contains(&jid2));
        assert!(!map.contains(&jid3));
        map.insert(jid2);
        assert!(map.contains(&jid3));
    }

    #[test]
    fn lookup_bare_jid_by_jid() {
        let mut map: HashSet<BareJid> = HashSet::new();
        let jid1 = BareJid::new("foo@bar").unwrap();
        let jid2 = BareJid::new("foo@baz").unwrap();
        let jid3 = Jid::new("foo@baz").unwrap();

        map.insert(jid1);
        assert!(!map.contains(&jid2));
        assert!(!map.contains(&jid3));
        map.insert(jid2);
        assert!(map.contains(&jid3));
    }
}
