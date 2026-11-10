//! HTML serialization matching parse5, which cheerio renders with.
//!
//! html5ever's serializer follows a newer revision of the spec that also
//! escapes `<` and `>` in attribute values; parse5 doesn't, and the
//! difference shows up in extracted content.

use dom_query::{NodeData, NodeId, NodeRef};
use html5ever::ns;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
    "keygen", "link", "meta", "param", "source", "track", "wbr",
];

// Elements whose text children are written unescaped. `noscript` is
// included because cheerio parses with scripting enabled.
const RAW_TEXT_ELEMENTS: &[&str] = &[
    "style",
    "script",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "plaintext",
    "noscript",
];

/// Serialize `node` and its descendants.
pub fn outer(node: &NodeRef<'_>) -> String {
    let mut out = String::new();
    write_node(node, &mut out);
    out
}

/// Serialize the children of `node`.
pub fn inner(node: &NodeRef<'_>) -> String {
    let mut out = String::new();
    write_children(node, &mut out);
    out
}

fn write_node(node: &NodeRef<'_>, out: &mut String) {
    enum Kind {
        Element {
            name: String,
            html: bool,
            attrs: String,
            template: Option<NodeId>,
        },
        Text(String),
        Comment(String),
        Doctype(String),
        Container,
        Other,
    }

    let kind = node
        .query(|tree_node| match &tree_node.data {
            NodeData::Element(element) => {
                let mut attrs = String::new();
                for attr in &element.attrs {
                    attrs.push(' ');
                    if let Some(prefix) = &attr.name.prefix {
                        attrs.push_str(prefix);
                        attrs.push(':');
                    }
                    attrs.push_str(&attr.name.local);
                    attrs.push_str("=\"");
                    escape(&attr.value, true, &mut attrs);
                    attrs.push('"');
                }
                Kind::Element {
                    name: element.name.local.to_string(),
                    html: element.name.ns == ns!(html),
                    attrs,
                    template: element.template_contents,
                }
            }
            NodeData::Text { contents } => Kind::Text(contents.to_string()),
            NodeData::Comment { contents } => Kind::Comment(contents.to_string()),
            NodeData::Doctype { name, .. } => Kind::Doctype(name.to_string()),
            NodeData::Document | NodeData::Fragment => Kind::Container,
            NodeData::ProcessingInstruction { .. } => Kind::Other,
        })
        .unwrap_or(Kind::Other);

    match kind {
        Kind::Element {
            name,
            html,
            attrs,
            template,
        } => {
            out.push('<');
            out.push_str(&name);
            out.push_str(&attrs);
            out.push('>');
            if html && VOID_ELEMENTS.contains(&name.as_str()) {
                return;
            }
            match template.and_then(|id| node.tree.get(&id)) {
                Some(contents) if html && name == "template" => write_children(&contents, out),
                _ => write_children(node, out),
            }
            out.push_str("</");
            out.push_str(&name);
            out.push('>');
        }
        Kind::Text(text) => {
            let raw = node
                .parent()
                .and_then(|parent| {
                    parent.qual_name_ref().map(|name| {
                        name.ns == ns!(html) && RAW_TEXT_ELEMENTS.contains(&&*name.local)
                    })
                })
                .unwrap_or(false);
            if raw {
                out.push_str(&text);
            } else {
                escape(&text, false, out);
            }
        }
        Kind::Comment(text) => {
            out.push_str("<!--");
            out.push_str(&text);
            out.push_str("-->");
        }
        Kind::Doctype(name) => {
            out.push_str("<!DOCTYPE ");
            out.push_str(&name);
            out.push('>');
        }
        Kind::Container => write_children(node, out),
        Kind::Other => {}
    }
}

fn write_children(node: &NodeRef<'_>, out: &mut String) {
    for child in node.children_it(false) {
        write_node(&child, out);
    }
}

fn escape(text: &str, attribute: bool, out: &mut String) {
    let mut last = 0;
    for (index, c) in text.char_indices() {
        let escaped = match c {
            '&' => "&amp;",
            '\u{a0}' => "&nbsp;",
            '"' if attribute => "&quot;",
            '<' if !attribute => "&lt;",
            '>' if !attribute => "&gt;",
            _ => continue,
        };
        out.push_str(&text[last..index]);
        out.push_str(escaped);
        last = index + c.len_utf8();
    }
    out.push_str(&text[last..]);
}
