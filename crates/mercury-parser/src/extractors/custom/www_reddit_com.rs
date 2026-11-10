//! www.reddit.com

use std::sync::LazyLock;

use regex::Regex;

use crate::extractors::{CustomExtractor, Field};

static PARENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((.*?)\)").unwrap());

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.reddit.com")
        .title(Field::new().selectors([
            r#"div[data-test-id="post-content"] h1"#,
            r#"div[data-test-id="post-content"] h2"#,
        ]))
        .author(Field::new().selector(r#"div[data-test-id="post-content"] a[href*="user/"]"#))
        .date_published(Field::new().selectors([
            r#"div[data-test-id="post-content"] span[data-click-id="timestamp"]"#,
            r#"div[data-test-id="post-content"] a[data-click-id="timestamp"]"#,
        ]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([r#"div[data-test-id="post-content"] p"#])
                .group([
                    r#"div[data-test-id="post-content"] a[target="_blank"]:not([data-click-id="timestamp"])"#,
                    r#"div[data-test-id="post-content"] div[data-click-id="media"]"#,
                ])
                .group([r#"div[data-test-id="post-content"] div[data-click-id="media"]"#])
                .group([r#"div[data-test-id="post-content"] a"#])
                .selector(r#"div[data-test-id="post-content"]"#)
                .transform(r#"div[role="img"]"#, |node| {
                    // External link image preview
                    let img = node.select("img");
                    let bg_img = css_property(
                        node.attr("style").as_deref().unwrap_or(""),
                        "background-image",
                    );
                    if let Some(bg_img) = bg_img.filter(|s| !s.is_empty())
                        && img.length() == 1
                    {
                        let captures = PARENS.captures(&bg_img)?;
                        img.set_attr("src", &captures[1].replace(['\'', '"'], ""));
                    }
                    // Upstream returns a selection, not a tag name, so the node is never converted.
                    None
                })
                .clean([
                    ".icon",
                    r#"span[id^="PostAwardBadges"]"#,
                    r#"div a[data-test-id="comments-page-link-num-comments"]"#,
                ]),
        )
}

/// cheerio's `.css(property)` getter, which parses the `style` attribute.
fn css_property(style: &str, property: &str) -> Option<String> {
    let style = style.trim();
    let mut values: Vec<(String, String)> = Vec::new();
    for part in style.split(';') {
        match part.find(':') {
            Some(n) if n >= 1 && n != part.len() - 1 => {
                let key = part[..n].trim().to_string();
                let value = part[n + 1..].trim().to_string();
                values.retain(|(k, _)| *k != key);
                values.push((key, value));
            }
            _ => {
                let trimmed = part.trim_end();
                if let Some((_, value)) = values.last_mut().filter(|_| !trimmed.is_empty()) {
                    value.push(';');
                    value.push_str(trimmed);
                }
            }
        }
    }
    values
        .into_iter()
        .find(|(k, _)| k == property)
        .map(|(_, v)| v)
}
