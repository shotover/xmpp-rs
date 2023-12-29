// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::parsers::{
    disco::DiscoInfoQuery,
    iq::{Iq, IqType},
    ns,
    private::Query as PrivateXMLQuery,
    roster::Roster,
    stanza_error::{DefinedCondition, ErrorType, StanzaError},
};

use crate::{disco, pubsub, upload, Agent, Event};

pub async fn handle_iq(agent: &mut Agent, iq: Iq) -> Vec<Event> {
    let mut events = vec![];
    let from = iq
        .from
        .clone()
        .unwrap_or_else(|| agent.client.bound_jid().unwrap().clone());
    if let IqType::Get(payload) = iq.payload {
        if payload.is("query", ns::DISCO_INFO) {
            let query = DiscoInfoQuery::try_from(payload);
            match query {
                Ok(query) => {
                    let mut disco_info = agent.disco.clone();
                    disco_info.node = query.node;
                    let iq = Iq::from_result(iq.id, Some(disco_info))
                        .with_to(iq.from.unwrap())
                        .into();
                    let _ = agent.client.send_stanza(iq).await;
                }
                Err(err) => {
                    let error = StanzaError::new(
                        ErrorType::Modify,
                        DefinedCondition::BadRequest,
                        "en",
                        &format!("{}", err),
                    );
                    let iq = Iq::from_error(iq.id, error)
                        .with_to(iq.from.unwrap())
                        .into();
                    let _ = agent.client.send_stanza(iq).await;
                }
            }
        } else {
            // We MUST answer unhandled get iqs with a service-unavailable error.
            let error = StanzaError::new(
                ErrorType::Cancel,
                DefinedCondition::ServiceUnavailable,
                "en",
                "No handler defined for this kind of iq.",
            );
            let iq = Iq::from_error(iq.id, error)
                .with_to(iq.from.unwrap())
                .into();
            let _ = agent.client.send_stanza(iq).await;
        }
    } else if let IqType::Result(Some(payload)) = iq.payload {
        // TODO: move private iqs like this one somewhere else, for
        // security reasons.
        if payload.is("query", ns::ROSTER) && Some(from.clone()) == iq.from {
            let roster = Roster::try_from(payload).unwrap();
            for item in roster.items.into_iter() {
                events.push(Event::ContactAdded(item));
            }
        } else if payload.is("pubsub", ns::PUBSUB) {
            let new_events = pubsub::handle_iq_result(&from, payload);
            events.extend(new_events);
        } else if payload.is("slot", ns::HTTP_UPLOAD) {
            let new_events = upload::handle_upload_result(&from, iq.id, payload, agent).await;
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
    } else if let IqType::Set(_) = iq.payload {
        // We MUST answer unhandled set iqs with a service-unavailable error.
        let error = StanzaError::new(
            ErrorType::Cancel,
            DefinedCondition::ServiceUnavailable,
            "en",
            "No handler defined for this kind of iq.",
        );
        let iq = Iq::from_error(iq.id, error)
            .with_to(iq.from.unwrap())
            .into();
        let _ = agent.client.send_stanza(iq).await;
    }
    events
}
