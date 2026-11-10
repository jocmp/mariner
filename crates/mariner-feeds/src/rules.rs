//! Which XML elements fill which fields, per feed flavor.
//!
//! Matching follows the reference parser's model: element names are matched
//! as written (prefix included, `-` read as `_`), at any depth inside the
//! current object, and the first occurrence of each rule wins. Several rules
//! may fill one field; then the last of their first occurrences wins, except
//! dates (see `sax`).

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flavor {
    /// RSS with the iTunes podcast namespace.
    Itunes,
    /// RSS mentioning FeedBurner.
    RssFeedBurner,
    Youtube,
    /// Atom mentioning FeedBurner.
    AtomFeedBurner,
    Atom,
    Rss,
}

impl Flavor {
    pub fn is_atom(self) -> bool {
        matches!(
            self,
            Flavor::Youtube | Flavor::AtomFeedBurner | Flavor::Atom
        )
    }

    /// Atom documents lose their namespace prefixes, and XHTML content
    /// becomes HTML text, before parsing.
    pub fn preprocesses(self) -> bool {
        matches!(self, Flavor::AtomFeedBurner | Flavor::Atom)
    }
}

/// Picks the flavor by looking at the document text, in the reference's
/// order. `None` when the text isn't an XML feed.
pub(crate) fn detect(text: &str) -> Option<Flavor> {
    static CDATA: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<!\[CDATA\[.*?\]\]>").unwrap());
    static ITUNES: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?i)xmlns:itunes\s?=\s?["']http://www\.itunes\.com/dtds/podcast-1\.0\.dtd["']"#,
        )
        .unwrap()
    });
    static ATOM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"<feed[^>]+xmlns\s?=\s?["'](https?://www\.w3\.org/2005/Atom|http://purl\.org/atom/ns\#)["'][^>]*>"#,
        )
        .unwrap()
    });

    let rss = text.contains("<rss") || text.contains("<rdf");
    let feedburner = text.contains("feedburner");
    if ITUNES.is_match(&CDATA.replace_all(text, "")) {
        Some(Flavor::Itunes)
    } else if rss && feedburner {
        Some(Flavor::RssFeedBurner)
    } else if text.contains(r#"xmlns:yt="http://www.youtube.com/xml/schemas/2015""#) {
        Some(Flavor::Youtube)
    } else if text.contains("<feed") && text.contains("Atom") && feedburner && !rss {
        Some(Flavor::AtomFeedBurner)
    } else if ATOM.is_match(text) {
        Some(Flavor::Atom)
    } else if rss {
        Some(Flavor::Rss)
    } else {
        None
    }
}

/// The kinds of object the parser builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    Feed(Flavor),
    Entry(Flavor),
    /// RSS `<image>`.
    Image,
    /// RSS `<guid>`: its text, plus `isPermaLink`.
    Guid,
    /// Swallows an element and everything in it, nesting into itself.
    Sink(&'static str),
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// One value; the first occurrence wins.
    Single,
    /// Every occurrence is appended.
    Multi,
    /// Builds a nested object of this class; the first occurrence wins.
    Object(Class),
    /// Builds a nested object of this class per occurrence.
    Collection(Class),
}

/// `None` in `with` means the attribute must be absent.
pub(crate) type With = &'static [(&'static str, Option<&'static str>)];

#[derive(Debug)]
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

const fn text(tag: &'static str, field: Field) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Single,
        value: None,
        with: &[],
    }
}

const fn text_with(tag: &'static str, field: Field, with: With) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Single,
        value: None,
        with,
    }
}

const fn texts(tag: &'static str, field: Field) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Multi,
        value: None,
        with: &[],
    }
}

const fn attr(tag: &'static str, field: Field, value: &'static str) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Single,
        value: Some(value),
        with: &[],
    }
}

const fn attr_with(tag: &'static str, field: Field, value: &'static str, with: With) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Single,
        value: Some(value),
        with,
    }
}

const fn attrs(tag: &'static str, field: Field, value: &'static str, with: With) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Multi,
        value: Some(value),
        with,
    }
}

const fn object(tag: &'static str, field: Field, class: Class) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Object(class),
        value: None,
        with: &[],
    }
}

const fn collection(tag: &'static str, field: Field, class: Class) -> Rule {
    Rule {
        tag,
        field,
        kind: Kind::Collection(class),
        value: None,
        with: &[],
    }
}

const HUB: With = &[("rel", Some("hub"))];
const SELF: With = &[("rel", Some("self"))];
const ALTERNATE: With = &[("rel", Some("alternate"))];
const TEXT_HTML: With = &[("type", Some("text/html"))];
const ATOM_XML: With = &[("type", Some("application/atom+xml"))];

