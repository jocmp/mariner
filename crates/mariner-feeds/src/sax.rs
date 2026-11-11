//! Walks an XML tree with a parser's classes, building objects the way a
//! SAX handler would.
//!
//! - Each rule fills its field from its first matching element only, even
//!   when that element is empty (it then fills nothing, and later
//!   occurrences are still ignored).
//! - Several rules can fill one field; each sets it when it first matches.
//! - `published` keeps the earliest date set, `updated` the latest.
//! - An element's text includes the text of unmatched elements inside it.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use crate::date;
use crate::media;
use crate::model::{Enclosure, Media};
use crate::parser::Parser;
use crate::rules::{Class, Field, Kind, Rule};
use crate::tree::{self, Element, Node};
use crate::xhtml;

/// Parses `text` with `parser`, returning the document's root object.
pub(crate) fn parse(text: &str, parser: &dyn Parser, declared_encoding: bool) -> Handler {
    let mut nodes = tree::parse(text);
    if parser.preprocesses() {
        tree::remove_namespaces(&mut nodes);
        xhtml::flatten_entry_content(&mut nodes, declared_encoding);
    }
    let mut handler = Handler::new(parser.feed());
    handler.walk(&nodes);
    handler
}

/// A read-only view of one parsed object.
#[derive(Clone, Copy)]
pub(crate) struct Parsed<'a> {
    handler: &'a Handler,
    index: usize,
}

impl<'a> Parsed<'a> {
    pub fn single(&self, field: Field) -> Option<&'a str> {
        self.object_data()
            .singles
            .get(&field)
            .and_then(|value| value.as_deref())
    }

    /// Every value of a field, including occurrences without one.
    pub fn multi(&self, field: Field) -> impl Iterator<Item = Option<&'a str>> + use<'a> {
        self.object_data()
            .multis
            .get(&field)
            .into_iter()
            .flatten()
            .map(|value| value.as_deref())
    }

    pub fn object(&self, field: Field) -> Option<Parsed<'a>> {
        let index = *self.object_data().objects.get(&field)?;
        Some(Parsed {
            handler: self.handler,
            index,
        })
    }

    pub fn entries(&self) -> impl Iterator<Item = Parsed<'a>> + use<'a> {
        let handler = self.handler;
        self.object_data()
            .entries
            .iter()
            .map(move |&index| Parsed { handler, index })
    }

    pub fn published(&self) -> Option<DateTime<Utc>> {
        self.object_data().published
    }

    pub fn updated(&self) -> Option<DateTime<Utc>> {
        self.object_data().updated
    }

    /// The element's own text, for classes that keep it.
    pub fn text(&self) -> Option<&'a str> {
        self.object_data().text.as_deref()
    }

    /// The class's kept attribute.
    pub fn attribute(&self) -> Option<&'a str> {
        self.object_data().attribute.as_deref()
    }

    pub fn media(&self) -> Option<&'a Media> {
        self.object_data().media.as_ref()
    }

    pub fn enclosures(&self) -> &'a [Enclosure] {
        &self.object_data().enclosures
    }

    fn object_data(&self) -> &'a Object {
        &self.handler.objects[self.index]
    }
}

#[derive(Default)]
struct Object {
    singles: HashMap<Field, Option<String>>,
    multis: HashMap<Field, Vec<Option<String>>>,
    objects: HashMap<Field, usize>,
    entries: Vec<usize>,
    published: Option<DateTime<Utc>>,
    updated: Option<DateTime<Utc>>,
    /// The element's own text and kept attribute, for classes that keep
    /// them.
    text: Option<String>,
    attribute: Option<String>,
    /// Read from an entry's element directly, outside the rules.
    media: Option<Media>,
    enclosures: Vec<Enclosure>,
}

struct Frame {
    object: usize,
    rule: Option<&'static Rule>,
    buffer: Option<String>,
}

pub(crate) struct Handler {
    objects: Vec<Object>,
    classes: Vec<&'static Class>,
    stack: Vec<Frame>,
    parsed: HashSet<(usize, *const Rule)>,
}

