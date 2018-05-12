// Copyright (c) 2017 Maxime “pep” Buquet <pep+code@bouah.net>
// Copyright (c) 2017 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use try_from::{TryFrom, TryInto};
use std::str::FromStr;

use minidom::{Element, IntoAttributeValue};

use jid::Jid;

use error::Error;

use ns;

generate_attribute_enum!(
/// Lists all of the possible status codes used in MUC presences.
Status, "status", ns::MUC_USER, "code", {
    /// Inform user that any occupant is allowed to see the user's full JID
    NonAnonymousRoom => 100,

    /// Inform user that his or her affiliation changed while not in the room
    AffiliationChange => 101,

    /// Inform occupants that room now shows unavailable members
    ConfigShowsUnavailableMembers => 102,

    /// Inform occupants that room now does not show unavailable members
    ConfigHidesUnavailableMembers => 103,

    /// Inform occupants that a non-privacy-related room configuration change has occurred
    ConfigNonPrivacyRelated => 104,

    /// Inform user that presence refers to itself
    SelfPresence => 110,

    /// Inform occupants that room logging is now enabled
    ConfigRoomLoggingEnabled => 170,

    /// Inform occupants that room logging is now disabled
    ConfigRoomLoggingDisabled => 171,

    /// Inform occupants that the room is now non-anonymous
    ConfigRoomNonAnonymous => 172,

    /// Inform occupants that the room is now semi-anonymous
    ConfigRoomSemiAnonymous => 173,

    /// Inform user that a new room has been created
    RoomHasBeenCreated => 201,

    /// Inform user that service has assigned or modified occupant's roomnick
    AssignedNick => 210,

    /// Inform user that he or she has been banned from the room
    Banned => 301,

    /// Inform all occupants of new room nickname
    NewNick => 303,

    /// Inform user that he or she has been kicked from the room
    Kicked => 307,

    /// Inform user that he or she is being removed from the room
    /// because of an affiliation change
    RemovalFromRoom => 321,

    /// Inform user that he or she is being removed from the room
    /// because the room has been changed to members-only and the
    /// user is not a member
    ConfigMembersOnly => 322,

    /// Inform user that he or she is being removed from the room
    /// because the MUC service is being shut down
    ServiceShutdown => 332,
});

/// Optional <actor/> element used in <item/> elements inside presence stanzas of type
/// "unavailable" that are sent to users who are kick or banned, as well as within IQs for tracking
/// purposes. -- CHANGELOG  0.17 (2002-10-23)
/// Possesses a 'jid' and a 'nick' attribute, so that an action can be attributed either to a real
/// JID or to a roomnick. -- CHANGELOG  1.25 (2012-02-08)
#[derive(Debug, Clone, PartialEq)]
pub enum Actor {
    Jid(Jid),
    Nick(String),
}

impl TryFrom<Element> for Actor {
    type Err = Error;

    fn try_from(elem: Element) -> Result<Actor, Error> {
        check_self!(elem, "actor", ns::MUC_USER);
        check_no_unknown_attributes!(elem, "actor", ["jid", "nick"]);
        for _ in elem.children() {
            return Err(Error::ParseError("Unknown child in actor element."));
        }
        let jid: Option<Jid> = get_attr!(elem, "jid", optional);
        let nick = get_attr!(elem, "nick", optional);

        match (jid, nick) {
            (Some(_), Some(_))
          | (None, None) =>
                return Err(Error::ParseError("Either 'jid' or 'nick' attribute is required.")),
            (Some(jid), _) => Ok(Actor::Jid(jid)),
            (_, Some(nick)) => Ok(Actor::Nick(nick)),
        }
    }
}

impl From<Actor> for Element {
    fn from(actor: Actor) -> Element {
        let elem = Element::builder("actor").ns(ns::MUC_USER);

        (match actor {
            Actor::Jid(jid) => elem.attr("jid", jid),
            Actor::Nick(nick) => elem.attr("nick", nick),
        }).build()
    }
}

