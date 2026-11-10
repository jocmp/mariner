//! Content cleaning passes: `clean-attributes.js`, `clean-h-ones.js`,
//! `clean-headers.js`, `clean-images.js`, `clean-tags.js`, `mark-to-keep.js`,
//! `remove-empty.js`, `rewrite-top-level.js`, `strip-junk-tags.js`, and
//! `strip-unlikely-candidates.js`.

use dom_query::{Document, NodeRef, Selection};

use super::constants::{
    CANDIDATES_BLACKLIST, CANDIDATES_WHITELIST, CLEAN_CONDITIONALLY_TAGS, HEADER_TAG_LIST,
    KEEP_CLASS, KEEP_SELECTORS, SPACER_RE, STRIP_OUTPUT_TAGS, WHITELIST_ATTRS,
};
use super::{
    add_class, convert_node_to, find, has_class, is, link_density, not, remove_class, select,
    tag_name,
};
use crate::extractors::generic::content::scoring::{
    get_or_init_score, get_score, get_weight, score_commas, set_score,
};
use crate::utils::js;
use crate::utils::text::normalize_spaces;
use crate::utils::url::parse_url;

/// Remove every attribute not in the whitelist from the article's
/// descendants, and drop the keep marker class.
pub fn clean_attributes(article: &Selection<'_>) {
    // Grab the parent because at this point the article will be wrapped in
    // a div which will have a score set on it.
    let parent = article.parent();
    let target = if parent.is_empty() {
        article.clone()
    } else {
        parent
    };

    for node in find(&target, "*").nodes() {
        node.retain_attrs(WHITELIST_ATTRS);
    }

    for node in find(&target, &format!(".{KEEP_CLASS}")).nodes() {
        remove_class(node, KEEP_CLASS);
    }
}

/// H1 tags are typically the article title, which should be extracted by
/// the title extractor instead. If there are fewer than 3 of them, strip
/// them. Otherwise, turn them into H2s.
pub fn clean_h_ones(article: &Selection<'_>) {
    let h_ones = find(article, "h1");

    if h_ones.length() < 3 {
        for node in h_ones.nodes() {
            node.remove_from_parent();
        }
    } else {
        for node in h_ones.nodes() {
            convert_node_to(node, "h2");
        }
    }
}

/// Remove headers that precede every paragraph, repeat the title, or look
/// like junk by class/id.
pub fn clean_headers(article: &Selection<'_>, title: Option<&str>) {
    for header in find(article, HEADER_TAG_LIST).nodes() {
        if has_class(header, KEEP_CLASS) {
            continue;
        }

        // Remove any headers that appear before all other p tags in the
        // document. This probably means that it was part of the title, a
        // subtitle or something else extraneous like a datestamp or byline,
        // all of which should be handled by other metadata handling.
        let has_previous_p = std::iter::successors(header.prev_element_sibling(), |node| {
            node.prev_element_sibling()
        })
        .any(|node| tag_name(&node) == "p");
        if !has_previous_p {
            header.remove_from_parent();
            continue;
        }

        // Remove any headers that match the title exactly.
        if title.is_some_and(|title| normalize_spaces(&header.text()) == title) {
            header.remove_from_parent();
            continue;
        }

        // If this header has a negative weight, it's probably junk.
        if get_weight(header) < 0.0 {
            header.remove_from_parent();
        }
    }
}

/// Drop small images and spacer images, and strip `height` attributes.
pub fn clean_images(article: &Selection<'_>) {
    for img in find(article, "img").nodes() {
        clean_for_height(img);
        remove_spacers(img);
    }
}

/// Clean an article of superfluous content: forms, ads, link-heavy
/// containers, and the like.
pub fn clean_tags(article: &Selection<'_>) {
    for node in find(article, CLEAN_CONDITIONALLY_TAGS).nodes() {
        // If marked to keep, skip it.
        let keep_selector = format!(".{KEEP_CLASS}");
        if has_class(node, KEEP_CLASS) || !find(&Selection::from(*node), &keep_selector).is_empty()
        {
            continue;
        }

        let mut weight = get_score(node).unwrap_or(0.0);
        if weight == 0.0 {
            weight = get_or_init_score(node, true);
            set_score(node, weight);
        }

        // Drop the node if its weight is < 0.
        if weight < 0.0 {
            node.remove_from_parent();
        } else {
            // Determine if the node seems like content.
            remove_unless_content(node, weight);
        }
    }
}

/// Mark elements to keep that would normally be removed, such as embedded
/// YouTube and Vimeo videos.
pub fn mark_to_keep(article: &Selection<'_>, url: &str) {
    let mut tags: Vec<String> = KEEP_SELECTORS.iter().map(ToString::to_string).collect();

    if let Some(parsed) = parse_url(url) {
        let hostname = parsed.host_str().unwrap_or_default();
        tags.push(format!(
            r#"iframe[src^="{}://{hostname}"]"#,
            parsed.scheme()
        ));
    }

    for node in find(article, &tags.join(",")).nodes() {
        add_class(node, KEEP_CLASS);
    }
}

