// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::{
    parsers::{
        iq::Iq,
        stanza_error::{DefinedCondition, ErrorType, StanzaError},
    },
    Element, Jid,
};

use crate::{Agent, Event};

pub async fn handle_iq_set(
    agent: &mut Agent,
    _events: &mut Vec<Event>,
    from: Jid,
    _to: Option<Jid>,
    id: String,
    _payload: Element,
) {
    // We MUST answer unhandled set iqs with a service-unavailable error.
    let error = StanzaError::new(
        ErrorType::Cancel,
        DefinedCondition::ServiceUnavailable,
        "en",
        "No handler defined for this kind of iq.",
    );
    let iq = Iq::from_error(id, error).with_to(from).into();
    let _ = agent.client.send_stanza(iq).await;
}