generate_element_with_only_attributes!(Continue, "continue", ns::MUC_USER, [
    thread: Option<String> = "thread" => optional,
]);

generate_elem_id!(Reason, "reason", ns::MUC_USER);

generate_attribute!(Affiliation, "affiliation", {
    Owner => "owner",
    Admin => "admin",
    Member => "member",
    Outcast => "outcast",
    None => "none",
}, Default = None);

generate_attribute!(Role, "role", {
    Moderator => "moderator",
    Participant => "participant",
    Visitor => "visitor",
    None => "none",
}, Default = None);

#[derive(Debug, Clone)]
pub struct Item {
    pub affiliation: Affiliation,
    pub jid: Option<Jid>,
    pub nick: Option<String>,
    pub role: Role,
    pub actor: Option<Actor>,
    pub continue_: Option<Continue>,
    pub reason: Option<Reason>,
}

impl TryFrom<Element> for Item {
    type Err = Error;

    fn try_from(elem: Element) -> Result<Item, Error> {
        check_self!(elem, "item", ns::MUC_USER);
        check_no_unknown_attributes!(elem, "item", ["affiliation", "jid", "nick", "role"]);
        let mut actor: Option<Actor> = None;
        let mut continue_: Option<Continue> = None;
        let mut reason: Option<Reason> = None;
        for child in elem.children() {
            if child.is("actor", ns::MUC_USER) {
                actor = Some(child.clone().try_into()?);
            } else if child.is("continue", ns::MUC_USER) {
                continue_ = Some(child.clone().try_into()?);
            } else if child.is("reason", ns::MUC_USER) {
                reason = Some(child.clone().try_into()?);
            } else {
                return Err(Error::ParseError("Unknown child in item element."));
            }
        }

        let affiliation: Affiliation = get_attr!(elem, "affiliation", required);
        let jid: Option<Jid> = get_attr!(elem, "jid", optional);
        let nick: Option<String> = get_attr!(elem, "nick", optional);
        let role: Role = get_attr!(elem, "role", required);

        Ok(Item{
            affiliation: affiliation,
            jid: jid,
            nick: nick,
            role: role,
            actor: actor,
            continue_: continue_,
            reason: reason,
        })
    }
}

impl From<Item> for Element {
    fn from(item: Item) -> Element {
        Element::builder("item")
                .ns(ns::MUC_USER)
                .attr("affiliation", item.affiliation)
                .attr("jid", item.jid)
                .attr("nick", item.nick)
                .attr("role", item.role)
                .append(item.actor)
                .append(item.continue_)
                .append(item.reason)
                .build()
    }
}

#[derive(Debug, Clone)]
pub struct MucUser {
    pub status: Vec<Status>,
    pub items: Vec<Item>,
}

impl TryFrom<Element> for MucUser {
    type Err = Error;

    fn try_from(elem: Element) -> Result<MucUser, Error> {
        check_self!(elem, "x", ns::MUC_USER);
        check_no_attributes!(elem, "x");
        let mut status = vec!();
        let mut items = vec!();
        for child in elem.children() {
            if child.is("status", ns::MUC_USER) {
                status.push(Status::try_from(child.clone())?);
            } else if child.is("item", ns::MUC_USER) {
                items.push(Item::try_from(child.clone())?);
            } else {
                return Err(Error::ParseError("Unknown child in x element."));
            }
        }
        Ok(MucUser {
            status,
            items,
        })
    }
}

