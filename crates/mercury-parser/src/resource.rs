//! Load a page into a cleaned document, from `src/resource/`.

#[cfg(feature = "fetch")]
mod fetch;

use std::sync::LazyLock;

use dom_query::Document;
use regex::Regex;

#[cfg(feature = "fetch")]
pub use fetch::{Response, fetch_resource};

use crate::error::Error;
use crate::utils::dom::{get_attrs, parse_document, select};
use crate::utils::js;
use crate::utils::text::get_encoding;

static IS_LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?i)https?://").unwrap());
static IS_IMAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?i).(png|gif|jpe?g)").unwrap());
static IS_SRCSET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i).(png|gif|jpe?g)(\?[^",
        crate::js_ws!(),
        "]+)?([",
        crate::js_ws!(),
        r"]+[\d.]+[wx])"
    ))
    .unwrap()
});

const TAGS_TO_REMOVE: &str = "script,style,form";

/// Build a document from already-decoded HTML.
pub fn from_html(html: &str) -> Result<Document, Error> {
    generate_doc(parse_document(html))
}

/// Build a document from a response body, decoding it with the charset
/// named by `content_type` or by the page's own `<meta>` tags.
pub fn from_bytes(body: &[u8], content_type: &str) -> Result<Document, Error> {
    // TODO upstream: implement is_text from Readability.
    if !content_type.contains("html") && !content_type.contains("text") {
        return Err(Error::NotText);
    }

    let encoding = get_encoding(Some(content_type));
    let mut doc = parse_document(&decode(body, &encoding));

    // After the first parse, check to see if the encoding matches. If the
    // encodings in the header and body don't match, use the one in the body.
    let meta_content_type = select(&doc, "meta[http-equiv=content-type i]")
        .attr("content")
        .or_else(|| select(&doc, "meta[charset]").attr("charset"))
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty());
    if let Some(meta_content_type) = meta_content_type {
        let proper_encoding = get_encoding(Some(&meta_content_type));
        if proper_encoding != encoding {
            doc = parse_document(&decode(body, &proper_encoding));
        }
    }

    generate_doc(doc)
}

fn generate_doc(doc: Document) -> Result<Document, Error> {
    let body = select(&doc, "body");
    if body.children().is_empty() && js::trim(&body.text()).is_empty() {
        return Err(Error::BadParse);
    }

    normalize_meta_tags(&doc);
    convert_lazy_loaded_images(&doc);
    clean(&doc);

    Ok(doc)
}

fn decode(body: &[u8], encoding: &str) -> String {
    let encoding =
        encoding_rs::Encoding::for_label(encoding.as_bytes()).unwrap_or(encoding_rs::UTF_8);
    encoding.decode(body).0.into_owned()
}

// For ease of use in extracting from meta tags, replace the "content"
// attribute on meta tags with the "value" attribute, and normalize
// "property" attributes to "name" (see, e.g., og or twitter meta tags).
fn normalize_meta_tags(doc: &Document) {
    for (from, to) in [("content", "value"), ("property", "name")] {
        for node in select(doc, &format!("meta[{from}]")).nodes() {
            if let Some(value) = node.attr(from) {
                node.set_attr(to, &value);
                node.remove_attr(from);
            }
        }
    }
}

// Many sites have img tags with no source, or a placeholder src, and the
// real image in another attribute. Fill in src (and srcset) so the images
// are no longer lazy loaded.
fn convert_lazy_loaded_images(doc: &Document) {
    for img in select(doc, "img").nodes() {
        for (attr, _) in get_attrs(img) {
            // Read the current value: an earlier attribute may have set it.
            let Some(value) = img.attr(&attr).map(|value| value.to_string()) else {
                continue;
            };

            if attr != "srcset" && IS_LINK.is_match(&value) && IS_SRCSET.is_match(&value) {
                img.set_attr("srcset", &value);
            } else if attr != "src"
                && attr != "srcset"
                && IS_LINK.is_match(&value)
                && IS_IMAGE.is_match(&value)
            {
                // If the value is a JSON object, extract the image src from
                // it.
                let src = extract_src_from_json(&value).unwrap_or(value);
                img.set_attr("src", &src);
            }
        }
    }
}

fn extract_src_from_json(value: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(value).ok()?;
    json.get("src")?.as_str().map(ToString::to_string)
}

fn clean(doc: &Document) {
    for node in select(doc, TAGS_TO_REMOVE).nodes() {
        node.remove_from_parent();
    }

    // Collect first, then remove, so sibling links stay intact while
    // traversing.
    let comments: Vec<_> = doc
        .root()
        .descendants_it()
        .filter(|node| node.is_comment())
        .collect();
    for comment in comments {
        comment.remove_from_parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::dom::outer_html;

    #[test]
    fn normalizes_meta_tags() {
        let doc = from_html(
            r#"<head><meta property="og:title" content="Hi"></head><body><p>x</p></body>"#,
        )
        .unwrap();
        assert_eq!(
            outer_html(&select(&doc, "meta")),
            r#"<meta value="Hi" name="og:title">"#
        );
    }

    #[test]
    fn converts_lazy_loaded_images() {
        let doc = from_html(
            r#"<body><img data-src="https://example.com/a.jpg"><img data-srcset="https://example.com/b.jpg 2x"></body>"#,
        )
        .unwrap();
        let imgs = select(&doc, "img");
        assert_eq!(
            imgs.nodes()[0].attr("src").as_deref(),
            Some("https://example.com/a.jpg")
        );
        assert_eq!(
            imgs.nodes()[1].attr("srcset").as_deref(),
            Some("https://example.com/b.jpg 2x")
        );
    }

    #[test]
    fn removes_scripts_and_comments() {
        let doc = from_html("<body><!-- hi --><script>x</script><p>a</p></body>").unwrap();
        assert_eq!(
            crate::utils::dom::inner_html(&doc.body().unwrap()),
            "<p>a</p>"
        );
    }

    #[test]
    fn rejects_empty_documents() {
        assert!(matches!(
            from_html("<html><body></body></html>"),
            Err(Error::BadParse)
        ));
    }

    #[test]
    fn decodes_with_meta_charset() {
        let (bytes, _, _) =
            encoding_rs::SHIFT_JIS.encode(r#"<meta charset="shift_jis"><p>日本語</p>"#);
        let doc = from_bytes(&bytes, "text/html").unwrap();
        assert_eq!(select(&doc, "p").text().to_string(), "日本語");
    }
}
