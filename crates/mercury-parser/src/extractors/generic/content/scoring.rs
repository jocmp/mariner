//! Content scoring, from `src/extractors/generic/content/scoring/`.
//!
//! Scores live in a `score` attribute on each node, as upstream: later
//! passes (and the cleaners) read them back from the DOM.

use std::sync::LazyLock;

use dom_query::{Document, NodeRef, Selection};
use regex::Regex;

use crate::utils::dom::constants::{
    NEGATIVE_SCORE_RE, PHOTO_HINTS_RE, POSITIVE_SCORE_RE, READABILITY_ASSET,
};
use crate::utils::dom::{convert_node_to, is, link_density, select, tag_name, text_length};
use crate::utils::js;
use crate::utils::text::has_sentence_end;

// Selectors that specify, very clearly, either hNews or other very
// content-specific style content, like Blogger templates.
const HNEWS_CONTENT_SELECTORS: &[(&str, &str)] = &[
    (".hentry", ".entry-content"),
    ("entry", ".entry-content"),
    (".entry", ".entry_content"),
    (".post", ".postbody"),
    (".post", ".post_body"),
    (".post", ".post-body"),
];

// Tags that should be ignored when trying to find the top candidate for a
// document.
const NON_TOP_CANDIDATE_TAGS: &[&str] = &[
    "br", "b", "i", "label", "hr", "area", "base", "basefont", "input", "img", "link", "meta",
];

static PARAGRAPH_SCORE_TAGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)^(p|li|span|pre)$").unwrap());
static CHILD_CONTENT_TAGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)^(td|blockquote|ol|ul|dl)$").unwrap());
static BAD_TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?i)^(address|form)$").unwrap());

/// Score content. Parents get the full value of their children's content
/// score, grandparents half.
pub fn score_content(doc: &Document, weight_nodes: bool) {
    // First, look for special hNews based selectors and give them a big
    // boost, if they exist.
    for (parent_selector, child_selector) in HNEWS_CONTENT_SELECTORS {
        for node in select(doc, &format!("{parent_selector} {child_selector}")).nodes() {
            // cheerio's `.parent(selector)` only considers the direct parent.
            if let Some(parent) = element_parent(node).filter(|parent| is(parent, parent_selector))
            {
                add_score(&parent, 80.0);
            }
        }
    }

    // Doubling this again. Previous solution caused a bug in which parents
    // weren't retaining scores. This is not ideal, and should be fixed.
    score_ps(doc, weight_nodes);
    score_ps(doc, weight_nodes);
}

/// After scores are calculated, find the highest-scoring candidate and
/// merge in any well-scored siblings.
pub fn find_top_candidate<'a>(doc: &'a Document) -> Selection<'a> {
    let mut candidate: Option<NodeRef<'a>> = None;
    let mut top_score = 0.0;

    for node in select(doc, "[score]").nodes() {
        // Ignore tags like BR, HR, etc.
        if is_non_top_candidate(node) {
            continue;
        }

        let score = get_score(node).unwrap_or(0.0);
        if score > top_score {
            top_score = score;
            candidate = Some(*node);
        }
    }

    // If we don't have a candidate, return the body.
    let Some(candidate) = candidate else {
        return select(doc, "body");
    };

    merge_siblings(&candidate, top_score)
}

/// The node's score, or `None` when unset or zero.
pub fn get_score(node: &NodeRef<'_>) -> Option<f64> {
    node.attr("score")
        .and_then(|score| js::parse_float(&score))
        .filter(|&score| score != 0.0 && !score.is_nan())
}

pub fn set_score(node: &NodeRef<'_>, score: f64) {
    node.set_attr("score", &js::number_to_string(score));
}

/// Add `amount` to the node's score, initializing it first if needed.
pub fn add_score(node: &NodeRef<'_>, amount: f64) {
    if !node.is_element() {
        return;
    }
    let score = get_or_init_score(node, true) + amount;
    set_score(node, score);
}

