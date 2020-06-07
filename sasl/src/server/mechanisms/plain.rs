use crate::common::Identity;
use crate::secret;
use crate::server::{Mechanism, MechanismError, Response, Validator};

pub struct Plain<V: Validator<secret::Plain>> {
    validator: V,
}

impl<V: Validator<secret::Plain>> Plain<V> {
    pub fn new(validator: V) -> Plain<V> {
        Plain {
            validator: validator,
        }
    }
}

impl<V: Validator<secret::Plain>> Mechanism for Plain<V> {
    fn name(&self) -> &str {
        "PLAIN"
    }

    fn respond(&mut self, payload: &[u8]) -> Result<Response, MechanismError> {
        let mut sp = payload.split(|&b| b == 0);
        sp.next();
        let username = sp
            .next()
            .ok_or_else(|| MechanismError::NoUsernameSpecified)?;
        let username = String::from_utf8(username.to_vec())
            .map_err(|_| MechanismError::ErrorDecodingUsername)?;
        let password = sp
            .next()
            .ok_or_else(|| MechanismError::NoPasswordSpecified)?;
        let password = String::from_utf8(password.to_vec())
            .map_err(|_| MechanismError::ErrorDecodingPassword)?;
        let ident = Identity::Username(username);
        self.validator.validate(&ident, &secret::Plain(password))?;
        Ok(Response::Success(ident, Vec::new()))
    }
}
