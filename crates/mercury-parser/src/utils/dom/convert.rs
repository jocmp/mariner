//! Node conversion: `convert-node-to.js`, `convert-to-paragraphs.js`,
//! `brs-to-ps.js`, and `paragraphize.js`.

use dom_query::{Document, NodeRef};

use super::constants::{BLOCK_LEVEL_TAGS, DIV_TO_P_BLOCK_TAGS};
use super::stability::children_are_stable;
use super::{
    fragment_nodes, get_attrs, inner_html, is, parse_fragment, replace_with_html, select, tag_name,
};

// Target tags whose parsing rules `children_are_stable` accounts for.
const SAFE_TARGET_TAGS: &[&str] = &[
    "p",
    "div",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "span",
    "figure",
    "figcaption",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "blockquote",
];

const RAW_TEXT_TAGS: &[&str] = &[
    "iframe",
    "noembed",
    "noframes",
    "noscript",
    "plaintext",
    "script",
    "style",
    "textarea",
    "title",
    "xmp",
];

/// Replace `node` with a `tag` element holding the same attributes and
/// contents.
///
/// Like upstream, this behaves as if round-tripping through an HTML string:
/// attributes are written unquoted (`class=a b` becomes `class="a" b=""`),
/// and the contents are re-parsed, so invalid nesting gets fixed up the way
/// a browser would.
pub fn convert_node_to(node: &NodeRef<'_>, tag: &str) {
    if !node.is_element() || node.parent().is_none() {
        return;
    }

    let attributes = get_attrs(node)
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ");

    if !convert_in_place(node, tag, &attributes) {
        let html = inner_html(node);
        replace_with_html(node, &format!("<{tag} {attributes}>{html}</{tag}>"));
    }
}

// Re-parsing is the expensive part of a conversion, and for most nodes it
// changes nothing. When the contents would re-parse to themselves, copy
// them into the new element instead. Like the re-parse, this leaves the
// original subtree intact but detached, which matters to callers still
// holding its nodes. Returns whether the conversion happened.
fn convert_in_place(node: &NodeRef<'_>, tag: &str, attributes: &str) -> bool {
    let node_tag = tag_name(node);
    if !SAFE_TARGET_TAGS.contains(&tag) || RAW_TEXT_TAGS.contains(&node_tag.as_str()) {
        return false;
    }

    // Re-parsing an `<html>` element's contents drops the `<head>` and
    // `<body>` tags but keeps what's inside them.
    let children: Vec<_> = if node_tag == "html" {
        node.children()
            .into_iter()
            .flat_map(|child| match tag_name(&child).as_str() {
                "head" | "body" => child.children(),
                _ => vec![child],
            })
            .collect()
    } else {
        node.children()
    };
    if !children_are_stable(tag, children.iter().copied()) {
        return false;
    }

    // Parse just the start tag, to apply upstream's unquoted-attribute
    // quirks. Bail if an attribute value ended the tag early.
    let fragment = parse_fragment(&format!("<{tag} {attributes}></{tag}>"));
    let parsed = fragment_nodes(&fragment);
    let [element] = parsed.nodes() else {
        return false;
    };
    if tag_name(element) != tag || element.first_child().is_some() {
        return false;
    }

    let replacement = node.tree.new_element(tag);
    for (name, value) in get_attrs(element) {
        replacement.set_attr(&name, &value);
    }
    for child in &children {
        replacement.append_child(&deep_copy(child));
    }
    node.replace_with(&replacement);
    true
}

// A detached copy of `node` and its descendants, in the same tree.
fn deep_copy<'a>(node: &NodeRef<'a>) -> NodeRef<'a> {
    let data = node
        .query(|tree_node| tree_node.data.clone())
        .expect("node exists");
    let copy = node.tree.get_unchecked(&node.tree.create_node(data));
    for child in node.children() {
        copy.append_child(&deep_copy(&child));
    }
    copy
}

/// Convert p-like elements to actual paragraph tags:
///
/// * Multiple consecutive `<br />` tags.
/// * `<div />` tags without block level elements inside of them.
/// * `<span />` tags who are not children of `<p />` or `<div />` tags.
pub fn convert_to_paragraphs(doc: &Document) {
    brs_to_ps(doc);
    convert_divs(doc);
    convert_spans(doc);
}

/// Convert runs of consecutive `<br />` tags into `<p />` tags.
pub fn brs_to_ps(doc: &Document) {
    let mut collapsing = false;

    for br in select(doc, "br").nodes() {
        let next_is_br = br
            .next_element_sibling()
            .is_some_and(|next| tag_name(&next) == "br");

        if next_is_br {
            collapsing = true;
            br.remove_from_parent();
        } else if collapsing {
            collapsing = false;
            paragraphize(br, true);
        }
    }
}

/// Given a `<br />`, replace it with a `<p />` holding the inline siblings
/// that follow it.
pub fn paragraphize(node: &NodeRef<'_>, br: bool) {
    if !br {
        return;
    }

    let p = node.tree.new_element("p");
    let mut sibling = node.next_sibling();

    // While the next node is text or not a block level element, append it
    // to the new p node.
    while let Some(current) = sibling {
        if current.is_element() && BLOCK_LEVEL_TAGS.contains(&tag_name(&current).as_str()) {
            break;
        }
        sibling = current.next_sibling();
        p.append_child(&current);
    }

    if node.parent().is_some() {
        node.replace_with(&p);
    }
}

fn convert_divs(doc: &Document) {
    for div in select(doc, "div").nodes() {
        let convertible = !div
            .element_children()
            .iter()
            .any(|child| is(child, DIV_TO_P_BLOCK_TAGS));

        if convertible {
            convert_node_to(div, "p");
        }
    }
}

fn convert_spans(doc: &Document) {
    for span in select(doc, "span").nodes() {
        let convertible = !span
            .ancestors_it(None)
            .any(|ancestor| ancestor.is_element() && is(&ancestor, "p, div, li, figcaption"));

        if convertible {
            convert_node_to(span, "p");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{outer_html, parse_document};
    use super::*;

    fn body(doc: &Document) -> String {
        doc.body().map(|body| inner_html(&body)).unwrap_or_default()
    }

    #[test]
    fn converts_nodes_with_unquoted_attributes() {
        let doc = parse_document(r#"<body><div class="a b" id="x">hi</div></body>"#);
        convert_node_to(&select(&doc, "div").nodes()[0], "p");
        assert_eq!(body(&doc), r#"<p class="a" b="" id="x">hi</p>"#);
    }

    #[test]
    fn reparses_contents_when_converting() {
        let doc = parse_document("<body><div>hi <ul><li>x</li></ul></div></body>");
        convert_node_to(&select(&doc, "div").nodes()[0], "p");
        assert_eq!(body(&doc), "<p>hi </p><ul><li>x</li></ul><p></p>");
    }

    #[test]
    fn converts_brs_to_ps() {
        let doc = parse_document("<body><div>a<br><br>b c<br>d<p>e</p></div></body>");
        brs_to_ps(&doc);
        assert_eq!(
            outer_html(&select(&doc, "div")),
            "<div>ab c<p>d</p><p>e</p></div>"
        );
    }

    #[test]
    fn converts_shallow_divs_and_orphan_spans() {
        let doc = parse_document(
            "<body><div>text</div><div><p>para</p></div><span>orphan</span><p><span>kept</span></p></body>",
        );
        convert_to_paragraphs(&doc);
        assert_eq!(
            body(&doc),
            "<p>text</p><div><p>para</p></div><p>orphan</p><p><span>kept</span></p>"
        );
    }
}
