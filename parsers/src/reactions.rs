// Copyright (c) 2022 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::message::MessagePayload;
use crate::util::helpers::Text;

generate_element!(
    /// Container for a set of reactions.
    Reactions, "reactions", REACTIONS,
    attributes: [
        /// The id of the message these reactions apply to.
        id: Required<String> = "id",
    ],
    children: [
        /// The list of reactions.
        reactions: Vec<Reaction> = ("reaction", REACTIONS) => Reaction,
    ]
);

impl MessagePayload for Reactions {}

generate_element!(
    /// One emoji reaction.
    Reaction, "reaction", REACTIONS,
    text: (
        /// The text of this reaction.
        emoji: Text<String>
    )
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Element;
    use std::convert::{TryFrom, TryInto};

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(Reactions, 24);
        assert_size!(Reaction, 12);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(Reactions, 48);
        assert_size!(Reaction, 24);
    }

    #[test]
    fn test_empty() {
        let elem: Element = "<reactions xmlns='urn:xmpp:reactions:0' id='foo'/>"
            .parse()
            .unwrap();
        let elem2 = elem.clone();
        let reactions = Reactions::try_from(elem2).unwrap();
        assert_eq!(reactions.id, "foo");
        assert_eq!(reactions.reactions.len(), 0);
    }

    #[test]
    fn test_multi() {
        let elem: Element =
            "<reactions xmlns='urn:xmpp:reactions:0' id='foo'><reaction>👋</reaction><reaction>🐢</reaction></reactions>"
                .parse()
                .unwrap();
        let elem2 = elem.clone();
        let reactions = Reactions::try_from(elem2).unwrap();
        assert_eq!(reactions.id, "foo");
        assert_eq!(reactions.reactions.len(), 2);
        let [hand, turtle]: [Reaction; 2] = reactions.reactions.try_into().unwrap();
        assert_eq!(hand.emoji, "👋");
        assert_eq!(turtle.emoji, "🐢");
    }

    #[test]
    fn test_zwj_emoji() {
        let elem: Element =
            "<reactions xmlns='urn:xmpp:reactions:0' id='foo'><reaction>👩🏾‍❤️‍👩🏼</reaction></reactions>"
                .parse()
                .unwrap();
        let elem2 = elem.clone();
        let mut reactions = Reactions::try_from(elem2).unwrap();
        assert_eq!(reactions.id, "foo");
        assert_eq!(reactions.reactions.len(), 1);
        let reaction = reactions.reactions.pop().unwrap();
        assert_eq!(
            reaction.emoji,
            "\u{1F469}\u{1F3FE}\u{200D}\u{2764}\u{FE0F}\u{200D}\u{1F469}\u{1F3FC}"
        );
    }
}
