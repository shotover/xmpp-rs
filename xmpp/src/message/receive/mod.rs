// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::parsers::{
    message::{Message, MessageType},
    ns,
};

use crate::{delay::message_time_info, pubsub, Agent, Event};

pub mod chat;
pub mod group_chat;

pub async fn handle_message<C: ServerConnector>(
    agent: &mut Agent<C>,
    message: Message,
) -> Vec<Event> {
    let mut events = vec![];
    let from = message.from.clone().unwrap();
    let time_info = message_time_info(&message);

    match message.type_ {
        MessageType::Groupchat => {
            group_chat::handle_message_group_chat(
                agent,
                &mut events,
                from.clone(),
                &message,
                time_info,
            )
            .await;
        }
        MessageType::Chat | MessageType::Normal => {
            chat::handle_message_chat(agent, &mut events, from.clone(), &message, time_info).await;
        }
        _ => {}
    }

    for child in message.payloads {
        if child.is("event", ns::PUBSUB_EVENT) {
            let new_events = pubsub::handle_event(&from, child, agent).await;
            events.extend(new_events);
        }
    }

    events
}
