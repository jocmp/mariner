//! The few inline `style` declarations that affect conversion.

use dom_query::NodeRef;

/// Inline `display: none`.
pub(crate) fn is_hidden(node: &NodeRef) -> bool {
    declarations(node).any(|(property, value)| {
        property == "display" && value.to_ascii_lowercase().starts_with("none")
    })
}

/// Inline `font-weight: bold`, exactly.
pub(crate) fn is_bold(node: &NodeRef) -> bool {
    declarations(node)
        .any(|(property, value)| property == "font-weight" && value.eq_ignore_ascii_case("bold"))
}

/// Inline `font-style: italic` or `oblique`, exactly.
pub(crate) fn is_italic(node: &NodeRef) -> bool {
    declarations(node).any(|(property, value)| {
        property == "font-style"
            && (value.eq_ignore_ascii_case("italic") || value.eq_ignore_ascii_case("oblique"))
    })
}

/// The URL in an inline `background-image` (or `background`) declaration.
pub(crate) fn background_image(node: &NodeRef) -> Option<String> {
    declarations(node)
        .filter(|(property, _)| property == "background-image" || property == "background")
        .find_map(|(_, value)| css_url(&value))
}

/// `(property, value)` pairs from the `style` attribute: the property
/// lowercased, both trimmed. Values keep their case, since URLs need it.
fn declarations(node: &NodeRef) -> impl Iterator<Item = (String, String)> {
    let style = node
        .attr("style")
        .map(|style| style.to_string())
        .unwrap_or_default();
    style
        .split(';')
        .filter_map(|declaration| {
            let (property, value) = declaration.split_once(':')?;
            Some((
                property.trim().to_ascii_lowercase(),
                value.trim().to_string(),
            ))
        })
        .collect::<Vec<_>>()
        .into_iter()
}

fn css_url(value: &str) -> Option<String> {
    let start = value.to_ascii_lowercase().find("url(")? + "url(".len();
    let end = start + value[start..].find(')')?;
    let url = value[start..end].trim().trim_matches(['"', '\'']).trim();
    (!url.is_empty()).then(|| url.to_string())
}
