// Copyright (c) 2017-2021 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::data_forms::DataForm;
use crate::forwarding::Forwarded;
use crate::iq::{IqGetPayload, IqResultPayload, IqSetPayload};
use crate::message::MessagePayload;
use crate::ns;
use crate::pubsub::NodeName;
use crate::rsm::{SetQuery, SetResult};
use crate::util::error::Error;
use crate::Element;
use minidom::Node;

generate_id!(
    /// An identifier matching a result message to the query requesting it.
    QueryId
);

/// Starts a query to the archive.
#[derive(Debug)]
pub struct Query {
    /// An optional identifier for matching forwarded messages to this
    /// query.
    pub queryid: Option<QueryId>,
    /// Must be set to Some when querying a PubSub node’s archive.
    pub node: Option<NodeName>,
    /// Used for filtering the results.
    pub form: Option<DataForm>,
    /// Used for paging through results.
    pub set: Option<SetQuery>,
    /// Used for reversing the order of the results.
    pub flip_page: bool,
}

impl IqGetPayload for Query {}
impl IqSetPayload for Query {}
impl IqResultPayload for Query {}

impl TryFrom<Element> for Query {
    type Error = Error;
    fn try_from(elem: Element) -> Result<Query, Error> {
        check_self!(elem, "query", MAM);
        check_no_unknown_attributes!(elem, "query", ["queryid", "node"]);

        let mut form = None;
        let mut set = None;
        let mut flip_page = None;
        for child in elem.children() {
            if child.is("x", ns::DATA_FORMS) {
                if form.is_some() {
                    return Err(Error::ParseError(
                        "Element query must not have more than one x child.",
                    ));
                }
                form = Some(DataForm::try_from(child.clone())?);
                continue;
            }
            if child.is("set", ns::RSM) {
                if set.is_some() {
                    return Err(Error::ParseError(
                        "Element query must not have more than one set child.",
                    ));
                }
                set = Some(SetQuery::try_from(child.clone())?);
                continue;
            }
            if child.is("flip-page", ns::MAM) {
                if flip_page.is_some() {
                    return Err(Error::ParseError(
                        "Element query must not have more than one flip-page child.",
                    ));
                }
                flip_page = Some(true);
                continue;
            }
            return Err(Error::ParseError("Unknown child in query element."));
        }
        Ok(Query {
            queryid: match elem.attr("queryid") {
                Some(value) => Some(value.parse()?),
                None => None,
            },
            node: match elem.attr("node") {
                Some(value) => Some(value.parse()?),
                None => None,
            },
            form,
            set,
            flip_page: flip_page.is_some(),
        })
    }
}

impl From<Query> for Element {
    fn from(elem: Query) -> Element {
        let mut builder = Element::builder("query", ns::MAM);
        builder = builder.attr("queryid", elem.queryid);
        builder = builder.attr("node", elem.node);
        builder = builder.append_all(elem.form.map(|elem| Node::Element(Element::from(elem))));
        builder = builder.append_all(elem.set.map(|elem| Node::Element(Element::from(elem))));
        if elem.flip_page {
            let flip_page = Element::builder("flip-page", ns::MAM).build();
            builder = builder.append(Node::Element(flip_page));
        }
        builder.build()
    }
}

generate_element!(
    /// The wrapper around forwarded stanzas.
    Result_, "result", MAM,
    attributes: [
        /// The stanza-id under which the archive stored this stanza.
        id: Required<String> = "id",

        /// The same queryid as the one requested in the
        /// [query](struct.Query.html).
        queryid: Option<QueryId> = "queryid",
    ],
    children: [
        /// The actual stanza being forwarded.
        forwarded: Required<Forwarded> = ("forwarded", FORWARD) => Forwarded
    ]
);

impl MessagePayload for Result_ {}

generate_attribute!(
    /// True when the end of a MAM query has been reached.
    Complete,
    "complete",
    bool
);

