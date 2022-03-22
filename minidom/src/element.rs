// Copyright (c) 2020 lumi <lumi@pew.im>
// Copyright (c) 2020 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
// Copyright (c) 2020 Bastien Orivel <eijebong+minidom@bananium.fr>
// Copyright (c) 2020 Maxime “pep” Buquet <pep@bouah.net>
// Copyright (c) 2020 Yue Liu <amznyue@amazon.com>
// Copyright (c) 2020 Matt Bilker <me@mbilker.us>
// Copyright (c) 2020 Xidorn Quan <me@upsuper.org>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Provides an `Element` type, which represents DOM nodes, and a builder to create them with.

use crate::convert::IntoAttributeValue;
use crate::error::{Error, Result};
use crate::namespaces::NSChoice;
use crate::node::Node;
use crate::prefixes::{Namespace, Prefix, Prefixes};
use crate::tree_builder::TreeBuilder;

use std::collections::{btree_map, BTreeMap};
use std::io::{BufRead, Write};

use std::borrow::Cow;
use std::str;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer as EventWriter;
use rxml::{EventRead, Lexer, PullDriver, RawParser};

use std::str::FromStr;

use std::slice;

/// helper function to escape a `&[u8]` and replace all
/// xml special characters (<, >, &, ', ") with their corresponding
/// xml escaped value.
pub fn escape(raw: &[u8]) -> Cow<[u8]> {
    let mut escapes: Vec<(usize, &'static [u8])> = Vec::new();
    let mut bytes = raw.iter();
    fn to_escape(b: u8) -> bool {
        matches!(b, b'<' | b'>' | b'\'' | b'&' | b'"')
    }

    let mut loc = 0;
    while let Some(i) = bytes.position(|&b| to_escape(b)) {
        loc += i;
        match raw[loc] {
            b'<' => escapes.push((loc, b"&lt;")),
            b'>' => escapes.push((loc, b"&gt;")),
            b'\'' => escapes.push((loc, b"&apos;")),
            b'&' => escapes.push((loc, b"&amp;")),
            b'"' => escapes.push((loc, b"&quot;")),
            _ => unreachable!("Only '<', '>','\', '&' and '\"' are escaped"),
        }
        loc += 1;
    }

    if escapes.is_empty() {
        Cow::Borrowed(raw)
    } else {
        let len = raw.len();
        let mut v = Vec::with_capacity(len);
        let mut start = 0;
        for (i, r) in escapes {
            v.extend_from_slice(&raw[start..i]);
            v.extend_from_slice(r);
            start = i + 1;
        }

        if start < len {
            v.extend_from_slice(&raw[start..]);
        }
        Cow::Owned(v)
    }
}

#[derive(Clone, Eq, Debug)]
/// A struct representing a DOM Element.
pub struct Element {
    name: String,
    namespace: String,
    /// This is only used when deserializing. If you have to use a custom prefix use
    /// `ElementBuilder::prefix`.
    pub(crate) prefix: Option<Prefix>,
    /// Namespace declarations
    pub prefixes: Prefixes,
    attributes: BTreeMap<String, String>,
    children: Vec<Node>,
}

impl<'a> From<&'a Element> for String {
    fn from(elem: &'a Element) -> String {
        let mut writer = Vec::new();
        elem.write_to(&mut writer).unwrap();
        String::from_utf8(writer).unwrap()
    }
}

impl FromStr for Element {
    type Err = Error;

    fn from_str(s: &str) -> Result<Element> {
        Element::from_reader(s.as_bytes())
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        if self.name() == other.name() && self.ns() == other.ns() && self.attrs().eq(other.attrs())
        {
            self.nodes()
                .zip(other.nodes())
                .all(|(node1, node2)| node1 == node2)
        } else {
            false
        }
    }
}

impl Element {
    pub(crate) fn new<P: Into<Prefixes>>(
        name: String,
        namespace: String,
        prefix: Option<Prefix>,
        prefixes: P,
        attributes: BTreeMap<String, String>,
        children: Vec<Node>,
    ) -> Element {
        Element {
            name,
            namespace,
            prefix,
            prefixes: prefixes.into(),
            attributes,
            children,
        }
    }

    /// Return a builder for an `Element` with the given `name`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem = Element::builder("name", "namespace")
    ///                    .attr("name", "value")
    ///                    .append("inner")
    ///                    .build();
    ///
    /// assert_eq!(elem.name(), "name");
    /// assert_eq!(elem.ns(), "namespace".to_owned());
    /// assert_eq!(elem.attr("name"), Some("value"));
    /// assert_eq!(elem.attr("inexistent"), None);
    /// assert_eq!(elem.text(), "inner");
    /// ```
    pub fn builder<S: AsRef<str>, NS: Into<String>>(name: S, namespace: NS) -> ElementBuilder {
        ElementBuilder {
            root: Element::new(
                name.as_ref().to_string(),
                namespace.into(),
                None,
                None,
                BTreeMap::new(),
                Vec::new(),
            ),
        }
    }

    /// Returns a bare minimum `Element` with this name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let bare = Element::bare("name", "namespace");
    ///
    /// assert_eq!(bare.name(), "name");
    /// assert_eq!(bare.ns(), "namespace");
    /// assert_eq!(bare.attr("name"), None);
    /// assert_eq!(bare.text(), "");
    /// ```
    pub fn bare<S: Into<String>, NS: Into<String>>(name: S, namespace: NS) -> Element {
        Element::new(
            name.into(),
            namespace.into(),
            None,
            None,
            BTreeMap::new(),
            Vec::new(),
        )
    }

    /// Returns a reference to the local name of this element (that is, without a possible prefix).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the namespace of this element.
    pub fn ns(&self) -> String {
        self.namespace.clone()
    }

    /// Returns a reference to the value of the given attribute, if it exists, else `None`.
    pub fn attr(&self, name: &str) -> Option<&str> {
        if let Some(value) = self.attributes.get(name) {
            return Some(value);
        }
        None
    }

    /// Returns an iterator over the attributes of this element.
    ///
    /// # Example
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elm: Element = "<elem xmlns=\"ns1\" a=\"b\" />".parse().unwrap();
    ///
    /// let mut iter = elm.attrs();
    ///
    /// assert_eq!(iter.next().unwrap(), ("a", "b"));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn attrs(&self) -> Attrs {
        Attrs {
            iter: self.attributes.iter(),
        }
    }

    /// Returns an iterator over the attributes of this element, with the value being a mutable
    /// reference.
    pub fn attrs_mut(&mut self) -> AttrsMut {
        AttrsMut {
            iter: self.attributes.iter_mut(),
        }
    }

    /// Modifies the value of an attribute.
    pub fn set_attr<S: Into<String>, V: IntoAttributeValue>(&mut self, name: S, val: V) {
        let name = name.into();
        let val = val.into_attribute_value();

        if let Some(value) = self.attributes.get_mut(&name) {
            *value = val
                .expect("removing existing value via set_attr, this is not yet supported (TODO)"); // TODO
            return;
        }

        if let Some(val) = val {
            self.attributes.insert(name, val);
        }
    }

    /// Returns whether the element has the given name and namespace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem = Element::builder("name", "namespace").build();
    ///
    /// assert_eq!(elem.is("name", "namespace"), true);
    /// assert_eq!(elem.is("name", "wrong"), false);
    /// assert_eq!(elem.is("wrong", "namespace"), false);
    /// assert_eq!(elem.is("wrong", "wrong"), false);
    ///
    /// assert_eq!(elem.is("name", NSChoice::OneOf("namespace")), true);
    /// assert_eq!(elem.is("name", NSChoice::OneOf("foo")), false);
    /// assert_eq!(elem.is("name", NSChoice::AnyOf(&["foo", "namespace"])), true);
    /// assert_eq!(elem.is("name", NSChoice::Any), true);
    /// ```
    pub fn is<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(&self, name: N, namespace: NS) -> bool {
        self.name == name.as_ref() && namespace.into().compare(self.namespace.as_ref())
    }

    /// Returns whether the element has the given namespace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem = Element::builder("name", "namespace").build();
    ///
    /// assert_eq!(elem.has_ns("namespace"), true);
    /// assert_eq!(elem.has_ns("wrong"), false);
    ///
    /// assert_eq!(elem.has_ns(NSChoice::OneOf("namespace")), true);
    /// assert_eq!(elem.has_ns(NSChoice::OneOf("foo")), false);
    /// assert_eq!(elem.has_ns(NSChoice::AnyOf(&["foo", "namespace"])), true);
    /// assert_eq!(elem.has_ns(NSChoice::Any), true);
    /// ```
    pub fn has_ns<'a, NS: Into<NSChoice<'a>>>(&self, namespace: NS) -> bool {
        namespace.into().compare(self.namespace.as_ref())
    }

    /// Parse a document from a `BufRead`.
    pub fn from_reader<R: BufRead>(reader: R) -> Result<Element> {
        let mut tree_builder = TreeBuilder::new();
        let mut driver = PullDriver::wrap(reader, Lexer::new(), RawParser::new());
        while let Some(event) = driver.read()? {
            tree_builder.process_event(event)?;

            if let Some(root) = tree_builder.root.take() {
                return Ok(root);
            }
        }
        Err(Error::EndOfDocument)
    }

    /// Output a document to a `Writer`.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.to_writer(&mut EventWriter::new(writer))
    }

    /// Output a document to a `Writer`.
    pub fn write_to_decl<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.to_writer_decl(&mut EventWriter::new(writer))
    }

    /// Output the document to quick-xml `Writer`
    pub fn to_writer<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<()> {
        self.write_to_inner(writer, &mut BTreeMap::new())
    }

    /// Output the document to quick-xml `Writer`
    pub fn to_writer_decl<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<()> {
        writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"utf-8"), None)))?;
        self.write_to_inner(writer, &mut BTreeMap::new())
    }

    /// Like `write_to()` but without the `<?xml?>` prelude
    pub fn write_to_inner<W: Write>(
        &self,
        writer: &mut EventWriter<W>,
        all_prefixes: &mut BTreeMap<Prefix, Namespace>,
    ) -> Result<()> {
        let local_prefixes: &BTreeMap<Option<String>, String> = self.prefixes.declared_prefixes();

        // Element namespace
        // If the element prefix hasn't been set yet via a custom prefix, add it.
        let mut existing_self_prefix: Option<Option<String>> = None;
        for (prefix, ns) in local_prefixes.iter().chain(all_prefixes.iter()) {
            if ns == &self.namespace {
                existing_self_prefix = Some(prefix.clone());
            }
        }

        let mut all_keys = all_prefixes.keys().cloned();
        let mut local_keys = local_prefixes.keys().cloned();
        let self_prefix: (Option<String>, bool) = match existing_self_prefix {
            // No prefix exists already for our namespace
            None => {
                if !local_keys.any(|p| p.is_none()) {
                    // Use the None prefix if available
                    (None, true)
                } else {
                    // Otherwise generate one. Check if it isn't already used, if so increase the
                    // number until we find a suitable one.
                    let mut prefix_n = 0u8;
                    while all_keys.any(|p| p == Some(format!("ns{}", prefix_n))) {
                        prefix_n += 1;
                    }
                    (Some(format!("ns{}", prefix_n)), true)
                }
            }
            // Some prefix has already been declared (or is going to be) for our namespace. We
            // don't need to declare a new one. We do however need to remember which one to use in
            // the tag name.
            Some(prefix) => (prefix, false),
        };

        let name = match self_prefix {
            (Some(ref prefix), _) => Cow::Owned(format!("{}:{}", prefix, self.name)),
            _ => Cow::Borrowed(&self.name),
        };
        let mut start = BytesStart::borrowed(name.as_bytes(), name.len());

        // Write self prefix if necessary
        match self_prefix {
            (Some(ref p), true) => {
                let key = format!("xmlns:{}", p);
                start.push_attribute((key.as_bytes(), self.namespace.as_bytes()));
                all_prefixes.insert(self_prefix.0, self.namespace.clone());
            }
            (None, true) => {
                let key = String::from("xmlns");
                start.push_attribute((key.as_bytes(), self.namespace.as_bytes()));
                all_prefixes.insert(self_prefix.0, self.namespace.clone());
            }
            _ => (),
        };

        // Custom prefixes/namespace sets
        for (prefix, ns) in local_prefixes {
            match all_prefixes.get(prefix) {
                p @ Some(_) if p == prefix.as_ref() => (),
                _ => {
                    let key = match prefix {
                        None => String::from("xmlns"),
                        Some(p) => format!("xmlns:{}", p),
                    };

                    start.push_attribute((key.as_bytes(), ns.as_ref()));
                    all_prefixes.insert(prefix.clone(), ns.clone());
                }
            }
        }

        for (key, value) in &self.attributes {
            start.push_attribute((key.as_bytes(), escape(value.as_bytes()).as_ref()));
        }

        if self.children.is_empty() {
            writer.write_event(Event::Empty(start))?;
            return Ok(());
        }

        writer.write_event(Event::Start(start))?;

        for child in &self.children {
            child.write_to_inner(writer, &mut all_prefixes.clone())?;
        }

        writer.write_event(Event::End(BytesEnd::borrowed(name.as_bytes())))?;
        Ok(())
    }

    /// Returns an iterator over references to every child node of this element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<root xmlns=\"ns1\">a<c1 />b<c2 />c</root>".parse().unwrap();
    ///
    /// let mut iter = elem.nodes();
    ///
    /// assert_eq!(iter.next().unwrap().as_text().unwrap(), "a");
    /// assert_eq!(iter.next().unwrap().as_element().unwrap().name(), "c1");
    /// assert_eq!(iter.next().unwrap().as_text().unwrap(), "b");
    /// assert_eq!(iter.next().unwrap().as_element().unwrap().name(), "c2");
    /// assert_eq!(iter.next().unwrap().as_text().unwrap(), "c");
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn nodes(&self) -> Nodes {
        self.children.iter()
    }

    /// Returns an iterator over mutable references to every child node of this element.
    #[inline]
    pub fn nodes_mut(&mut self) -> NodesMut {
        self.children.iter_mut()
    }

    /// Returns an iterator over references to every child element of this element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<root xmlns=\"ns1\">hello<child1 xmlns=\"ns1\"/>this<child2 xmlns=\"ns1\"/>is<child3 xmlns=\"ns1\"/>ignored</root>".parse().unwrap();
    ///
    /// let mut iter = elem.children();
    /// assert_eq!(iter.next().unwrap().name(), "child1");
    /// assert_eq!(iter.next().unwrap().name(), "child2");
    /// assert_eq!(iter.next().unwrap().name(), "child3");
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn children(&self) -> Children {
        Children {
            iter: self.children.iter(),
        }
    }

    /// Returns an iterator over mutable references to every child element of this element.
    #[inline]
    pub fn children_mut(&mut self) -> ChildrenMut {
        ChildrenMut {
            iter: self.children.iter_mut(),
        }
    }

    /// Returns an iterator over references to every text node of this element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<root xmlns=\"ns1\">hello<c /> world!</root>".parse().unwrap();
    ///
    /// let mut iter = elem.texts();
    /// assert_eq!(iter.next().unwrap(), "hello");
    /// assert_eq!(iter.next().unwrap(), " world!");
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn texts(&self) -> Texts {
        Texts {
            iter: self.children.iter(),
        }
    }

    /// Returns an iterator over mutable references to every text node of this element.
    #[inline]
    pub fn texts_mut(&mut self) -> TextsMut {
        TextsMut {
            iter: self.children.iter_mut(),
        }
    }

    /// Appends a child node to the `Element`, returning the appended node.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let mut elem = Element::bare("root", "ns1");
    ///
    /// assert_eq!(elem.children().count(), 0);
    ///
    /// elem.append_child(Element::bare("child", "ns1"));
    ///
    /// {
    ///     let mut iter = elem.children();
    ///     assert_eq!(iter.next().unwrap().name(), "child");
    ///     assert_eq!(iter.next(), None);
    /// }
    ///
    /// let child = elem.append_child(Element::bare("new", "ns1"));
    ///
    /// assert_eq!(child.name(), "new");
    /// ```
    pub fn append_child(&mut self, child: Element) -> &mut Element {
        self.children.push(Node::Element(child));
        if let Node::Element(ref mut cld) = *self.children.last_mut().unwrap() {
            cld
        } else {
            unreachable!()
        }
    }

    /// Appends a text node to an `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let mut elem = Element::bare("node", "ns1");
    ///
    /// assert_eq!(elem.text(), "");
    ///
    /// elem.append_text_node("text");
    ///
    /// assert_eq!(elem.text(), "text");
    /// ```
    pub fn append_text_node<S: Into<String>>(&mut self, child: S) {
        self.children.push(Node::Text(child.into()));
    }

    /// Appends a node to an `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, Node};
    ///
    /// let mut elem = Element::bare("node", "ns1");
    ///
    /// elem.append_node(Node::Text("hello".to_owned()));
    ///
    /// assert_eq!(elem.text(), "hello");
    /// ```
    pub fn append_node(&mut self, node: Node) {
        self.children.push(node);
    }

    /// Returns the concatenation of all text nodes in the `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<node xmlns=\"ns1\">hello,<split /> world!</node>".parse().unwrap();
    ///
    /// assert_eq!(elem.text(), "hello, world!");
    /// ```
    pub fn text(&self) -> String {
        self.texts().fold(String::new(), |ret, new| ret + new)
    }

    /// Returns a reference to the first child element with the specific name and namespace, if it
    /// exists in the direct descendants of this `Element`, else returns `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem: Element = r#"<node xmlns="ns"><a/><a xmlns="other_ns" /><b/></node>"#.parse().unwrap();
    /// assert!(elem.get_child("a", "ns").unwrap().is("a", "ns"));
    /// assert!(elem.get_child("a", "other_ns").unwrap().is("a", "other_ns"));
    /// assert!(elem.get_child("b", "ns").unwrap().is("b", "ns"));
    /// assert_eq!(elem.get_child("c", "ns"), None);
    /// assert_eq!(elem.get_child("b", "other_ns"), None);
    /// assert_eq!(elem.get_child("a", "inexistent_ns"), None);
    /// ```
    pub fn get_child<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &self,
        name: N,
        namespace: NS,
    ) -> Option<&Element> {
        let namespace = namespace.into();
        for fork in &self.children {
            if let Node::Element(ref e) = *fork {
                if e.is(name.as_ref(), namespace) {
                    return Some(e);
                }
            }
        }
        None
    }

    /// Returns a mutable reference to the first child element with the specific name and namespace,
    /// if it exists in the direct descendants of this `Element`, else returns `None`.
    pub fn get_child_mut<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &mut self,
        name: N,
        namespace: NS,
    ) -> Option<&mut Element> {
        let namespace = namespace.into();
        for fork in &mut self.children {
            if let Node::Element(ref mut e) = *fork {
                if e.is(name.as_ref(), namespace) {
                    return Some(e);
                }
            }
        }
        None
    }

    /// Returns whether a specific child with this name and namespace exists in the direct
    /// descendants of the `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem: Element = r#"<node xmlns="ns"><a /><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert_eq!(elem.has_child("a", "other_ns"), true);
    /// assert_eq!(elem.has_child("a", "ns"), true);
    /// assert_eq!(elem.has_child("a", "inexistent_ns"), false);
    /// assert_eq!(elem.has_child("b", "ns"), true);
    /// assert_eq!(elem.has_child("b", "other_ns"), false);
    /// assert_eq!(elem.has_child("b", "inexistent_ns"), false);
    /// ```
    pub fn has_child<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &self,
        name: N,
        namespace: NS,
    ) -> bool {
        self.get_child(name, namespace).is_some()
    }

    /// Removes the first child with this name and namespace, if it exists, and returns an
    /// `Option<Element>` containing this child if it succeeds.
    /// Returns `None` if no child matches this name and namespace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let mut elem: Element = r#"<node xmlns="ns"><a /><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert!(elem.remove_child("a", "ns").unwrap().is("a", "ns"));
    /// assert!(elem.remove_child("a", "ns").is_none());
    /// assert!(elem.remove_child("inexistent", "inexistent").is_none());
    /// ```
    pub fn remove_child<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &mut self,
        name: N,
        namespace: NS,
    ) -> Option<Element> {
        let name = name.as_ref();
        let namespace = namespace.into();
        let idx = self.children.iter().position(|x| {
            if let Node::Element(ref elm) = x {
                elm.is(name, namespace)
            } else {
                false
            }
        })?;
        self.children.remove(idx).into_element()
    }

    /// Remove the leading nodes up to the first child element and
    /// return it
    pub fn unshift_child(&mut self) -> Option<Element> {
        while self.children.len() > 0 {
            if let Some(el) = self.children.remove(0).into_element() {
                return Some(el);
            }
        }

        None
    }
}

