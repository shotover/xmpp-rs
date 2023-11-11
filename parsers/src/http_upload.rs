// Copyright (c) 2021 Maxime “pep” Buquet <pep@bouah.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::iq::{IqGetPayload, IqResultPayload};
use crate::ns;
use crate::util::error::Error;
use crate::Element;

generate_element!(
    /// Requesting a slot
    SlotRequest, "request", HTTP_UPLOAD,
    attributes: [
        /// The filename to be uploaded.
        filename: Required<String> = "filename",

        /// Size of the file to be uploaded.
        size: Required<u64> = "size",

        /// Content-Type of the file.
        content_type: Option<String> = "content-type",
    ]
);

impl IqGetPayload for SlotRequest {}

/// Slot header
#[derive(Debug, Clone, PartialEq)]
pub enum Header {
    /// Authorization header
    Authorization(String),

    /// Cookie header
    Cookie(String),

    /// Expires header
    Expires(String),
}

impl TryFrom<Element> for Header {
    type Error = Error;
    fn try_from(elem: Element) -> Result<Header, Error> {
        check_self!(elem, "header", HTTP_UPLOAD);
        check_no_children!(elem, "header");
        check_no_unknown_attributes!(elem, "header", ["name"]);
        let name: String = get_attr!(elem, "name", Required);
        let text = elem.text();

        Ok(match name.to_lowercase().as_str() {
            "authorization" => Header::Authorization(text),
            "cookie" => Header::Cookie(text),
            "expires" => Header::Expires(text),
            _ => {
                return Err(Error::ParseError(
                    "Header name must be either 'Authorization', 'Cookie', or 'Expires'.",
                ))
            }
        })
    }
}

impl From<Header> for Element {
    fn from(elem: Header) -> Element {
        let (attr, val) = match elem {
            Header::Authorization(val) => ("Authorization", val),
            Header::Cookie(val) => ("Cookie", val),
            Header::Expires(val) => ("Expires", val),
        };

        Element::builder("header", ns::HTTP_UPLOAD)
            .attr("name", attr)
            .append(val)
            .build()
    }
}

generate_element!(
    /// Put URL
    Put, "put", HTTP_UPLOAD,
    attributes: [
        /// URL
        url: Required<String> = "url",
    ],
    children: [
        /// Header list
        headers: Vec<Header> = ("header", HTTP_UPLOAD) => Header
    ]
);

generate_element!(
    /// Get URL
    Get, "get", HTTP_UPLOAD,
    attributes: [
        /// URL
        url: Required<String> = "url",
    ]
);

generate_element!(
    /// Requesting a slot
    SlotResult, "slot", HTTP_UPLOAD,
    children: [
        /// Put URL and headers
        put: Required<Put> = ("put", HTTP_UPLOAD) => Put,
        /// Get URL
        get: Required<Get> = ("get", HTTP_UPLOAD) => Get
    ]
);

impl IqResultPayload for SlotResult {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Element;

    #[test]
    fn test_slot_request() {
        let elem: Element = "<request xmlns='urn:xmpp:http:upload:0'
            filename='très cool.jpg'
            size='23456'
            content-type='image/jpeg' />"
            .parse()
            .unwrap();
        let slot = SlotRequest::try_from(elem).unwrap();
        assert_eq!(slot.filename, String::from("très cool.jpg"));
        assert_eq!(slot.size, 23456);
        assert_eq!(slot.content_type, Some(String::from("image/jpeg")));
    }

    #[test]
    fn test_slot_result() {
        let elem: Element = "<slot xmlns='urn:xmpp:http:upload:0'>
            <put url='https://upload.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg'>
              <header name='Authorization'>Basic Base64String==</header>
              <header name='Cookie'>foo=bar; user=romeo</header>
            </put>
            <get url='https://download.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg' />
          </slot>"
            .parse()
            .unwrap();
        let slot = SlotResult::try_from(elem).unwrap();
        assert_eq!(slot.put.url, String::from("https://upload.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg"));
        assert_eq!(
            slot.put.headers[0],
            Header::Authorization(String::from("Basic Base64String=="))
        );
        assert_eq!(
            slot.put.headers[1],
            Header::Cookie(String::from("foo=bar; user=romeo"))
        );
        assert_eq!(slot.get.url, String::from("https://download.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg"));
    }

    #[test]
    fn test_result_no_header() {
        let elem: Element = "<slot xmlns='urn:xmpp:http:upload:0'>
            <put url='https://URL' />
            <get url='https://URL' />
          </slot>"
            .parse()
            .unwrap();
        let slot = SlotResult::try_from(elem).unwrap();
        assert_eq!(slot.put.url, String::from("https://URL"));
        assert_eq!(slot.put.headers.len(), 0);
        assert_eq!(slot.get.url, String::from("https://URL"));
    }

    #[test]
    fn test_result_bad_header() {
        let elem: Element = "<slot xmlns='urn:xmpp:http:upload:0'>
            <put url='https://URL'>
              <header name='EvilHeader'>EvilValue</header>
            </put>
            <get url='https://URL' />
          </slot>"
            .parse()
            .unwrap();
        SlotResult::try_from(elem).unwrap_err();
    }
}
