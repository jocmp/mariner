//! www.ladbible.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.ladbible.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector("[class*=Byline]"))
        .date_published(Field::new().selector("time").timezone("Europe/London"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("[class*=ArticleContainer]").clean([
            "time",
            "source",
            r#"a[href^="https://www.ladbible.com/"]"#,
            "picture",
            "[class*=StyledCardBlock]",
        ]))
}