pub(crate) fn rules(class: Class) -> &'static [Rule] {
    use Flavor::*;
    match class {
        Class::Feed(Rss) => RSS_FEED,
        Class::Feed(Itunes) => ITUNES_FEED,
        Class::Feed(RssFeedBurner) => RSS_FEEDBURNER_FEED,
        Class::Feed(Atom) => ATOM_FEED,
        Class::Feed(AtomFeedBurner) => ATOM_FEEDBURNER_FEED,
        Class::Feed(Youtube) => YOUTUBE_FEED,
        Class::Entry(Rss) => RSS_ENTRY,
        Class::Entry(Itunes) => ITUNES_ENTRY,
        Class::Entry(RssFeedBurner) => RSS_FEEDBURNER_ENTRY,
        Class::Entry(Atom) => ATOM_ENTRY,
        Class::Entry(AtomFeedBurner) => ATOM_FEEDBURNER_ENTRY,
        Class::Entry(Youtube) => YOUTUBE_ENTRY,
        Class::Image => IMAGE,
        Class::Guid => &[],
        Class::Sink("itunes:category") => ITUNES_CATEGORY,
        Class::Sink(_) => &[],
    }
}

static RSS_FEED: &[Rule] = &[
    text("description", Field::Description),
    object("image", Field::Image, Class::Image),
    text("language", Field::Language),
    text("link", Field::Url),
    attr("a10:link", Field::Url, "href"),
    text("title", Field::Title),
    attrs("atom:link", Field::Hubs, "href", HUB),
    collection("item", Field::Entries, Class::Entry(Flavor::Rss)),
];

static ITUNES_FEED: &[Rule] = &[
    text("description", Field::Description),
    object("image", Field::Image, Class::Image),
    text("language", Field::Language),
    text("link", Field::Url),
    text("title", Field::Title),
    attr("itunes:image", Field::ItunesImage, "href"),
    collection(
        "itunes:category",
        Field::Nested,
        Class::Sink("itunes:category"),
    ),
    collection("itunes:owner", Field::Nested, Class::Sink("itunes:owner")),
    collection("item", Field::Entries, Class::Entry(Flavor::Itunes)),
];

static RSS_FEEDBURNER_FEED: &[Rule] = &[
    text("title", Field::Title),
    text("description", Field::Description),
    text("link", Field::Url),
    attrs("atom10:link", Field::Hubs, "href", HUB),
    collection("item", Field::Entries, Class::Entry(Flavor::RssFeedBurner)),
];

static ATOM_FEED: &[Rule] = &[
    text("title", Field::Title),
    text("subtitle", Field::Description),
    attr_with("link", Field::Url, "href", TEXT_HTML),
    attr_with("link", Field::FeedUrl, "href", SELF),
    attrs("link", Field::Links, "href", &[]),
    attrs("link", Field::Hubs, "href", HUB),
    collection("entry", Field::Entries, Class::Entry(Flavor::Atom)),
    text("icon", Field::Icon),
];

static ATOM_FEEDBURNER_FEED: &[Rule] = &[
    text("title", Field::Title),
    text("subtitle", Field::Description),
    attr_with("link", Field::UrlTextHtml, "href", TEXT_HTML),
    attr_with("link", Field::UrlNotype, "href", &[("type", None)]),
    attr_with("link", Field::FeedUrlLink, "href", ATOM_XML),
    attr_with("atom10:link", Field::FeedUrlAtom10Link, "href", ATOM_XML),
    attrs("atom10:link", Field::Hubs, "href", HUB),
    collection(
        "entry",
        Field::Entries,
        Class::Entry(Flavor::AtomFeedBurner),
    ),
];

static YOUTUBE_FEED: &[Rule] = &[
    text("title", Field::Title),
    attr_with("link", Field::Url, "href", ALTERNATE),
    attr_with("link", Field::FeedUrl, "href", SELF),
    text("name", Field::Other),
    text("yt:channelId", Field::YoutubeChannelId),
    collection("entry", Field::Entries, Class::Entry(Flavor::Youtube)),
];

static IMAGE: &[Rule] = &[
    text("description", Field::Other),
    text("height", Field::Other),
    text("link", Field::Other),
    text("title", Field::Other),
    text("url", Field::Url),
    text("width", Field::Other),
];

static ITUNES_CATEGORY: &[Rule] = &[collection(
    "itunes:category",
    Field::Nested,
    Class::Sink("itunes:category"),
)];

