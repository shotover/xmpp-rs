// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::{parsers::message::Message, Jid};

use crate::{Agent, Event};

pub async fn handle_message_group_chat(
    agent: &mut Agent,
    events: &mut Vec<Event>,
    from: Jid,
    message: &Message,
) {
    let langs: Vec<&str> = agent.lang.iter().map(String::as_str).collect();
    if let Some((_lang, body)) = message.get_best_body(langs) {
        let event = match from.clone() {
            Jid::Full(full) => Event::RoomMessage(
                message.id.clone(),
                from.to_bare(),
                full.resource_str().to_owned(),
                body.clone(),
            ),
            Jid::Bare(bare) => Event::ServiceMessage(message.id.clone(), bare, body.clone()),
        };
        events.push(event)
    }
}
