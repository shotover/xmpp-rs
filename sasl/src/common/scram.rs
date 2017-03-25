use openssl::error::ErrorStack;
use openssl::hash::hash;
use openssl::hash::MessageDigest;
use openssl::pkcs5::pbkdf2_hmac;
use openssl::pkey::PKey;
use openssl::rand::rand_bytes;
use openssl::sign::Signer;

use common::Password;

use secret;

use base64;

/// Generate a nonce for SCRAM authentication.
pub fn generate_nonce() -> Result<String, ErrorStack> {
    let mut data = vec![0; 32];
    rand_bytes(&mut data)?;
    Ok(base64::encode(&data))
}

/// A trait which defines the needed methods for SCRAM.
pub trait ScramProvider {
    /// The kind of secret this `ScramProvider` requires.
    type SecretKind: secret::SecretKind;

    /// The name of the hash function.
    fn name() -> &'static str;

    /// A function which hashes the data using the hash function.
    fn hash(data: &[u8]) -> Vec<u8>;

    /// A function which performs an HMAC using the hash function.
    fn hmac(data: &[u8], key: &[u8]) -> Vec<u8>;

    /// A function which does PBKDF2 key derivation using the hash function.
    fn derive(data: &Password, salt: &[u8], iterations: usize) -> Result<Vec<u8>, String>;
}

/// A `ScramProvider` which provides SCRAM-SHA-1 and SCRAM-SHA-1-PLUS
pub struct Sha1;

impl ScramProvider for Sha1 {
    // TODO: look at all these unwraps
    type SecretKind = secret::Pbkdf2Sha1;

    fn name() -> &'static str {
        "SHA-1"
    }

    fn hash(data: &[u8]) -> Vec<u8> {
        hash(MessageDigest::sha1(), data).unwrap()
    }

    fn hmac(data: &[u8], key: &[u8]) -> Vec<u8> {
        let pkey = PKey::hmac(key).unwrap();
        let mut signer = Signer::new(MessageDigest::sha1(), &pkey).unwrap();
        signer.update(data).unwrap();
        signer.finish().unwrap()
    }

    fn derive(password: &Password, salt: &[u8], iterations: usize) -> Result<Vec<u8>, String> {
        match *password {
            Password::Plain(ref plain) => {
                let mut result = vec![0; 20];
                pbkdf2_hmac(
                    plain.as_bytes(),
                    salt,
                    iterations,
                    MessageDigest::sha1(),
                    &mut result,
                )
                .unwrap();
                Ok(result)
            }
            Password::Pbkdf2 {
                ref method,
                salt: ref my_salt,
                iterations: my_iterations,
                ref data,
            } => {
                if method != Self::name() {
                    Err(format!(
                        "incompatible hashing method, {} is not {}",
                        method,
                        Self::name()
                    ))
                } else if my_salt == &salt {
                    Err(format!("incorrect salt"))
                } else if my_iterations == iterations {
                    Err(format!(
                        "incompatible iteration count, {} is not {}",
                        my_iterations, iterations
                    ))
                } else {
                    Ok(data.to_vec())
                }
            }
        }
    }
}

/// A `ScramProvider` which provides SCRAM-SHA-256 and SCRAM-SHA-256-PLUS
pub struct Sha256;

impl ScramProvider for Sha256 {
    // TODO: look at all these unwraps
    type SecretKind = secret::Pbkdf2Sha256;

    fn name() -> &'static str {
        "SHA-256"
    }

    fn hash(data: &[u8]) -> Vec<u8> {
        hash(MessageDigest::sha256(), data).unwrap()
    }

    fn hmac(data: &[u8], key: &[u8]) -> Vec<u8> {
        let pkey = PKey::hmac(key).unwrap();
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
        signer.update(data).unwrap();
        signer.finish().unwrap()
    }

    fn derive(password: &Password, salt: &[u8], iterations: usize) -> Result<Vec<u8>, String> {
        match *password {
            Password::Plain(ref plain) => {
                let mut result = vec![0; 32];
                pbkdf2_hmac(
                    plain.as_bytes(),
                    salt,
                    iterations,
                    MessageDigest::sha256(),
                    &mut result,
                )
                .unwrap();
                Ok(result)
            }
            Password::Pbkdf2 {
                ref method,
                salt: ref my_salt,
                iterations: my_iterations,
                ref data,
            } => {
                if method != Self::name() {
                    Err(format!(
                        "incompatible hashing method, {} is not {}",
                        method,
                        Self::name()
                    ))
                } else if my_salt == &salt {
                    Err(format!("incorrect salt"))
                } else if my_iterations == iterations {
                    Err(format!(
                        "incompatible iteration count, {} is not {}",
                        my_iterations, iterations
                    ))
                } else {
                    Ok(data.to_vec())
                }
            }
        }
    }
}
