// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(bare_trait_objects)]

use futures::stream::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs::File;
pub use tokio_xmpp::parsers;
use tokio_xmpp::parsers::{
    bookmarks, bookmarks2,
    caps::{compute_disco, hash_caps, Caps},
    disco::{DiscoInfoQuery, DiscoInfoResult, Feature, Identity},
    hashes::Algo,
    http_upload::SlotRequest,
    iq::Iq,
    message::{Body, Message, MessageType},
    muc::{user::MucUser, Muc},
    ns,
    presence::{Presence, Type as PresenceType},
    private::Query as PrivateXMLQuery,
    pubsub::pubsub::{Items, PubSub},
    roster::{Item as RosterItem, Roster},
    Error as ParsersError,
};
use tokio_xmpp::{AsyncClient as TokioXmppClient, Event as TokioXmppEvent};
pub use tokio_xmpp::{BareJid, Element, FullJid, Jid};
#[macro_use]
extern crate log;

pub mod iq;
pub mod message;
pub mod presence;
pub mod pubsub;
pub mod upload;

pub type Error = tokio_xmpp::Error;

#[derive(Debug)]
pub enum ClientType {
    Bot,
    Pc,
}

impl Default for ClientType {
    fn default() -> Self {
        ClientType::Bot
    }
}

impl ToString for ClientType {
    fn to_string(&self) -> String {
        String::from(match self {
            ClientType::Bot => "bot",
            ClientType::Pc => "pc",
        })
    }
}

#[derive(PartialEq)]
pub enum ClientFeature {
    #[cfg(feature = "avatars")]
    Avatars,
    ContactList,
    JoinRooms,
}

pub type Id = Option<String>;
pub type RoomNick = String;

#[derive(Debug)]
pub enum Event {
    Online,
    Disconnected(Error),
    ContactAdded(RosterItem),
    ContactRemoved(RosterItem),
    ContactChanged(RosterItem),
    #[cfg(feature = "avatars")]
    AvatarRetrieved(Jid, String),
    ChatMessage(Id, BareJid, Body),
    JoinRoom(BareJid, bookmarks2::Conference),
    LeaveRoom(BareJid),
    LeaveAllRooms,
    RoomJoined(BareJid),
    RoomLeft(BareJid),
    RoomMessage(Id, BareJid, RoomNick, Body),
    /// A private message received from a room, containing the message ID, the room's BareJid,
    /// the sender's nickname, and the message body.
    RoomPrivateMessage(Id, BareJid, RoomNick, Body),
    ServiceMessage(Id, BareJid, Body),
    HttpUploadedFile(String),
}

pub struct ClientBuilder<'a> {
    jid: BareJid,
    password: &'a str,
    website: String,
    default_nick: String,
    lang: Vec<String>,
    disco: (ClientType, String),
    features: Vec<ClientFeature>,
    resource: Option<String>,
}

