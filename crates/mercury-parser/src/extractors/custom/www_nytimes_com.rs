//! www.nytimes.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.nytimes.com")
        .title(Field::new().selectors([
            r#"h1[data-testid="headline"]"#,
            "h1.g-headline",
            r#"h1[itemprop="headline"]"#,
            "h1.headline",
            "h1 .balancedHeadline",
        ]))
        .author(
            Field::new()
                .attr(r#"meta[name="author"]"#, "value")
                .selectors([".g-byline", ".byline"])
                .attr(r#"meta[name="byl"]"#, "value"),
        )
        .content(
            Field::new()
                .selectors([
                    "div.g-blocks",
                    r#"section[name="articleBody"]"#,
                    "article#story",
                ])
                .transform("img.g-lazy", |node| {
                    let src = node.attr("src")?;
                    let width = 640;

                    let src = src.replacen("{{size}}", &width.to_string(), 1);
                    node.set_attr("src", &src);
                    None
                })
                .clean([
                    ".ad",
                    "header#story-header",
                    ".story-body-1 .lede.video",
                    ".visually-hidden",
                    "#newsletter-promo",
                    ".promo",
                    ".comments-button",
                    ".hidden",
                    ".comments",
                    ".supplemental",
                    ".nocontent",
                    ".story-footer-links",
                ]),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr(r#"meta[name="article:published"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
}
