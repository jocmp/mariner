//! `src/extractors/generic/lead-image-url/`.
//!
//! Like content and next page extraction, uses a scoring system to
//! determine what the most likely image may be. Short circuits on really
//! probable things like og:image meta tags.

use std::sync::LazyLock;

use dom_query::{Document, NodeRef};
use regex::Regex;

use crate::cleaners::clean_image;
use crate::utils::dom::constants::PHOTO_HINTS_RE;
use crate::utils::dom::{
    extract_from_meta, find, fragment_nodes, parse_fragment, select, tag_name,
};
use crate::utils::js;

// Meta tag names that denote likely article leading images, from most
// distinct to least distinct.
const LEAD_IMAGE_URL_META_TAGS: &[&str] = &["og:image", "twitter:image", "image_src"];

const LEAD_IMAGE_URL_SELECTORS: &[&str] = &["link[rel=image_src]"];

static POSITIVE_LEAD_IMAGE_URL_HINTS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)upload|wp-content|large|photo|wp-image").unwrap());

static NEGATIVE_LEAD_IMAGE_URL_HINTS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?i)spacer|sprite|blank|throbber|gradient|tile|bg|background|icon|social|header|hdr|advert|spinner|loader|loading|default|rating|share|facebook|twitter|theme|promo|ads|wp-includes",
    )
    .unwrap()
});

static GIF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\.gif(\?.*)?$").unwrap());
static JPG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\.jpe?g(\?.*)?$").unwrap());

/// Find the article's lead image.
pub fn extract(
    doc: &Document,
    content: Option<&str>,
    meta_cache: &[String],
    html: &str,
) -> Option<String> {
    // Upstream re-adds the original markup when cleaning has removed the
    // `<head>` (content cleaning rewrites `html`/`body` to divs).
    if select(doc, "head").is_empty()
        && let Some(first) = select(doc, "*").nodes().first()
    {
        let fragment = parse_fragment(html);
        dom_query::Selection::from(*first).prepend_selection(&fragment_nodes(&fragment));
    }

    // Check to see if we have a matching meta tag that we can make use of.
    // Common practice is now to use large images on things like Open Graph
    // or Twitter cards.
    if let Some(image_url) = extract_from_meta(doc, LEAD_IMAGE_URL_META_TAGS, meta_cache, false)
        && let Some(clean_url) = clean_image(&image_url)
    {
        return Some(clean_url);
    }

    // Next, try to find the "best" image via the content. We'd rather not
    // have to fetch each image and check dimensions, so try to do some
    // analysis and determine them instead.
    let fragment = parse_fragment(content.unwrap_or_default());
    let imgs = find(&fragment_nodes(&fragment), "img");
    let image_count = imgs.length();
    let mut img_scores: Vec<(String, f64)> = Vec::new();

    for (index, img) in imgs.nodes().iter().enumerate() {
        let Some(src) = img.attr("src").filter(|src| !src.is_empty()) else {
            continue;
        };

        let score = score_image_url(&src)
            + score_attr(img)
            + score_by_parents(img)
            + score_by_sibling(img)
            + score_by_dimensions(img, &src)
            + score_by_position(image_count, index);

        match img_scores
            .iter_mut()
            .find(|(existing, _)| existing.as_str() == &*src)
        {
            Some(entry) => entry.1 = score,
            None => img_scores.push((src.to_string(), score)),
        }
    }

    let (top_url, top_score) = img_scores.iter().fold((None, 0.0), |acc, (url, score)| {
        if *score > acc.1 {
            (Some(url.as_str()), *score)
        } else {
            acc
        }
    });

    if top_score > 0.0
        && let Some(clean_url) = top_url.and_then(clean_image)
    {
        return Some(clean_url);
    }

    // If nothing else worked, check to see if there are any really probable
    // nodes in the doc, like <link rel="image_src" />.
    for selector in LEAD_IMAGE_URL_SELECTORS {
        let node = select(doc, selector).first();
        for attr in ["src", "href", "value"] {
            if let Some(value) = node.attr(attr).filter(|value| !value.is_empty())
                && let Some(clean_url) = clean_image(&value)
            {
                return Some(clean_url);
            }
        }
    }

    None
}

/// Scores image URLs based on a variety of heuristics.
pub fn score_image_url(url: &str) -> f64 {
    let url = js::trim(url);
    let mut score = 0.0;

    if POSITIVE_LEAD_IMAGE_URL_HINTS_RE.is_match(url) {
        score += 20.0;
    }
    if NEGATIVE_LEAD_IMAGE_URL_HINTS_RE.is_match(url) {
        score -= 20.0;
    }

    // TODO upstream: consider removing this, as gifs are much more
    // common/popular than they once were.
    if GIF_RE.is_match(url) {
        score -= 10.0;
    }
    if JPG_RE.is_match(url) {
        score += 10.0;
    }

    // PNGs are neutral.
    score
}

