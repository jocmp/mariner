//! A minimal XML tree. Names keep their prefixes as written.
//!
//! Like libxml2's SAX parser, a well-formedness error (an undefined entity,
//! a stray `&` or `<`) ends the document there: everything before it is
//! kept, and elements still open are left unclosed.

use quick_xml::XmlVersion;
use quick_xml::escape::resolve_xml_entity;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Element(Element),
    Text(String),
    /// A CDATA section. Same as text, except when serialized as HTML.
    CData(String),
    Comment(String),
}

#[derive(Debug, Clone)]
pub(crate) struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// False when the document ended (or broke) before the end tag.
    pub closed: bool,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Parses `text` into its top-level nodes.
pub(crate) fn parse(text: &str) -> Vec<Node> {
    let mut reader = Reader::from_str(text);
    let config = reader.config_mut();
    config.expand_empty_elements = true;

    let mut stack: Vec<Element> = Vec::new();
    let mut top = Vec::new();
    loop {
        let node = match reader.read_event() {
            Ok(Event::Start(start)) => match element(&start) {
                Some(element) => {
                    stack.push(element);
                    continue;
                }
                None => break,
            },
            Ok(Event::End(_)) => {
                let Some(mut element) = stack.pop() else {
                    continue;
                };
                element.closed = true;
                Node::Element(element)
            }
            Ok(Event::Text(text)) => Node::Text(text.xml_content(XmlVersion::Implicit1_0).into()),
            Ok(Event::CData(data)) => Node::CData(data.xml_content(XmlVersion::Implicit1_0).into()),
            Ok(Event::GeneralRef(reference)) => {
                let resolved = match reference.resolve_char_ref() {
                    Ok(Some(c)) => c.to_string(),
                    Ok(None) => match resolve_xml_entity(&reference) {
                        Some(value) => value.to_string(),
                        None => break,
                    },
                    Err(_) => break,
                };
                Node::Text(resolved)
            }
            Ok(Event::Comment(comment)) => Node::Comment(comment.to_string()),
            Ok(Event::Eof) | Err(_) => break,
            Ok(_) => continue,
        };
        match stack.last_mut() {
            Some(parent) => push(&mut parent.children, node),
            None => push(&mut top, node),
        }
    }
    while let Some(element) = stack.pop() {
        let node = Node::Element(element);
        match stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => top.push(node),
        }
    }
    top
}

/// Appends a node, merging adjacent text.
fn push(nodes: &mut Vec<Node>, node: Node) {
    if let (Some(Node::Text(last)), Node::Text(text)) = (nodes.last_mut(), &node) {
        last.push_str(text);
        return;
    }
    nodes.push(node);
}

fn element(start: &BytesStart) -> Option<Element> {
    let name = start.name().as_ref().to_string();
    let mut attrs = Vec::new();
    for attr in start.attributes().with_checks(false) {
        let attr = attr.ok()?;
        let key = attr.key.as_ref().to_string();
        let value = attr
            .normalized_value_with(XmlVersion::Implicit1_0, 1, resolve_xml_entity)
            .ok()?;
        attrs.push((key, value.into_owned()));
    }
    Some(Element {
        name,
        attrs,
        children: Vec::new(),
        closed: false,
    })
}

/// Drops namespace prefixes from element and attribute names, and drops
/// namespace declarations.
pub(crate) fn remove_namespaces(nodes: &mut [Node]) {
    for node in nodes {
        if let Node::Element(element) = node {
            element.name = local_name(&element.name).to_string();
            element
                .attrs
                .retain(|(key, _)| key != "xmlns" && !key.starts_with("xmlns:"));
            for (key, _) in &mut element.attrs {
                *key = local_name(key).to_string();
            }
            remove_namespaces(&mut element.children);
        }
    }
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}
