//#![deny(missing_docs)]

//! This crate provides a framework for SASL authentication and a few authentication mechanisms.
//!
//! # Examples
//!
//! ## Simple client-sided usage
//!
//! ```rust
//! use sasl::client::Mechanism;
//! use sasl::common::Credentials;
//! use sasl::client::mechanisms::Plain;
//!
//! let creds = Credentials::default()
//!                         .with_username("user")
//!                         .with_password("pencil");
//!
//! let mut mechanism = Plain::from_credentials(creds).unwrap();
//!
//! let initial_data = mechanism.initial().unwrap();
//!
//! assert_eq!(initial_data, b"\0user\0pencil");
//! ```
//!
//! ## More complex usage
//!
//! ```rust
//! #[macro_use] extern crate sasl;
//!
//! use sasl::server::{Validator, Provider, Mechanism as ServerMechanism, Response};
//! use sasl::server::mechanisms::{Plain as ServerPlain, Scram as ServerScram};
//! use sasl::client::Mechanism as ClientMechanism;
//! use sasl::client::mechanisms::{Plain as ClientPlain, Scram as ClientScram};
//! use sasl::common::{Identity, Credentials, Password, ChannelBinding};
//! use sasl::common::scram::{ScramProvider, Sha1, Sha256};
//! use sasl::secret;
//!
//! const USERNAME: &'static str = "user";
//! const PASSWORD: &'static str = "pencil";
//! const SALT: [u8; 8] = [35, 71, 92, 105, 212, 219, 114, 93];
//! const ITERATIONS: usize = 4096;
//!
//! struct MyValidator;
//!
//! impl Validator<secret::Plain> for MyValidator {
//!     fn validate(&self, identity: &Identity, value: &secret::Plain) -> Result<(), String> {
//!         let &secret::Plain(ref password) = value;
//!         if identity != &Identity::Username(USERNAME.to_owned()) {
//!             Err("authentication failed".to_owned())
//!         }
//!         else if password != PASSWORD {
//!             Err("authentication failed".to_owned())
//!         }
//!         else {
//!             Ok(())
//!         }
//!     }
//! }
//!
//! impl Provider<secret::Pbkdf2Sha1> for MyValidator {
//!     fn provide(&self, identity: &Identity) -> Result<secret::Pbkdf2Sha1, String> {
//!         if identity != &Identity::Username(USERNAME.to_owned()) {
//!             Err("authentication failed".to_owned())
//!         }
//!         else {
//!             let digest = sasl::common::scram::Sha1::derive
//!                 ( &Password::Plain((PASSWORD.to_owned()))
//!                 , &SALT[..]
//!                 , ITERATIONS )?;
//!             Ok(secret::Pbkdf2Sha1 {
//!                 salt: SALT.to_vec(),
//!                 iterations: ITERATIONS,
//!                 digest: digest,
//!             })
//!         }
//!     }
//! }
//!
//! impl_validator_using_provider!(MyValidator, secret::Pbkdf2Sha1);
//!
//! impl Provider<secret::Pbkdf2Sha256> for MyValidator {
//!     fn provide(&self, identity: &Identity) -> Result<secret::Pbkdf2Sha256, String> {
//!         if identity != &Identity::Username(USERNAME.to_owned()) {
//!             Err("authentication failed".to_owned())
//!         }
//!         else {
//!             let digest = sasl::common::scram::Sha256::derive
//!                 ( &Password::Plain((PASSWORD.to_owned()))
//!                 , &SALT[..]
//!                 , ITERATIONS )?;
//!             Ok(secret::Pbkdf2Sha256 {
//!                 salt: SALT.to_vec(),
//!                 iterations: ITERATIONS,
//!                 digest: digest,
//!             })
//!         }
//!     }
//! }
//!
//! impl_validator_using_provider!(MyValidator, secret::Pbkdf2Sha256);
//!
//! fn finish<CM, SM>(cm: &mut CM, sm: &mut SM) -> Result<Identity, String>
//!     where CM: ClientMechanism,
//!           SM: ServerMechanism {
//!     let init = cm.initial()?;
//!     println!("C: {}", String::from_utf8_lossy(&init));
//!     let mut resp = sm.respond(&init)?;
//!     loop {
//!         let msg;
//!         match resp {
//!             Response::Proceed(ref data) => {
//!                 println!("S: {}", String::from_utf8_lossy(&data));
//!                 msg = cm.response(data)?;
//!                 println!("C: {}", String::from_utf8_lossy(&msg));
//!             },
//!             _ => break,
//!         }
//!         resp = sm.respond(&msg)?;
//!     }
//!     if let Response::Success(ret, fin) = resp {
//!         println!("S: {}", String::from_utf8_lossy(&fin));
//!         cm.success(&fin)?;
//!         Ok(ret)
//!     }
//!     else {
//!         unreachable!();
//!     }
//! }
//!
//! fn main() {
//!     let mut mech = ServerPlain::new(MyValidator);
//!     let expected_response = Response::Success(Identity::Username("user".to_owned()), Vec::new());
//!     assert_eq!(mech.respond(b"\0user\0pencil"), Ok(expected_response));
//!
//!     let mut mech = ServerPlain::new(MyValidator);
//!     assert_eq!(mech.respond(b"\0user\0marker"), Err("authentication failed".to_owned()));
//!
//!     let creds = Credentials::default()
//!                             .with_username(USERNAME)
//!                             .with_password(PASSWORD);
//!     let mut client_mech = ClientPlain::from_credentials(creds.clone()).unwrap();
//!     let mut server_mech = ServerPlain::new(MyValidator);
//!
//!     assert_eq!(finish(&mut client_mech, &mut server_mech), Ok(Identity::Username(USERNAME.to_owned())));
//!
//!     let mut client_mech = ClientScram::<Sha1>::from_credentials(creds.clone()).unwrap();
//!     let mut server_mech = ServerScram::<Sha1, _>::new(MyValidator, ChannelBinding::Unsupported);
//!
//!     assert_eq!(finish(&mut client_mech, &mut server_mech), Ok(Identity::Username(USERNAME.to_owned())));
//!
//!     let mut client_mech = ClientScram::<Sha256>::from_credentials(creds.clone()).unwrap();
//!     let mut server_mech = ServerScram::<Sha256, _>::new(MyValidator, ChannelBinding::Unsupported);
//!
//!     assert_eq!(finish(&mut client_mech, &mut server_mech), Ok(Identity::Username(USERNAME.to_owned())));
//! }
//! ```
//!
//! # Usage
//!
//! You can use this in your crate by adding this under `dependencies` in your `Cargo.toml`:
//!
//! ```toml,ignore
//! sasl = "*"
//! ```

extern crate base64;

#[cfg(feature = "scram")]
extern crate openssl;

mod error;

pub mod client;
#[macro_use]
pub mod server;
pub mod common;
pub mod secret;

pub use error::Error;
