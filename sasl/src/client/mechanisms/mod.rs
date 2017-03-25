//! Provides a few SASL mechanisms.

mod anonymous;
mod plain;

#[cfg(feature = "scram")]
mod scram;

pub use self::anonymous::Anonymous;
pub use self::plain::Plain;

#[cfg(feature = "scram")]
pub use self::scram::Scram;
