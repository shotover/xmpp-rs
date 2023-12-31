// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::sync::{Arc, RwLock};
use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    parsers::{
        disco::{DiscoInfoResult, Feature, Identity},
        ns,
    },
    AsyncClient as TokioXmppClient, AsyncConfig, BareJid, Jid,
};

use crate::{Agent, ClientFeature};

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

pub struct ClientBuilder<'a, C: ServerConnector> {
    jid: BareJid,
    password: &'a str,
    server_connector: C,
    website: String,
    default_nick: String,
    lang: Vec<String>,
    disco: (ClientType, String),
    features: Vec<ClientFeature>,
    resource: Option<String>,
}

#[cfg(any(feature = "starttls-rust", feature = "starttls-native"))]
impl ClientBuilder<'_, tokio_xmpp::starttls::ServerConfig> {
    pub fn new<'a>(
        jid: BareJid,
        password: &'a str,
    ) -> ClientBuilder<'a, tokio_xmpp::starttls::ServerConfig> {
        Self::new_with_server(jid, password, tokio_xmpp::starttls::ServerConfig::UseSrv)
    }
}

impl<C: ServerConnector> ClientBuilder<'_, C> {
    pub fn new_with_server<'a>(
        jid: BareJid,
        password: &'a str,
        server_connector: C,
    ) -> ClientBuilder<'a, C> {
        ClientBuilder {
            jid,
            password,
            server_connector,
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

    pub fn build(self) -> Agent<C> {
        let jid: Jid = if let Some(resource) = &self.resource {
            self.jid.with_resource_str(resource).unwrap().into()
        } else {
            self.jid.clone().into()
        };

        let config = AsyncConfig {
            jid,
            password: self.password.into(),
            server: self.server_connector.clone(),
        };
        let client = TokioXmppClient::new_with_config(config);
        self.build_impl(client)
    }

    // This function is meant to be used for testing build
    pub(crate) fn build_impl(self, client: TokioXmppClient<C>) -> Agent<C> {
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
