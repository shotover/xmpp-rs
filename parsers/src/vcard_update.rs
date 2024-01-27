// Copyright (c) 2024 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! This module implements vCard avatar updates defined in
//! [XEP-0153](https://xmpp.org/extensions/xep-0153.html).
//!
//! Specifically, as it appears in `<presence>` stanzas, as shown in [XEP-0153 example 3](https://xmpp.org/extensions/xep-0153.html#example-3).
//!
//! For [XEP-0054](https://xmpp.org/extensions/xep-0054.html) vCard support,
//! see [`vcard`][crate::vcard] module.

use crate::util::text_node_codecs::{Codec, FixedHex, OptionalCodec};

generate_element!(
    /// The presence payload for an avatar VCard update
    VCardUpdate, "x", VCARD_UPDATE,
    children: [
        /// The photo element. Is empty if "a client is not yet ready to advertise an image".
        photo: Option<Photo> = ("photo", VCARD_UPDATE) => Photo,
    ]
);

generate_element!(
    /// The photo element containing the avatar metadata
    Photo, "photo", VCARD_UPDATE,
    text: (
        /// The SHA1 hash of the avatar. Empty when there is no photo.
        data: OptionalCodec<FixedHex<20>>
    )
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Element;
    use std::str::FromStr;

    #[test]
    fn test_vcard_update() {
        // Test xml stolen from https://xmpp.org/extensions/xep-0153.html#example-3
        // Changes: I did set the last d to uppercase to try to trip up a potentially case-sensitive parser.
        let test_vcard = r"<x xmlns='vcard-temp:x:update'>
    <photo>01b87fcd030b72895ff8e88db57ec525450f000D</photo>
  </x>";

        let test_vcard = Element::from_str(&test_vcard).expect("Failed to parse XML");
        let test_vcard = VCardUpdate::try_from(test_vcard).expect("Failed to parse vCardUpdate");

        let photo = test_vcard.photo.expect("No photo found");

        assert_eq!(
            photo.data,
            Some([
                0x01, 0xb8, 0x7f, 0xcd, 0x03, 0x0b, 0x72, 0x89, 0x5f, 0xf8, 0xe8, 0x8d, 0xb5, 0x7e,
                0xc5, 0x25, 0x45, 0x0f, 0x00, 0x0d
            ])
        );
    }

    #[test]
    fn test_vcard_update_empty() {
        // Test xml stolen from https://xmpp.org/extensions/xep-0153.html#example-7
        let test_vcard = r"<x xmlns='vcard-temp:x:update'><photo/></x>";

        let test_vcard = Element::from_str(&test_vcard).expect("Failed to parse XML");
        let test_vcard = VCardUpdate::try_from(test_vcard).expect("Failed to parse vCardUpdate");

        let photo = test_vcard.photo.expect("No photo found");

        assert_eq!(photo.data, None)
    }
}
