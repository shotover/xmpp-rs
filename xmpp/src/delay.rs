// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use chrono::{DateTime, Utc};
use tokio_xmpp::{
    parsers::{delay::Delay, message::Message, ns},
    Jid,
};

/// Time information associated with a stanza.
///
/// Contains information about when the message was received, and any claim about when it was sent.
#[derive(Debug, Clone)]
pub struct StanzaTimeInfo {
    /// Time information when the message was received by the library
    pub received: DateTime<Utc>,

    /// Time information claimed by the sender or an intermediary.
    ///
    /// **Warning**: this has security implications. See [XEP-0203 security section](https://xmpp.org/extensions/xep-0203.html#security).
    pub delays: Vec<Delay>,
}

impl StanzaTimeInfo {
    pub fn delay_from(&self, jid: &Jid) -> Option<&Delay> {
        self.delays.iter().find(|delay| {
            if let Some(from) = &delay.from {
                return from == jid;
            }
            return false;
        })
    }
}

/// Parsing a [`Message`], store the current time it was processed, as well [XEP-0203](https://xmpp.org/extensions/xep-0203.html#protocol)
/// [`Delay`] contained in the message's payloads.
///
/// Specifically, this method will look for any <delay/> element in the message's payloads. If they were found,
/// they will be added to the [`StanzaTimeInfo`] result.
pub fn message_time_info(message: &Message) -> StanzaTimeInfo {
    let mut delays = vec![];

    // Scan the message payloads for XEP-0203 delays.
    for payload in &message.payloads {
        if payload.is("delay", ns::DELAY) {
            match Delay::try_from(payload.clone()) {
                Ok(delay) => delays.push(delay),
                Err(e) => {
                    error!("Wrong <delay> format in payload from {}:{}\n{:?}\nUsing received time only.",
                    message.from.as_ref().unwrap().to_owned(),
                    e,
                    payload);
                }
            }
        }
    }

    StanzaTimeInfo {
        received: Utc::now(),
        delays,
    }
}
