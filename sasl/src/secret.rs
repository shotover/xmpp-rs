#[cfg(feature = "scram")]
use crate::common::scram::DeriveError;

pub trait Secret {}

pub trait Pbkdf2Secret {
    fn salt(&self) -> &[u8];
    fn iterations(&self) -> usize;
    fn digest(&self) -> &[u8];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plain(pub String);

impl Secret for Plain {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pbkdf2Sha1 {
    pub salt: Vec<u8>,
    pub iterations: usize,
    pub digest: Vec<u8>,
}

impl Pbkdf2Sha1 {
    #[cfg(feature = "scram")]
    pub fn derive(
        password: &str,
        salt: &[u8],
        iterations: usize,
    ) -> Result<Pbkdf2Sha1, DeriveError> {
        use crate::common::scram::{ScramProvider, Sha1};
        use crate::common::Password;
        let digest = Sha1::derive(&Password::Plain(password.to_owned()), salt, iterations)?;
        Ok(Pbkdf2Sha1 {
            salt: salt.to_vec(),
            iterations: iterations,
            digest: digest,
        })
    }
}

impl Secret for Pbkdf2Sha1 {}

impl Pbkdf2Secret for Pbkdf2Sha1 {
    fn salt(&self) -> &[u8] {
        &self.salt
    }
    fn iterations(&self) -> usize {
        self.iterations
    }
    fn digest(&self) -> &[u8] {
        &self.digest
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pbkdf2Sha256 {
    pub salt: Vec<u8>,
    pub iterations: usize,
    pub digest: Vec<u8>,
}

impl Pbkdf2Sha256 {
    #[cfg(feature = "scram")]
    pub fn derive(
        password: &str,
        salt: &[u8],
        iterations: usize,
    ) -> Result<Pbkdf2Sha256, DeriveError> {
        use crate::common::scram::{ScramProvider, Sha256};
        use crate::common::Password;
        let digest = Sha256::derive(&Password::Plain(password.to_owned()), salt, iterations)?;
        Ok(Pbkdf2Sha256 {
            salt: salt.to_vec(),
            iterations: iterations,
            digest: digest,
        })
    }
}

impl Secret for Pbkdf2Sha256 {}

impl Pbkdf2Secret for Pbkdf2Sha256 {
    fn salt(&self) -> &[u8] {
        &self.salt
    }
    fn iterations(&self) -> usize {
        self.iterations
    }
    fn digest(&self) -> &[u8] {
        &self.digest
    }
}
