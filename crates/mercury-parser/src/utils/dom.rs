//! DOM helpers, from `src/utils/dom/`.
//!
//! The JS version works against cheerio. These helpers pin down the cheerio
//! behaviors the extraction heuristics depend on — invalid selectors match
//! nothing instead of throwing, `$.html()` renders every node in a
//! selection, fragments parse in a `<template>` context — on top of
//! [`dom_query`].

mod clean;
pub mod constants;
mod convert;
mod extract;
mod links;
mod serialize;
mod stability;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use dom_query::{Document, Matcher, NodeRef, Selection};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{ParseOpts, QualName, local_name, ns};
use regex::Regex;

pub use clean::{
    clean_attributes, clean_h_ones, clean_headers, clean_images, clean_tags, mark_to_keep,
    remove_empty, rewrite_top_level, strip_junk_tags, strip_unlikely_candidates,
};
pub use convert::{brs_to_ps, convert_node_to, convert_to_paragraphs, paragraphize};
pub use extract::{extract_from_meta, extract_from_selectors, strip_tags, within_comment};
pub use links::{link_density, make_links_absolute, text_length};

use crate::js_ws;
use crate::utils::js;

static WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[", js_ws!(), "]+")).unwrap());

/// Parse a full HTML document the way cheerio does (scripting enabled, so
/// `<noscript>` contents stay raw text).
pub fn parse_document(html: &str) -> Document {
    html5ever::parse_document(Document::default(), parse_opts()).one(html)
}

/// Parse an HTML fragment into a standalone document, in a `<template>`
/// context like cheerio's `$('<p>…</p>')`. The parsed nodes are the
/// children of the returned document's `html` element.
pub fn parse_fragment(html: &str) -> Document {
    html5ever::parse_fragment(
        Document::fragment_sink(),
        parse_opts(),
        QualName::new(None, ns!(html), local_name!("template")),
        Vec::new(),
        true,
    )
    .one(html)
}

/// The top-level nodes of a document made by [`parse_fragment`].
pub fn fragment_nodes(fragment: &Document) -> Selection<'_> {
    let root = fragment.root();
    let nodes = match root.first_child() {
        Some(html) if html.has_name("html") => html.children(),
        _ => root.children(),
    };
    Selection::from(nodes)
}

/// `$(css)`: every element in the document matching `css`. Invalid
/// selectors match nothing.
pub fn select<'a>(doc: &'a Document, css: &str) -> Selection<'a> {
    matcher(css).map_or_else(Selection::default, |m| doc.select_matcher(&m))
}

/// `$(css, $context)` / `findWithin`: descendants of `context` matching
/// `css`, excluding the context nodes themselves.
pub fn find<'a>(context: &Selection<'a>, css: &str) -> Selection<'a> {
    matcher(css).map_or_else(Selection::default, |m| context.select_matcher(&m))
}

/// Whether `node` matches `css`.
pub fn is(node: &NodeRef<'_>, css: &str) -> bool {
    matcher(css).is_some_and(|m| node.is_match(&m))
}

/// Filter `selection` down to nodes that don't match `css`, like cheerio's
/// `.not(css)`.
pub fn not<'a>(selection: &Selection<'a>, css: &str) -> Selection<'a> {
    let Some(m) = matcher(css) else {
        return selection.clone();
    };
    Selection::from(
        selection
            .nodes()
            .iter()
            .filter(|node| !node.is_match(&m))
            .copied()
            .collect::<Vec<_>>(),
    )
}

/// A selection over the whole document containing `node`.
pub fn document_of<'a>(node: &NodeRef<'a>) -> Selection<'a> {
    Selection::from(node.tree.root())
}

/// `$.html($selection)`: the outer HTML of every node, concatenated.
pub fn outer_html(selection: &Selection<'_>) -> String {
    selection.nodes().iter().map(html).collect()
}

/// The outer HTML of `node`, serialized like cheerio (parse5).
pub fn html(node: &NodeRef<'_>) -> String {
    serialize::outer(node)
}

/// The inner HTML of `node`, serialized like [`html`].
pub fn inner_html(node: &NodeRef<'_>) -> String {
    serialize::inner(node)
}

/// `$node.html()`: the inner HTML of the first node in `selection`.
pub fn inner_html_of(selection: &Selection<'_>) -> String {
    selection
        .nodes()
        .first()
        .map(inner_html)
        .unwrap_or_default()
}

/// `.text()`: the text of every node, concatenated.
pub fn text(selection: &Selection<'_>) -> String {
    selection.text().to_string()
}

/// The lowercase tag name of an element, or `""` for other nodes.
pub fn tag_name(node: &NodeRef<'_>) -> String {
    node.qual_name_ref()
        .map(|name| name.local.to_string())
        .unwrap_or_default()
}

/// An element's attributes as `(name, value)` pairs, in source order.
pub fn get_attrs(node: &NodeRef<'_>) -> Vec<(String, String)> {
    node.attrs()
        .into_iter()
        .map(|attr| {
            let name = match &attr.name.prefix {
                Some(prefix) => format!("{prefix}:{}", attr.name.local),
                None => attr.name.local.to_string(),
            };
            (name, attr.value.to_string())
        })
        .collect()
}

