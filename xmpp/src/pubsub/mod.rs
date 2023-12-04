// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::Agent;
use crate::Event;
use std::convert::TryFrom;
use std::str::FromStr;
use tokio_xmpp::parsers::{
    bookmarks2::{Autojoin, Conference},
    ns,
    pubsub::event::PubSubEvent,
    pubsub::pubsub::PubSub,
    BareJid, Element, Jid,
};

#[cfg(feature = "avatars")]
pub(crate) mod avatar;

pub(crate) async fn handle_event(from: &Jid, elem: Element, agent: &mut Agent) -> Vec<Event> {
    let mut events = Vec::new();
    let event = PubSubEvent::try_from(elem);
    trace!("PubSub event: {:#?}", event);
    match event {
        Ok(PubSubEvent::PublishedItems { node, items }) => {
            match node.0 {
                #[cfg(feature = "avatars")]
                ref node if node == ns::AVATAR_METADATA => {
                    let new_events =
                        avatar::handle_metadata_pubsub_event(&from, agent, items).await;
                    events.extend(new_events);
                }
                ref node if node == ns::BOOKMARKS2 => {
                    // TODO: Check that our bare JID is the sender.
                    assert_eq!(items.len(), 1);
                    let item = items.clone().pop().unwrap();
                    let jid = BareJid::from_str(&item.id.clone().unwrap().0).unwrap();
                    let payload = item.payload.clone().unwrap();
                    match Conference::try_from(payload) {
                        Ok(conference) => {
                            if conference.autojoin == Autojoin::True {
                                events.push(Event::JoinRoom(jid, conference));
                            } else {
                                events.push(Event::LeaveRoom(jid));
                            }
                        }
                        Err(err) => println!("not bookmark: {}", err),
                    }
                }
                ref node => unimplemented!("node {}", node),
            }
        }
        Ok(PubSubEvent::RetractedItems { node, items }) => {
            match node.0 {
                ref node if node == ns::BOOKMARKS2 => {
                    // TODO: Check that our bare JID is the sender.
                    assert_eq!(items.len(), 1);
                    let item = items.clone().pop().unwrap();
                    let jid = BareJid::from_str(&item.0).unwrap();
                    events.push(Event::LeaveRoom(jid));
                }
                ref node => unimplemented!("node {}", node),
            }
        }
        Ok(PubSubEvent::Purge { node }) => {
            match node.0 {
                ref node if node == ns::BOOKMARKS2 => {
                    // TODO: Check that our bare JID is the sender.
                    events.push(Event::LeaveAllRooms);
                }
                ref node => unimplemented!("node {}", node),
            }
        }
        Err(e) => {
            error!("Error parsing PubSub event: {}", e);
        }
        _ => unimplemented!("PubSub event: {:#?}", event),
    }
    events
}

pub(crate) fn handle_iq_result(from: &Jid, elem: Element) -> impl IntoIterator<Item = Event> {
    let mut events = Vec::new();
    let pubsub = PubSub::try_from(elem).unwrap();
    trace!("PubSub: {:#?}", pubsub);
    if let PubSub::Items(items) = pubsub {
        match items.node.0.clone() {
            #[cfg(feature = "avatars")]
            ref node if node == ns::AVATAR_DATA => {
                let new_events = avatar::handle_data_pubsub_iq(&from, &items);
                events.extend(new_events);
            }
            ref node if node == ns::BOOKMARKS2 => {
                events.push(Event::LeaveAllRooms);
                for item in items.items {
                    let item = item.0;
                    let jid = BareJid::from_str(&item.id.clone().unwrap().0).unwrap();
                    let payload = item.payload.clone().unwrap();
                    match Conference::try_from(payload) {
                        Ok(conference) => {
                            if let Autojoin::True = conference.autojoin {
                                events.push(Event::JoinRoom(jid, conference));
                            }
                        }
                        Err(err) => panic!("Wrong payload type in bookmarks 2 item: {}", err),
                    }
                }
            }
            _ => unimplemented!(),
        }
    }
    events
}
