//! Media RSS (`media:*`) and enclosures, read from any RSS or Atom entry.
//!
//! This sits beside the reference rules rather than inside them: it reads
//! the entry's elements by the names written in the document, so Atom's
//! prefix stripping doesn't hide them, and it never changes a field the
//! rules fill.
//!
//! - `title` and `description` are the first `media:title` and
//!   `media:description` outside any `media:content` (the first one wins
//!   even when empty, as with the rules).
//! - Each `media:content` with a `url` becomes a content, with the first
//!   `media:title` and `media:description` inside it.
//! - Every `media:thumbnail` with a `url` is listed, wherever it is.
//! - Star ratings and views come from the first `media:starRating` and
//!   `media:statistics`.

use crate::model::{Enclosure, Media, MediaContent, MediaThumbnail};
use crate::tree::{Element, Node};

/// The entry's Media RSS, or `None` when it has no `media:` elements.
pub(crate) fn media(entry: &Element) -> Option<Media> {
    let mut media = Media::default();
    let mut found = false;
    let (mut title_seen, mut description_seen) = (false, false);
    let (mut rating_seen, mut statistics_seen) = (false, false);
    visit(entry, &mut |element| {
        let Some(local) = element.qualified_name.strip_prefix("media:") else {
            return Visit::Descend;
        };
        found = true;
        match local {
            "content" => {
                if let Some(url) = element.attr("url") {
                    media.contents.push(content(element, url));
                }
                return Visit::Skip;
            }
            "thumbnail" => {
                if let Some(url) = element.attr("url") {
                    media.thumbnails.push(MediaThumbnail {
                        url: url.to_string(),
                        width: attr(element, "width"),
                        height: attr(element, "height"),
                    });
                }
            }
            "title" if !title_seen => {
                title_seen = true;
                media.title = text(element);
            }
            "description" if !description_seen => {
                description_seen = true;
                media.description = text(element);
            }
            "starRating" if !rating_seen => {
                rating_seen = true;
                media.star_average = attr(element, "average");
                media.star_count = attr(element, "count");
            }
            "statistics" if !statistics_seen => {
                statistics_seen = true;
                media.views = attr(element, "views");
            }
            _ => {}
        }
        Visit::Descend
    });
    found.then_some(media)
}

/// Every RSS `<enclosure>` and Atom `<link rel="enclosure">` with a URL.
pub(crate) fn enclosures(entry: &Element) -> Vec<Enclosure> {
    let mut enclosures = Vec::new();
    visit(entry, &mut |element| {
        let url = match element.name.as_str() {
            "enclosure" => element.attr("url"),
            "link" if element.attr("rel") == Some("enclosure") => element.attr("href"),
            _ => None,
        };
        if let Some(url) = url.filter(|url| !url.trim().is_empty()) {
            enclosures.push(Enclosure {
                url: url.to_string(),
                mime_type: attr(element, "type"),
                length: attr(element, "length"),
            });
        }
        Visit::Descend
    });
    enclosures
}

enum Visit {
    Descend,
    Skip,
}

/// Calls `f` on every element below `root`, in document order, skipping
/// the children of elements it returns `Visit::Skip` for.
fn visit(root: &Element, f: &mut impl FnMut(&Element) -> Visit) {
    for node in &root.children {
        if let Node::Element(element) = node
            && let Visit::Descend = f(element)
        {
            visit(element, f);
        }
    }
}

fn content(element: &Element, url: &str) -> MediaContent {
    let mut title = None;
    let mut description = None;
    visit(element, &mut |child| {
        match child.qualified_name.as_str() {
            "media:title" if title.is_none() => title = Some(text(child)),
            "media:description" if description.is_none() => description = Some(text(child)),
            _ => {}
        }
        Visit::Descend
    });
    MediaContent {
        url: url.to_string(),
        mime_type: attr(element, "type"),
        medium: attr(element, "medium"),
        width: attr(element, "width"),
        height: attr(element, "height"),
        file_size: attr(element, "fileSize"),
        duration: attr(element, "duration"),
        title: title.flatten(),
        description: description.flatten(),
    }
}

fn attr(element: &Element, name: &str) -> Option<String> {
    element.attr(name).map(String::from)
}

/// An element's text, including text inside its children; `None` when
/// it has none.
fn text(element: &Element) -> Option<String> {
    let mut out = String::new();
    collect_text(element, &mut out);
    (!element.children.is_empty() && !out.is_empty()).then_some(out)
}

fn collect_text(element: &Element, out: &mut String) {
    for node in &element.children {
        match node {
            Node::Text(text) | Node::CData(text) => out.push_str(text),
            Node::Element(child) => collect_text(child, out),
            Node::Comment(_) => {}
        }
    }
}
