use stringprep::{nameprep, nodeprep, resourceprep};

use crate::Error;

/// The [`NodePart`] is the optional part before the (optional) `@` in any [`Jid`], whether [`BareJid`] or [`FullJid`].
#[derive(Clone, Debug, PartialEq, Hash, PartialOrd)]
pub struct NodePart(pub(crate) String);

fn length_check(len: usize, error_empty: Error, error_too_long: Error) -> Result<(), Error> {
    if len == 0 {
        Err(error_empty)
    } else if len > 1023 {
        Err(error_too_long)
    } else {
        Ok(())
    }
}

impl NodePart {
    /// Build a new [`NodePart`] from a string slice. Will fail in case of stringprep validation error.
    pub fn new(s: &str) -> Result<NodePart, Error> {
        let node = nodeprep(s).map_err(|_| Error::NodePrep)?;
        length_check(node.len(), Error::NodeEmpty, Error::NodeTooLong)?;
        Ok(NodePart(node.to_string()))
    }
}

/// The [`DomainPart`] is the part between the (optional) `@` and the (optional) `/` in any [`Jid`], whether [`BareJid`] or [`FullJid`].
#[derive(Clone, Debug, PartialEq, Hash, PartialOrd)]
pub struct DomainPart(pub(crate) String);

impl DomainPart {
    /// Build a new [`DomainPart`] from a string slice. Will fail in case of stringprep validation error.
    pub fn new(s: &str) -> Result<DomainPart, Error> {
        let domain = nameprep(s).map_err(|_| Error::NamePrep)?;
        length_check(domain.len(), Error::DomainEmpty, Error::DomainTooLong)?;
        Ok(DomainPart(domain.to_string()))
    }
}

/// The [`ResourcePart`] is the optional part after the `/` in a [`Jid`]. It is mandatory in [`FullJid`].
#[derive(Clone, Debug, PartialEq, Hash, PartialOrd)]
pub struct ResourcePart(pub(crate) String);

impl ResourcePart {
    /// Build a new [`ResourcePart`] from a string slice. Will fail in case of stringprep validation error.
    pub fn new(s: &str) -> Result<ResourcePart, Error> {
        let resource = resourceprep(s).map_err(|_| Error::ResourcePrep)?;
        length_check(resource.len(), Error::ResourceEmpty, Error::ResourceTooLong)?;
        Ok(ResourcePart(resource.to_string()))
    }
}
