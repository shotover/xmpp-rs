// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    parsers::{
        muc::Muc,
        presence::{Presence, Type as PresenceType},
    },
    BareJid,
};

use crate::{Agent, RoomNick};

pub async fn join_room<C: ServerConnector>(
    agent: &mut Agent<C>,
    room: BareJid,
    nick: Option<String>,
    password: Option<String>,
    lang: &str,
    status: &str,
) {
    let mut muc = Muc::new();
    if let Some(password) = password {
        muc = muc.with_password(password);
    }

    let nick = nick.unwrap_or_else(|| agent.default_nick.read().unwrap().clone());
    let room_jid = room.with_resource_str(&nick).unwrap();
    let mut presence = Presence::new(PresenceType::None).with_to(room_jid);
    presence.add_payload(muc);
    presence.set_status(String::from(lang), String::from(status));
    let _ = agent.client.send_stanza(presence.into()).await;
}

/// Send a "leave room" request to the server (specifically, an "unavailable" presence stanza).
///
/// The returned future will resolve when the request has been sent,
/// not when the room has actually been left.
///
/// If successful, a `RoomLeft` event should be received later as a confirmation. See [XEP-0045](https://xmpp.org/extensions/xep-0045.html#exit).
///
/// Note that this method does NOT remove the room from the auto-join list; the latter
/// is more a list of bookmarks that the account knows about and that have a flag set
/// to indicate that they should be joined automatically after connecting (see the JoinRoom event).
///
/// Regarding the latter, see the these [ModernXMPP minutes about auto-join behavior](https://docs.modernxmpp.org/meetings/2019-01-brussels/#bookmarks).
///
/// # Arguments
///
/// * `room_jid`: The JID of the room to leave.
/// * `nickname`: The nickname to use in the room.
/// * `lang`: The language of the status message.
/// * `status`: The status message to send.
pub async fn leave_room<C: ServerConnector>(
    agent: &mut Agent<C>,
    room_jid: BareJid,
    nickname: RoomNick,
    lang: impl Into<String>,
    status: impl Into<String>,
) {
    // XEP-0045 specifies that, to leave a room, the client must send a presence stanza
    // with type="unavailable".
    let mut presence = Presence::new(PresenceType::Unavailable).with_to(
        room_jid
            .with_resource_str(nickname.as_str())
            .expect("Invalid room JID after adding resource part."),
    );

    // Optionally, the client may include a status message in the presence stanza.
    // TODO: Should this be optional? The XEP says "MAY", but the method signature requires the arguments.
    // XEP-0045: "The occupant MAY include normal <status/> information in the unavailable presence stanzas"
    presence.set_status(lang, status);

    // Send the presence stanza.
    if let Err(e) = agent.client.send_stanza(presence.into()).await {
        // Report any errors to the log.
        error!("Failed to send leave room presence: {}", e);
    }
}