generate_element!(
    /// Notes the end of a page in a query.
    Fin, "fin", MAM,
    attributes: [
        /// True when the end of a MAM query has been reached.
        complete: Default<Complete> = "complete",
    ],
    children: [
        /// Describes the current page, it should contain at least [first]
        /// (with an [index]) and [last], and generally [count].
        ///
        /// [first]: ../rsm/struct.SetResult.html#structfield.first
        /// [index]: ../rsm/struct.SetResult.html#structfield.first_index
        /// [last]: ../rsm/struct.SetResult.html#structfield.last
        /// [count]: ../rsm/struct.SetResult.html#structfield.count
        set: Required<SetResult> = ("set", RSM) => SetResult
    ]
);

impl IqResultPayload for Fin {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::error::Error;
    use minidom::Element;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(QueryId, 12);
        assert_size!(Query, 120);
        assert_size!(Result_, 176);
        assert_size!(Complete, 1);
        assert_size!(Fin, 44);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(QueryId, 24);
        assert_size!(Query, 240);
        assert_size!(Result_, 336);
        assert_size!(Complete, 1);
        assert_size!(Fin, 88);
    }

    #[test]
    fn test_query() {
        let elem: Element = "<query xmlns='urn:xmpp:mam:2'/>".parse().unwrap();
        Query::try_from(elem).unwrap();
    }

    #[test]
    fn test_result() {
        #[cfg(not(feature = "component"))]
        let elem: Element = r#"<result xmlns='urn:xmpp:mam:2' queryid='f27' id='28482-98726-73623'>
  <forwarded xmlns='urn:xmpp:forward:0'>
    <delay xmlns='urn:xmpp:delay' stamp='2010-07-10T23:08:25Z'/>
    <message xmlns='jabber:client' from="witch@shakespeare.lit" to="macbeth@shakespeare.lit">
      <body>Hail to thee</body>
    </message>
  </forwarded>
</result>
"#
        .parse()
        .unwrap();
        #[cfg(feature = "component")]
        let elem: Element = r#"<result xmlns='urn:xmpp:mam:2' queryid='f27' id='28482-98726-73623'>
  <forwarded xmlns='urn:xmpp:forward:0'>
    <delay xmlns='urn:xmpp:delay' stamp='2010-07-10T23:08:25Z'/>
    <message xmlns='jabber:component:accept' from="witch@shakespeare.lit" to="macbeth@shakespeare.lit">
      <body>Hail to thee</body>
    </message>
  </forwarded>
</result>
"#.parse().unwrap();
        Result_::try_from(elem).unwrap();
    }

    #[test]
    fn test_fin() {
        let elem: Element = r#"<fin xmlns='urn:xmpp:mam:2'>
  <set xmlns='http://jabber.org/protocol/rsm'>
    <first index='0'>28482-98726-73623</first>
    <last>09af3-cc343-b409f</last>
  </set>
</fin>
"#
        .parse()
        .unwrap();
        Fin::try_from(elem).unwrap();
    }

    #[test]
    fn test_query_x() {
        let elem: Element = r#"<query xmlns='urn:xmpp:mam:2'>
  <x xmlns='jabber:x:data' type='submit'>
    <field var='FORM_TYPE' type='hidden'>
      <value>urn:xmpp:mam:2</value>
    </field>
    <field var='with'>
      <value>juliet@capulet.lit</value>
    </field>
  </x>
</query>
"#
        .parse()
        .unwrap();
        Query::try_from(elem).unwrap();
    }

    #[test]
    fn test_query_x_set() {
        let elem: Element = r#"<query xmlns='urn:xmpp:mam:2'>
  <x xmlns='jabber:x:data' type='submit'>
    <field var='FORM_TYPE' type='hidden'>
      <value>urn:xmpp:mam:2</value>
    </field>
    <field var='start'>
      <value>2010-08-07T00:00:00Z</value>
    </field>
  </x>
  <set xmlns='http://jabber.org/protocol/rsm'>
    <max>10</max>
  </set>
</query>
"#
        .parse()
        .unwrap();
        Query::try_from(elem).unwrap();
    }

    #[test]
    fn test_query_x_set_flipped() {
        let elem: Element = r#"<query xmlns='urn:xmpp:mam:2'>
  <x xmlns='jabber:x:data' type='submit'>
    <field var='FORM_TYPE' type='hidden'>
      <value>urn:xmpp:mam:2</value>
    </field>
    <field var='start'>
      <value>2010-08-07T00:00:00Z</value>
    </field>
  </x>
  <set xmlns='http://jabber.org/protocol/rsm'>
    <max>10</max>
  </set>
  <flip-page/>
</query>
"#
        .parse()
        .unwrap();
        Query::try_from(elem).unwrap();
    }

    #[test]
    fn test_invalid_child() {
        let elem: Element = "<query xmlns='urn:xmpp:mam:2'><coucou/></query>"
            .parse()
            .unwrap();
        let error = Query::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown child in query element.");
    }

    #[test]
    fn test_serialise_empty() {
        let elem: Element = "<query xmlns='urn:xmpp:mam:2'/>".parse().unwrap();
        let replace = Query {
            queryid: None,
            node: None,
            form: None,
            set: None,
            flip_page: false,
        };
        let elem2 = replace.into();
        assert_eq!(elem, elem2);
    }

    #[test]
    fn test_serialize_query_with_form() {
        let reference: Element = "<query xmlns='urn:xmpp:mam:2'><x xmlns='jabber:x:data' type='submit'><field xmlns='jabber:x:data' var='FORM_TYPE' type='hidden'><value xmlns='jabber:x:data'>urn:xmpp:mam:2</value></field><field xmlns='jabber:x:data' var='with'><value xmlns='jabber:x:data'>juliet@capulet.lit</value></field></x><flip-page/></query>"
        .parse()
        .unwrap();

        let elem: Element = "<x xmlns='jabber:x:data' type='submit'><field xmlns='jabber:x:data' var='FORM_TYPE' type='hidden'><value xmlns='jabber:x:data'>urn:xmpp:mam:2</value></field><field xmlns='jabber:x:data' var='with'><value xmlns='jabber:x:data'>juliet@capulet.lit</value></field></x>"
          .parse()
          .unwrap();

        let form = DataForm::try_from(elem).unwrap();

        let query = Query {
            queryid: None,
            node: None,
            set: None,
            form: Some(form),
            flip_page: true,
        };
        let serialized: Element = query.into();
        assert_eq!(serialized, reference);
    }

    #[test]
    fn test_serialize_result() {
        let reference: Element = "<result xmlns='urn:xmpp:mam:2' queryid='f27' id='28482-98726-73623'><forwarded xmlns='urn:xmpp:forward:0'><delay xmlns='urn:xmpp:delay' stamp='2002-09-10T23:08:25+00:00'/><message xmlns='jabber:client' to='juliet@capulet.example/balcony' from='romeo@montague.example/home'/></forwarded></result>"
        .parse()
        .unwrap();

        let elem: Element = "<forwarded xmlns='urn:xmpp:forward:0'><delay xmlns='urn:xmpp:delay' stamp='2002-09-10T23:08:25+00:00'/><message xmlns='jabber:client' to='juliet@capulet.example/balcony' from='romeo@montague.example/home'/></forwarded>"
          .parse()
          .unwrap();

        let forwarded = Forwarded::try_from(elem).unwrap();

        let result = Result_ {
            id: String::from("28482-98726-73623"),
            queryid: Some(QueryId(String::from("f27"))),
            forwarded,
        };
        let serialized: Element = result.into();
        assert_eq!(serialized, reference);
    }

    #[test]
    fn test_serialize_fin() {
        let reference: Element = "<fin xmlns='urn:xmpp:mam:2'><set xmlns='http://jabber.org/protocol/rsm'><first index='0'>28482-98726-73623</first><last>09af3-cc343-b409f</last></set></fin>"
        .parse()
        .unwrap();

        let elem: Element = "<set xmlns='http://jabber.org/protocol/rsm'><first index='0'>28482-98726-73623</first><last>09af3-cc343-b409f</last></set>"
          .parse()
          .unwrap();

        let set = SetResult::try_from(elem).unwrap();

        let fin = Fin {
            set,
            complete: Complete::default(),
        };
        let serialized: Element = fin.into();
        assert_eq!(serialized, reference);
    }
}
