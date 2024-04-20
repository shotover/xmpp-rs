#[cfg(feature = "anonymous")]
mod anonymous;
mod plain;
#[cfg(feature = "scram")]
mod scram;

#[cfg(feature = "anonymous")]
#[cfg_attr(docsrs, doc(cfg(feature = "anonymous")))]
pub use self::anonymous::Anonymous;
pub use self::plain::Plain;
#[cfg(feature = "scram")]
#[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
pub use self::scram::Scram;
