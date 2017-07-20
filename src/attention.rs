// Copyright (c) 2017 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use try_from::TryFrom;

use minidom::Element;

use error::Error;

use ns;

#[derive(Debug, Clone)]
pub struct Attention;

impl TryFrom<Element> for Attention {
    type Err = Error;

    fn try_from(elem: Element) -> Result<Attention, Error> {
        if !elem.is("attention", ns::ATTENTION) {
            return Err(Error::ParseError("This is not an attention element."));
        }
        for _ in elem.children() {
            return Err(Error::ParseError("Unknown child in attention element."));
        }
        for _ in elem.attrs() {
            return Err(Error::ParseError("Unknown attribute in attention element."));
        }
        Ok(Attention)
    }
}

impl Into<Element> for Attention {
    fn into(self) -> Element {
        Element::builder("attention")
                .ns(ns::ATTENTION)
                .build()
    }
}

#[cfg(test)]
mod tests {
    use try_from::TryFrom;
    use minidom::Element;
    use error::Error;
    use super::Attention;

    #[test]
    fn test_simple() {
        let elem: Element = "<attention xmlns='urn:xmpp:attention:0'/>".parse().unwrap();
        Attention::try_from(elem).unwrap();
    }

    #[test]
    fn test_invalid_child() {
        let elem: Element = "<attention xmlns='urn:xmpp:attention:0'><coucou/></attention>".parse().unwrap();
        let error = Attention::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown child in attention element.");
    }

    #[test]
    fn test_invalid_attribute() {
        let elem: Element = "<attention xmlns='urn:xmpp:attention:0' coucou=''/>".parse().unwrap();
        let error = Attention::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown attribute in attention element.");
    }

    #[test]
    fn test_serialise() {
        let elem: Element = "<attention xmlns='urn:xmpp:attention:0'/>".parse().unwrap();
        let attention = Attention;
        let elem2: Element = attention.into();
        assert_eq!(elem, elem2);
    }
}
