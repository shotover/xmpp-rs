// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::parsers::iq::{Iq, IqType};

use crate::{Agent, Event};

pub mod get;
pub mod result;
pub mod set;

pub async fn handle_iq<C: ServerConnector>(agent: &mut Agent<C>, iq: Iq) -> Vec<Event> {
    let mut events = vec![];
    let from = iq
        .from
        .clone()
        .unwrap_or_else(|| agent.client.bound_jid().unwrap().to_bare().into());
    if let IqType::Get(payload) = iq.payload {
        get::handle_iq_get(agent, &mut events, from, iq.to, iq.id, payload).await;
    } else if let IqType::Result(Some(payload)) = iq.payload {
        result::handle_iq_result(agent, &mut events, from, iq.to, iq.id, payload).await;
    } else if let IqType::Set(payload) = iq.payload {
        set::handle_iq_set(agent, &mut events, from, iq.to, iq.id, payload).await;
    }
    events
}
