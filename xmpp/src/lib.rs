// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(bare_trait_objects)]

use futures::stream::StreamExt;
use reqwest::{
    header::HeaderMap as ReqwestHeaderMap, Body as ReqwestBody, Client as ReqwestClient,
};
use std::cell::RefCell;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_xmpp::{AsyncClient as TokioXmppClient, Event as TokioXmppEvent};
use xmpp_parsers::{
    bookmarks2::Conference,
    caps::{compute_disco, hash_caps, Caps},
    disco::{DiscoInfoQuery, DiscoInfoResult, Feature, Identity},
    hashes::Algo,
    http_upload::{Header as HttpUploadHeader, SlotRequest, SlotResult},
    iq::{Iq, IqType},
    message::{Body, Message, MessageType},
    muc::{
        user::{MucUser, Status},
        Muc,
    },
    ns,
    presence::{Presence, Type as PresenceType},
    pubsub::pubsub::{Items, PubSub},
    roster::{Item as RosterItem, Roster},
    stanza_error::{DefinedCondition, ErrorType, StanzaError},
    BareJid, Element, FullJid, Jid,
};
#[macro_use]
extern crate log;

mod pubsub;

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
    Disconnected,
    ContactAdded(RosterItem),
    ContactRemoved(RosterItem),
    ContactChanged(RosterItem),
    #[cfg(feature = "avatars")]
    AvatarRetrieved(Jid, String),
    ChatMessage(Id, BareJid, Body),
    JoinRoom(BareJid, Conference),
    LeaveRoom(BareJid),
    LeaveAllRooms,
    RoomJoined(BareJid),
    RoomLeft(BareJid),
    RoomMessage(Id, BareJid, RoomNick, Body),
    ServiceMessage(Id, BareJid, Body),
    HttpUploadedFile(String),
}

#[derive(Default)]
pub struct ClientBuilder<'a> {
    jid: &'a str,
    password: &'a str,
    website: String,
    default_nick: String,
    lang: Vec<String>,
    disco: (ClientType, String),
    features: Vec<ClientFeature>,
}

impl ClientBuilder<'_> {
    pub fn new<'a>(jid: &'a str, password: &'a str) -> ClientBuilder<'a> {
        ClientBuilder {
            jid,
            password,
            website: String::from("https://gitlab.com/xmpp-rs/tokio-xmpp"),
            default_nick: String::from("xmpp-rs"),
            lang: vec![String::from("en")],
            disco: (ClientType::default(), String::from("tokio-xmpp")),
            features: vec![],
        }
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

    pub fn build(self) -> Result<Agent, Error> {
        let client = TokioXmppClient::new(self.jid, self.password)?;
        Ok(self.build_impl(client)?)
    }

    // This function is meant to be used for testing build
    pub(crate) fn build_impl(self, client: TokioXmppClient) -> Result<Agent, Error> {
        let disco = self.make_disco();
        let node = self.website;

        let agent = Agent {
            client,
            default_nick: Rc::new(RefCell::new(self.default_nick)),
            lang: Rc::new(self.lang),
            disco,
            node,
            uploads: Vec::new(),
        };

        Ok(agent)
    }
}

pub struct Agent {
    client: TokioXmppClient,
    default_nick: Rc<RefCell<String>>,
    lang: Rc<Vec<String>>,
    disco: DiscoInfoResult,
    node: String,
    uploads: Vec<(String, Jid, PathBuf)>,
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

        let nick = nick.unwrap_or_else(|| self.default_nick.borrow().clone());
        let room_jid = room.with_resource(nick);
        let mut presence = Presence::new(PresenceType::None).with_to(Jid::Full(room_jid));
        presence.add_payload(muc);
        presence.set_status(String::from(lang), String::from(status));
        let _ = self.client.send_stanza(presence.into()).await;
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

    fn make_initial_presence(disco: &DiscoInfoResult, node: &str) -> Presence {
        let caps_data = compute_disco(disco);
        let hash = hash_caps(&caps_data, Algo::Sha_1).unwrap();
        let caps = Caps::new(node, hash);

        let mut presence = Presence::new(PresenceType::None);
        presence.add_payload(caps);
        presence
    }

