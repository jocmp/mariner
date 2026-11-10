//! wired.jp

use url::Url;

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("wired.jp")
        .title(Field::new().selectors([r#"h1[data-testid="ContentHeaderHed"]"#, "h1.post-title"]))
        .author(
            Field::new()
                .attr(r#"meta[name="article:author"]"#, "value")
                .selector(r#"p[itemprop="author"]"#),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr("time", "datetime"),
        )
        .dek(Field::new().selectors([r#"div[class^="ContentHeaderDek"]"#, ".post-intro"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([
                    r#"div[data-attribute-verso-pattern="article-body"]"#,
                    "article.article-detail",
                ])
                .transform("img[data-original]", |node| {
                    let data_original = node.attr("data-original");
                    let src = node.attr("src");
                    let url = resolve_url(
                        data_original.as_deref().unwrap_or("undefined"),
                        src.as_deref(),
                    );
                    node.set_attr("src", &url);
                    None
                })
                .clean([
                    ".post-category",
                    "time",
                    "h1.post-title",
                    ".social-area-syncer",
                ]),
        )
}

/// Upstream's `resolveUrl`: `new URL(url, base).href`, or `url` unchanged
/// when that throws.
fn resolve_url(url: &str, base: Option<&str>) -> String {
    let resolved = match base {
        Some(base) => Url::parse(base).and_then(|base| base.join(url)),
        None => Url::parse(url),
    };
    resolved.map_or_else(|_| url.to_string(), String::from)
}
