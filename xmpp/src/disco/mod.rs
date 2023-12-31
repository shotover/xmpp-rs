// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    parsers::{
        bookmarks,
        disco::{DiscoInfoResult, Feature},
        iq::Iq,
        ns,
        private::Query as PrivateXMLQuery,
        pubsub::pubsub::{Items, PubSub},
        Error as ParsersError,
    },
    Element, Jid,
};

use crate::Agent;

// This method is a workaround due to prosody bug https://issues.prosody.im/1664
// FIXME: To be removed in the future
// The server doesn't return disco#info feature when querying the account
// so we add it manually because we know it's true
pub async fn handle_disco_info_result_payload<C: ServerConnector>(
    agent: &mut Agent<C>,
    payload: Element,
    from: Jid,
) {
    match DiscoInfoResult::try_from(payload.clone()) {
        Ok(disco) => {
            handle_disco_info_result(agent, disco, from).await;
        }
        Err(e) => match e {
            ParsersError::ParseError(reason) => {
                if reason == "disco#info feature not present in disco#info." {
                    let mut payload = payload.clone();
                    let disco_feature =
                        Feature::new("http://jabber.org/protocol/disco#info").into();
                    payload.append_child(disco_feature);
                    match DiscoInfoResult::try_from(payload) {
                        Ok(disco) => {
                            handle_disco_info_result(agent, disco, from).await;
                        }
                        Err(e) => {
                            panic!("Wrong disco#info format after workaround: {}", e)
                        }
                    }
                } else {
                    panic!(
                        "Wrong disco#info format (reason cannot be worked around): {}",
                        e
                    )
                }
            }
            _ => panic!("Wrong disco#info format: {}", e),
        },
    }
}

pub async fn handle_disco_info_result<C: ServerConnector>(
    agent: &mut Agent<C>,
    disco: DiscoInfoResult,
    from: Jid,
) {
    // Safe unwrap because no DISCO is received when we are not online
    if from == agent.client.bound_jid().unwrap().to_bare() && agent.awaiting_disco_bookmarks_type {
        info!("Received disco info about bookmarks type");
        // Trigger bookmarks query
        // TODO: only send this when the JoinRooms feature is enabled.
        agent.awaiting_disco_bookmarks_type = false;
        let mut perform_bookmarks2 = false;
        info!("{:#?}", disco.features);
        for feature in disco.features {
            if feature.var == "urn:xmpp:bookmarks:1#compat" {
                perform_bookmarks2 = true;
            }
        }

        if perform_bookmarks2 {
            // XEP-0402 bookmarks (modern)
            let iq = Iq::from_get("bookmarks", PubSub::Items(Items::new(ns::BOOKMARKS2))).into();
            let _ = agent.client.send_stanza(iq).await;
        } else {
            // XEP-0048 v1.0 bookmarks (legacy)
            let iq = Iq::from_get(
                "bookmarks-legacy",
                PrivateXMLQuery {
                    storage: bookmarks::Storage::new(),
                },
            )
            .into();
            let _ = agent.client.send_stanza(iq).await;
        }
    } else {
        unimplemented!("Ignored disco#info response from {}", from);
    }
}
