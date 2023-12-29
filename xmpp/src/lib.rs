// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(bare_trait_objects)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
pub use tokio_xmpp::parsers;
use tokio_xmpp::parsers::{
    caps::{compute_disco, hash_caps, Caps},
    disco::DiscoInfoResult,
    hashes::Algo,
    message::{Body, Message, MessageType},
    muc::{user::MucUser, Muc},
    presence::{Presence, Type as PresenceType},
};
use tokio_xmpp::AsyncClient as TokioXmppClient;
pub use tokio_xmpp::{BareJid, Element, FullJid, Jid};
#[macro_use]
extern crate log;

pub mod builder;
pub mod disco;
pub mod event;
pub mod event_loop;
pub mod feature;
pub mod iq;
pub mod message;
pub mod presence;
pub mod pubsub;
pub mod upload;

// Module re-exports
pub use builder::{ClientBuilder, ClientType};
pub use event::Event;
pub use feature::ClientFeature;

pub type Error = tokio_xmpp::Error;
pub type Id = Option<String>;
pub type RoomNick = String;

pub struct Agent {
    client: TokioXmppClient,
    default_nick: Arc<RwLock<String>>,
    lang: Arc<Vec<String>>,
    disco: DiscoInfoResult,
    node: String,
    uploads: Vec<(String, Jid, PathBuf)>,
    awaiting_disco_bookmarks_type: bool,
}

impl Agent {
    pub async fn disconnect(&mut self) -> Result<(), Error> {
        self.client.send_end().await
    }

    pub async fn join_room(
        &mut self,
        room: BareJid,
        nick: Option<String>,
        password: Option<String>,
        lang: &str,
        status: &str,
    ) {
        let mut muc = Muc::new();
        if let Some(password) = password {
            muc = muc.with_password(password);
        }

        let nick = nick.unwrap_or_else(|| self.default_nick.read().unwrap().clone());
        let room_jid = room.with_resource_str(&nick).unwrap();
        let mut presence = Presence::new(PresenceType::None).with_to(room_jid);
        presence.add_payload(muc);
        presence.set_status(String::from(lang), String::from(status));
        let _ = self.client.send_stanza(presence.into()).await;
    }

    /// Send a "leave room" request to the server (specifically, an "unavailable" presence stanza).
    ///
    /// The returned future will resolve when the request has been sent,
    /// not when the room has actually been left.
    ///
    /// If successful, a `RoomLeft` event should be received later as a confirmation.
    ///
    /// See: https://xmpp.org/extensions/xep-0045.html#exit
    ///
    /// Note that this method does NOT remove the room from the auto-join list; the latter
    /// is more a list of bookmarks that the account knows about and that have a flag set
    /// to indicate that they should be joined automatically after connecting (see the JoinRoom event).
    ///
    /// Regarding the latter, see the these minutes about auto-join behavior:
    /// https://docs.modernxmpp.org/meetings/2019-01-brussels/#bookmarks
    ///
    /// # Arguments
    ///
    /// * `room_jid`: The JID of the room to leave.
    /// * `nickname`: The nickname to use in the room.
    /// * `lang`: The language of the status message.
    /// * `status`: The status message to send.
    pub async fn leave_room(
        &mut self,
        room_jid: BareJid,
        nickname: RoomNick,
        lang: impl Into<String>,
        status: impl Into<String>,
    ) {
        // XEP-0045 specifies that, to leave a room, the client must send a presence stanza
        // with type="unavailable".
        let mut presence = Presence::new(PresenceType::Unavailable).with_to(
            room_jid
                .with_resource_str(nickname.as_str())
                .expect("Invalid room JID after adding resource part."),
        );

        // Optionally, the client may include a status message in the presence stanza.
        // TODO: Should this be optional? The XEP says "MAY", but the method signature requires the arguments.
        // XEP-0045: "The occupant MAY include normal <status/> information in the unavailable presence stanzas"
        presence.set_status(lang, status);

        // Send the presence stanza.
        if let Err(e) = self.client.send_stanza(presence.into()).await {
            // Report any errors to the log.
            error!("Failed to send leave room presence: {}", e);
        }
    }

    pub async fn send_message(
        &mut self,
        recipient: Jid,
        type_: MessageType,
        lang: &str,
        text: &str,
    ) {
        let mut message = Message::new(Some(recipient));
        message.type_ = type_;
        message
            .bodies
            .insert(String::from(lang), Body(String::from(text)));
        let _ = self.client.send_stanza(message.into()).await;
    }

    pub async fn send_room_private_message(
        &mut self,
        room: BareJid,
        recipient: RoomNick,
        lang: &str,
        text: &str,
    ) {
        let recipient: Jid = room.with_resource_str(&recipient).unwrap().into();
        let mut message = Message::new(recipient).with_payload(MucUser::new());
        message.type_ = MessageType::Chat;
        message
            .bodies
            .insert(String::from(lang), Body(String::from(text)));
        let _ = self.client.send_stanza(message.into()).await;
    }

    fn make_initial_presence(disco: &DiscoInfoResult, node: &str) -> Presence {
        let caps_data = compute_disco(disco);
        let hash = hash_caps(&caps_data, Algo::Sha_1).unwrap();
        let caps = Caps::new(node, hash);

        let mut presence = Presence::new(PresenceType::None);
        presence.add_payload(caps);
        presence
    }

    /// Wait for new events.
    ///
    /// # Returns
    ///
    /// - `Some(events)` if there are new events; multiple may be returned at once.
    /// - `None` if the underlying stream is closed.
    pub async fn wait_for_events(&mut self) -> Option<Vec<Event>> {
        event_loop::wait_for_events(self).await
    }

    pub async fn upload_file_with(&mut self, service: &str, path: &Path) {
        upload::send::upload_file_with(self, service, path).await
    }
}

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