/// An iterator over references to child elements of an `Element`.
pub struct Children<'a> {
    iter: slice::Iter<'a, Node>,
}

impl<'a> Iterator for Children<'a> {
    type Item = &'a Element;

    fn next(&mut self) -> Option<&'a Element> {
        for item in &mut self.iter {
            if let Node::Element(ref child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over mutable references to child elements of an `Element`.
pub struct ChildrenMut<'a> {
    iter: slice::IterMut<'a, Node>,
}

impl<'a> Iterator for ChildrenMut<'a> {
    type Item = &'a mut Element;

    fn next(&mut self) -> Option<&'a mut Element> {
        for item in &mut self.iter {
            if let Node::Element(ref mut child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over references to child text nodes of an `Element`.
pub struct Texts<'a> {
    iter: slice::Iter<'a, Node>,
}

impl<'a> Iterator for Texts<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        for item in &mut self.iter {
            if let Node::Text(ref child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over mutable references to child text nodes of an `Element`.
pub struct TextsMut<'a> {
    iter: slice::IterMut<'a, Node>,
}

impl<'a> Iterator for TextsMut<'a> {
    type Item = &'a mut String;

    fn next(&mut self) -> Option<&'a mut String> {
        for item in &mut self.iter {
            if let Node::Text(ref mut child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over references to all child nodes of an `Element`.
pub type Nodes<'a> = slice::Iter<'a, Node>;

/// An iterator over mutable references to all child nodes of an `Element`.
pub type NodesMut<'a> = slice::IterMut<'a, Node>;

/// An iterator over the attributes of an `Element`.
pub struct Attrs<'a> {
    iter: btree_map::Iter<'a, String, String>,
}

impl<'a> Iterator for Attrs<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(x, y)| (x.as_ref(), y.as_ref()))
    }
}

/// An iterator over the attributes of an `Element`, with the values mutable.
pub struct AttrsMut<'a> {
    iter: btree_map::IterMut<'a, String, String>,
}

impl<'a> Iterator for AttrsMut<'a> {
    type Item = (&'a str, &'a mut String);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(x, y)| (x.as_ref(), y))
    }
}

/// A builder for `Element`s.
pub struct ElementBuilder {
    root: Element,
}

impl ElementBuilder {
    /// Sets a custom prefix. It is not possible to set the same prefix twice.
    pub fn prefix<S: Into<Namespace>>(
        mut self,
        prefix: Prefix,
        namespace: S,
    ) -> Result<ElementBuilder> {
        if self.root.prefixes.get(&prefix).is_some() {
            return Err(Error::DuplicatePrefix);
        }
        self.root.prefixes.insert(prefix, namespace.into());
        Ok(self)
    }

