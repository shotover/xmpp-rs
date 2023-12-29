// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[derive(PartialEq)]
pub enum ClientFeature {
    #[cfg(feature = "avatars")]
    Avatars,
    ContactList,
    JoinRooms,
}
