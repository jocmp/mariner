//! medium.com

use std::sync::LazyLock;

use regex::Regex;

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::{inner_html_of, outer_html};

static SINGLE_LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z()]+$").unwrap());
static YOUTUBE_THUMB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://i.embed.ly/.+url=https://i\.ytimg\.com/vi/([0-9A-Za-z_]+)/").unwrap()
});

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("medium.com")
        .title(
            Field::new()
                .selector("h1")
                .attr(r#"meta[name="og:title"]"#, "value"),
        )
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .content(
            Field::new()
                .selector("article")
                .transform("section span:first-of-type", |node| {
                    let text = inner_html_of(node);
                    if text.len() == 1 && SINGLE_LETTER.is_match(&text) {
                        node.replace_with_html(text);
                    }
                    None
                })
                .transform("iframe", |node| {
                    let thumb = decode_uri_component(
                        node.attr("data-thumbnail")
                            .as_deref()
                            .unwrap_or("undefined"),
                    );
                    let parent = node.ancestors(None).filter("figure");

                    if let Some(captures) = YOUTUBE_THUMB.captures(&thumb) {
                        node.set_attr(
                            "src",
                            &format!("https://www.youtube.com/embed/{}", &captures[1]),
                        );
                        let caption = parent.select("figcaption");
                        parent.set_html(format!("{}{}", outer_html(node), outer_html(&caption)));
                        return None;
                    }

                    // If we can't draw the YouTube preview, remove the figure.
                    parent.remove();
                    None
                })
                .transform("figure", |node| {
                    // ignore if figure has an iframe
                    if node.select("iframe").exists() {
                        return None;
                    }

                    let img = node.select("img").last();
                    let caption = node.select("figcaption");

                    node.set_html(format!("{}{}", outer_html(&img), outer_html(&caption)));
                    None
                })
                .transform("img", |node| {
                    if node
                        .attr("width")
                        .and_then(|w| parse_int(&w))
                        .is_some_and(|w| w < 100)
                    {
                        node.remove();
                    }
                    None
                })
                .clean(["span a", "svg"]),
        )
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .next_page_url(Field::new())
        .excerpt(Field::new())
}

/// JS `parseInt(value, 10)`: leading whitespace, an optional sign, then as
/// many digits as are present. `None` stands in for `NaN`.
fn parse_int(value: &str) -> Option<i64> {
    let value = value.trim_start();
    let (sign, digits) = match value.as_bytes().first() {
        Some(b'-') => (-1, &value[1..]),
        Some(b'+') => (1, &value[1..]),
        _ => (1, value),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    digits[..end].parse::<i64>().ok().map(|n| sign * n)
}

/// JS `decodeURIComponent`, returning the input unchanged where upstream
/// would throw a `URIError`.
fn decode_uri_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok());
            match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                Some(byte) => {
                    out.push(byte);
                    i += 3;
                    continue;
                }
                None => return value.to_string(),
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| value.to_string())
}