/// An alt attribute usually means a non-presentational image.
pub fn score_attr(img: &NodeRef<'_>) -> f64 {
    if img.attr("alt").is_some_and(|alt| !alt.is_empty()) {
        5.0
    } else {
        0.0
    }
}

/// Look through the parent and grandparent for figure-like container
/// elements, giving a bonus if we find them.
pub fn score_by_parents(img: &NodeRef<'_>) -> f64 {
    let mut score = 0.0;

    if img
        .ancestors_it(None)
        .any(|ancestor| tag_name(&ancestor) == "figure")
    {
        score += 25.0;
    }

    let parent = element_parent(img);
    let grandparent = parent.as_ref().and_then(element_parent);
    for node in [parent, grandparent] {
        if PHOTO_HINTS_RE.is_match(&sig(node.as_ref())) {
            score += 15.0;
        }
    }

    score
}

/// Look at the immediate sibling and see if it looks like a caption.
/// Bonus if so.
pub fn score_by_sibling(img: &NodeRef<'_>) -> f64 {
    let mut score = 0.0;
    let sibling = img.next_element_sibling();

    if sibling
        .as_ref()
        .is_some_and(|sibling| tag_name(sibling) == "figcaption")
    {
        score += 25.0;
    }
    if PHOTO_HINTS_RE.is_match(&sig(sibling.as_ref())) {
        score += 15.0;
    }

    score
}

/// Penalize small or skinny images, and reward large ones.
pub fn score_by_dimensions(img: &NodeRef<'_>, src: &str) -> f64 {
    let mut score = 0.0;
    let width = img
        .attr("width")
        .and_then(|width| js::parse_float(&width))
        .filter(|&w| w != 0.0);
    let height = img
        .attr("height")
        .and_then(|height| js::parse_float(&height))
        .filter(|&h| h != 0.0);

    // Penalty for skinny images.
    if width.is_some_and(|width| width <= 50.0) {
        score -= 50.0;
    }

    // Penalty for short images.
    if height.is_some_and(|height| height <= 50.0) {
        score -= 50.0;
    }

    if let (Some(width), Some(height)) = (width, height)
        && !src.contains("sprite")
    {
        let area = width * height;
        if area < 5000.0 {
            // Smaller than 50 x 100.
            score -= 100.0;
        } else {
            score += js_round(area / 1000.0);
        }
    }

    score
}

/// Earlier images score higher.
pub fn score_by_position(image_count: usize, index: usize) -> f64 {
    image_count as f64 / 2.0 - index as f64
}

fn sig(node: Option<&NodeRef<'_>>) -> String {
    let attr = |name| {
        node.and_then(|node| node.attr(name))
            .map(|value| value.to_string())
            .unwrap_or_default()
    };
    format!("{} {}", attr("class"), attr("id"))
}

// cheerio's `.parent()`: the parent element, never the document itself.
fn element_parent<'a>(node: &NodeRef<'a>) -> Option<NodeRef<'a>> {
    node.parent().filter(NodeRef::is_element)
}

// `Math.round`, which rounds halves toward positive infinity.
fn js_round(n: f64) -> f64 {
    (n + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_image_urls() {
        assert_eq!(
            score_image_url("http://example.com/wp-content/image.jpg"),
            30.0
        );
        assert_eq!(score_image_url("http://example.com/sprite.gif"), -30.0);
        assert_eq!(score_image_url("http://example.com/image.png"), 0.0);
    }

    #[test]
    fn prefers_meta_images() {
        let doc = crate::utils::dom::parse_document(
            r#"<html><head><meta name="og:image" value="https://example.com/lead.jpg"></head><body></body></html>"#,
        );
        let cache = vec!["og:image".to_string()];
        assert_eq!(
            extract(&doc, Some("<div></div>"), &cache, ""),
            Some("https://example.com/lead.jpg".to_string())
        );
    }

    #[test]
    fn scores_content_images() {
        let doc = crate::utils::dom::parse_document("<html><head></head><body></body></html>");
        let content = r#"<div><figure><img src="https://example.com/upload/a.jpg" alt="x"><figcaption>c</figcaption></figure><img src="https://example.com/icon.png" width="10" height="10"></div>"#;
        assert_eq!(
            extract(&doc, Some(content), &[], ""),
            Some("https://example.com/upload/a.jpg".to_string())
        );
    }
}