impl Handler {
    /// The document's root object.
    pub fn root(&self) -> Parsed<'_> {
        Parsed {
            handler: self,
            index: 0,
        }
    }

    fn new(root: &'static Class) -> Self {
        Handler {
            objects: vec![Object::default()],
            classes: vec![root],
            stack: vec![Frame {
                object: 0,
                rule: None,
                buffer: None,
            }],
            parsed: HashSet::new(),
        }
    }

    fn walk(&mut self, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::Element(element) => self.element(element),
                Node::Text(text) | Node::CData(text) => self.characters(text),
                Node::Comment(_) => {}
            }
        }
    }

    fn element(&mut self, element: &Element) {
        let name = element.name.replace('-', "_");
        let depth = self.stack.len();
        self.start(&name, &element.attrs);
        if self.stack.len() > depth
            && let Some(frame) = self.stack.last()
            && frame.rule.is_some_and(|rule| rule.field == Field::Entries)
        {
            let entry = &mut self.objects[frame.object];
            entry.media = media::media(element);
            entry.enclosures = media::enclosures(element);
        }
        self.walk(&element.children);
        if element.closed {
            self.end(&name);
        }
    }

    fn characters(&mut self, text: &str) {
        let frame = self.stack.last_mut().expect("root frame");
        frame.buffer.get_or_insert_with(String::new).push_str(text);
    }

    fn start(&mut self, name: &str, attrs: &[(String, String)]) {
        let mut object = self.stack.last().expect("root frame").object;

        let collection = self.classes[object]
            .rules
            .iter()
            .find(|rule| rule.kind.is_collection() && rule.tag == name && rule.matches(attrs));
        if let Some(rule) = collection {
            let Kind::Collection(class) = rule.kind else {
                unreachable!()
            };
            object = self.new_object(class, attrs);
            self.stack.push(Frame {
                object,
                rule: Some(rule),
                buffer: None,
            });
        }

        let candidates = self.classes[object]
            .rules
            .iter()
            .filter(|rule| !rule.kind.is_collection() && rule.tag == name);
        for rule in candidates.clone() {
            if let Some(attribute) = rule.value.filter(|_| rule.matches(attrs))
                && !self.parsed.contains(&(object, rule as *const Rule))
            {
                let value = attrs
                    .iter()
                    .find(|(key, _)| key == attribute)
                    .map(|(_, value)| value.clone());
                self.set(object, rule, value);
                self.mark_parsed(object, rule);
            }
        }

        if collection.is_none()
            && let Some(rule) = candidates.clone().find(|rule| rule.matches(attrs))
        {
            let target = match rule.kind {
                Kind::Object(class) => self.new_object(class, attrs),
                _ => object,
            };
            self.stack.push(Frame {
                object: target,
                rule: Some(rule),
                buffer: None,
            });
        }
    }

    fn end(&mut self, name: &str) {
        let depth = self.stack.len();
        if depth < 2 {
            return;
        }
        let rule = self.stack[depth - 1]
            .rule
            .expect("pushed frames carry a rule");
        if rule.tag != name {
            return;
        }
        let frame = self.stack.pop().expect("checked depth");
        let parent = self.stack[depth - 2].object;

        if !self.parsed.contains(&(parent, rule as *const Rule)) {
            match rule.kind {
                Kind::Collection(_) if rule.field == Field::Entries => {
                    self.objects[parent].entries.push(frame.object);
                }
                Kind::Collection(_) => {}
                Kind::Object(class) => {
                    if class.keeps_text {
                        self.objects[frame.object].text = frame.buffer;
                    }
                    self.objects[parent]
                        .objects
                        .insert(rule.field, frame.object);
                    self.mark_parsed(parent, rule);
                }
                Kind::Single | Kind::Multi => {
                    if frame.buffer.is_some() {
                        self.set(parent, rule, frame.buffer);
                    }
                    self.mark_parsed(parent, rule);
                }
            }
        }
    }

    fn new_object(&mut self, class: &'static Class, attrs: &[(String, String)]) -> usize {
        let mut object = Object::default();
        if let Some(name) = class.attribute
            && attrs
                .iter()
                .any(|(key, value)| key == name || value == name)
        {
            object.attribute = attrs
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone());
        }
        self.objects.push(object);
        self.classes.push(class);
        self.objects.len() - 1
    }

    fn set(&mut self, object: usize, rule: &Rule, value: Option<String>) {
        let object = &mut self.objects[object];
        match rule.field {
            Field::Published => {
                if let Some(time) = value.as_deref().and_then(date::parse) {
                    object.published = Some(object.published.map_or(time, |old| old.min(time)));
                }
            }
            Field::Updated => {
                if let Some(time) = value.as_deref().and_then(date::parse) {
                    object.updated = Some(object.updated.map_or(time, |old| old.max(time)));
                }
            }
            field if rule.kind.is_multi() => object.multis.entry(field).or_default().push(value),
            field => {
                object.singles.insert(field, value);
            }
        }
    }

    fn mark_parsed(&mut self, object: usize, rule: &Rule) {
        if !rule.kind.is_multi() {
            self.parsed.insert((object, rule as *const Rule));
        }
    }
}
