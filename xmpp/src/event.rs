// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::parsers::{bookmarks2, message::Body, roster::Item as RosterItem, BareJid, Jid};

use crate::{Error, Id, RoomNick};

#[derive(Debug)]
pub enum Event {
    Online,
    Disconnected(Error),
    ContactAdded(RosterItem),
    ContactRemoved(RosterItem),
    ContactChanged(RosterItem),
    #[cfg(feature = "avatars")]
    AvatarRetrieved(Jid, String),
    ChatMessage(Id, BareJid, Body),
    JoinRoom(BareJid, bookmarks2::Conference),
    LeaveRoom(BareJid),
    LeaveAllRooms,
    RoomJoined(BareJid),
    RoomLeft(BareJid),
    RoomMessage(Id, BareJid, RoomNick, Body),
    /// A private message received from a room, containing the message ID, the room's BareJid,
    /// the sender's nickname, and the message body.
    RoomPrivateMessage(Id, BareJid, RoomNick, Body),
    ServiceMessage(Id, BareJid, Body),
    HttpUploadedFile(String),
}