    /// Sets an attribute.
    pub fn attr<S: Into<String>, V: IntoAttributeValue>(
        mut self,
        name: S,
        value: V,
    ) -> ElementBuilder {
        self.root.set_attr(name, value);
        self
    }

    /// Appends anything implementing `Into<Node>` into the tree.
    pub fn append<T: Into<Node>>(mut self, node: T) -> ElementBuilder {
        self.root.append_node(node.into());
        self
    }

    /// Appends an iterator of things implementing `Into<Node>` into the tree.
    pub fn append_all<T: Into<Node>, I: IntoIterator<Item = T>>(
        mut self,
        iter: I,
    ) -> ElementBuilder {
        for node in iter {
            self.root.append_node(node.into());
        }
        self
    }

    /// Builds the `Element`.
    pub fn build(self) -> Element {
        self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_new() {
        use std::iter::FromIterator;

        let elem = Element::new(
            "name".to_owned(),
            "namespace".to_owned(),
            None,
            (None, "namespace".to_owned()),
            BTreeMap::from_iter(vec![("name".to_string(), "value".to_string())].into_iter()),
            Vec::new(),
        );

        assert_eq!(elem.name(), "name");
        assert_eq!(elem.ns(), "namespace".to_owned());
        assert_eq!(elem.attr("name"), Some("value"));
        assert_eq!(elem.attr("inexistent"), None);
    }

    #[test]
    fn test_from_reader_simple() {
        let xml = b"<foo xmlns='ns1'></foo>";
        let elem = Element::from_reader(&xml[..]);

        let elem2 = Element::builder("foo", "ns1").build();

        assert_eq!(elem.unwrap(), elem2);
    }

    #[test]
    fn test_from_reader_nested() {
        let xml = b"<foo xmlns='ns1'><bar xmlns='ns1' baz='qxx' /></foo>";
        let elem = Element::from_reader(&xml[..]);

        let nested = Element::builder("bar", "ns1").attr("baz", "qxx").build();
        let elem2 = Element::builder("foo", "ns1").append(nested).build();

        assert_eq!(elem.unwrap(), elem2);
    }

    #[test]
    fn test_from_reader_with_prefix() {
        let xml = b"<foo xmlns='ns1'><prefix:bar xmlns:prefix='ns1' baz='qxx' /></foo>";
        let elem = Element::from_reader(&xml[..]);

        let nested = Element::builder("bar", "ns1").attr("baz", "qxx").build();
        let elem2 = Element::builder("foo", "ns1").append(nested).build();

        assert_eq!(elem.unwrap(), elem2);
    }

    #[test]
    fn test_from_reader_split_prefix() {
        let xml = b"<foo:bar xmlns:foo='ns1'/>";
        let elem = Element::from_reader(&xml[..]).unwrap();

        assert_eq!(elem.name(), String::from("bar"));
        assert_eq!(elem.ns(), String::from("ns1"));
        // Ensure the prefix is properly added to the store
        assert_eq!(
            elem.prefixes.get(&Some(String::from("foo"))),
            Some(&String::from("ns1"))
        );
    }

    #[test]
    fn parses_spectest_xml() {
        // From: https://gitlab.com/lumi/minidom-rs/issues/8
        let xml = br#"<rng:grammar xmlns:rng="http://relaxng.org/ns/structure/1.0">
                <rng:name xmlns:rng="http://relaxng.org/ns/structure/1.0"></rng:name>
            </rng:grammar>
        "#;
        let _ = Element::from_reader(&xml[..]).unwrap();
    }

    #[test]
    fn does_not_unescape_cdata() {
        let xml = b"<test xmlns='test'><![CDATA[&apos;&gt;blah<blah>]]></test>";
        let elem = Element::from_reader(&xml[..]).unwrap();
        assert_eq!(elem.text(), "&apos;&gt;blah<blah>");
    }

    #[test]
    fn test_compare_all_ns() {
        let xml = b"<foo xmlns='foo' xmlns:bar='baz'><bar:meh xmlns:bar='baz' /></foo>";
        let elem = Element::from_reader(&xml[..]).unwrap();

        let elem2 = elem.clone();

        let xml3 = b"<foo xmlns='foo'><bar:meh xmlns:bar='baz'/></foo>";
        let elem3 = Element::from_reader(&xml3[..]).unwrap();

        let xml4 = b"<prefix:foo xmlns:prefix='foo'><bar:meh xmlns:bar='baz'/></prefix:foo>";
        let elem4 = Element::from_reader(&xml4[..]).unwrap();

        assert_eq!(elem, elem2);
        assert_eq!(elem, elem3);
        assert_eq!(elem, elem4);
    }
}
