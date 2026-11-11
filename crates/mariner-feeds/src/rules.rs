//! The vocabulary for describing which XML elements fill which fields:
//! rules, grouped into classes. Each parser in `parser/` declares its
//! classes with these; `sax` runs them.
//!
//! Element names are matched as written (prefix included, `-` read as
//! `_`), at any depth inside the current object, and the first occurrence
//! of each rule wins. Several rules may fill one field; then the last of
//! their first occurrences wins, except dates (see `sax`).

/// An object the parser builds (a feed, an entry, an RSS image...): the
/// rules for elements inside it.
pub(crate) struct Class {
    pub rules: &'static [Rule],
    /// Keeps the element's own text, as a GUID does.
    pub keeps_text: bool,
    /// An attribute of the element itself to keep. Read when the element
    /// has that attribute, or any attribute whose value is that name.
    pub attribute: Option<&'static str>,
}

impl Class {
    pub const fn new(rules: &'static [Rule]) -> Self {
        Class {
            rules,
            keeps_text: false,
            attribute: None,
        }
    }
}

/// Swallows an element and everything in it.
pub(crate) static SINK: Class = Class::new(&[]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Field {
    Title,
    /// An Atom title whose type is HTML-like; read as HTML text.
    RawTitle,
    TitleType,
    Description,
    Summary,
    Content,
    Url,
    UrlTextHtml,
    UrlNotype,
    FeedUrl,
    FeedUrlLink,
    FeedUrlAtom10Link,
    Links,
    Hubs,
    Language,
    Icon,
    Author,
    Published,
    Updated,
    EntryId,
    Image,
    Categories,
    OrigLink,
    EnclosureUrl,
    EnclosureType,
    EnclosureLength,
    ItunesAuthor,
    ItunesDuration,
    ItunesEpisode,
    ItunesSeason,
    ItunesEpisodeType,
    ItunesExplicit,
    ItunesImage,
    ItunesSubtitle,
    ItunesSummary,
    ItunesTitle,
    YoutubeVideoId,
    YoutubeChannelId,
    /// Matched but unused; still takes part in rule selection.
    Other,
    Entries,
    Nested,
}

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// One value; the first occurrence wins.
    Single,
    /// Every occurrence is appended.
    Multi,
    /// Builds a nested object of this class; the first occurrence wins.
    Object(&'static Class),
    /// Builds a nested object of this class per occurrence.
    Collection(&'static Class),
}

impl Kind {
    pub fn is_collection(self) -> bool {
        matches!(self, Kind::Collection(_))
    }

    pub fn is_multi(self) -> bool {
        matches!(self, Kind::Multi)
    }
}

/// `None` in `with` means the attribute must be absent.
pub(crate) type With = &'static [(&'static str, Option<&'static str>)];

pub(crate) struct Rule {
    pub tag: &'static str,
    pub field: Field,
    pub kind: Kind,
    /// Read this attribute instead of the element's text.
    pub value: Option<&'static str>,
    pub with: With,
}

impl Rule {
    pub fn matches(&self, attrs: &[(String, String)]) -> bool {
        self.with.iter().all(|(key, expected)| {
            let actual = attrs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str());
            actual == *expected
        })
    }
}

pub(crate) const HUB: With = &[("rel", Some("hub"))];
pub(crate) const SELF: With = &[("rel", Some("self"))];
pub(crate) const ALTERNATE: With = &[("rel", Some("alternate"))];
pub(crate) const TEXT_HTML: With = &[("type", Some("text/html"))];

/// The element's text, once.
pub(crate) const fn text(tag: &'static str, field: Field) -> Rule {
    rule(tag, field, Kind::Single, None, &[])
}

/// The element's text, once, from elements with these attributes.
pub(crate) const fn text_with(tag: &'static str, field: Field, with: With) -> Rule {
    rule(tag, field, Kind::Single, None, with)
}

/// The text of every such element.
pub(crate) const fn texts(tag: &'static str, field: Field) -> Rule {
    rule(tag, field, Kind::Multi, None, &[])
}

/// An attribute's value, once.
pub(crate) const fn attr(tag: &'static str, field: Field, value: &'static str) -> Rule {
    rule(tag, field, Kind::Single, Some(value), &[])
}

/// An attribute's value, once, from elements with these attributes.
pub(crate) const fn attr_with(
    tag: &'static str,
    field: Field,
    value: &'static str,
    with: With,
) -> Rule {
    rule(tag, field, Kind::Single, Some(value), with)
}

/// An attribute's value from every such element.
pub(crate) const fn attrs(
    tag: &'static str,
    field: Field,
    value: &'static str,
    with: With,
) -> Rule {
    rule(tag, field, Kind::Multi, Some(value), with)
}

/// A nested object, once.
pub(crate) const fn object(tag: &'static str, field: Field, class: &'static Class) -> Rule {
    rule(tag, field, Kind::Object(class), None, &[])
}

/// A nested object per element.
pub(crate) const fn collection(tag: &'static str, field: Field, class: &'static Class) -> Rule {
    rule(tag, field, Kind::Collection(class), None, &[])
}

const fn rule(
    tag: &'static str,
    field: Field,
    kind: Kind,
    value: Option<&'static str>,
    with: With,
) -> Rule {
    Rule {
        tag,
        field,
        kind,
        value,
        with,
    }
}
