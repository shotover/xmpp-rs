// Copyright (c) 2017 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::convert::TryFrom;
use std::str::FromStr;

use minidom::{Element, IntoElements, IntoAttributeValue, ElementEmitter};
use jid::Jid;

use error::Error;
use ns;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    ContentAccept,
    ContentAdd,
    ContentModify,
    ContentReject,
    ContentRemove,
    DescriptionInfo,
    SecurityInfo,
    SessionAccept,
    SessionInfo,
    SessionInitiate,
    SessionTerminate,
    TransportAccept,
    TransportInfo,
    TransportReject,
    TransportReplace,
}

impl FromStr for Action {
    type Err = Error;

    fn from_str(s: &str) -> Result<Action, Error> {
        Ok(match s {
            "content-accept" => Action::ContentAccept,
            "content-add" => Action::ContentAdd,
            "content-modify" => Action::ContentModify,
            "content-reject" => Action::ContentReject,
            "content-remove" => Action::ContentRemove,
            "description-info" => Action::DescriptionInfo,
            "security-info" => Action::SecurityInfo,
            "session-accept" => Action::SessionAccept,
            "session-info" => Action::SessionInfo,
            "session-initiate" => Action::SessionInitiate,
            "session-terminate" => Action::SessionTerminate,
            "transport-accept" => Action::TransportAccept,
            "transport-info" => Action::TransportInfo,
            "transport-reject" => Action::TransportReject,
            "transport-replace" => Action::TransportReplace,

            _ => return Err(Error::ParseError("Unknown action.")),
        })
    }
}

