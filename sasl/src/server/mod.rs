use common::Identity;
use secret::Secret;

#[macro_export]
macro_rules! impl_validator_using_provider {
    ( $validator:ty, $secret:ty ) => {
        impl $crate::server::Validator<$secret> for $validator {
            fn validate(
                &self,
                identity: &$crate::common::Identity,
                value: &$secret,
            ) -> Result<(), String> {
                if &(self as &$crate::server::Provider<$secret>).provide(identity)? == value {
                    Ok(())
                } else {
                    Err("authentication failure".to_owned())
                }
            }
        }
    };
}

pub trait Provider<S: Secret>: Validator<S> {
    fn provide(&self, identity: &Identity) -> Result<S, String>;
}

pub trait Validator<S: Secret> {
    fn validate(&self, identity: &Identity, value: &S) -> Result<(), String>;
}

pub trait Mechanism {
    fn name(&self) -> &str;
    fn respond(&mut self, payload: &[u8]) -> Result<Response, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Success(Identity, Vec<u8>),
    Proceed(Vec<u8>),
}

pub mod mechanisms;
