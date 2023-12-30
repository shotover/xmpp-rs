//! XMPP implementation with asynchronous I/O using Tokio.

#![deny(unsafe_code, missing_docs, bare_trait_objects)]

#[cfg(all(feature = "tls-native", feature = "tls-rust"))]
compile_error!("Both tls-native and tls-rust features can't be enabled at the same time.");

#[cfg(all(not(feature = "tls-native"), not(feature = "tls-rust")))]
compile_error!("One of tls-native and tls-rust features must be enabled.");

mod starttls;
mod stream_start;
mod xmpp_codec;
pub use crate::xmpp_codec::Packet;
mod event;
pub use event::Event;
mod client;
mod happy_eyeballs;
pub mod stream_features;
pub mod xmpp_stream;
pub use client::{
    async_client::{
        Client as AsyncClient, Config as AsyncConfig, ServerConfig as AsyncServerConfig,
    },
    connect::{client_login, AsyncReadAndWrite, ServerConnector},
    simple_client::Client as SimpleClient,
};
mod component;
pub use crate::component::Component;
mod error;
pub use crate::error::{AuthError, ConnecterError, Error, ParseError, ProtocolError};
pub use starttls::starttls;

// Re-exports
pub use minidom::Element;
pub use xmpp_parsers as parsers;
pub use xmpp_parsers::{BareJid, FullJid, Jid, JidParseError};
