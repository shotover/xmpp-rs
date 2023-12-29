// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(bare_trait_objects)]

pub use tokio_xmpp::parsers;
use tokio_xmpp::{AsyncClient, AsyncServerConfig};
pub use tokio_xmpp::{BareJid, Element, FullJid, Jid};
#[macro_use]
extern crate log;

pub mod agent;
pub mod builder;
pub mod delay;
pub mod disco;
pub mod event;
pub mod event_loop;
pub mod feature;
pub mod iq;
pub mod message;
pub mod muc;
pub mod presence;
pub mod pubsub;
pub mod upload;

// Module re-exports
pub use agent::Agent;
pub use builder::{ClientBuilder, ClientType};
pub use event::Event;
pub use feature::ClientFeature;

type TokioXmppClient = AsyncClient<AsyncServerConfig>;

pub type Error = tokio_xmpp::Error;
pub type Id = Option<String>;
pub type RoomNick = String;

#[cfg(test)]
mod tests {
    use super::{Agent, BareJid, ClientBuilder, ClientFeature, ClientType, Event};
    use std::str::FromStr;
    use tokio_xmpp::AsyncClient as TokioXmppClient;

    #[tokio::test]
    async fn test_simple() {
        let jid = BareJid::from_str("foo@bar").unwrap();

        let client = TokioXmppClient::new(jid.clone(), "meh");

        // Client instance
        let client_builder = ClientBuilder::new(jid, "meh")
            .set_client(ClientType::Bot, "xmpp-rs")
            .set_website("https://gitlab.com/xmpp-rs/xmpp-rs")
            .set_default_nick("bot")
            .enable_feature(ClientFeature::ContactList);

        #[cfg(feature = "avatars")]
        let client_builder = client_builder.enable_feature(ClientFeature::Avatars);

        let mut agent: Agent = client_builder.build_impl(client);

        while let Some(events) = agent.wait_for_events().await {
            assert!(match events[0] {
                Event::Disconnected(_) => true,
                _ => false,
            });
            assert_eq!(events.len(), 1);
            break;
        }
    }
}