impl From<MucUser> for Element {
    fn from(muc_user: MucUser) -> Element {
        Element::builder("x")
                .ns(ns::MUC_USER)
                .append(muc_user.status)
                .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;
    use compare_elements::NamespaceAwareCompare;

    #[test]
    fn test_simple() {
        let elem: Element = "
            <x xmlns='http://jabber.org/protocol/muc#user'/>
        ".parse().unwrap();
        MucUser::try_from(elem).unwrap();
    }

    #[test]
    fn test_invalid_child() {
        let elem: Element = "
            <x xmlns='http://jabber.org/protocol/muc#user'>
                <coucou/>
            </x>
        ".parse().unwrap();
        let error = MucUser::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown child in x element.");
    }

    #[test]
    fn test_serialise() {
        let elem: Element = "
            <x xmlns='http://jabber.org/protocol/muc#user'/>
        ".parse().unwrap();
        let muc = MucUser { status: vec!(), items: vec!() };
        let elem2 = muc.into();
        assert!(elem.compare_to(&elem2));
    }

    #[test]
    fn test_invalid_attribute() {
        let elem: Element = "
            <x xmlns='http://jabber.org/protocol/muc#user' coucou=''/>
        ".parse().unwrap();
        let error = MucUser::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown attribute in x element.");
    }

    #[test]
    fn test_status_simple() {
        let elem: Element = "
            <status xmlns='http://jabber.org/protocol/muc#user' code='110'/>
        ".parse().unwrap();
        Status::try_from(elem).unwrap();
    }

    #[test]
    fn test_status_invalid() {
        let elem: Element = "
            <status xmlns='http://jabber.org/protocol/muc#user'/>
        ".parse().unwrap();
        let error = Status::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'code' missing.");
    }

    #[test]
    fn test_status_invalid_child() {
        let elem: Element = "
            <status xmlns='http://jabber.org/protocol/muc#user' code='110'>
                <foo/>
            </status>
        ".parse().unwrap();
        let error = Status::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown child in status element.");
    }

    #[test]
    fn test_status_simple_code() {
        let elem: Element = "
            <status xmlns='http://jabber.org/protocol/muc#user' code='307'/>
        ".parse().unwrap();
        let status = Status::try_from(elem).unwrap();
        assert_eq!(status, Status::Kicked);
    }

    #[test]
    fn test_status_invalid_code() {
        let elem: Element = "
            <status xmlns='http://jabber.org/protocol/muc#user' code='666'/>
        ".parse().unwrap();
        let error = Status::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Invalid status code value.");
    }

    #[test]
    fn test_status_invalid_code2() {
        let elem: Element = "
            <status xmlns='http://jabber.org/protocol/muc#user' code='coucou'/>
        ".parse().unwrap();
        let error = Status::try_from(elem).unwrap_err();
        let error = match error {
            Error::ParseIntError(error) => error,
            _ => panic!(),
        };
        assert_eq!(error.description(), "invalid digit found in string");
    }

    #[test]
    fn test_actor_required_attributes() {
        let elem: Element = "
            <actor xmlns='http://jabber.org/protocol/muc#user'/>
        ".parse().unwrap();
        let error = Actor::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Either 'jid' or 'nick' attribute is required.");
    }

    #[test]
    fn test_actor_required_attributes2() {
        let elem: Element = "
            <actor xmlns='http://jabber.org/protocol/muc#user'
                   jid='foo@bar/baz'
                   nick='baz'/>
        ".parse().unwrap();
        let error = Actor::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Either 'jid' or 'nick' attribute is required.");
    }

    #[test]
    fn test_actor_jid() {
        let elem: Element = "
            <actor xmlns='http://jabber.org/protocol/muc#user'
                   jid='foo@bar/baz'/>
        ".parse().unwrap();
        let actor = Actor::try_from(elem).unwrap();
        let jid = match actor {
            Actor::Jid(jid) => jid,
            _ => panic!(),
        };
        assert_eq!(jid, "foo@bar/baz".parse::<Jid>().unwrap());
    }

    #[test]
    fn test_actor_nick() {
        let elem: Element = "
            <actor xmlns='http://jabber.org/protocol/muc#user' nick='baz'/>
        ".parse().unwrap();
        let actor = Actor::try_from(elem).unwrap();
        let nick = match actor {
            Actor::Nick(nick) => nick,
            _ => panic!(),
        };
        assert_eq!(nick, "baz".to_owned());
    }

    #[test]
    fn test_continue_simple() {
        let elem: Element = "
            <continue xmlns='http://jabber.org/protocol/muc#user'/>
        ".parse().unwrap();
        Continue::try_from(elem).unwrap();
    }

    #[test]
    fn test_continue_thread_attribute() {
        let elem: Element = "
            <continue xmlns='http://jabber.org/protocol/muc#user'
                      thread='foo'/>
        ".parse().unwrap();
        let continue_ = Continue::try_from(elem).unwrap();
        assert_eq!(continue_.thread, Some("foo".to_owned()));
    }

    #[test]
    fn test_continue_invalid() {
        let elem: Element = "
            <continue xmlns='http://jabber.org/protocol/muc#user'>
                <foobar/>
            </continue>
        ".parse().unwrap();
        let continue_ = Continue::try_from(elem).unwrap_err();
        let message = match continue_ {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown child in continue element.".to_owned());
    }

    #[test]
    fn test_reason_simple() {
        let elem: Element = "
            <reason xmlns='http://jabber.org/protocol/muc#user'>Reason</reason>"
        .parse().unwrap();
        let reason = Reason::try_from(elem).unwrap();
        assert_eq!(reason.0, "Reason".to_owned());
    }

    #[test]
    fn test_reason_invalid_attribute() {
        let elem: Element = "
            <reason xmlns='http://jabber.org/protocol/muc#user' foo='bar'/>
        ".parse().unwrap();
        let error = Reason::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown attribute in reason element.".to_owned());
    }

    #[test]
    fn test_reason_invalid() {
        let elem: Element = "
            <reason xmlns='http://jabber.org/protocol/muc#user'>
                <foobar/>
            </reason>
        ".parse().unwrap();
        let error = Reason::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown child in reason element.".to_owned());
    }

    #[test]
    fn test_item_invalid_attr(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  foo='bar'/>
        ".parse().unwrap();
        let error = Item::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown attribute in item element.".to_owned());
    }

    #[test]
    fn test_item_affiliation_role_attr(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  affiliation='member'
                  role='moderator'/>
        ".parse().unwrap();
        Item::try_from(elem).unwrap();
    }

    #[test]
    fn test_item_affiliation_role_invalid_attr(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  affiliation='member'/>
        ".parse().unwrap();
        let error = Item::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'role' missing.".to_owned());
    }

    #[test]
    fn test_item_nick_attr(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  affiliation='member'
                  role='moderator'
                  nick='foobar'/>
        ".parse().unwrap();
        let item = Item::try_from(elem).unwrap();
        match item {
            Item { nick, .. } => assert_eq!(nick, Some("foobar".to_owned())),
        }
    }

    #[test]
    fn test_item_affiliation_role_invalid_attr2(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  role='moderator'/>
        ".parse().unwrap();
        let error = Item::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'affiliation' missing.".to_owned());
    }

    #[test]
    fn test_item_role_actor_child(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  affiliation='member'
                  role='moderator'>
                <actor nick='foobar'/>
            </item>
        ".parse().unwrap();
        let item = Item::try_from(elem).unwrap();
        match item {
            Item { actor, .. } =>
                assert_eq!(actor, Some(Actor::Nick("foobar".to_owned()))),
        }
    }

    #[test]
    fn test_item_role_continue_child(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  affiliation='member'
                  role='moderator'>
                <continue thread='foobar'/>
            </item>
        ".parse().unwrap();
        let item = Item::try_from(elem).unwrap();
        let continue_1 = Continue { thread: Some("foobar".to_owned()) };
        match item {
            Item { continue_: Some(continue_2), .. } => assert_eq!(continue_2.thread, continue_1.thread),
            _ => panic!(),
        }
    }

    #[test]
    fn test_item_role_reason_child(){
        let elem: Element = "
            <item xmlns='http://jabber.org/protocol/muc#user'
                  affiliation='member'
                  role='moderator'>
                <reason>foobar</reason>
            </item>
        ".parse().unwrap();
        let item = Item::try_from(elem).unwrap();
        match item {
            Item { reason, .. } =>
                assert_eq!(reason, Some(Reason("foobar".to_owned()))),
        }
    }
}
