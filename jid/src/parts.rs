use alloc::borrow::{Cow, ToOwned};
use alloc::string::{String, ToString};
use core::borrow::Borrow;
use core::fmt;
use core::mem;
use core::ops::Deref;
use core::str::FromStr;

use stringprep::{nameprep, nodeprep, resourceprep};

use crate::{BareJid, Error, Jid};

fn length_check(len: usize, error_empty: Error, error_too_long: Error) -> Result<(), Error> {
    if len == 0 {
        Err(error_empty)
    } else if len > 1023 {
        Err(error_too_long)
    } else {
        Ok(())
    }
}

macro_rules! def_part_parse_doc {
    ($name:ident, $other:ident, $more:expr) => {
        concat!(
            "Parse a [`",
            stringify!($name),
            "`] from a `",
            stringify!($other),
            "`, copying its contents.\n",
            "\n",
            "If the given `",
            stringify!($other),
            "` does not conform to the restrictions imposed by `",
            stringify!($name),
            "`, an error is returned.\n",
            $more,
        )
    };
}

macro_rules! def_part_into_inner_doc {
    ($name:ident, $other:ident, $more:expr) => {
        concat!(
            "Consume the `",
            stringify!($name),
            "` and return the inner `",
            stringify!($other),
            "`.\n",
            $more,
        )
    };
}

macro_rules! def_part_types {
    (
        $(#[$mainmeta:meta])*
        pub struct $name:ident(String) use $prepfn:ident(err = $preperr:path, empty = $emptyerr:path, long = $longerr:path);

        $(#[$refmeta:meta])*
        pub struct ref $borrowed:ident(str);
    ) => {
        $(#[$mainmeta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(pub(crate) String);

        impl $name {
            #[doc = def_part_parse_doc!($name, str, "Depending on whether the contents are changed by normalisation operations, this function either returns a copy or a reference to the original data.")]
            pub fn new(s: &str) -> Result<Cow<'_, $borrowed>, Error> {
                let node = $prepfn(s).map_err(|_| $preperr)?;
                length_check(node.len(), $emptyerr, $longerr)?;
                match node {
                    Cow::Borrowed(v) => Ok(Cow::Borrowed($borrowed::from_str_unchecked(v))),
                    Cow::Owned(v) => Ok(Cow::Owned(Self(v))),
                }
            }

            #[doc = def_part_into_inner_doc!($name, String, "")]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Error> {
                Ok(Self::new(s)?.into_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                <$borrowed as fmt::Display>::fmt(Borrow::<$borrowed>::borrow(self), f)
            }
        }

        impl Deref for $name {
            type Target = $borrowed;

            fn deref(&self) -> &Self::Target {
                Borrow::<$borrowed>::borrow(self)
            }
        }

        impl AsRef<$borrowed> for $name {
            fn as_ref(&self) -> &$borrowed {
                Borrow::<$borrowed>::borrow(self)
            }
        }

        impl AsRef<String> for $name {
            fn as_ref(&self) -> &String {
                &self.0
            }
        }

        impl Borrow<$borrowed> for $name {
            fn borrow(&self) -> &$borrowed {
                $borrowed::from_str_unchecked(self.0.as_str())
            }
        }

        // useful for use in hashmaps
        impl Borrow<String> for $name {
            fn borrow(&self) -> &String {
                &self.0
            }
        }

        // useful for use in hashmaps
        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.0.as_str()
            }
        }

        impl<'x> TryFrom<&'x str> for $name {
            type Error = Error;

            fn try_from(s: &str) -> Result<Self, Error> {
                Self::from_str(s)
            }
        }

        impl From<&$borrowed> for $name {
            fn from(other: &$borrowed) -> Self {
                other.to_owned()
            }
        }

        impl<'x> From<Cow<'x, $borrowed>> for $name {
            fn from(other: Cow<'x, $borrowed>) -> Self {
                other.into_owned()
            }
        }

        $(#[$refmeta])*
        #[repr(transparent)]
        #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $borrowed(pub(crate) str);

        impl $borrowed {
            pub(crate) fn from_str_unchecked(s: &str) -> &Self {
                // SAFETY: repr(transparent) thing can be transmuted to/from
                // its inner.
                unsafe { mem::transmute(s) }
            }

            /// Access the contents as [`str`] slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $borrowed {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ToOwned for $borrowed {
            type Owned = $name;

            fn to_owned(&self) -> Self::Owned {
                $name(self.0.to_string())
            }
        }

        impl AsRef<str> for $borrowed {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $borrowed {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", &self.0)
            }
        }
    }
}

def_part_types! {
    /// The [`NodePart`] is the optional part before the (optional) `@` in any
    /// [`Jid`][crate::Jid], whether [`BareJid`][crate::BareJid] or
    /// [`FullJid`][crate::FullJid].
    ///
    /// The corresponding slice type is [`NodeRef`].
    pub struct NodePart(String) use nodeprep(err = Error::NodePrep, empty = Error::NodeEmpty, long = Error::NodeTooLong);

    /// `str`-like type which conforms to the requirements of [`NodePart`].
    ///
    /// See [`NodePart`] for details.
    pub struct ref NodeRef(str);
}

def_part_types! {
    /// The [`DomainPart`] is the part between the (optional) `@` and the
    /// (optional) `/` in any [`Jid`][crate::Jid], whether
    /// [`BareJid`][crate::BareJid] or [`FullJid`][crate::FullJid].
    pub struct DomainPart(String) use nameprep(err = Error::NamePrep, empty = Error::DomainEmpty, long = Error::DomainTooLong);

    /// `str`-like type which conforms to the requirements of [`DomainPart`].
    ///
    /// See [`DomainPart`] for details.
    pub struct ref DomainRef(str);
}

def_part_types! {
    /// The [`ResourcePart`] is the optional part after the `/` in a
    /// [`Jid`][crate::Jid]. It is mandatory in [`FullJid`][crate::FullJid].
    pub struct ResourcePart(String) use resourceprep(err = Error::ResourcePrep, empty = Error::ResourceEmpty, long = Error::ResourceTooLong);

    /// `str`-like type which conforms to the requirements of
    /// [`ResourcePart`].
    ///
    /// See [`ResourcePart`] for details.
    pub struct ref ResourceRef(str);
}

impl DomainRef {
    /// Construct a bare JID (a JID without a resource) from this domain and
    /// the given node (local part).
    pub fn with_node(&self, node: &NodeRef) -> BareJid {
        BareJid::from_parts(Some(node), self)
    }
}

impl From<DomainPart> for BareJid {
    fn from(other: DomainPart) -> Self {
        BareJid {
            inner: other.into(),
        }
    }
}

impl From<DomainPart> for Jid {
    fn from(other: DomainPart) -> Self {
        Jid {
            normalized: other.0,
            at: None,
            slash: None,
        }
    }
}

impl<'x> From<&'x DomainRef> for BareJid {
    fn from(other: &'x DomainRef) -> Self {
        Self::from_parts(None, other)
    }
}

impl NodeRef {
    /// Construct a bare JID (a JID without a resource) from this node (the
    /// local part) and the given domain.
    pub fn with_domain(&self, domain: &DomainRef) -> BareJid {
        BareJid::from_parts(Some(self), domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodepart_comparison() {
        let n1 = NodePart::new("foo").unwrap();
        let n2 = NodePart::new("bar").unwrap();
        let n3 = NodePart::new("foo").unwrap();
        assert_eq!(n1, n3);
        assert_ne!(n1, n2);
    }
}