impl IntoAttributeValue for Action {
    fn into_attribute_value(self) -> Option<String> {
        Some(String::from(match self {
            Action::ContentAccept => "content-accept",
            Action::ContentAdd => "content-add",
            Action::ContentModify => "content-modify",
            Action::ContentReject => "content-reject",
            Action::ContentRemove => "content-remove",
            Action::DescriptionInfo => "description-info",
            Action::SecurityInfo => "security-info",
            Action::SessionAccept => "session-accept",
            Action::SessionInfo => "session-info",
            Action::SessionInitiate => "session-initiate",
            Action::SessionTerminate => "session-terminate",
            Action::TransportAccept => "transport-accept",
            Action::TransportInfo => "transport-info",
            Action::TransportReject => "transport-reject",
            Action::TransportReplace => "transport-replace",
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Creator {
    Initiator,
    Responder,
}

impl FromStr for Creator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Creator, Error> {
        Ok(match s {
            "initiator" => Creator::Initiator,
            "responder" => Creator::Responder,

            _ => return Err(Error::ParseError("Unknown creator.")),
        })
    }
}

impl From<Creator> for String {
    fn from(creator: Creator) -> String {
        String::from(match creator {
            Creator::Initiator => "initiator",
            Creator::Responder => "responder",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Senders {
    Both,
    Initiator,
    None_,
    Responder,
}

impl Default for Senders {
    fn default() -> Senders {
        Senders::Both
    }
}

impl FromStr for Senders {
    type Err = Error;

    fn from_str(s: &str) -> Result<Senders, Error> {
        Ok(match s {
            "both" => Senders::Both,
            "initiator" => Senders::Initiator,
            "none" => Senders::None_,
            "responder" => Senders::Responder,

            _ => return Err(Error::ParseError("Unknown senders.")),
        })
    }
}

impl From<Senders> for String {
    fn from(senders: Senders) -> String {
        String::from(match senders {
            Senders::Both => "both",
            Senders::Initiator => "initiator",
            Senders::None_ => "none",
            Senders::Responder => "responder",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Content {
    pub creator: Creator,
    pub disposition: String,
    pub name: String,
    pub senders: Senders,
    pub description: Option<Element>,
    pub transport: Option<Element>,
    pub security: Option<Element>,
}

impl TryFrom<Element> for Content {
    type Error = Error;

    fn try_from(elem: Element) -> Result<Content, Error> {
        if !elem.is("content", ns::JINGLE) {
            return Err(Error::ParseError("This is not a content element."));
        }

        let mut content = Content {
            creator: get_attr!(elem, "creator", required),
            disposition: get_attr!(elem, "disposition", optional).unwrap_or(String::from("session")),
            name: get_attr!(elem, "name", required),
            senders: get_attr!(elem, "senders", default),
            description: None,
            transport: None,
            security: None,
        };
        for child in elem.children() {
            if child.name() == "description" {
                if content.description.is_some() {
                    return Err(Error::ParseError("Content must not have more than one description."));
                }
                content.description = Some(child.clone());
            } else if child.name() == "transport" {
                if content.transport.is_some() {
                    return Err(Error::ParseError("Content must not have more than one transport."));
                }
                content.transport = Some(child.clone());
            } else if child.name() == "security" {
                if content.security.is_some() {
                    return Err(Error::ParseError("Content must not have more than one security."));
                }
                content.security = Some(child.clone());
            }
        }
        Ok(content)
    }
}

impl Into<Element> for Content {
    fn into(self) -> Element {
        Element::builder("content")
                .ns(ns::JINGLE)
                .attr("creator", String::from(self.creator))
                .attr("disposition", self.disposition)
                .attr("name", self.name)
                .attr("senders", String::from(self.senders))
                .append(self.description)
                .append(self.transport)
                .append(self.security)
                .build()
    }
}

impl IntoElements for Content {
    fn into_elements(self, emitter: &mut ElementEmitter) {
        emitter.append_child(self.into());
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reason {
    AlternativeSession, //(String),
    Busy,
    Cancel,
    ConnectivityError,
    Decline,
    Expired,
    FailedApplication,
    FailedTransport,
    GeneralError,
    Gone,
    IncompatibleParameters,
    MediaError,
    SecurityError,
    Success,
    Timeout,
    UnsupportedApplications,
    UnsupportedTransports,
}

impl FromStr for Reason {
    type Err = Error;

    fn from_str(s: &str) -> Result<Reason, Error> {
        Ok(match s {
            "alternative-session" => Reason::AlternativeSession,
            "busy" => Reason::Busy,
            "cancel" => Reason::Cancel,
            "connectivity-error" => Reason::ConnectivityError,
            "decline" => Reason::Decline,
            "expired" => Reason::Expired,
            "failed-application" => Reason::FailedApplication,
            "failed-transport" => Reason::FailedTransport,
            "general-error" => Reason::GeneralError,
            "gone" => Reason::Gone,
            "incompatible-parameters" => Reason::IncompatibleParameters,
            "media-error" => Reason::MediaError,
            "security-error" => Reason::SecurityError,
            "success" => Reason::Success,
            "timeout" => Reason::Timeout,
            "unsupported-applications" => Reason::UnsupportedApplications,
            "unsupported-transports" => Reason::UnsupportedTransports,

            _ => return Err(Error::ParseError("Unknown reason.")),
        })
    }
}

impl Into<Element> for Reason {
    fn into(self) -> Element {
        Element::builder(match self {
            Reason::AlternativeSession => "alternative-session",
            Reason::Busy => "busy",
            Reason::Cancel => "cancel",
            Reason::ConnectivityError => "connectivity-error",
            Reason::Decline => "decline",
            Reason::Expired => "expired",
            Reason::FailedApplication => "failed-application",
            Reason::FailedTransport => "failed-transport",
            Reason::GeneralError => "general-error",
            Reason::Gone => "gone",
            Reason::IncompatibleParameters => "incompatible-parameters",
            Reason::MediaError => "media-error",
            Reason::SecurityError => "security-error",
            Reason::Success => "success",
            Reason::Timeout => "timeout",
            Reason::UnsupportedApplications => "unsupported-applications",
            Reason::UnsupportedTransports => "unsupported-transports",
        }).build()
    }
}

#[derive(Debug, Clone)]
pub struct ReasonElement {
    pub reason: Reason,
    pub text: Option<String>,
}

impl TryFrom<Element> for ReasonElement {
    type Error = Error;

    fn try_from(elem: Element) -> Result<ReasonElement, Error> {
        if !elem.is("reason", ns::JINGLE) {
            return Err(Error::ParseError("This is not a reason element."));
        }
        let mut reason = None;
        let mut text = None;
        for child in elem.children() {
            if child.ns() != Some(ns::JINGLE) {
                return Err(Error::ParseError("Reason contains a foreign element."));
            }
            match child.name() {
                "text" => {
                    if text.is_some() {
                        return Err(Error::ParseError("Reason must not have more than one text."));
                    }
                    text = Some(child.text());
                },
                name => {
                    if reason.is_some() {
                        return Err(Error::ParseError("Reason must not have more than one reason."));
                    }
                    reason = Some(name.parse()?);
                },
            }
        }
        let reason = reason.ok_or(Error::ParseError("Reason doesn’t contain a valid reason."))?;
        Ok(ReasonElement {
            reason: reason,
            text: text,
        })
    }
}

impl Into<Element> for ReasonElement {
    fn into(self) -> Element {
        let reason: Element = self.reason.into();
        Element::builder("reason")
                .append(reason)
                .append(self.text)
                .build()
    }
}

impl IntoElements for ReasonElement {
    fn into_elements(self, emitter: &mut ElementEmitter) {
        emitter.append_child(self.into());
    }
}

#[derive(Debug, Clone)]
pub struct Jingle {
    pub action: Action,
    pub initiator: Option<Jid>,
    pub responder: Option<Jid>,
    pub sid: String,
    pub contents: Vec<Content>,
    pub reason: Option<ReasonElement>,
    pub other: Vec<Element>,
}

impl TryFrom<Element> for Jingle {
    type Error = Error;

    fn try_from(root: Element) -> Result<Jingle, Error> {
        if !root.is("jingle", ns::JINGLE) {
            return Err(Error::ParseError("This is not a Jingle element."));
        }

        let mut jingle = Jingle {
            action: get_attr!(root, "action", required),
            initiator: get_attr!(root, "initiator", optional),
            responder: get_attr!(root, "responder", optional),
            sid: get_attr!(root, "sid", required),
            contents: vec!(),
            reason: None,
            other: vec!(),
        };

        for child in root.children().cloned() {
            if child.is("content", ns::JINGLE) {
                let content = Content::try_from(child)?;
                jingle.contents.push(content);
            } else if child.is("reason", ns::JINGLE) {
                if jingle.reason.is_some() {
                    return Err(Error::ParseError("Jingle must not have more than one reason."));
                }
                let reason = ReasonElement::try_from(child)?;
                jingle.reason = Some(reason);
            } else {
                jingle.other.push(child);
            }
        }

        Ok(jingle)
    }
}

impl Into<Element> for Jingle {
    fn into(self) -> Element {
        Element::builder("jingle")
                .ns(ns::JINGLE)
                .attr("action", self.action)
                .attr("initiator", match self.initiator { Some(initiator) => Some(String::from(initiator)), None => None })
                .attr("responder", match self.responder { Some(responder) => Some(String::from(responder)), None => None })
                .attr("sid", self.sid)
                .append(self.contents)
                .append(self.reason)
                .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'/>".parse().unwrap();
        let jingle = Jingle::try_from(elem).unwrap();
        assert_eq!(jingle.action, Action::SessionInitiate);
        assert_eq!(jingle.sid, "coucou");
    }

    #[test]
    fn test_invalid_jingle() {
        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1'/>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'action' missing.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-info'/>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'sid' missing.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='coucou' sid='coucou'/>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown action.");
    }

    #[test]
    fn test_content() {
        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content creator='initiator' name='coucou'><description/><transport/></content></jingle>".parse().unwrap();
        let jingle = Jingle::try_from(elem).unwrap();
        assert_eq!(jingle.contents[0].creator, Creator::Initiator);
        assert_eq!(jingle.contents[0].name, "coucou");
        assert_eq!(jingle.contents[0].senders, Senders::Both);
        assert_eq!(jingle.contents[0].disposition, "session");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content creator='initiator' name='coucou' senders='both'><description/><transport/></content></jingle>".parse().unwrap();
        let jingle = Jingle::try_from(elem).unwrap();
        assert_eq!(jingle.contents[0].senders, Senders::Both);

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content creator='initiator' name='coucou' disposition='early-session'><description/><transport/></content></jingle>".parse().unwrap();
        let jingle = Jingle::try_from(elem).unwrap();
        assert_eq!(jingle.contents[0].disposition, "early-session");
    }

    #[test]
    fn test_invalid_content() {
        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content/></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'creator' missing.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content creator='initiator'/></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'name' missing.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content creator='coucou' name='coucou'/></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown creator.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content creator='initiator' name='coucou' senders='coucou'/></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown senders.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><content creator='initiator' name='coucou' senders=''/></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown senders.");
    }

    #[test]
    fn test_reason() {
        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><reason><success/></reason></jingle>".parse().unwrap();
        let jingle = Jingle::try_from(elem).unwrap();
        let reason = jingle.reason.unwrap();
        assert_eq!(reason.reason, Reason::Success);
        assert_eq!(reason.text, None);

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><reason><success/><text>coucou</text></reason></jingle>".parse().unwrap();
        let jingle = Jingle::try_from(elem).unwrap();
        let reason = jingle.reason.unwrap();
        assert_eq!(reason.reason, Reason::Success);
        assert_eq!(reason.text, Some(String::from("coucou")));
    }

    #[test]
    fn test_invalid_reason() {
        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><reason/></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Reason doesn’t contain a valid reason.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><reason><a/></reason></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown reason.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><reason><a xmlns='http://www.w3.org/1999/xhtml'/></reason></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Reason contains a foreign element.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><reason><decline/></reason><reason/></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Jingle must not have more than one reason.");

        let elem: Element = "<jingle xmlns='urn:xmpp:jingle:1' action='session-initiate' sid='coucou'><reason><decline/><text/><text/></reason></jingle>".parse().unwrap();
        let error = Jingle::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Reason must not have more than one text.");
    }
}
