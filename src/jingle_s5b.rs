// Copyright (c) 2017 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::convert::TryFrom;
use std::str::FromStr;

use minidom::{Element, IntoAttributeValue};

use error::Error;

use ns;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Assisted,
    Direct,
    Proxy,
    Tunnel,
}

impl Default for Type {
    fn default() -> Type {
        Type::Direct
    }
}

impl FromStr for Type {
    type Err = Error;

    fn from_str(s: &str) -> Result<Type, Error> {
        Ok(match s {
            "assisted" => Type::Assisted,
            "direct" => Type::Direct,
            "proxy" => Type::Proxy,
            "tunnel" => Type::Tunnel,

            _ => return Err(Error::ParseError("Invalid 'type' attribute in candidate element.")),
        })
    }
}

impl IntoAttributeValue for Type {
    fn into_attribute_value(self) -> Option<String> {
        Some(match self {
            Type::Assisted => String::from("assisted"),
            Type::Direct => return None,
            Type::Proxy => String::from("proxy"),
            Type::Tunnel => String::from("tunnel"),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub cid: String,
    pub host: String,
    pub jid: String,
    pub port: Option<u16>,
    pub priority: u32,
    pub type_: Type,
}

impl<'a> Into<Element> for &'a Candidate {
    fn into(self) -> Element {
        Element::builder("candidate")
                .ns(ns::JINGLE_S5B)
                .attr("cid", self.cid.clone())
                .attr("host", self.host.clone())
                .attr("jid", self.jid.clone())
                .attr("port", match self.port { Some(port) => Some(format!("{}", port)), None => None })
                .attr("priority", format!("{}", self.priority))
                .attr("type", self.type_.clone())
                .build()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Tcp,
    Udp,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Tcp
    }
}

impl FromStr for Mode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Mode, Error> {
        Ok(match s {
            "tcp" => Mode::Tcp,
            "udp" => Mode::Udp,

            _ => return Err(Error::ParseError("Invalid 'mode' attribute.")),
        })
    }
}

impl IntoAttributeValue for Mode {
    fn into_attribute_value(self) -> Option<String> {
        match self {
            Mode::Tcp => None,
            Mode::Udp => Some(String::from("udp")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TransportPayload {
    Activated(String),
    Candidates(Vec<Candidate>),
    CandidateError,
    CandidateUsed(String),
    ProxyError,
    None,
}

#[derive(Debug, Clone)]
pub struct Transport {
    pub sid: String,
    pub dstaddr: Option<String>,
    pub mode: Mode,
    pub payload: TransportPayload,
}

impl<'a> TryFrom<&'a Element> for Transport {
    type Error = Error;

    fn try_from(elem: &'a Element) -> Result<Transport, Error> {
        if elem.is("transport", ns::JINGLE_S5B) {
            let sid = elem.attr("sid")
                          .ok_or(Error::ParseError("Required attribute 'sid' missing in JingleS5B transport element."))?
                          .parse()?;
            let dstaddr = elem.attr("dstaddr")
                              .and_then(|value| Some(value.to_owned()));
            let mode = match elem.attr("mode") {
                None => Default::default(),
                Some(mode) => mode.parse()?,
            };

            let mut payload = None;
            for child in elem.children() {
                payload = Some(if child.is("candidate", ns::JINGLE_S5B) {
                    let mut candidates = match payload {
                        Some(TransportPayload::Candidates(candidates)) => candidates,
                        Some(_) => return Err(Error::ParseError("Non-activated child already present in JingleS5B transport element.")),
                        None => vec!(),
                    };
                    let cid = child.attr("cid")
                                   .ok_or(Error::ParseError("Required attribute 'cid' missing in JingleS5B candidate element."))?
                                   .parse()?;
                    let host = child.attr("host")
                                    .ok_or(Error::ParseError("Required attribute 'host' missing in JingleS5B candidate element."))?
                                    .parse()?;
                    let jid = child.attr("jid")
                                   .ok_or(Error::ParseError("Required attribute 'jid' missing in JingleS5B candidate element."))?
                                   .parse()?;
                    let port = match child.attr("port") {
                        Some(s) => Some(s.parse()?),
                        None => None,
                    };
                    let priority = child.attr("priority")
                                       .ok_or(Error::ParseError("Required attribute 'priority' missing in JingleS5B candidate element."))?
                                       .parse()?;
                    let type_ = match child.attr("type") {
                        Some(s) => s.parse()?,
                        None => Default::default(),
                    };
                    candidates.push(Candidate {
                        cid: cid,
                        host: host,
                        jid: jid,
                        port: port,
                        priority: priority,
                        type_: type_,
                    });
                    TransportPayload::Candidates(candidates)
                } else if child.is("activated", ns::JINGLE_S5B) {
                    if let Some(_) = payload {
                        return Err(Error::ParseError("Non-activated child already present in JingleS5B transport element."));
                    }
                    let cid = child.attr("cid")
                                   .ok_or(Error::ParseError("Required attribute 'cid' missing in JingleS5B activated element."))?
                                   .parse()?;
                    TransportPayload::Activated(cid)
                } else if child.is("candidate-error", ns::JINGLE_S5B) {
                    if let Some(_) = payload {
                        return Err(Error::ParseError("Non-candidate-error child already present in JingleS5B transport element."));
                    }
                    TransportPayload::CandidateError
                } else if child.is("candidate-used", ns::JINGLE_S5B) {
                    if let Some(_) = payload {
                        return Err(Error::ParseError("Non-candidate-used child already present in JingleS5B transport element."));
                    }
                    let cid = child.attr("cid")
                                   .ok_or(Error::ParseError("Required attribute 'cid' missing in JingleS5B candidate-used element."))?
                                   .parse()?;
                    TransportPayload::CandidateUsed(cid)
                } else if child.is("proxy-error", ns::JINGLE_S5B) {
                    if let Some(_) = payload {
                        return Err(Error::ParseError("Non-proxy-error child already present in JingleS5B transport element."));
                    }
                    TransportPayload::ProxyError
                } else {
                    return Err(Error::ParseError("Unknown child in JingleS5B transport element."));
                });
            }
            let payload = payload.unwrap_or(TransportPayload::None);
            Ok(Transport {
                sid: sid,
                dstaddr: dstaddr,
                mode: mode,
                payload: payload,
            })
        } else {
            Err(Error::ParseError("This is not an JingleS5B transport element."))
        }
    }
}

impl<'a> Into<Element> for &'a Transport {
    fn into(self) -> Element {
        Element::builder("transport")
                .ns(ns::JINGLE_S5B)
                .attr("sid", self.sid.clone())
                .attr("dstaddr", self.dstaddr.clone())
                .attr("mode", self.mode.clone())
                .append(match self.payload {
                     TransportPayload::Candidates(ref candidates) => {
                         candidates.iter()
                                   .map(|candidate| -> Element { candidate.into() })
                                   .collect::<Vec<Element>>()
                     },
                     TransportPayload::Activated(ref cid) => {
                         vec!(Element::builder("activated")
                                      .ns(ns::JINGLE_S5B)
                                      .attr("cid", cid.to_owned())
                                      .build())
                     },
                     TransportPayload::CandidateError => {
                         vec!(Element::builder("candidate-error")
                                      .ns(ns::JINGLE_S5B)
                                      .build())
                     },
                     TransportPayload::CandidateUsed(ref cid) => {
                         vec!(Element::builder("candidate-used")
                                      .ns(ns::JINGLE_S5B)
                                      .attr("cid", cid.to_owned())
                                      .build())
                     },
                     TransportPayload::ProxyError => {
                         vec!(Element::builder("proxy-error")
                                      .ns(ns::JINGLE_S5B)
                                      .build())
                     },
                     TransportPayload::None => vec!(),
                 })
                .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let elem: Element = "<transport xmlns='urn:xmpp:jingle:transports:s5b:1' sid='coucou'/>".parse().unwrap();
        let transport = Transport::try_from(&elem).unwrap();
        assert_eq!(transport.sid, "coucou");
        assert_eq!(transport.dstaddr, None);
        assert_eq!(transport.mode, Mode::Tcp);
        match transport.payload {
            TransportPayload::None => (),
            _ => panic!("Wrong element inside transport!"),
        }
    }

    #[test]
    fn test_serialise_activated() {
        let elem: Element = "<transport xmlns='urn:xmpp:jingle:transports:s5b:1' sid='coucou'><activated cid='coucou'/></transport>".parse().unwrap();
        let transport = Transport {
            sid: String::from("coucou"),
            dstaddr: None,
            mode: Mode::Tcp,
            payload: TransportPayload::Activated(String::from("coucou")),
        };
        let elem2: Element = (&transport).into();
        assert_eq!(elem, elem2);
    }

    #[test]
    fn test_serialise_candidate() {
        let elem: Element = "<transport xmlns='urn:xmpp:jingle:transports:s5b:1' sid='coucou'><candidate cid='coucou' host='coucou' jid='coucou@coucou' priority='0'/></transport>".parse().unwrap();
        let transport = Transport {
            sid: String::from("coucou"),
            dstaddr: None,
            mode: Mode::Tcp,
            payload: TransportPayload::Candidates(vec!(Candidate {
                cid: String::from("coucou"),
                host: String::from("coucou"),
                jid: String::from("coucou@coucou"),
                port: None,
                priority: 0u32,
                type_: Type::Direct,
            })),
        };
        let elem2: Element = (&transport).into();
        assert_eq!(elem, elem2);
    }
}
