//! www.theatlantic.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.theatlantic.com")
        .title(Field::new().selectors(["h1", ".c-article-header__hed"]))
        .author(
            Field::new()
                .attr(r#"meta[name="author"]"#, "value")
                .selector(".c-byline__author"),
        )
        .content(Field::new().selectors(["article", ".article-body"]).clean([
            ".partner-box",
            ".callout",
            ".c-article-writer__image",
            ".c-article-writer__content",
            ".c-letters-cta__text",
            ".c-footer__logo",
            ".c-recirculation-link",
            ".twitter-tweet",
        ]))
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .date_published(Field::new().attr(r#"time[itemprop="datePublished"]"#, "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
}
