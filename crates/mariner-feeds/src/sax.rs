//! Walks an XML tree with the rules in `rules`, building objects the way a
//! SAX handler would, then turns them into the model.
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
use crate::model::{Enclosure, Entry, Feed, Format, Itunes};
use crate::rules::{self, Class, Field, Flavor, Kind, Rule};
use crate::tree::{self, Element, Node};
use crate::xhtml;

pub(crate) fn parse(text: &str, flavor: Flavor, declared_encoding: bool) -> Feed {
    let mut nodes = tree::parse(text);
    if flavor.preprocesses() {
        tree::remove_namespaces(&mut nodes);
        xhtml::flatten_entry_content(&mut nodes, declared_encoding);
    }
    let mut handler = Handler::new(Class::Feed(flavor));
    handler.walk(&nodes);
    handler.feed(flavor)
}

#[derive(Default)]
struct Object {
    singles: HashMap<Field, Option<String>>,
    multis: HashMap<Field, Vec<Option<String>>>,
    objects: HashMap<Field, usize>,
    entries: Vec<usize>,
    published: Option<DateTime<Utc>>,
    updated: Option<DateTime<Utc>>,
    /// Guid text, and its `isPermaLink` attribute.
    guid: Option<String>,
    perma_link: Option<String>,
}

impl Object {
    fn single(&self, field: Field) -> Option<&str> {
        self.singles.get(&field).and_then(|value| value.as_deref())
    }

    fn multi(&self, field: Field) -> impl Iterator<Item = &Option<String>> {
        self.multis.get(&field).into_iter().flatten()
    }
}

struct Frame {
    object: usize,
    rule: Option<&'static Rule>,
    buffer: Option<String>,
}

struct Handler {
    objects: Vec<Object>,
    classes: Vec<Class>,
    stack: Vec<Frame>,
    parsed: HashSet<(usize, *const Rule)>,
}