/// Builds an RSS entry's rule list: the shared RSS item rules (with or
/// without reading `enclosure` as the image), then the flavor's own, then a
/// plain `content` element.
macro_rules! rss_entry {
    ([$($enclosure:expr)?]; $($extra:expr),* $(,)?) => {
        &[
            text("title", Field::Title),
            text("content:encoded", Field::Content),
            text("a10:content", Field::Content),
            text("description", Field::Summary),
            text("link", Field::Url),
            attr("a10:link", Field::Url, "href"),
            text("author", Field::Author),
            text("dc:creator", Field::Author),
            text("a10:name", Field::Author),
            text("pubDate", Field::Published),
            text("pubdate", Field::Published),
            text("issued", Field::Published),
            text("dc:date", Field::Published),
            text("dc:Date", Field::Published),
            text("dcterms:created", Field::Published),
            text("dcterms:modified", Field::Updated),
            text("a10:updated", Field::Updated),
            object("guid", Field::EntryId, Class::Guid),
            text("dc:identifier", Field::Other),
            attr("media:thumbnail", Field::Image, "url"),
            attr("media:content", Field::Image, "url"),
            $($enclosure,)?
            text("comments", Field::Other),
            texts("category", Field::Categories),
            $($extra,)*
            text("content", Field::Content),
        ]
    };
}

static RSS_ENTRY: &[Rule] = rss_entry!([attr("enclosure", Field::Image, "url")];);

static RSS_FEEDBURNER_ENTRY: &[Rule] = rss_entry!(
    [attr("enclosure", Field::Image, "url")];
    text("feedburner:origLink", Field::OrigLink),
);

static ITUNES_ENTRY: &[Rule] = rss_entry!(
    [];
    text("itunes:author", Field::ItunesAuthor),
    text("itunes:block", Field::Other),
    text("itunes:duration", Field::ItunesDuration),
    text("itunes:explicit", Field::ItunesExplicit),
    text("itunes:keywords", Field::Other),
    text("itunes:subtitle", Field::ItunesSubtitle),
    attr("itunes:image", Field::ItunesImage, "href"),
    text("itunes:isClosedCaptioned", Field::Other),
    text("itunes:order", Field::Other),
    text("itunes:season", Field::ItunesSeason),
    text("itunes:episode", Field::ItunesEpisode),
    text("itunes:title", Field::ItunesTitle),
    text("itunes:episodeType", Field::ItunesEpisodeType),
    text("itunes:summary", Field::ItunesSummary),
    attr("enclosure", Field::EnclosureLength, "length"),
    attr("enclosure", Field::EnclosureType, "type"),
    attr("enclosure", Field::EnclosureUrl, "url"),
    collection("psc:chapter", Field::Nested, Class::Sink("psc:chapter")),
);

/// Builds an Atom entry's rule list: the shared Atom entry rules (with or
/// without their link rules), then the flavor's own, then `content`.
macro_rules! atom_entry {
    ([$($links:expr),*]; $($extra:expr),* $(,)?) => {
        &[
            text_with("title", Field::RawTitle, &[("type", Some("html"))]),
            text_with("title", Field::RawTitle, &[("type", Some("xhtml"))]),
            text_with("title", Field::RawTitle, &[("type", Some("xml"))]),
            text_with("title", Field::Title, &[("type", Some("text"))]),
            text_with("title", Field::Title, &[("type", None)]),
            attr("title", Field::TitleType, "type"),
            text("name", Field::Author),
            text("content", Field::Content),
            text("summary", Field::Summary),
            attr("enclosure", Field::Image, "href"),
            text("published", Field::Published),
            text("id", Field::EntryId),
            text("created", Field::Published),
            text("issued", Field::Published),
            text("updated", Field::Updated),
            text("modified", Field::Updated),
            attrs("category", Field::Categories, "term", &[]),
            $($links,)*
            $($extra,)*
            text("content", Field::Content),
        ]
    };
}

static ATOM_ENTRY: &[Rule] = atom_entry!(
    [
        attr_with("link", Field::Url, "href", &[("type", Some("text/html")), ("rel", Some("alternate"))]),
        attrs("link", Field::Links, "href", &[])
    ];
    attr("media:thumbnail", Field::Image, "url"),
    attr("media:content", Field::Image, "url"),
);

static ATOM_FEEDBURNER_ENTRY: &[Rule] = atom_entry!(
    [
        attr_with("link", Field::Url, "href", &[("type", Some("text/html")), ("rel", Some("alternate"))]),
        attrs("link", Field::Links, "href", &[])
    ];
    text("feedburner:origLink", Field::OrigLink),
    attr("media:thumbnail", Field::Image, "url"),
    attr("media:content", Field::Image, "url"),
);

static YOUTUBE_ENTRY: &[Rule] = atom_entry!(
    [];
    attr_with("link", Field::Url, "href", ALTERNATE),
    text("media:description", Field::Content),
    text("yt:videoId", Field::YoutubeVideoId),
    text("yt:channelId", Field::YoutubeChannelId),
    text("media:title", Field::Other),
    attr("media:content", Field::Other, "url"),
    attr("media:thumbnail", Field::Other, "url"),
);
