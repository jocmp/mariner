//! Decide whether a subtree would re-parse to itself.
//!
//! Upstream converts nodes by serializing their contents and parsing the
//! string back, which lets the HTML parser fix up nesting it wouldn't have
//! produced (a `<div>` inside a `<p>`, a `<tr>` outside a table, ...). For
//! a tree the parser would build unchanged, moving the children is
//! equivalent and much cheaper. These checks are conservative: anything
//! they can't vouch for takes the re-parse path.

use dom_query::{NodeData, NodeRef};
use html5ever::ns;

use super::tag_name;

// Start tags that close an open `<p>`, plus `table`, which only does in
// no-quirks documents.
const P_CLOSERS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "plaintext",
    "pre",
    "search",
    "section",
    "summary",
    "table",
    "ul",
    "xmp",
];

const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

// Elements whose text is written raw, so it must not contain their own
// end tag.
const RAW_TEXT: &[&str] = &[
    "iframe", "noembed", "noframes", "noscript", "script", "style", "xmp",
];

// Elements the parser ignores or restructures anywhere but the document
// root.
const NEVER_STABLE: &[&str] = &[
    "html",
    "head",
    "body",
    "frameset",
    "frame",
    "template",
    "plaintext",
    "image",
    "select",
    "option",
    "optgroup",
    "datalist",
    "math",
];

// HTML elements that break out of foreign (SVG) content.
const FOREIGN_BREAKOUT: &[&str] = &[
    "b",
    "big",
    "blockquote",
    "body",
    "br",
    "center",
    "code",
    "dd",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "hr",
    "i",
    "img",
    "li",
    "listing",
    "menu",
    "meta",
    "nobr",
    "ol",
    "p",
    "pre",
    "ruby",
    "s",
    "small",
    "span",
    "strong",
    "strike",
    "sub",
    "sup",
    "table",
    "tt",
    "u",
    "ul",
    "var",
    "font",
];

#[derive(Clone, Copy, Default)]
struct Scope {
    in_p: bool,
    in_a: bool,
    in_button: bool,
    in_nobr: bool,
    in_form: bool,
    in_li: bool,
    in_dd_dt: bool,
    in_ruby_text: bool,
    in_svg: bool,
}

/// Whether `children`, parsed as the contents of a `tag` element in a
/// `<template>` context, would produce the same nodes.
pub fn children_are_stable<'a>(tag: &str, children: impl IntoIterator<Item = NodeRef<'a>>) -> bool {
    let scope = Scope {
        in_p: tag == "p",
        ..Scope::default()
    };
    let heading = HEADINGS.contains(&tag);

    children.into_iter().all(|child| {
        if heading && child.is_element() && HEADINGS.contains(&tag_name(&child).as_str()) {
            return false;
        }
        node_is_stable(&child, scope)
    })
}

