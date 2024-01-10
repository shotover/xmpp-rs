// Copyright (c) 2024 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! This module implements vCard, for the purpose of vCard-based avatars as defined in
//! [XEP-0054](https://xmpp.org/extensions/xep-0054.html).
//!
//! Only the <PHOTO> element is supported as a member of this legacy vCard. For more modern and complete
//! user profile management, see [XEP-0292](https://xmpp.org/extensions/xep-0292.html): vCard4 Over XMPP.

use crate::iq::{IqGetPayload, IqResultPayload, IqSetPayload};
use crate::util::text_node_codecs::{Base64, Codec, Text};
use crate::{ns, Error};
use minidom::Element;

generate_element!(
    /// A photo element.
    Photo, "PHOTO", VCARD,
    attributes: [],
    children: [
        /// The type of the photo.
        type_: Required<Type> = ("TYPE", VCARD) => Type,
        /// The binary data of the photo.
        binval: Required<Binval> = ("BINVAL", VCARD) => Binval,
    ]
);

generate_element!(
    /// The type of the photo.
    Type, "TYPE", VCARD,
    text: (
        /// The type as a plain text string; at least "image/jpeg", "image/gif" and "image/png" SHOULD be supported.
        data: Text
    )
);

generate_element!(
    /// The binary data of the photo.
    Binval, "BINVAL", VCARD,
    text: (
        /// The actual data.
        data: Base64
    )
);

/// A <vCard> element; only the <PHOTO> element is supported for this legacy vCard ; the rest is ignored.
pub struct VCard {
    /// A photo element.
    pub photo: Option<Photo>,
}

impl TryFrom<Element> for VCard {
    type Error = crate::util::error::Error;

    fn try_from(value: Element) -> Result<Self, Self::Error> {
        // Check that the root element is <vCard>
        if !value.is("vCard", ns::VCARD) {
            return Err(Error::ParseError(
                "Root element is not <vCard xmlns='vcard-temp'>",
            ));
        }

        // Parse the <PHOTO> element, if any.
        let photo = value
            .get_child("PHOTO", ns::VCARD)
            .map(|photo| Photo::try_from(photo.clone()))
            .transpose()?;

        // Return the result.
        Ok(VCard { photo })
    }
}

impl Into<Element> for VCard {
    fn into(self) -> Element {
        let mut builder = Element::builder("vCard", ns::VCARD);

        if let Some(photo) = self.photo {
            builder = builder.append(photo);
        }

        builder.build()
    }
}

impl IqGetPayload for VCard {}
impl IqSetPayload for VCard {}
impl IqResultPayload for VCard {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Element;
    use base64::Engine;
    use std::str::FromStr;

    #[test]
    fn test_vcard() {
        // Create some bytes:
        let bytes = [0u8, 1, 2, 129];

        // Test xml stolen from https://xmpp.org/extensions/xep-0153.html#example-5
        let test_vcard = format!(
            r"<vCard xmlns='vcard-temp'>
    <BDAY>1476-06-09</BDAY>
    <ADR>
      <CTRY>Italy</CTRY>
      <LOCALITY>Verona</LOCALITY>
      <HOME/>
    </ADR>
    <NICKNAME/>
    <N><GIVEN>Juliet</GIVEN><FAMILY>Capulet</FAMILY></N>
    <EMAIL>jcapulet@shakespeare.lit</EMAIL>
    <PHOTO>
      <TYPE>image/jpeg</TYPE>
      <BINVAL>{}</BINVAL>
    </PHOTO>
  </vCard>",
            base64::prelude::BASE64_STANDARD.encode(&bytes)
        );

        let test_vcard = Element::from_str(&test_vcard).expect("Failed to parse XML");
        let test_vcard = VCard::try_from(test_vcard).expect("Failed to parse vCard");

        let photo = test_vcard.photo.expect("No photo found");

        assert_eq!(photo.type_.data, "image/jpeg".to_string());
        assert_eq!(photo.binval.data, bytes);
    }
}
