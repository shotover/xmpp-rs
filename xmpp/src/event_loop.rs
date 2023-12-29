// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use futures::StreamExt;
use tokio_xmpp::{
    parsers::{
        disco::DiscoInfoQuery, iq::Iq, message::Message, presence::Presence, roster::Roster,
    },
    Event as TokioXmppEvent,
};

use crate::{iq, message, presence, Agent, Event};

/// Wait for new events.
///
/// # Returns
///
/// - `Some(events)` if there are new events; multiple may be returned at once.
/// - `None` if the underlying stream is closed.
pub async fn wait_for_events(agent: &mut Agent) -> Option<Vec<Event>> {
    if let Some(event) = agent.client.next().await {
        let mut events = Vec::new();

        match event {
            TokioXmppEvent::Online { resumed: false, .. } => {
                let presence = Agent::make_initial_presence(&agent.disco, &agent.node).into();
                let _ = agent.client.send_stanza(presence).await;
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
                let _ = agent.client.send_stanza(iq).await;

                // Query account disco to know what bookmarks spec is used
                let iq = Iq::from_get("disco-account", DiscoInfoQuery { node: None }).into();
                let _ = agent.client.send_stanza(iq).await;
                agent.awaiting_disco_bookmarks_type = true;
            }
            TokioXmppEvent::Online { resumed: true, .. } => {}
            TokioXmppEvent::Disconnected(e) => {
                events.push(Event::Disconnected(e));
            }
            TokioXmppEvent::Stanza(elem) => {
                if elem.is("iq", "jabber:client") {
                    let iq = Iq::try_from(elem).unwrap();
                    let new_events = iq::handle_iq(agent, iq).await;
                    events.extend(new_events);
                } else if elem.is("message", "jabber:client") {
                    let message = Message::try_from(elem).unwrap();
                    let new_events = message::handle_message(agent, message).await;
                    events.extend(new_events);
                } else if elem.is("presence", "jabber:client") {
                    let presence = Presence::try_from(elem).unwrap();
                    let new_events = presence::handle_presence(agent, presence).await;
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
