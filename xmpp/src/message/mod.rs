// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::{
    parsers::{
        message::{Message, MessageType},
        muc::user::MucUser,
        ns,
    },
    Jid,
};

use crate::{pubsub, Agent, Event};

pub async fn handle_message(agent: &mut Agent, message: Message) -> Vec<Event> {
    let mut events = vec![];
    let from = message.from.clone().unwrap();
    let langs: Vec<&str> = agent.lang.iter().map(String::as_str).collect();
    match message.get_best_body(langs) {
        Some((_lang, body)) => match message.type_ {
            MessageType::Groupchat => {
                let event = match from.clone() {
                    Jid::Full(full) => Event::RoomMessage(
                        message.id.clone(),
                        from.to_bare(),
                        full.resource_str().to_owned(),
                        body.clone(),
                    ),
                    Jid::Bare(bare) => {
                        Event::ServiceMessage(message.id.clone(), bare, body.clone())
                    }
                };
                events.push(event)
            }
            MessageType::Chat | MessageType::Normal => {
                let mut found_special_message = false;

                for payload in &message.payloads {
                    if let Ok(_) = MucUser::try_from(payload.clone()) {
                        let event = match from.clone() {
                            Jid::Bare(bare) => {
                                // TODO: Can a service message be of type Chat/Normal and not Groupchat?
                                warn!("Received misformed MessageType::Chat in muc#user namespace from a bare JID.");
                                Event::ServiceMessage(message.id.clone(), bare, body.clone())
                            }
                            Jid::Full(full) => Event::RoomPrivateMessage(
                                message.id.clone(),
                                full.to_bare(),
                                full.resource_str().to_owned(),
                                body.clone(),
                            ),
                        };

                        found_special_message = true;
                        events.push(event);
                    }
                }

                if !found_special_message {
                    let event =
                        Event::ChatMessage(message.id.clone(), from.to_bare(), body.clone());
                    events.push(event)
                }
            }
            _ => (),
        },
        None => (),
    }
    for child in message.payloads {
        if child.is("event", ns::PUBSUB_EVENT) {
            let new_events = pubsub::handle_event(&from, child, agent).await;
            events.extend(new_events);
        }
    }

    events
}
