//! www.broadwayworld.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.broadwayworld.com")
        .title(Field::new().selectors(["h1[itemprop=headline]", "h1.article-title"]))
        .author(Field::new().selector("span[itemprop=author]"))
        .content(Field::new().selector("div[itemprop=articlebody]"))
        .date_published(Field::new().attr("meta[itemprop=datePublished]", "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new())
        .next_page_url(Field::new())
        .excerpt(Field::new())
}