impl Handler {
    fn new(root: Class) -> Self {
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
        self.start(&name, &element.attrs);
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

        let collection = rules::rules(self.classes[object]).iter().find(|rule| {
            matches!(rule.kind, Kind::Collection(_)) && rule.tag == name && rule.matches(attrs)
        });
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

        let candidates = rules::rules(self.classes[object])
            .iter()
            .filter(|rule| !matches!(rule.kind, Kind::Collection(_)) && rule.tag == name);
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
                    if class == Class::Guid {
                        self.objects[frame.object].guid = frame.buffer;
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

    fn new_object(&mut self, class: Class, attrs: &[(String, String)]) -> usize {
        let mut object = Object::default();
        if class == Class::Guid
            && attrs
                .iter()
                .any(|(key, value)| key == "isPermaLink" || value == "isPermaLink")
        {
            object.perma_link = attrs
                .iter()
                .find(|(key, _)| key == "isPermaLink")
                .map(|(_, value)| value.clone());
        }
        self.objects.push(object);
        self.classes.push(class);
        self.objects.len() - 1
    }

    fn set(&mut self, object: usize, rule: &Rule, value: Option<String>) {
        let object = &mut self.objects[object];
        match (rule.field, rule.kind) {
            (Field::Published, _) => {
                if let Some(time) = value.as_deref().and_then(date::parse) {
                    object.published = Some(object.published.map_or(time, |old| old.min(time)));
                }
            }
            (Field::Updated, _) => {
                if let Some(time) = value.as_deref().and_then(date::parse) {
                    object.updated = Some(object.updated.map_or(time, |old| old.max(time)));
                }
            }
            (field, Kind::Multi) => object.multis.entry(field).or_default().push(value),
            (field, _) => {
                object.singles.insert(field, value);
            }
        }
    }

    fn mark_parsed(&mut self, object: usize, rule: &Rule) {
        if rule.kind != Kind::Multi {
            self.parsed.insert((object, rule as *const Rule));
        }
    }

    fn feed(&self, flavor: Flavor) -> Feed {
        let feed = &self.objects[0];
        let image = feed
            .objects
            .get(&Field::Image)
            .map(|&index| &self.objects[index]);
        let (site_url, self_url) = match flavor {
            Flavor::Atom => {
                let self_url = feed.single(Field::FeedUrl);
                let site_url = feed.single(Field::Url).or_else(|| {
                    feed.multi(Field::Links)
                        .filter(|link| link.as_deref() != self_url)
                        .last()
                        .and_then(|link| link.as_deref())
                });
                (site_url, self_url)
            }
            Flavor::AtomFeedBurner => (
                feed.single(Field::UrlTextHtml)
                    .or(feed.single(Field::UrlNotype)),
                feed.single(Field::FeedUrlLink)
                    .or(feed.single(Field::FeedUrlAtom10Link)),
            ),
            Flavor::Youtube => (feed.single(Field::Url), feed.single(Field::FeedUrl)),
            _ => (feed.single(Field::Url), None),
        };
        Feed {
            format: if flavor.is_atom() {
                Format::Atom
            } else {
                Format::Rss
            },
            title: owned(feed.single(Field::Title)),
            description: owned(feed.single(Field::Description)),
            site_url: owned(site_url),
            self_url: owned(self_url),
            hubs: feed.multi(Field::Hubs).flatten().cloned().collect(),
            language: owned(feed.single(Field::Language)),
            icon: owned(feed.single(Field::Icon)),
            image_url: owned(image.and_then(|image| image.single(Field::Url))),
            itunes_image: owned(feed.single(Field::ItunesImage)),
            entries: feed
                .entries
                .iter()
                .map(|&index| self.entry(&self.objects[index], flavor))
                .collect(),
        }
    }

    fn entry(&self, entry: &Object, flavor: Flavor) -> Entry {
        let guid = entry
            .objects
            .get(&Field::EntryId)
            .map(|&index| &self.objects[index]);
        let (id, url) = if flavor.is_atom() {
            let url = entry.single(Field::Url).or_else(|| {
                entry
                    .multi(Field::Links)
                    .next()
                    .and_then(|link| link.as_deref())
            });
            (entry.single(Field::EntryId), url)
        } else {
            let guid_text = guid.and_then(|guid| guid.guid.as_deref());
            let perma_link = guid.is_some_and(|guid| guid.perma_link.as_deref() != Some("false"));
            let url = entry
                .single(Field::Url)
                .or(guid_text.filter(|_| perma_link));
            (guid_text, url)
        };
        let url = match flavor {
            Flavor::RssFeedBurner | Flavor::AtomFeedBurner => entry.single(Field::OrigLink).or(url),
            _ => url,
        };
        let title = match entry.singles.get(&Field::RawTitle) {
            Some(Some(raw)) => Some(xhtml::text_content(raw)),
            _ => owned(entry.single(Field::Title)),
        };
        let author = entry
            .single(Field::Author)
            .or(entry.single(Field::ItunesAuthor));
        let enclosure = entry.single(Field::EnclosureUrl).map(|url| Enclosure {
            url: url.to_string(),
            mime_type: owned(entry.single(Field::EnclosureType)),
            length: owned(entry.single(Field::EnclosureLength)),
        });
        let itunes = (flavor == Flavor::Itunes).then(|| Itunes {
            duration: owned(entry.single(Field::ItunesDuration)),
            episode: owned(entry.single(Field::ItunesEpisode)),
            season: owned(entry.single(Field::ItunesSeason)),
            episode_type: owned(entry.single(Field::ItunesEpisodeType)),
            explicit: owned(entry.single(Field::ItunesExplicit)),
            image: owned(entry.single(Field::ItunesImage)),
            subtitle: owned(entry.single(Field::ItunesSubtitle)),
            summary: owned(entry.single(Field::ItunesSummary)),
            title: owned(entry.single(Field::ItunesTitle)),
        });
        Entry {
            id: id.map(|id| id.trim().to_string()),
            title: title.map(|title| title.trim().to_string()),
            url: url.map(|url| url.trim().to_string()),
            author: owned(author),
            content: owned(entry.single(Field::Content)),
            summary: owned(entry.single(Field::Summary)),
            published: entry.published.or(entry.updated),
            updated: entry.updated,
            categories: entry
                .multi(Field::Categories)
                .map(|category| category.as_deref().unwrap_or_default().trim())
                .filter(|category| !category.is_empty())
                .map(String::from)
                .collect(),
            image: owned(entry.single(Field::Image)),
            enclosure,
            itunes,
        }
    }
}

fn owned(value: Option<&str>) -> Option<String> {
    value.map(String::from)
}
