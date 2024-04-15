// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    parsers::{message::Message, muc::user::MucUser},
    Jid,
};

use crate::{delay::StanzaTimeInfo, Agent, Event};

pub async fn handle_message_chat<C: ServerConnector>(
    agent: &mut Agent<C>,
    events: &mut Vec<Event>,
    from: Jid,
    message: &Message,
    time_info: StanzaTimeInfo,
) {
    let langs: Vec<&str> = agent.lang.iter().map(String::as_str).collect();
    if let Some((_lang, body)) = message.get_best_body(langs) {
        let mut found_special_message = false;

        for payload in &message.payloads {
            if let Ok(_) = MucUser::try_from(payload.clone()) {
                let event = match from.clone().try_into_full() {
                    Err(bare) => {
                        // TODO: Can a service message be of type Chat/Normal and not Groupchat?
                        warn!("Received misformed MessageType::Chat in muc#user namespace from a bare JID.");
                        Event::ServiceMessage(
                            message.id.clone(),
                            bare,
                            body.clone(),
                            time_info.clone(),
                        )
                    }
                    Ok(full) => Event::RoomPrivateMessage(
                        message.id.clone(),
                        full.to_bare(),
                        full.resource().to_string(),
                        body.clone(),
                        time_info.clone(),
                    ),
                };

                found_special_message = true;
                events.push(event);
            }
        }

        if !found_special_message {
            let event =
                Event::ChatMessage(message.id.clone(), from.to_bare(), body.clone(), time_info);
            events.push(event);
        }
    }
}
