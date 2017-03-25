pub trait SecretKind {
    type Value: PartialEq;
}

pub trait Pbkdf2SecretValue {
    fn salt(&self) -> &[u8];
    fn iterations(&self) -> usize;
    fn digest(&self) -> &[u8];
}

pub struct Plain;

#[derive(PartialEq)]
pub struct PlainValue(pub String);

impl SecretKind for Plain {
    type Value = PlainValue;
}

pub struct Pbkdf2Sha1 {
    pub salt: Vec<u8>,
    pub iterations: usize,
}

#[derive(PartialEq)]
pub struct Pbkdf2Sha1Value {
    pub salt: Vec<u8>,
    pub iterations: usize,
    pub digest: Vec<u8>,
}

impl SecretKind for Pbkdf2Sha1 {
    type Value = Pbkdf2Sha1Value;
}

impl Pbkdf2SecretValue for Pbkdf2Sha1Value {
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

pub struct Pbkdf2Sha256 {
    pub salt: Vec<u8>,
    pub iterations: usize,
}

#[derive(PartialEq)]
pub struct Pbkdf2Sha256Value {
    pub salt: Vec<u8>,
    pub iterations: usize,
    pub digest: Vec<u8>,
}

impl SecretKind for Pbkdf2Sha256 {
    type Value = Pbkdf2Sha256Value;
}

impl Pbkdf2SecretValue for Pbkdf2Sha256Value {
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