/// Replace `node` with the nodes parsed from `html`, like cheerio's
/// `$node.replaceWith(html)`. Does nothing when `node` is detached.
pub fn replace_with_html(node: &NodeRef<'_>, html: &str) {
    if node.parent().is_none() {
        return;
    }
    let fragment = parse_fragment(html);
    let nodes = fragment_nodes(&fragment);
    if nodes.is_empty() {
        node.remove_from_parent();
    } else {
        Selection::from(*node).replace_with_selection(&nodes);
    }
}

/// Append the nodes parsed from `html` to `node`, like `$node.append(html)`.
pub fn append_html(node: &NodeRef<'_>, html: &str) {
    let fragment = parse_fragment(html);
    Selection::from(*node).append_selection(&fragment_nodes(&fragment));
}

/// Create a detached `<div>` in `node`'s tree.
pub fn new_div<'a>(node: &NodeRef<'a>) -> NodeRef<'a> {
    node.tree.new_element("div")
}

/// Wrap `node` in `wrapper`, like `$node.wrap(wrapper)`. Works for detached
/// nodes too.
pub fn wrap(node: &NodeRef<'_>, wrapper: &NodeRef<'_>) {
    if node.parent().is_some() {
        node.insert_before(wrapper);
    }
    wrapper.append_child(node);
}

/// cheerio's `.hasClass(name)`.
pub fn has_class(node: &NodeRef<'_>, name: &str) -> bool {
    let Some(class) = node.attr("class").filter(|class| !class.is_empty()) else {
        return false;
    };
    !name.is_empty() && class.split(js::is_whitespace).any(|part| part == name)
}

/// cheerio's `.addClass(name)`, which keeps the existing attribute's
/// spacing and appends missing classes.
pub fn add_class(node: &NodeRef<'_>, name: &str) {
    if !node.is_element() || name.is_empty() {
        return;
    }
    let names: Vec<&str> = WHITESPACE_RE.split(name).collect();

    match node.attr("class").filter(|class| !class.is_empty()) {
        Some(class) => {
            let mut set_class = format!(" {class} ");
            for name in names {
                let append = format!("{name} ");
                if !set_class.contains(&format!(" {append}")) {
                    set_class.push_str(&append);
                }
            }
            node.set_attr("class", js::trim(&set_class));
        }
        None => node.set_attr("class", js::trim(&names.join(" "))),
    }
}

/// cheerio's `.removeClass(name)`. An element left without classes keeps
/// an empty `class` attribute.
pub fn remove_class(node: &NodeRef<'_>, name: &str) {
    let Some(class) = node.attr("class") else {
        return;
    };
    let mut classes: Vec<&str> = split_names(&class);
    let before = classes.len();
    let names = split_names(name);
    classes.retain(|class| !names.contains(class));

    if classes.len() != before {
        node.set_attr("class", &classes.join(" "));
    }
}

fn split_names(names: &str) -> Vec<&str> {
    if names.is_empty() {
        return Vec::new();
    }
    WHITESPACE_RE.split(js::trim(names)).collect()
}

fn matcher(css: &str) -> Option<Rc<Matcher>> {
    thread_local! {
        static MATCHERS: RefCell<HashMap<String, Option<Rc<Matcher>>>> = RefCell::default();
    }

    MATCHERS.with(|matchers| {
        matchers
            .borrow_mut()
            .entry(css.to_string())
            .or_insert_with(|| Matcher::new(css).ok().map(Rc::new))
            .clone()
    })
}

fn parse_opts() -> ParseOpts {
    ParseOpts {
        tree_builder: TreeBuilderOpts {
            scripting_enabled: true,
            drop_doctype: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_selectors_match_nothing() {
        let doc = parse_document("<p>hi</p>");
        assert!(select(&doc, "p[").is_empty());
        assert_eq!(select(&doc, "p").length(), 1);
    }

    #[test]
    fn noscript_contents_stay_raw_text() {
        let doc = parse_document(r#"<body><noscript><img src="a.jpg"></noscript></body>"#);
        assert_eq!(text(&select(&doc, "noscript")), r#"<img src="a.jpg">"#);
        assert_eq!(
            outer_html(&select(&doc, "noscript")),
            r#"<noscript><img src="a.jpg"></noscript>"#
        );
    }

    #[test]
    fn fragments_parse_in_template_context() {
        let fragment = parse_fragment("<td>cell</td><p>x</p>");
        assert_eq!(
            outer_html(&fragment_nodes(&fragment)),
            "<td>cell</td><p>x</p>"
        );
    }

    #[test]
    fn replaces_nodes_with_html() {
        let doc = parse_document("<body><div><span>a</span></div></body>");
        let span = select(&doc, "span");
        replace_with_html(&span.nodes()[0], "<b>1</b><i>2</i>");
        assert_eq!(
            outer_html(&select(&doc, "div")),
            "<div><b>1</b><i>2</i></div>"
        );
    }

    #[test]
    fn wraps_detached_nodes() {
        let doc = parse_document("<body><p>a</p></body>");
        let p = select(&doc, "p").nodes()[0];
        let holder = new_div(&p);
        holder.append_child(&p);
        let wrapper = new_div(&p);
        wrap(&holder, &wrapper);
        assert_eq!(html(&wrapper), "<div><div><p>a</p></div></div>");
        assert!(select(&doc, "p").is_empty());
    }
}