impl ClientBuilder<'_> {
    pub fn new<'a>(jid: BareJid, password: &'a str) -> ClientBuilder<'a> {
        ClientBuilder {
            jid,
            password,
            website: String::from("https://gitlab.com/xmpp-rs/tokio-xmpp"),
            default_nick: String::from("xmpp-rs"),
            lang: vec![String::from("en")],
            disco: (ClientType::default(), String::from("tokio-xmpp")),
            features: vec![],
            resource: None,
        }
    }

    /// Optionally set a resource associated to this device on the client
    pub fn set_resource(mut self, resource: &str) -> Self {
        self.resource = Some(resource.to_string());
        self
    }

    pub fn set_client(mut self, type_: ClientType, name: &str) -> Self {
        self.disco = (type_, String::from(name));
        self
    }

    pub fn set_website(mut self, url: &str) -> Self {
        self.website = String::from(url);
        self
    }

    pub fn set_default_nick(mut self, nick: &str) -> Self {
        self.default_nick = String::from(nick);
        self
    }

    pub fn set_lang(mut self, lang: Vec<String>) -> Self {
        self.lang = lang;
        self
    }

    pub fn enable_feature(mut self, feature: ClientFeature) -> Self {
        self.features.push(feature);
        self
    }

    fn make_disco(&self) -> DiscoInfoResult {
        let identities = vec![Identity::new(
            "client",
            self.disco.0.to_string(),
            "en",
            self.disco.1.to_string(),
        )];
        let mut features = vec![Feature::new(ns::DISCO_INFO)];
        #[cfg(feature = "avatars")]
        {
            if self.features.contains(&ClientFeature::Avatars) {
                features.push(Feature::new(format!("{}+notify", ns::AVATAR_METADATA)));
            }
        }
        if self.features.contains(&ClientFeature::JoinRooms) {
            features.push(Feature::new(format!("{}+notify", ns::BOOKMARKS2)));
        }
        DiscoInfoResult {
            node: None,
            identities,
            features,
            extensions: vec![],
        }
    }

    pub fn build(self) -> Agent {
        let jid: Jid = if let Some(resource) = &self.resource {
            self.jid.with_resource_str(resource).unwrap().into()
        } else {
            self.jid.clone().into()
        };

        let client = TokioXmppClient::new(jid, self.password);
        self.build_impl(client)
    }

    // This function is meant to be used for testing build
    pub(crate) fn build_impl(self, client: TokioXmppClient) -> Agent {
        let disco = self.make_disco();
        let node = self.website;

        Agent {
            client,
            default_nick: Arc::new(RwLock::new(self.default_nick)),
            lang: Arc::new(self.lang),
            disco,
            node,
            uploads: Vec::new(),
            awaiting_disco_bookmarks_type: false,
        }
    }
}

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

    // This method is a workaround due to prosody bug https://issues.prosody.im/1664
    // FIXME: To be removed in the future
    // The server doesn't return disco#info feature when querying the account
    // so we add it manually because we know it's true
    async fn handle_disco_info_result_payload(&mut self, payload: Element, from: Jid) {
        match DiscoInfoResult::try_from(payload.clone()) {
            Ok(disco) => {
                self.handle_disco_info_result(disco, from).await;
            }
            Err(e) => match e {
                ParsersError::ParseError(reason) => {
                    if reason == "disco#info feature not present in disco#info." {
                        let mut payload = payload.clone();
                        let disco_feature =
                            Feature::new("http://jabber.org/protocol/disco#info").into();
                        payload.append_child(disco_feature);
                        match DiscoInfoResult::try_from(payload) {
                            Ok(disco) => {
                                self.handle_disco_info_result(disco, from).await;
                            }
                            Err(e) => {
                                panic!("Wrong disco#info format after workaround: {}", e)
                            }
                        }
                    } else {
                        panic!(
                            "Wrong disco#info format (reason cannot be worked around): {}",
                            e
                        )
                    }
                }
                _ => panic!("Wrong disco#info format: {}", e),
            },
        }
    }

    async fn handle_disco_info_result(&mut self, disco: DiscoInfoResult, from: Jid) {
        // Safe unwrap because no DISCO is received when we are not online
        if from == self.client.bound_jid().unwrap().to_bare() && self.awaiting_disco_bookmarks_type
        {
            info!("Received disco info about bookmarks type");
            // Trigger bookmarks query
            // TODO: only send this when the JoinRooms feature is enabled.
            self.awaiting_disco_bookmarks_type = false;
            let mut perform_bookmarks2 = false;
            info!("{:#?}", disco.features);
            for feature in disco.features {
                if feature.var == "urn:xmpp:bookmarks:1#compat" {
                    perform_bookmarks2 = true;
                }
            }

            if perform_bookmarks2 {
                // XEP-0402 bookmarks (modern)
                let iq =
                    Iq::from_get("bookmarks", PubSub::Items(Items::new(ns::BOOKMARKS2))).into();
                let _ = self.client.send_stanza(iq).await;
            } else {
                // XEP-0048 v1.0 bookmarks (legacy)
                let iq = Iq::from_get(
                    "bookmarks-legacy",
                    PrivateXMLQuery {
                        storage: bookmarks::Storage::new(),
                    },
                )
                .into();
                let _ = self.client.send_stanza(iq).await;
            }
        } else {
            unimplemented!("Ignored disco#info response from {}", from);
        }
    }

    /// Wait for new events.
    ///
    /// # Returns
    ///
    /// - `Some(events)` if there are new events; multiple may be returned at once.
    /// - `None` if the underlying stream is closed.
    pub async fn wait_for_events(&mut self) -> Option<Vec<Event>> {
        if let Some(event) = self.client.next().await {
            let mut events = Vec::new();

            match event {
                TokioXmppEvent::Online { resumed: false, .. } => {
                    let presence = Self::make_initial_presence(&self.disco, &self.node).into();
                    let _ = self.client.send_stanza(presence).await;
                    events.push(Event::Online);
                    // TODO: only send this when the ContactList feature is enabled.
                    let iq = Iq::from_get(
                        "roster",
                        Roster {
                            ver: None,
                            items: vec![],
                        },
                    )
                    .into();
                    let _ = self.client.send_stanza(iq).await;

                    // Query account disco to know what bookmarks spec is used
                    let iq = Iq::from_get("disco-account", DiscoInfoQuery { node: None }).into();
                    let _ = self.client.send_stanza(iq).await;
                    self.awaiting_disco_bookmarks_type = true;
                }
                TokioXmppEvent::Online { resumed: true, .. } => {}
                TokioXmppEvent::Disconnected(e) => {
                    events.push(Event::Disconnected(e));
                }
                TokioXmppEvent::Stanza(elem) => {
                    if elem.is("iq", "jabber:client") {
                        let iq = Iq::try_from(elem).unwrap();
                        let new_events = iq::handle_iq(self, iq).await;
                        events.extend(new_events);
                    } else if elem.is("message", "jabber:client") {
                        let message = Message::try_from(elem).unwrap();
                        let new_events = message::handle_message(self, message).await;
                        events.extend(new_events);
                    } else if elem.is("presence", "jabber:client") {
                        let presence = Presence::try_from(elem).unwrap();
                        let new_events = presence::handle_presence(self, presence).await;
                        events.extend(new_events);
                    } else if elem.is("error", "http://etherx.jabber.org/streams") {
                        println!("Received a fatal stream error: {}", String::from(&elem));
                    } else {
                        panic!("Unknown stanza: {}", String::from(&elem));
                    }
                }
            }

            Some(events)
        } else {
            None
        }
    }

    pub async fn upload_file_with(&mut self, service: &str, path: &Path) {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let file = File::open(path).await.unwrap();
        let size = file.metadata().await.unwrap().len();
        let slot_request = SlotRequest {
            filename: name,
            size: size,
            content_type: None,
        };
        let to = service.parse::<Jid>().unwrap();
        let request = Iq::from_get("upload1", slot_request).with_to(to.clone());
        self.uploads
            .push((String::from("upload1"), to, path.to_path_buf()));
        self.client.send_stanza(request.into()).await.unwrap();
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
