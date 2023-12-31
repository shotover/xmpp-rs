// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    parsers::message::{Body, Message, MessageType},
    Jid,
};

use crate::Agent;

pub async fn send_message<C: ServerConnector>(
    agent: &mut Agent<C>,
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
    let _ = agent.client.send_stanza(message.into()).await;
}
