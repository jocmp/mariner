//! www.oreilly.co.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.oreilly.co.jp")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector("h3"),
        )
        .author(Field::new().selectors([r#"span[itemprop="author"]"#, r#"li[itemprop="author"]"#]))
        .date_published(
            Field::new()
                .attr(r#"dd[itemprop="datePublished"]"#, "content")
                .attr(r#"meta[itemprop="datePublished"]"#, "value"),
        )
        .lead_image_url(
            Field::new()
                .attr(r#"meta[name="og:image:secure_url"]"#, "value")
                .attr(r#"meta[name="og:image"]"#, "value"),
        )
        .content(
            Field::new()
                .selectors(["section.detail", "#content"])
                .default_cleaner(false)
                .clean([".social-tools"]),
        )
}