fn node_is_stable(node: &NodeRef<'_>, scope: Scope) -> bool {
    match node.query(|tree_node| match &tree_node.data {
        NodeData::Text { contents } => Some(!contents.contains(['\r', '\0'])),
        // Comments serialize as `<!--data-->`, which only round-trips when
        // the data can't end the comment early.
        NodeData::Comment { contents } => Some(
            !contents.contains("--") && !contents.starts_with('>') && !contents.starts_with("->"),
        ),
        NodeData::Element(_) => None,
        _ => Some(false),
    }) {
        Some(Some(stable)) => return stable,
        Some(None) => {}
        None => return false,
    }

    let Some(name) = node.qual_name_ref().map(|name| name.clone()) else {
        return false;
    };
    let tag = name.local.to_string();
    let parent_tag = node
        .parent()
        .map(|parent| tag_name(&parent))
        .unwrap_or_default();

    if name.ns != ns!(html) {
        // Only plain SVG is vouched for.
        if name.ns != ns!(svg) || FOREIGN_BREAKOUT.contains(&tag.as_str()) {
            return false;
        }
        let integration_point = matches!(tag.as_str(), "foreignObject" | "desc" | "title");
        let scope = Scope {
            in_svg: !integration_point,
            ..scope
        };
        return node
            .children_it(false)
            .all(|child| node_is_stable(&child, scope));
    }

    if scope.in_svg || NEVER_STABLE.contains(&tag.as_str()) {
        return false;
    }
    if scope.in_p && P_CLOSERS.contains(&tag.as_str()) {
        return false;
    }

    let mut scope = scope;
    match tag.as_str() {
        "p" => scope.in_p = true,
        "a" if scope.in_a => return false,
        "a" => scope.in_a = true,
        "button" if scope.in_button => return false,
        "button" => scope.in_button = true,
        "nobr" if scope.in_nobr => return false,
        "nobr" => scope.in_nobr = true,
        "form" if scope.in_form => return false,
        "form" => scope.in_form = true,
        "li" if scope.in_li => return false,
        "li" => scope.in_li = true,
        "dd" | "dt" if scope.in_dd_dt => return false,
        "dd" | "dt" => scope.in_dd_dt = true,
        "rt" | "rp" if scope.in_ruby_text => return false,
        "rt" | "rp" => scope.in_ruby_text = true,
        "ol" | "ul" | "menu" => scope.in_li = false,
        "dl" => scope.in_dd_dt = false,
        _ => {}
    }

    // Headings directly inside headings get split.
    if HEADINGS.contains(&tag.as_str())
        && node
            .children_it(false)
            .any(|child| HEADINGS.contains(&tag_name(&child).as_str()))
    {
        return false;
    }

    // Table parts must sit exactly where the parser puts them, with only
    // whitespace text between them.
    let allowed_children: Option<&[&str]> = match tag.as_str() {
        "table" => Some(&[
            "caption", "colgroup", "thead", "tbody", "tfoot", "script", "style",
        ]),
        "thead" | "tbody" | "tfoot" => Some(&["tr", "script", "style"]),
        "tr" => Some(&["td", "th", "script", "style"]),
        "colgroup" => Some(&["col"]),
        _ => None,
    };
    let required_parent: Option<&[&str]> = match tag.as_str() {
        "caption" | "colgroup" | "thead" | "tbody" | "tfoot" => Some(&["table"]),
        "tr" => Some(&["thead", "tbody", "tfoot"]),
        "td" | "th" => Some(&["tr"]),
        "col" => Some(&["colgroup"]),
        _ => None,
    };
    if required_parent.is_some_and(|parents| !parents.contains(&parent_tag.as_str())) {
        return false;
    }
    if let Some(allowed) = allowed_children {
        let table_children_ok = node.children_it(false).all(|child| {
            if child.is_element() {
                allowed.contains(&tag_name(&child).as_str())
            } else if child.is_text() {
                child
                    .text()
                    .chars()
                    .all(|c| matches!(c, ' ' | '\t' | '\n' | '\x0C'))
            } else {
                child.is_comment()
            }
        });
        if !table_children_ok {
            return false;
        }
    }
    // A table resets the scopes that don't cross cells.
    if matches!(tag.as_str(), "table" | "td" | "th" | "caption") {
        scope = Scope {
            in_form: scope.in_form,
            ..Scope::default()
        };
    }

    // The parser drops one leading newline from these.
    if matches!(tag.as_str(), "pre" | "listing" | "textarea")
        && node
            .first_child()
            .is_some_and(|child| child.is_text() && child.text().starts_with('\n'))
    {
        return false;
    }

    if RAW_TEXT.contains(&tag.as_str()) || tag == "textarea" || tag == "title" {
        let end_tag = format!("</{tag}");
        return node.children_it(false).all(|child| {
            child.is_text()
                && !child.text().to_ascii_lowercase().contains(&end_tag)
                && !child.text().contains(['\r', '\0'])
        });
    }

    node.children_it(false)
        .all(|child| node_is_stable(&child, scope))
}

#[cfg(test)]
mod tests {
    use super::super::{fragment_nodes, html, parse_document, parse_fragment, select};
    use super::*;

    fn stable(target: &str, html_contents: &str) -> bool {
        let doc = parse_document(&format!(
            "<body><div id=\"root\">{html_contents}</div></body>"
        ));
        let root = select(&doc, "#root").nodes()[0];
        children_are_stable(target, root.children())
    }

    // Anything the validator accepts really must round-trip.
    fn assert_round_trips(target: &str, html_contents: &str) {
        let doc = parse_document(&format!(
            "<body><div id=\"root\">{html_contents}</div></body>"
        ));
        let root = select(&doc, "#root").nodes()[0];
        let inner = super::super::inner_html(&root);
        let reparsed = parse_fragment(&format!("<{target}>{inner}</{target}>"));
        let reparsed = html(&fragment_nodes(&reparsed).nodes()[0]);
        assert_eq!(reparsed, format!("<{target}>{inner}</{target}>"));
    }

    #[test]
    fn accepts_ordinary_content() {
        let content = r#"<p>Hi <a href="x">there</a></p><ul><li>a<ul><li>b</li></ul></li></ul><table><tbody><tr><td><p>x</p></td></tr></tbody></table><svg viewBox="0 0 1 1"><path d="M0"></path></svg>"#;
        assert!(stable("div", content));
        assert_round_trips("div", content);
        assert!(stable("p", "text <b>bold</b> <img src=a.jpg>"));
        assert_round_trips("p", "text <b>bold</b> <img src=a.jpg>");
    }

    #[test]
    fn rejects_content_the_parser_would_restructure() {
        assert!(!stable("p", "<div>block</div>"));
        assert!(!stable("h2", "<h3>nested</h3>"));
        assert!(!stable("div", "<pre>\n\ncode</pre>"));
        assert!(!stable("div", "<select><option>a</option></select>"));
    }

    #[test]
    fn rejects_misplaced_table_parts() {
        let doc = parse_document(
            "<body><div id=\"root\"></div><table><tbody><tr><td>x</td></tr></tbody></table></body>",
        );
        let root = select(&doc, "#root").nodes()[0];
        let tr = select(&doc, "tr").nodes()[0];
        root.append_child(&tr);
        assert!(!children_are_stable("div", root.children()));
    }
}