    async fn handle_iq(&mut self, iq: Iq) -> Vec<Event> {
        let mut events = vec![];
        let from = iq
            .from
            .clone()
            .unwrap_or_else(|| self.client.bound_jid().unwrap().clone());
        if let IqType::Get(payload) = iq.payload {
            if payload.is("query", ns::DISCO_INFO) {
                let query = DiscoInfoQuery::try_from(payload);
                match query {
                    Ok(query) => {
                        let mut disco_info = self.disco.clone();
                        disco_info.node = query.node;
                        let iq = Iq::from_result(iq.id, Some(disco_info))
                            .with_to(iq.from.unwrap())
                            .into();
                        let _ = self.client.send_stanza(iq).await;
                    }
                    Err(err) => {
                        let error = StanzaError::new(
                            ErrorType::Modify,
                            DefinedCondition::BadRequest,
                            "en",
                            &format!("{}", err),
                        );
                        let iq = Iq::from_error(iq.id, error)
                            .with_to(iq.from.unwrap())
                            .into();
                        let _ = self.client.send_stanza(iq).await;
                    }
                }
            } else {
                // We MUST answer unhandled get iqs with a service-unavailable error.
                let error = StanzaError::new(
                    ErrorType::Cancel,
                    DefinedCondition::ServiceUnavailable,
                    "en",
                    "No handler defined for this kind of iq.",
                );
                let iq = Iq::from_error(iq.id, error)
                    .with_to(iq.from.unwrap())
                    .into();
                let _ = self.client.send_stanza(iq).await;
            }
        } else if let IqType::Result(Some(payload)) = iq.payload {
            // TODO: move private iqs like this one somewhere else, for
            // security reasons.
            if payload.is("query", ns::ROSTER) && Some(from.clone()) == iq.from {
                let roster = Roster::try_from(payload).unwrap();
                for item in roster.items.into_iter() {
                    events.push(Event::ContactAdded(item));
                }
            } else if payload.is("pubsub", ns::PUBSUB) {
                let new_events = pubsub::handle_iq_result(&from, payload);
                events.extend(new_events);
            } else if payload.is("slot", ns::HTTP_UPLOAD) {
                let new_events = handle_upload_result(&from, iq.id, payload, self).await;
                events.extend(new_events);
            }
        } else if let IqType::Set(_) = iq.payload {
            // We MUST answer unhandled set iqs with a service-unavailable error.
            let error = StanzaError::new(
                ErrorType::Cancel,
                DefinedCondition::ServiceUnavailable,
                "en",
                "No handler defined for this kind of iq.",
            );
            let iq = Iq::from_error(iq.id, error)
                .with_to(iq.from.unwrap())
                .into();
            let _ = self.client.send_stanza(iq).await;
        }

        events
    }

    async fn handle_message(&mut self, message: Message) -> Vec<Event> {
        let mut events = vec![];
        let from = message.from.clone().unwrap();
        let langs: Vec<&str> = self.lang.iter().map(String::as_str).collect();
        match message.get_best_body(langs) {
            Some((_lang, body)) => match message.type_ {
                MessageType::Groupchat => {
                    let event = match from.clone() {
                        Jid::Full(full) => Event::RoomMessage(
                            message.id.clone(),
                            from.clone().into(),
                            full.resource,
                            body.clone(),
                        ),
                        Jid::Bare(bare) => {
                            Event::ServiceMessage(message.id.clone(), bare, body.clone())
                        }
                    };
                    events.push(event)
                }
                MessageType::Chat | MessageType::Normal => {
                    let event =
                        Event::ChatMessage(message.id.clone(), from.clone().into(), body.clone());
                    events.push(event)
                }
                _ => (),
            },
            None => (),
        }
        for child in message.payloads {
            if child.is("event", ns::PUBSUB_EVENT) {
                let new_events = pubsub::handle_event(&from, child, self).await;
                events.extend(new_events);
            }
        }

        events
    }

