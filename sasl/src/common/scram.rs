use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2;
use rand_os::{
    rand_core::{Error as RngError, RngCore},
    OsRng,
};
use sha1::{Digest, Sha1 as Sha1_hash};
use sha2::Sha256 as Sha256_hash;

use crate::common::Password;

use crate::secret;

use base64;

/// Generate a nonce for SCRAM authentication.
pub fn generate_nonce() -> Result<String, RngError> {
    let mut data = [0u8; 32];
    let mut rng = OsRng::new()?;
    rng.fill_bytes(&mut data);
    Ok(base64::encode(&data))
}

/// A trait which defines the needed methods for SCRAM.
pub trait ScramProvider {
    /// The kind of secret this `ScramProvider` requires.
    type Secret: secret::Secret;

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
    type Secret = secret::Pbkdf2Sha1;

    fn name() -> &'static str {
        "SHA-1"
    }

    fn hash(data: &[u8]) -> Vec<u8> {
        let hash = Sha1_hash::digest(data);
        let mut vec = Vec::with_capacity(Sha1_hash::output_size());
        vec.extend_from_slice(hash.as_slice());
        vec
    }

    fn hmac(data: &[u8], key: &[u8]) -> Vec<u8> {
        type HmacSha1 = Hmac<Sha1_hash>;
        let mut mac = HmacSha1::new_varkey(key).unwrap();
        mac.input(data);
        let result = mac.result();
        let mut vec = Vec::with_capacity(Sha1_hash::output_size());
        vec.extend_from_slice(result.code().as_slice());
        vec
    }

    fn derive(password: &Password, salt: &[u8], iterations: usize) -> Result<Vec<u8>, String> {
        match *password {
            Password::Plain(ref plain) => {
                let mut result = vec![0; 20];
                pbkdf2::<Hmac<Sha1_hash>>(plain.as_bytes(), salt, iterations, &mut result);
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
    type Secret = secret::Pbkdf2Sha256;

    fn name() -> &'static str {
        "SHA-256"
    }

    fn hash(data: &[u8]) -> Vec<u8> {
        let hash = Sha256_hash::digest(data);
        let mut vec = Vec::with_capacity(Sha256_hash::output_size());
        vec.extend_from_slice(hash.as_slice());
        vec
    }

    fn hmac(data: &[u8], key: &[u8]) -> Vec<u8> {
        type HmacSha256 = Hmac<Sha256_hash>;
        let mut mac = HmacSha256::new_varkey(key).unwrap();
        mac.input(data);
        let result = mac.result();
        let mut vec = Vec::with_capacity(Sha256_hash::output_size());
        vec.extend_from_slice(result.code().as_slice());
        vec
    }

    fn derive(password: &Password, salt: &[u8], iterations: usize) -> Result<Vec<u8>, String> {
        match *password {
            Password::Plain(ref plain) => {
                let mut result = vec![0; 32];
                pbkdf2::<Hmac<Sha256_hash>>(plain.as_bytes(), salt, iterations, &mut result);
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