/// Remove paragraphs with no text, images, or iframes.
pub fn remove_empty(article: &Selection<'_>) {
    for p in find(article, "p").nodes() {
        let paragraph = Selection::from(*p);
        if find(&paragraph, "iframe, img").is_empty() && js::trim(&p.text()).is_empty() {
            p.remove_from_parent();
        }
    }
}

/// Rewrite the `html` and `body` elements to divs, to avoid later
/// complications with multiple body tags.
pub fn rewrite_top_level(doc: &Document) {
    for tag in ["html", "body"] {
        if let Some(node) = select(doc, tag).nodes().first() {
            convert_node_to(node, "div");
        }
    }
}

/// Remove title, script, iframe, and similar tags, except those marked to
/// keep.
pub fn strip_junk_tags(article: &Selection<'_>) {
    let junk = find(article, &STRIP_OUTPUT_TAGS.join(","));
    for node in not(&junk, &format!(".{KEEP_CLASS}")).nodes() {
        node.remove_from_parent();
    }
}

/// Remove any non-link nodes whose class or id suggests they aren't
/// article content (comments, sidebars, and so on).
pub fn strip_unlikely_candidates(doc: &Document) {
    for node in not(&select(doc, "*"), "a").nodes() {
        let class = node.attr("class").unwrap_or_default();
        let id = node.attr("id").unwrap_or_default();
        if class.is_empty() && id.is_empty() {
            continue;
        }

        let class_and_id = format!("{class} {id}");
        if CANDIDATES_WHITELIST.is_match(&class_and_id) {
            continue;
        }
        if CANDIDATES_BLACKLIST.is_match(&class_and_id) {
            node.remove_from_parent();
        }
    }
}

fn clean_for_height(img: &NodeRef<'_>) {
    let height = img.attr("height").and_then(|h| js::parse_int(&h));
    let width = img
        .attr("width")
        .and_then(|w| js::parse_int(&w))
        .filter(|&w| w != 0)
        .unwrap_or(20);

    // Remove images that explicitly have very small heights or widths,
    // because they are most likely shims or icons, which aren't very
    // useful for reading.
    if height.filter(|&h| h != 0).unwrap_or(20) < 10 || width < 10 {
        img.remove_from_parent();
    } else if height.is_some_and(|h| h != 0) {
        // Don't ever specify a height on images, so that we can scale with
        // respect to width without screwing up the aspect ratio.
        img.remove_attr("height");
    }
}

// Cleans out images where the source string matches transparent, spacer,
// etc.
fn remove_spacers(img: &NodeRef<'_>) {
    let src = img.attr("src");
    if SPACER_RE.is_match(src.as_deref().unwrap_or("undefined")) {
        img.remove_from_parent();
    }
}

fn remove_unless_content(node: &NodeRef<'_>, weight: f64) {
    // Explicitly save entry-content-asset tags, which are noted as valuable
    // in the Publisher guidelines.
    if has_class(node, "entry-content-asset") {
        return;
    }

    let selection = Selection::from(*node);
    let content = normalize_spaces(&node.text());

    if score_commas(&content) >= 10 {
        return;
    }

    let p_count = find(&selection, "p").length();
    let input_count = find(&selection, "input").length();

    // Looks like a form, too many inputs.
    if input_count as f64 > p_count as f64 / 3.0 {
        node.remove_from_parent();
        return;
    }

    let content_length = js::len(&content);
    let img_count = find(&selection, "img").length();

    // Content is too short, and there are no images, so this is probably
    // junk content.
    if content_length < 25 && img_count == 0 {
        node.remove_from_parent();
        return;
    }

    let density = link_density(&selection);

    // Too high of link density, is probably a menu or something similar.
    if weight < 25.0 && density > 0.2 && content_length > 75 {
        node.remove_from_parent();
        return;
    }

    // Too high of a link density, despite the score being high.
    if weight >= 25.0 && density > 0.5 {
        // Don't remove the node if it's a list and the previous sibling
        // ends with a colon though. That means it's probably content.
        if is(node, "ol, ul") {
            let previous_text = node
                .prev_element_sibling()
                .map(|previous| normalize_spaces(&previous.text()))
                .unwrap_or_default();
            if previous_text.ends_with(':') {
                return;
            }
        }

        node.remove_from_parent();
        return;
    }

    let script_count = find(&selection, "script").length();

    // Too many script tags, not enough content.
    if script_count > 0 && content_length < 150 {
        node.remove_from_parent();
    }
}