    async fn handle_presence(&mut self, presence: Presence) -> Vec<Event> {
        let mut events = vec![];
        let from: BareJid = match presence.from.clone().unwrap() {
            Jid::Full(FullJid { node, domain, .. }) => BareJid { node, domain },
            Jid::Bare(bare) => bare,
        };
        for payload in presence.payloads.into_iter() {
            let muc_user = match MucUser::try_from(payload) {
                Ok(muc_user) => muc_user,
                _ => continue,
            };
            for status in muc_user.status.into_iter() {
                if status == Status::SelfPresence {
                    events.push(Event::RoomJoined(from.clone()));
                    break;
                }
            }
        }

        events
    }

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
                    // TODO: only send this when the JoinRooms feature is enabled.
                    let iq =
                        Iq::from_get("bookmarks", PubSub::Items(Items::new(ns::BOOKMARKS2))).into();
                    let _ = self.client.send_stanza(iq).await;
                }
                TokioXmppEvent::Online { resumed: true, .. } => {}
                TokioXmppEvent::Disconnected(_) => {
                    events.push(Event::Disconnected);
                }
                TokioXmppEvent::Stanza(elem) => {
                    if elem.is("iq", "jabber:client") {
                        let iq = Iq::try_from(elem).unwrap();
                        let new_events = self.handle_iq(iq).await;
                        events.extend(new_events);
                    } else if elem.is("message", "jabber:client") {
                        let message = Message::try_from(elem).unwrap();
                        let new_events = self.handle_message(message).await;
                        events.extend(new_events);
                    } else if elem.is("presence", "jabber:client") {
                        let presence = Presence::try_from(elem).unwrap();
                        let new_events = self.handle_presence(presence).await;
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

async fn handle_upload_result(
    from: &Jid,
    iqid: String,
    elem: Element,
    agent: &mut Agent,
) -> impl IntoIterator<Item = Event> {
    let mut res: Option<(usize, PathBuf)> = None;

    for (i, (id, to, filepath)) in agent.uploads.iter().enumerate() {
        if to == from && id == &iqid {
            res = Some((i, filepath.to_path_buf()));
            break;
        }
    }

    if let Some((index, file)) = res {
        agent.uploads.remove(index);
        let slot = SlotResult::try_from(elem).unwrap();

        let mut headers = ReqwestHeaderMap::new();
        for header in slot.put.headers {
            let (attr, val) = match header {
                HttpUploadHeader::Authorization(val) => ("Authorization", val),
                HttpUploadHeader::Cookie(val) => ("Cookie", val),
                HttpUploadHeader::Expires(val) => ("Expires", val),
            };
            headers.insert(attr, val.parse().unwrap());
        }

        let web = ReqwestClient::new();
        let stream = FramedRead::new(File::open(file).await.unwrap(), BytesCodec::new());
        let body = ReqwestBody::wrap_stream(stream);
        let res = web
            .put(slot.put.url.as_str())
            .headers(headers)
            .body(body)
            .send()
            .await
            .unwrap();
        if res.status() == 201 {
            return vec![Event::HttpUploadedFile(slot.get.url)];
        }
    }

    return vec![];
}

#[cfg(test)]
mod tests {
    use super::{Agent, ClientBuilder, ClientFeature, ClientType, Event};
    use tokio_xmpp::AsyncClient as TokioXmppClient;

    #[tokio::test]
    async fn test_simple() {
        let client = TokioXmppClient::new("foo@bar", "meh").unwrap();

        // Client instance
        let client_builder = ClientBuilder::new("foo@bar", "meh")
            .set_client(ClientType::Bot, "xmpp-rs")
            .set_website("https://gitlab.com/xmpp-rs/xmpp-rs")
            .set_default_nick("bot")
            .enable_feature(ClientFeature::Avatars)
            .enable_feature(ClientFeature::ContactList);

        let mut agent: Agent = client_builder.build_impl(client).unwrap();

        while let Some(events) = agent.wait_for_events().await {
            assert!(match events[0] {
                Event::Disconnected => true,
                _ => false,
            });
            assert_eq!(events.len(), 1);
            break;
        }
    }
}
