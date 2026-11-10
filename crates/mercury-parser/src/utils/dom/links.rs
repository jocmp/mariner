//! `make-links-absolute.js` and `link-density.js`.

use std::sync::LazyLock;

use dom_query::{Document, Selection};
use regex::Regex;

use super::{find, select};
use crate::js_ws;
use crate::utils::js;
use url::Url;

use crate::utils::url::{resolve_url, resolve_url_with};

static SRCSET_CANDIDATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        "(?:[",
        js_ws!(),
        "]*)([^",
        js_ws!(),
        "]+(?:[",
        js_ws!(),
        r"]*[\d.]+[wx])?)(?:[",
        js_ws!(),
        "]*,[",
        js_ws!(),
        "]*)?"
    ))
    .unwrap()
});
static WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[", js_ws!(), "]+")).unwrap());

/// Resolve `href` and `src` attributes throughout the document, and
/// `srcset` attributes within `content`, against `url` (or the document's
/// `<base>`).
pub fn make_links_absolute(content: &Selection<'_>, doc: &Document, url: &str) {
    for attr in ["href", "src"] {
        absolutize(doc, url, attr);
    }
    absolutize_set(url, content);
}

/// `text` with whitespace collapsed and trimmed, measured in UTF-16 units.
pub fn text_length(text: &str) -> usize {
    js::len(&WHITESPACE_RE.replace_all(js::trim(text), " "))
}

/// What fraction of the text in `node` is link text.
pub fn link_density(node: &Selection<'_>) -> f64 {
    let total_text_length = text_length(&node.text());
    let link_length = text_length(&find(node, "a").text());

    if total_text_length > 0 {
        link_length as f64 / total_text_length as f64
    } else if link_length > 0 {
        1.0
    } else {
        0.0
    }
}

fn absolutize(doc: &Document, root_url: &str, attr: &str) {
    let base_href = select(doc, "base")
        .attr("href")
        .map(|href| href.to_string());
    let base_url = resolve_url(
        base_href
            .as_deref()
            .filter(|href| !href.is_empty())
            .unwrap_or(root_url),
        root_url,
    );

    // Match attributes by local name, like cheerio's `$('[href]')`, so
    // namespaced ones such as `xlink:href` are resolved too.
    let base = Url::parse(&base_url).ok();
    let elements: Vec<_> = doc
        .root()
        .descendants_it()
        .filter(|node| node.is_element())
        .collect();
    for node in elements {
        let Some(url) = node.attr(attr).filter(|url| !url.is_empty()) else {
            continue;
        };
        node.set_attr(attr, &resolve_url_with(&url, base.as_ref()));
    }
}

fn absolutize_set(root_url: &str, content: &Selection<'_>) {
    let root = Url::parse(root_url).ok();
    for node in find(content, "[srcset]").nodes() {
        let Some(url_set) = node.attr("srcset").filter(|set| !set.is_empty()) else {
            continue;
        };

        // A comma should be considered part of the candidate URL unless
        // preceded by a descriptor. Descriptors can only contain positive
        // numbers followed immediately by either 'w' or 'x'.
        let candidates: Vec<&str> = SRCSET_CANDIDATE_RE
            .find_iter(&url_set)
            .map(|m| m.as_str())
            .collect();
        if candidates.is_empty() {
            continue;
        }

        let mut absolute_candidates: Vec<String> = Vec::new();
        for candidate in candidates {
            let candidate = js::trim(candidate);
            let candidate = candidate.strip_suffix(',').unwrap_or(candidate);
            let mut parts: Vec<String> =
                WHITESPACE_RE.split(candidate).map(str::to_string).collect();
            parts[0] = resolve_url_with(&parts[0], root.as_ref());

            let absolute = parts.join(" ");
            if !absolute_candidates.contains(&absolute) {
                absolute_candidates.push(absolute);
            }
        }

        node.set_attr("srcset", &absolute_candidates.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::super::{outer_html, parse_document};
    use super::*;

    #[test]
    fn makes_links_absolute() {
        let doc = parse_document(
            r#"<body><div><a href="/foo">a</a><img src="bar.jpg" srcset="a.jpg 1x, /b.jpg 2x,c.jpg 3x"></div></body>"#,
        );
        let content = select(&doc, "body");
        make_links_absolute(&content, &doc, "https://example.com/post/");
        assert_eq!(
            outer_html(&select(&doc, "div")),
            r#"<div><a href="https://example.com/foo">a</a><img src="https://example.com/post/bar.jpg" srcset="https://example.com/post/a.jpg 1x, https://example.com/b.jpg 2x, https://example.com/post/c.jpg 3x"></div>"#
        );
    }

    #[test]
    fn measures_link_density() {
        let doc = parse_document("<body><div>four <a>link</a></div></body>");
        assert!((link_density(&select(&doc, "div")) - 4.0 / 9.0).abs() < f64::EPSILON);
    }
}
