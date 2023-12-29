// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::{
    parsers::{
        message::{Body, Message, MessageType},
        muc::user::MucUser,
    },
    BareJid, Jid,
};

use crate::{Agent, RoomNick};

pub async fn send_room_private_message(
    agent: &mut Agent,
    room: BareJid,
    recipient: RoomNick,
    lang: &str,
    text: &str,
) {
    let recipient: Jid = room.with_resource_str(&recipient).unwrap().into();
    let mut message = Message::new(recipient).with_payload(MucUser::new());
    message.type_ = MessageType::Chat;
    message
        .bodies
        .insert(String::from(lang), Body(String::from(text)));
    let _ = agent.client.send_stanza(message.into()).await;
}
