//! www.inquisitr.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.inquisitr.com")
        .title(Field::new().selector("h1.entry-title.story--header--title"))
        .author(Field::new().selector("div.story--header--author"))
        .date_published(Field::new().attr(r#"meta[name="datePublished"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["article.story", ".entry-content."])
                .clean([
                    ".post-category",
                    ".story--header--socials",
                    ".story--header--content",
                ]),
        )
}
