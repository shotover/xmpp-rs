use crate::common::Identity;
use crate::server::{Mechanism, MechanismError, Response};
use getrandom::getrandom;

pub struct Anonymous;

impl Anonymous {
    pub fn new() -> Anonymous {
        Anonymous
    }
}

impl Mechanism for Anonymous {
    fn name(&self) -> &str {
        "ANONYMOUS"
    }

    fn respond(&mut self, payload: &[u8]) -> Result<Response, MechanismError> {
        if !payload.is_empty() {
            return Err(MechanismError::FailedToDecodeMessage);
        }
        let mut rand = [0u8; 16];
        getrandom(&mut rand)?;
        let username = format!("{:02x?}", rand);
        let ident = Identity::Username(username);
        Ok(Response::Success(ident, Vec::new()))
    }
}
