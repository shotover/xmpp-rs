// Copyright (c) 2022 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::ns;
use crate::util::error::Error;
use crate::util::helpers::PlainText;
use crate::Element;
use std::convert::TryFrom;

generate_attribute!(
    Event, "event", {
        New => "new",
        Reset => "reset",
        Edit => "edit",
        Init => "init",
        Cancel => "cancel",
    }, Default = Edit
);

generate_element!(
    Insert,
    "t",
    RTT,
    attributes: [
        pos: Option<u32> = "p",
    ],
    text: (
        text: PlainText<Option<String>>
    )
);

impl TryFrom<Action> for Insert {
    type Error = Error;

    fn try_from(action: Action) -> Result<Insert, Error> {
        match action {
            Action::Insert(insert) => Ok(insert),
            _ => Err(Error::ParseError("This is not an insert action.")),
        }
    }
}

// TODO: add a way in the macro to set a default value.
/*
generate_element!(
    Erase,
    "e",
    RTT,
    attributes: [
        pos: Option<u32> = "p",
        num: Default<u32> = "n",
    ]
);
*/

#[derive(Debug, Clone, PartialEq)]
pub struct Erase {
    pub pos: Option<u32>,
    pub num: u32,
}

impl TryFrom<Element> for Erase {
    type Error = Error;
    fn try_from(elem: Element) -> Result<Erase, Error> {
        check_self!(elem, "e", RTT);
        check_no_unknown_attributes!(elem, "e", ["p", "n"]);
        let pos = get_attr!(elem, "p", Option);
        let num = get_attr!(elem, "n", Option).unwrap_or(1);
        check_no_children!(elem, "e");
        Ok(Erase { pos, num })
    }
}

impl From<Erase> for Element {
    fn from(elem: Erase) -> Element {
        Element::builder("e", ns::RTT)
            .attr("p", elem.pos)
            .attr("n", elem.num)
            .build()
    }
}

impl TryFrom<Action> for Erase {
    type Error = Error;

    fn try_from(action: Action) -> Result<Erase, Error> {
        match action {
            Action::Erase(erase) => Ok(erase),
            _ => Err(Error::ParseError("This is not an erase action.")),
        }
    }
}

generate_element!(
    Wait,
    "w",
    RTT,
    attributes: [
        time: Required<u32> = "n",
    ]
);

impl TryFrom<Action> for Wait {
    type Error = Error;

    fn try_from(action: Action) -> Result<Wait, Error> {
        match action {
            Action::Wait(wait) => Ok(wait),
            _ => Err(Error::ParseError("This is not a wait action.")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Insert(Insert),
    Erase(Erase),
    Wait(Wait),
}

impl TryFrom<Element> for Action {
    type Error = Error;

    fn try_from(elem: Element) -> Result<Action, Error> {
        match elem.name() {
            "t" => Insert::try_from(elem).map(|insert| Action::Insert(insert)),
            "e" => Erase::try_from(elem).map(|erase| Action::Erase(erase)),
            "w" => Wait::try_from(elem).map(|wait| Action::Wait(wait)),
            _ => Err(Error::ParseError("This is not a rtt action element.")),
        }
    }
}

impl From<Action> for Element {
    fn from(action: Action) -> Element {
        match action {
            Action::Insert(insert) => Element::from(insert),
            Action::Erase(erase) => Element::from(erase),
            Action::Wait(wait) => Element::from(wait),
        }
    }
}

// TODO: Allow a wildcard name to the macro, to simplify the following code:
/*
generate_element!(
    Rtt, "rtt", RTT,
    attributes: [
        seq: Required<u32> = "seq",
        event: Default<Event> = "event",
        id: Option<String> = "id",
    ],
    children: [
        actions: Vec<Action> = (*, RTT) => Action,
    ]
);
*/

#[derive(Debug, Clone, PartialEq)]
pub struct Rtt {
    pub seq: u32,
    pub event: Event,
    pub id: Option<String>,
    pub actions: Vec<Action>,
}

impl TryFrom<Element> for Rtt {
    type Error = Error;
    fn try_from(elem: Element) -> Result<Rtt, Error> {
        check_self!(elem, "rtt", RTT);

        check_no_unknown_attributes!(elem, "rtt", ["seq", "event", "id"]);
        let seq = get_attr!(elem, "seq", Required);
        let event = get_attr!(elem, "event", Default);
        let id = get_attr!(elem, "id", Option);

        let mut actions = Vec::new();
        for child in elem.children() {
            if child.ns() != ns::RTT {
                return Err(Error::ParseError("Unknown child in rtt element."));
            }
            actions.push(Action::try_from(child.clone())?);
        }

        Ok(Rtt {
            seq,
            event,
            id,
            actions: actions,
        })
    }
}

impl From<Rtt> for Element {
    fn from(elem: Rtt) -> Element {
        Element::builder("rtt", ns::RTT)
            .attr("seq", elem.seq)
            .attr("event", elem.event)
            .attr("id", elem.id)
            .append_all(elem.actions)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_size() {
        assert_size!(Event, 1);
        assert_size!(Insert, 32);
        assert_size!(Erase, 12);
        assert_size!(Wait, 4);
        assert_size!(Action, 40);
        assert_size!(Rtt, 56);
    }

    #[test]
    fn simple() {
        let elem: Element = "<rtt xmlns='urn:xmpp:rtt:0' seq='0'/>".parse().unwrap();
        let rtt = Rtt::try_from(elem).unwrap();
        assert_eq!(rtt.seq, 0);
        assert_eq!(rtt.event, Event::Edit);
        assert_eq!(rtt.id, None);
        assert_eq!(rtt.actions.len(), 0);
    }

    #[test]
    fn sequence() {
        let elem: Element = "<rtt xmlns='urn:xmpp:rtt:0' seq='0' event='new'><t>Hello,</t><w n='50'/><e/><t>!</t></rtt>"
            .parse()
            .unwrap();

        let rtt = Rtt::try_from(elem).unwrap();
        assert_eq!(rtt.seq, 0);
        assert_eq!(rtt.event, Event::New);
        assert_eq!(rtt.id, None);

        let mut actions = rtt.actions.into_iter();
        assert_eq!(actions.len(), 4);

        let t: Insert = actions.next().unwrap().try_into().unwrap();
        assert_eq!(t.pos, None);
        assert_eq!(t.text, Some(String::from("Hello,")));

        let w: Wait = actions.next().unwrap().try_into().unwrap();
        assert_eq!(w.time, 50);

        let e: Erase = actions.next().unwrap().try_into().unwrap();
        assert_eq!(e.pos, None);
        assert_eq!(e.num, 1);

        let t: Insert = actions.next().unwrap().try_into().unwrap();
        assert_eq!(t.pos, None);
        assert_eq!(t.text, Some(String::from("!")));
    }
}
