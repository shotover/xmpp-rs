// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio_xmpp::connect::ServerConnector;
pub use tokio_xmpp::parsers;
use tokio_xmpp::parsers::{disco::DiscoInfoResult, message::MessageType};
pub use tokio_xmpp::{AsyncClient as TokioXmppClient, BareJid, Element, FullJid, Jid};

use crate::{event_loop, message, muc, upload, Error, Event, RoomNick};

pub struct Agent<C: ServerConnector> {
    pub(crate) client: TokioXmppClient<C>,
    pub(crate) default_nick: Arc<RwLock<String>>,
    pub(crate) lang: Arc<Vec<String>>,
    pub(crate) disco: DiscoInfoResult,
    pub(crate) node: String,
    pub(crate) uploads: Vec<(String, Jid, PathBuf)>,
    pub(crate) awaiting_disco_bookmarks_type: bool,
}

impl<C: ServerConnector> Agent<C> {
    pub async fn disconnect(&mut self) -> Result<(), Error> {
        self.client.send_end().await
    }

    pub async fn join_room(
        &mut self,
        room: BareJid,
        nick: Option<String>,
        password: Option<String>,
        lang: &str,
        status: &str,
    ) {
        muc::room::join_room(self, room, nick, password, lang, status).await
    }

    /// Send a "leave room" request to the server (specifically, an "unavailable" presence stanza).
    ///
    /// The returned future will resolve when the request has been sent,
    /// not when the room has actually been left.
    ///
    /// If successful, a `RoomLeft` event should be received later as a confirmation.
    ///
    /// See: https://xmpp.org/extensions/xep-0045.html#exit
    ///
    /// Note that this method does NOT remove the room from the auto-join list; the latter
    /// is more a list of bookmarks that the account knows about and that have a flag set
    /// to indicate that they should be joined automatically after connecting (see the JoinRoom event).
    ///
    /// Regarding the latter, see the these minutes about auto-join behavior:
    /// https://docs.modernxmpp.org/meetings/2019-01-brussels/#bookmarks
    ///
    /// # Arguments
    ///
    /// * `room_jid`: The JID of the room to leave.
    /// * `nickname`: The nickname to use in the room.
    /// * `lang`: The language of the status message.
    /// * `status`: The status message to send.
    pub async fn leave_room(
        &mut self,
        room_jid: BareJid,
        nickname: RoomNick,
        lang: impl Into<String>,
        status: impl Into<String>,
    ) {
        muc::room::leave_room(self, room_jid, nickname, lang, status).await
    }

    pub async fn send_message(
        &mut self,
        recipient: Jid,
        type_: MessageType,
        lang: &str,
        text: &str,
    ) {
        message::send::send_message(self, recipient, type_, lang, text).await
    }

    pub async fn send_room_private_message(
        &mut self,
        room: BareJid,
        recipient: RoomNick,
        lang: &str,
        text: &str,
    ) {
        muc::private_message::send_room_private_message(self, room, recipient, lang, text).await
    }

    /// Wait for new events.
    ///
    /// # Returns
    ///
    /// - `Some(events)` if there are new events; multiple may be returned at once.
    /// - `None` if the underlying stream is closed.
    pub async fn wait_for_events(&mut self) -> Option<Vec<Event>> {
        event_loop::wait_for_events(self).await
    }

    pub async fn upload_file_with(&mut self, service: &str, path: &Path) {
        upload::send::upload_file_with(self, service, path).await
    }
}
