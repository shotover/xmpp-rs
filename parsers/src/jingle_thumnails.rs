//! Jingle thumbnails (XEP-0264)

// Copyright (c) 2023 XMPP-RS contributors.=
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at http://mozilla.org/MPL/2.0/.

generate_element!(
    /// A Jingle thumbnail.
    Thumbnail, "thumbnail", JINGLE_THUMBNAILS,
    attributes: [
        /// The URI of the thumbnail.
        uri: Required<String> = "uri",
        /// The media type of the thumbnail.
        media_type: Required<String> = "media-type",
        /// The width of the thumbnail.
        width: Required<u32> = "width",
        /// The height of the thumbnail.
        height: Required<u32> = "height",
    ]
);

#[cfg(test)]
mod tests {
    use crate::jingle_thumnails::Thumbnail;
    use minidom::Element;

    #[test]
    fn test_simple_parse() {
        // Extracted from https://xmpp.org/extensions/xep-0264.html#example-1
        let test_xml = "<thumbnail xmlns='urn:xmpp:thumbs:1'
        uri='cid:sha1+ffd7c8d28e9c5e82afea41f97108c6b4@bob.xmpp.org'
        media-type='image/png'
        width='128'
        height='96'/>";

        let elem: Element = test_xml.parse().unwrap();

        let thumbnail = Thumbnail::try_from(elem).unwrap();

        assert_eq!(
            thumbnail.uri,
            "cid:sha1+ffd7c8d28e9c5e82afea41f97108c6b4@bob.xmpp.org"
        );
        assert_eq!(thumbnail.media_type, "image/png");
        assert_eq!(thumbnail.width, 128);
        assert_eq!(thumbnail.height, 96);
    }
}
