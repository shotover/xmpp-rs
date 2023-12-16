// Copyright (c) 2018 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//!
//! Chatroom bookmarks from [XEP-0048 v1.0](https://xmpp.org/extensions/attic/xep-0048-1.0.html). Only use on older servers
//! which do not advertise `urn:xmpp:bookmarks:1#compat` on the user's BareJID in a disco info request.
//! On modern compliant servers, use the [`crate::bookmarks2`] module instead.
//!
//! See [ModernXMPP docs](https://docs.modernxmpp.org/client/groupchat/#bookmarks) on how to handle all historic
//! and newer specifications for your clients.
//!
//! This module exposes the [`Autojoin`][crate::bookmarks::Autojoin] boolean flag, the [`Conference`][crate::bookmarks::Conference] chatroom element, and the [`crate::ns::BOOKMARKS`] XML namespace.

use jid::BareJid;

pub use crate::bookmarks2::Autojoin;

generate_element!(
    /// A conference bookmark.
    Conference, "conference", BOOKMARKS,
    attributes: [
        /// Whether a conference bookmark should be joined automatically.
        autojoin: Default<Autojoin> = "autojoin",

        /// The JID of the conference.
        jid: Required<BareJid> = "jid",

        /// A user-defined name for this conference.
        name: Option<String> = "name",
    ],
    children: [
        /// The nick the user will use to join this conference.
        nick: Option<String> = ("nick", BOOKMARKS) => String,

        /// The password required to join this conference.
        password: Option<String> = ("password", BOOKMARKS) => String
    ]
);

impl Conference {
    /// Turns a XEP-0048 Conference element into a XEP-0402 "Bookmarks2" Conference element, in a
    /// tuple with the room JID.
    pub fn into_bookmarks2(self) -> (BareJid, crate::bookmarks2::Conference) {
        (
            self.jid,
            crate::bookmarks2::Conference {
                autojoin: self.autojoin,
                name: self.name,
                nick: self.nick,
                password: self.password,
                extensions: vec![],
            },
        )
    }
}

generate_element!(
    /// An URL bookmark.
    Url, "url", BOOKMARKS,
    attributes: [
        /// A user-defined name for this URL.
        name: Option<String> = "name",

        /// The URL of this bookmark.
        url: Required<String> = "url",
    ]
);

generate_element!(
    /// Container element for multiple bookmarks.
    #[derive(Default)]
    Storage, "storage", BOOKMARKS,
    children: [
        /// Conferences the user has expressed an interest in.
        conferences: Vec<Conference> = ("conference", BOOKMARKS) => Conference,

        /// URLs the user is interested in.
        urls: Vec<Url> = ("url", BOOKMARKS) => Url
    ]
);

impl Storage {
    /// Create an empty bookmarks storage.
    pub fn new() -> Storage {
        Storage::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Element;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(Conference, 56);
        assert_size!(Url, 24);
        assert_size!(Storage, 24);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(Conference, 112);
        assert_size!(Url, 48);
        assert_size!(Storage, 48);
    }

    #[test]
    fn empty() {
        let elem: Element = "<storage xmlns='storage:bookmarks'/>".parse().unwrap();
        let elem1 = elem.clone();
        let storage = Storage::try_from(elem).unwrap();
        assert_eq!(storage.conferences.len(), 0);
        assert_eq!(storage.urls.len(), 0);

        let elem2 = Element::from(Storage::new());
        assert_eq!(elem1, elem2);
    }

    #[test]
    fn complete() {
        let elem: Element = "<storage xmlns='storage:bookmarks'><url name='Example' url='https://example.org/'/><conference autojoin='true' jid='test-muc@muc.localhost' name='Test MUC'><nick>Coucou</nick><password>secret</password></conference></storage>".parse().unwrap();
        let storage = Storage::try_from(elem).unwrap();
        assert_eq!(storage.urls.len(), 1);
        assert_eq!(storage.urls[0].clone().name.unwrap(), "Example");
        assert_eq!(storage.urls[0].url, "https://example.org/");
        assert_eq!(storage.conferences.len(), 1);
        assert_eq!(storage.conferences[0].autojoin, Autojoin::True);
        assert_eq!(
            storage.conferences[0].jid,
            BareJid::new("test-muc@muc.localhost").unwrap()
        );
        assert_eq!(storage.conferences[0].clone().name.unwrap(), "Test MUC");
        assert_eq!(storage.conferences[0].clone().nick.unwrap(), "Coucou");
        assert_eq!(storage.conferences[0].clone().password.unwrap(), "secret");
    }
}
