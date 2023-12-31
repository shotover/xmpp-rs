// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    parsers::{ns, private::Query as PrivateXMLQuery, roster::Roster},
    Element, Jid,
};

use crate::{disco, pubsub, upload, Agent, Event};

pub async fn handle_iq_result<C: ServerConnector>(
    agent: &mut Agent<C>,
    events: &mut Vec<Event>,
    from: Jid,
    _to: Option<Jid>,
    id: String,
    payload: Element,
) {
    // TODO: move private iqs like this one somewhere else, for
    // security reasons.
    if payload.is("query", ns::ROSTER) && from == agent.client.bound_jid().unwrap().to_bare() {
        let roster = Roster::try_from(payload).unwrap();
        for item in roster.items.into_iter() {
            events.push(Event::ContactAdded(item));
        }
    } else if payload.is("pubsub", ns::PUBSUB) {
        let new_events = pubsub::handle_iq_result(&from, payload);
        events.extend(new_events);
    } else if payload.is("slot", ns::HTTP_UPLOAD) {
        let new_events = upload::receive::handle_upload_result(&from, id, payload, agent).await;
        events.extend(new_events);
    } else if payload.is("query", ns::PRIVATE) {
        match PrivateXMLQuery::try_from(payload) {
            Ok(query) => {
                for conf in query.storage.conferences {
                    let (jid, room) = conf.into_bookmarks2();
                    events.push(Event::JoinRoom(jid, room));
                }
            }
            Err(e) => {
                panic!("Wrong XEP-0048 v1.0 Bookmark format: {}", e);
            }
        }
    } else if payload.is("query", ns::DISCO_INFO) {
        disco::handle_disco_info_result_payload(agent, payload, from).await;
    }
}
