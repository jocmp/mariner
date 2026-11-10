//! The generic content extractor, from `src/extractors/generic/content/`.

pub mod scoring;

use dom_query::{Document, Selection};

use crate::cleaners::clean_content;
use crate::utils::dom::{
    convert_to_paragraphs, outer_html, parse_document, strip_unlikely_candidates,
};
use crate::utils::js;
use crate::utils::text::normalize_spaces;

/// Options controlling how aggressive content extraction is.
#[derive(Clone, Copy, Debug)]
pub struct ContentOptions {
    /// Remove elements whose class or id suggests non-article content
    /// (like "comment") before scoring.
    pub strip_unlikely_candidates: bool,
    /// Adjust scores based on class names and ids.
    pub weight_nodes: bool,
    /// Remove superfluous content like forms and ads from the result.
    pub clean_conditionally: bool,
}

impl Default for ContentOptions {
    fn default() -> Self {
        Self {
            strip_unlikely_candidates: true,
            weight_nodes: true,
            clean_conditionally: true,
        }
    }
}

/// Extract the article content from `doc`.
///
/// The first pass uses the most restrictive options, which return the
/// highest quality content. On each failure, extraction is retried on a
/// fresh parse of `html` with one more option relaxed.
pub fn extract(doc: &Document, html: &str, title: Option<&str>, url: &str) -> Option<String> {
    let mut opts = ContentOptions::default();
    let (mut content, sufficient) = attempt(doc, title, url, opts);
    if sufficient {
        return content;
    }

    // We didn't succeed on the first pass; one by one disable our
    // extraction opts and try again. The last attempt is returned even if
    // it's insufficient.
    let relaxations: [fn(&mut ContentOptions); 3] = [
        |opts| opts.strip_unlikely_candidates = false,
        |opts| opts.weight_nodes = false,
        |opts| opts.clean_conditionally = false,
    ];
    for relax in relaxations {
        relax(&mut opts);
        let doc = parse_document(html);
        let (next, sufficient) = attempt(&doc, title, url, opts);
        content = next;
        if sufficient {
            break;
        }
    }

    content
}

/// Using a variety of scoring techniques, find the node most likely to be
/// the article text.
pub fn extract_best_node<'a>(doc: &'a Document, opts: ContentOptions) -> Selection<'a> {
    if opts.strip_unlikely_candidates {
        strip_unlikely_candidates(doc);
    }

    convert_to_paragraphs(doc);
    scoring::score_content(doc, opts.weight_nodes);
    scoring::find_top_candidate(doc)
}

// The cleaned node's HTML, and whether it has enough text to be
// article-like.
fn attempt(
    doc: &Document,
    title: Option<&str>,
    url: &str,
    opts: ContentOptions,
) -> (Option<String>, bool) {
    match content_node(doc, title, url, opts) {
        Some(node) => (
            Some(normalize_spaces(&outer_html(&node))),
            node_is_sufficient(&node),
        ),
        None => (None, false),
    }
}

fn content_node<'a>(
    doc: &'a Document,
    title: Option<&str>,
    url: &str,
    opts: ContentOptions,
) -> Option<Selection<'a>> {
    let node = extract_best_node(doc, opts);
    if node.is_empty() {
        return None;
    }
    clean_content(&node, doc, opts.clean_conditionally, title, url, true);
    Some(node)
}

// Whether a node has enough text to be article-like.
fn node_is_sufficient(node: &Selection<'_>) -> bool {
    js::len(js::trim(&node.text())) >= 100
}
