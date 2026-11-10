//! Atom XHTML content as HTML text, and HTML titles as plain text.
//!
//! An entry's `content`, `summary` or `title` of type `xhtml` becomes the
//! HTML of its wrapping `<div>`'s children, serialized like libxml2's HTML
//! serializer (which the reference uses): void elements without a closing
//! tag, line breaks around block elements, boolean attributes minimized,
//! URL attributes percent-escaped, and non-ASCII characters written as
//! character references when the document declared no encoding.

use crate::tree::{Element, Node};

/// Replaces the children of every `entry > content|summary|title` of type
/// `xhtml` with the serialized inner HTML of their `<div>` children.
pub(crate) fn flatten_entry_content(nodes: &mut [Node], declared_encoding: bool) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.name == "entry" {
            for child in &mut element.children {
                if let Node::Element(child) = child
                    && matches!(child.name.as_str(), "content" | "summary" | "title")
                    && child.attr("type") == Some("xhtml")
                {
                    let html = inner_html(child, declared_encoding);
                    child.children = if html.is_empty() {
                        Vec::new()
                    } else {
                        vec![Node::Text(html)]
                    };
                }
            }
        }
        flatten_entry_content(&mut element.children, declared_encoding);
    }
}

/// The text of an HTML fragment with whitespace collapsed, like XPath's
/// `normalize-space`.
pub(crate) fn text_content(html: &str) -> String {
    let text = dom_query::Document::fragment(html).text();
    text.split([' ', '\t', '\r', '\n'])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn inner_html(element: &Element, declared_encoding: bool) -> String {
    let mut out = String::new();
    let writer = Writer { declared_encoding };
    for child in &element.children {
        if let Node::Element(div) = child
            && div.name == "div"
        {
            writer.children(&mut out, div);
        }
    }
    out
}

struct Writer {
    declared_encoding: bool,
}

/// libxml2's knowledge of an HTML element: whether it's inline, and
/// whether it's void.
struct Info {
    inline: bool,
    void: bool,
}

fn info(name: &str) -> Option<Info> {
    const INLINE: &[&str] = &[
        "a", "abbr", "acronym", "applet", "b", "basefont", "bdo", "big", "br", "button", "cite",
        "code", "del", "dfn", "em", "embed", "font", "i", "iframe", "img", "input", "ins", "kbd",
        "label", "map", "object", "q", "s", "samp", "script", "select", "small", "span", "strike",
        "strong", "sub", "sup", "textarea", "tt", "u", "var",
    ];
    const BLOCK: &[&str] = &[
        "address",
        "area",
        "base",
        "blockquote",
        "body",
        "caption",
        "center",
        "col",
        "colgroup",
        "dd",
        "dir",
        "div",
        "dl",
        "dt",
        "fieldset",
        "form",
        "frame",
        "frameset",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "hr",
        "html",
        "isindex",
        "legend",
        "li",
        "link",
        "menu",
        "meta",
        "noframes",
        "noscript",
        "ol",
        "optgroup",
        "option",
        "p",
        "param",
        "pre",
        "style",
        "table",
        "tbody",
        "td",
        "tfoot",
        "th",
        "thead",
        "title",
        "tr",
        "ul",
    ];
    const VOID: &[&str] = &[
        "area", "base", "basefont", "br", "col", "embed", "frame", "hr", "img", "input", "isindex",
        "link", "meta", "param",
    ];
    let inline = INLINE.contains(&name);
    (inline || BLOCK.contains(&name)).then(|| Info {
        inline,
        void: VOID.contains(&name),
    })
}

fn is_text(node: &Node) -> bool {
    matches!(node, Node::Text(_))
}

/// Whether line breaks go around this element's content. Never for
/// inline elements, unknown ones, or names starting with `p`.
fn breaks(info: &Option<Info>, name: &str) -> bool {
    info.as_ref().is_some_and(|info| !info.inline) && !name.starts_with('p')
}

impl Writer {
    fn children(&self, out: &mut String, parent: &Element) {
        for (index, node) in parent.children.iter().enumerate() {
            let next = parent.children.get(index + 1);
            self.node(out, node, parent, next);
        }
    }

    fn node(&self, out: &mut String, node: &Node, parent: &Element, next: Option<&Node>) {
        match node {
            Node::Text(text) if matches!(parent.name.as_str(), "script" | "style") => {
                out.push_str(text)
            }
            Node::Text(text) => self.escape(out, text, false),
            Node::CData(text) => out.push_str(text),
            Node::Comment(text) => {
                out.push_str("<!--");
                out.push_str(text);
                out.push_str("-->");
            }
            Node::Element(element) => self.element(out, element, parent, next),
        }
    }

    fn element(&self, out: &mut String, element: &Element, parent: &Element, next: Option<&Node>) {
        let info = info(&element.name);
        out.push('<');
        out.push_str(&element.name);
        for (key, value) in &element.attrs {
            self.attribute(out, element, key, value);
        }
        if element.children.is_empty() {
            out.push('>');
            if !info.as_ref().is_some_and(|info| info.void) {
                out.push_str("</");
                out.push_str(&element.name);
                out.push('>');
            }
        } else {
            out.push('>');
            let several = element.children.len() > 1;
            let block = breaks(&info, &element.name) && several;
            if block && !is_text(&element.children[0]) {
                out.push('\n');
            }
            self.children(out, element);
            if block && !element.children.last().is_some_and(is_text) {
                out.push('\n');
            }
            out.push_str("</");
            out.push_str(&element.name);
            out.push('>');
        }
        let block = info.as_ref().is_some_and(|info| !info.inline);
        if block && next.is_some_and(|next| !is_text(next)) && !parent.name.starts_with('p') {
            out.push('\n');
        }
    }

    fn attribute(&self, out: &mut String, element: &Element, key: &str, value: &str) {
        const BOOLEAN: &[&str] = &[
            "checked", "compact", "declare", "defer", "disabled", "ismap", "multiple", "nohref",
            "noresize", "noshade", "nowrap", "readonly", "selected",
        ];
        out.push(' ');
        out.push_str(key);
        if BOOLEAN.contains(&key.to_ascii_lowercase().as_str()) {
            return;
        }
        let lower = key.to_ascii_lowercase();
        let uri = matches!(lower.as_str(), "href" | "action" | "src")
            || (lower == "name" && element.name == "a");
        let value = if uri {
            self.escape_uri(value.trim_start_matches([' ', '\t', '\n', '\r']))
        } else {
            value.to_string()
        };
        let (quote, escape_quote) = match (value.contains('"'), value.contains('\'')) {
            (true, false) => ('\'', false),
            (true, true) => ('"', true),
            _ => ('"', false),
        };
        let mut escaped = String::new();
        self.escape(&mut escaped, &value, true);
        if escape_quote {
            escaped = escaped.replace('"', "&quot;");
        }
        out.push('=');
        out.push(quote);
        out.push_str(&escaped);
        out.push(quote);
    }

    fn escape(&self, out: &mut String, text: &str, attribute: bool) {
        for c in text.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' if !attribute => out.push_str("&gt;"),
                c if !c.is_ascii() && !self.declared_encoding => {
                    out.push_str(&format!("&#x{:X};", c as u32));
                }
                c => out.push(c),
            }
        }
    }

    /// Percent-escapes everything but unreserved characters and
    /// `@/:=?;#%&,+<>[]`. Without a declared encoding, non-ASCII characters
    /// are left for `escape` to write as character references.
    fn escape_uri(&self, value: &str) -> String {
        let mut out = String::new();
        for c in value.chars() {
            let kept = c.is_ascii_alphanumeric() || "-_.!~*'()@/:=?;#%&,+<>[]".contains(c);
            if kept || (!c.is_ascii() && !self.declared_encoding) {
                out.push(c);
            } else {
                let mut bytes = [0; 4];
                for byte in c.encode_utf8(&mut bytes).bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
        out
    }
}