/// Add a quarter of a child's score to its parent.
pub fn add_to_parent(node: &NodeRef<'_>, score: f64) {
    if let Some(parent) = element_parent(node) {
        add_score(&parent, score * 0.25);
    }
}

/// The node's existing score, or a fresh one based on its tag (and
/// class/id, when `weight_nodes` is set). Initializing also adds to the
/// parent's score.
pub fn get_or_init_score(node: &NodeRef<'_>, weight_nodes: bool) -> f64 {
    if let Some(score) = get_score(node) {
        return score;
    }

    let mut score = score_node(node);
    if weight_nodes {
        score += get_weight(node);
    }
    add_to_parent(node, score);

    score
}

/// Score an individual node. Has some smarts for paragraphs, otherwise just
/// scores based on tag.
pub fn score_node(node: &NodeRef<'_>) -> f64 {
    let tag = tag_name(node);

    if PARAGRAPH_SCORE_TAGS.is_match(&tag) {
        score_paragraph(node)
    } else if tag == "div" {
        5.0
    } else if CHILD_CONTENT_TAGS.is_match(&tag) {
        3.0
    } else if BAD_TAGS.is_match(&tag) {
        -3.0
    } else if tag == "th" {
        -5.0
    } else {
        0.0
    }
}

/// Score a paragraph using things like the number of commas. Higher is
/// better.
pub fn score_paragraph(node: &NodeRef<'_>) -> f64 {
    let text = node.text();
    let text = js::trim(&text);
    let text_length = js::len(text);

    // If this paragraph is less than 25 characters, don't count it.
    if text_length < 25 {
        return 0.0;
    }

    // Add points for any commas within this paragraph, and for every 50
    // characters, up to 3 points.
    let mut score = 1.0 + score_commas(text) as f64 + score_length(text_length);

    // Articles can end with short paragraphs when people are being clever
    // but they can also end with short paragraphs setting up lists of junk
    // that we strip. This negative tweaks junk setup paragraphs just below
    // the cutoff threshold.
    if text.ends_with(':') {
        score -= 1.0;
    }

    score
}

/// One point for every comma in `text`.
pub fn score_commas(text: &str) -> usize {
    text.matches(',').count()
}

/// A bonus of up to 3 points for every 50 characters past the first 100.
pub fn score_length(text_length: usize) -> f64 {
    let chunks = text_length as f64 / 50.0;
    if chunks > 0.0 {
        (chunks - 2.0).clamp(0.0, 3.0)
    } else {
        0.0
    }
}

/// Score a node based on its class and id.
pub fn get_weight(node: &NodeRef<'_>) -> f64 {
    let classes = node.attr("class").filter(|class| !class.is_empty());
    let id = node.attr("id").filter(|id| !id.is_empty());
    let mut score = 0.0;

    if let Some(id) = &id {
        // If an id exists, try to score on both positive and negative.
        if POSITIVE_SCORE_RE.is_match(id) {
            score += 25.0;
        }
        if NEGATIVE_SCORE_RE.is_match(id) {
            score -= 25.0;
        }
    }

    if let Some(classes) = &classes {
        if score == 0.0 {
            // If classes exist and the id did not contribute to the score,
            // try to score on both positive and negative.
            if POSITIVE_SCORE_RE.is_match(classes) {
                score += 25.0;
            }
            if NEGATIVE_SCORE_RE.is_match(classes) {
                score -= 25.0;
            }
        }

        // Even if the score was set by id, add score for possible photo
        // matches: "try to keep photos if we can".
        if PHOTO_HINTS_RE.is_match(classes) {
            score += 10.0;
        }

        // Add 25 if the class matches entry-content-asset, a class
        // apparently instructed for use in the Readability publisher
        // guidelines.
        if READABILITY_ASSET.is_match(classes) {
            score += 25.0;
        }
    }

    score
}

