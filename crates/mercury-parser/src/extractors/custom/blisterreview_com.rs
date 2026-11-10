//! blisterreview.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("blisterreview.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector("h1.entry-title"),
        )
        .author(Field::new().selector("span.author-name"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr("time.entry-date", "datetime")
                .attr(r#"meta[itemprop="datePublished"]"#, "content"),
        )
        .dek(Field::new())
        .lead_image_url(
            Field::new()
                .attr(r#"meta[name="og:image"]"#, "value")
                .attr(r#"meta[property="og:image"]"#, "content")
                .attr(r#"meta[itemprop="image"]"#, "content")
                .attr(r#"meta[name="twitter:image"]"#, "content")
                .attr("img.attachment-large", "src"),
        )
        .content(
            Field::new()
                .group([
                    ".elementor-section-wrap",
                    ".elementor-text-editor > p, .elementor-text-editor > ul > li, .attachment-large, .wp-caption-text",
                ])
                .rename("figcaption", "p")
                .clean([".comments-area"]),
        )
}
