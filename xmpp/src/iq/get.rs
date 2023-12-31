// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    parsers::{
        disco::DiscoInfoQuery,
        iq::Iq,
        ns,
        stanza_error::{DefinedCondition, ErrorType, StanzaError},
    },
    Element, Jid,
};

use crate::{Agent, Event};

pub async fn handle_iq_get<C: ServerConnector>(
    agent: &mut Agent<C>,
    _events: &mut Vec<Event>,
    from: Jid,
    _to: Option<Jid>,
    id: String,
    payload: Element,
) {
    if payload.is("query", ns::DISCO_INFO) {
        let query = DiscoInfoQuery::try_from(payload);
        match query {
            Ok(query) => {
                let mut disco_info = agent.disco.clone();
                disco_info.node = query.node;
                let iq = Iq::from_result(id, Some(disco_info)).with_to(from).into();
                let _ = agent.client.send_stanza(iq).await;
            }
            Err(err) => {
                let error = StanzaError::new(
                    ErrorType::Modify,
                    DefinedCondition::BadRequest,
                    "en",
                    &format!("{}", err),
                );
                let iq = Iq::from_error(id, error).with_to(from).into();
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
        let iq = Iq::from_error(id, error).with_to(from).into();
        let _ = agent.client.send_stanza(iq).await;
    }
}