fn score_ps(doc: &Document, weight_nodes: bool) {
    for node in select(doc, "p, pre").nodes() {
        if node.has_attr("score") {
            continue;
        }

        // The raw score for this paragraph, before we add any parent/child
        // scores.
        let score = get_or_init_score(node, weight_nodes);
        set_score(node, score);

        let Some(parent) = element_parent(node) else {
            continue;
        };
        let raw_score = score_node(node);

        // Spans become divs before scoring, which detaches the original
        // parent; upstream keeps scoring the detached node, so its
        // grandparent bonus is lost too.
        if tag_name(&parent) == "span" {
            convert_node_to(&parent, "div");
        }
        add_score(&parent, raw_score);

        // Add half of the individual content score to the grandparent.
        if let Some(grandparent) = element_parent(&parent) {
            if tag_name(&grandparent) == "span" {
                convert_node_to(&grandparent, "div");
            }
            add_score(&grandparent, raw_score / 2.0);
        }
    }
}

// Look through the siblings of the top candidate to see if any of them are
// decently scored. If they are, they may be split parts of the content
// (like two divs, a preamble and a body).
fn merge_siblings<'a>(candidate: &NodeRef<'a>, top_score: f64) -> Selection<'a> {
    let Some(parent) = element_parent(candidate) else {
        return Selection::from(*candidate);
    };

    let sibling_score_threshold = f64::max(10.0, top_score * 0.25);
    let wrapping_div = candidate.tree.new_element("div");
    let candidate_class = candidate.attr("class");

    for sibling in parent.element_children() {
        // Ignore tags like BR, HR, etc.
        if is_non_top_candidate(&sibling) {
            continue;
        }

        let Some(sibling_score) = get_score(&sibling) else {
            continue;
        };

        if sibling.id == candidate.id {
            wrapping_div.append_child(&sibling);
            continue;
        }

        let mut content_bonus = 0.0;
        let density = link_density(&Selection::from(sibling));

        // If the sibling has a very low link density, give it a small bonus.
        if density < 0.05 {
            content_bonus += 20.0;
        }

        // If the sibling has a high link density, give it a penalty.
        if density >= 0.5 {
            content_bonus -= 20.0;
        }

        // If the sibling has the same class as the candidate, give it a
        // bonus.
        if sibling.attr("class") == candidate_class {
            content_bonus += top_score * 0.2;
        }

        let new_score = sibling_score + content_bonus;

        if new_score >= sibling_score_threshold {
            wrapping_div.append_child(&sibling);
            continue;
        }

        if tag_name(&sibling) == "p" {
            let sibling_content = sibling.text();
            let sibling_content_length = text_length(&sibling_content);

            if (sibling_content_length > 80 && density < 0.25)
                || (sibling_content_length <= 80
                    && density == 0.0
                    && has_sentence_end(&sibling_content))
            {
                wrapping_div.append_child(&sibling);
            }
        }
    }

    let children = wrapping_div.element_children();
    if children.len() == 1 && children[0].id == candidate.id {
        return Selection::from(*candidate);
    }

    Selection::from(wrapping_div)
}

fn is_non_top_candidate(node: &NodeRef<'_>) -> bool {
    NON_TOP_CANDIDATE_TAGS.contains(&tag_name(node).as_str())
}

// cheerio's `.parent()`: the parent element, never the document itself.
fn element_parent<'a>(node: &NodeRef<'a>) -> Option<NodeRef<'a>> {
    node.parent().filter(NodeRef::is_element)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::dom::parse_document;

    #[test]
    fn scores_paragraphs() {
        let doc = parse_document(
            "<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>",
        );
        let p = select(&doc, "p").nodes()[0];
        // 1 base + 2 commas + (123/50 - 2) length bonus.
        assert!((score_paragraph(&p) - (3.0 + (123.0 / 50.0 - 2.0))).abs() < 1e-9);
    }

    #[test]
    fn weights_by_class_and_id() {
        let doc = parse_document(r#"<div id="article" class="sidebar photo">x</div>"#);
        let div = select(&doc, "div").nodes()[0];
        assert_eq!(get_weight(&div), 35.0);
    }

    #[test]
    fn scores_lengths() {
        assert_eq!(score_length(50), 0.0);
        assert_eq!(score_length(150), 1.0);
        assert_eq!(score_length(1000), 3.0);
    }
}
