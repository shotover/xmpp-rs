// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::parsers::{
    muc::user::{MucUser, Status},
    presence::{Presence, Type as PresenceType},
};

use crate::{Agent, Event};

/// Translate a `Presence` stanza into a list of higher-level `Event`s.
pub async fn handle_presence(_agent: &mut Agent, presence: Presence) -> Vec<Event> {
    // Allocate an empty vector to store the events.
    let mut events = vec![];

    // Extract the JID of the sender (i.e. the one whose presence is being sent).
    let from = presence.from.unwrap().to_bare();

    // Search through the payloads for a MUC user status.

    if let Some(muc) = presence
        .payloads
        .iter()
        .filter_map(|p| MucUser::try_from(p.clone()).ok())
        .next()
    {
        // If a MUC user status was found, search through the statuses for a self-presence.
        if muc.status.iter().any(|s| *s == Status::SelfPresence) {
            // If a self-presence was found, then the stanza is about the client's own presence.

            match presence.type_ {
                PresenceType::None => {
                    // According to https://xmpp.org/extensions/xep-0045.html#enter-pres, no type should be seen as "available".
                    events.push(Event::RoomJoined(from.clone()));
                }
                PresenceType::Unavailable => {
                    // According to https://xmpp.org/extensions/xep-0045.html#exit, the server will use type "unavailable" to notify the client that it has left the room/
                    events.push(Event::RoomLeft(from.clone()));
                }
                _ => unimplemented!("Presence type {:?}", presence.type_), // TODO: What to do here?
            }
        }
    }

    // Return the list of events.
    events
}
