//! Contains wrapper for `<stream:features/>`

use crate::error::AuthError;
use xmpp_parsers::{ns, Element};

/// Wraps `<stream:features/>`, usually the very first nonza of an
/// XMPPStream.
///
/// TODO: should this rather go into xmpp-parsers, kept in a decoded
/// struct?
#[derive(Debug)]
pub struct StreamFeatures(pub Element);

impl StreamFeatures {
    /// Wrap the nonza
    pub fn new(element: Element) -> Self {
        StreamFeatures(element)
    }

    /// Can initiate TLS session with this server?
    pub fn can_starttls(&self) -> bool {
        self.0.get_child("starttls", ns::TLS).is_some()
    }

    /// Iterate over SASL mechanisms
    pub fn sasl_mechanisms<'a>(&'a self) -> Result<impl Iterator<Item = String> + 'a, AuthError> {
        Ok(self
            .0
            .get_child("mechanisms", ns::SASL)
            .ok_or(AuthError::NoMechanism)?
            .children()
            .filter(|child| child.is("mechanism", ns::SASL))
            .map(|mech_el| mech_el.text()))
    }

    /// Does server support user resource binding?
    pub fn can_bind(&self) -> bool {
        self.0.get_child("bind", ns::BIND).is_some()
    }
}
