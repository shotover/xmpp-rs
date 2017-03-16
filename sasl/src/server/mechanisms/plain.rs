use server::Mechanism;
use common::{Secret, Credentials, Password};

pub struct Plain {
    password: String,
}

impl<V: Validator> Mechanism<V> for Plain {
    fn name(&self) -> &str { "PLAIN" }

    fn from_initial_message(validator: &V, msg: &[u8]) -> Result<(Self, String), String> {
    }
}
