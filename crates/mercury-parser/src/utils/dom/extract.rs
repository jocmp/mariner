//! `extract-from-meta.js`, `extract-from-selectors.js`, `strip-tags.js`, and
//! `within-comment.js`.

use dom_query::{Document, NodeRef};

use super::{fragment_nodes, inner_html, parse_fragment, select, text};

/// Find the value of the first meta tag named in `meta_names` that appears
/// exactly once (with a non-empty value) in the document. `cached_names`
/// is every meta `name` in the document.
pub fn extract_from_meta(
    doc: &Document,
    meta_names: &[&str],
    cached_names: &[String],
    clean_tags: bool,
) -> Option<String> {
    let found_names = meta_names
        .iter()
        .filter(|name| cached_names.iter().any(|cached| cached == *name));

    for name in found_names {
        let nodes = select(doc, &format!(r#"meta[name="{name}"]"#));

        // Get the value of every matching node, removing empty values. A
        // missing value attribute is kept, as upstream keeps `undefined`.
        let values: Vec<Option<String>> = nodes
            .nodes()
            .iter()
            .map(|node| node.attr("value").map(|value| value.to_string()))
            .filter(|value| value.as_deref() != Some(""))
            .collect();

        // If we have more than one value for the same name, we have a
        // conflict and can't trust any of them. Skip this name. If we have
        // zero, our meta tags had no values. Skip this name also.
        if let [value] = values.as_slice() {
            return match value {
                // Meta values that contain HTML should be stripped, as they
                // weren't subject to cleaning previously.
                Some(value) if clean_tags => Some(strip_tags(value)),
                Some(value) => Some(value.clone()),
                // Upstream stringifies `undefined` here.
                None if clean_tags => Some("undefined".to_string()),
                None => None,
            };
        }
    }

    None
}

/// Given a list of selectors, find content that may be extractable from the
/// document. This is for flat meta-information, like author, title, date
/// published, etc.
pub fn extract_from_selectors(
    doc: &Document,
    selectors: &[&str],
    max_children: usize,
    text_only: bool,
) -> Option<String> {
    for selector in selectors {
        let nodes = select(doc, selector);

        // If we didn't get exactly one of this selector, this may be a list
        // of articles or comments. Skip it.
        if nodes.length() != 1 {
            continue;
        }

        let node = nodes.nodes()[0];
        if is_good_node(&node, max_children) {
            let content = if text_only {
                node.text().to_string()
            } else {
                inner_html(&node)
            };
            if !content.is_empty() {
                return Some(content);
            }
        }
    }

    None
}

/// Strip all tags from a string of text, returning the text unchanged when
/// nothing is left.
pub fn strip_tags(value: &str) -> String {
    // Wrapping text in an element prevents errors when text has no HTML.
    let fragment = parse_fragment(&format!("<span>{value}</span>"));
    let clean_text = text(&fragment_nodes(&fragment));

    if clean_text.is_empty() {
        value.to_string()
    } else {
        clean_text
    }
}

/// Whether any ancestor of `node` has "comment" in its class or id.
pub fn within_comment(node: &NodeRef<'_>) -> bool {
    node.ancestors_it(None)
        .filter(NodeRef::is_element)
        .any(|parent| {
            let class = parent.attr("class");
            let id = parent.attr("id");
            let class_and_id = format!(
                "{} {}",
                class.as_deref().unwrap_or("undefined"),
                id.as_deref().unwrap_or("undefined")
            );
            class_and_id.contains("comment")
        })
}

fn is_good_node(node: &NodeRef<'_>, max_children: usize) -> bool {
    // If it has a number of children, it's more likely a container element.
    // Skip it.
    if node.element_children().len() > max_children {
        return false;
    }

    // If it looks to be within a comment, skip it.
    !within_comment(node)
}

#[cfg(test)]
mod tests {
    use super::super::parse_document;
    use super::*;

    fn meta_names(doc: &Document) -> Vec<String> {
        select(doc, "meta")
            .nodes()
            .iter()
            .filter_map(|node| node.attr("name").map(|name| name.to_string()))
            .collect()
    }

    #[test]
    fn extracts_unique_meta_values() {
        let doc = parse_document(
            r#"<head><meta name="title" value="Hello <b>there</b>"><meta name="dup" value="a"><meta name="dup" value="b"></head>"#,
        );
        let names = meta_names(&doc);
        assert_eq!(
            extract_from_meta(&doc, &["dup", "title"], &names, true),
            Some("Hello there".to_string())
        );
        assert_eq!(extract_from_meta(&doc, &["dup"], &names, true), None);
    }

    #[test]
    fn extracts_from_single_matching_selectors() {
        let doc = parse_document(
            r#"<body><div class="comments"><p class="author">Nope</p></div><span class="by">Jane</span></body>"#,
        );
        assert_eq!(
            extract_from_selectors(&doc, &[".author", ".by"], 1, true),
            Some("Jane".to_string())
        );
    }

    #[test]
    fn strips_tags() {
        assert_eq!(strip_tags("a <b>b</b> &amp; c"), "a b & c");
        assert_eq!(strip_tags("<img src=x>"), "<img src=x>");
    }
}
