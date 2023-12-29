// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::parsers::{
    caps::{compute_disco, hash_caps, Caps},
    disco::DiscoInfoResult,
    hashes::Algo,
    presence::{Presence, Type as PresenceType},
};

pub(crate) fn make_initial_presence(disco: &DiscoInfoResult, node: &str) -> Presence {
    let caps_data = compute_disco(disco);
    let hash = hash_caps(&caps_data, Algo::Sha_1).unwrap();
    let caps = Caps::new(node, hash);

    let mut presence = Presence::new(PresenceType::None);
    presence.add_payload(caps);
    presence
}
