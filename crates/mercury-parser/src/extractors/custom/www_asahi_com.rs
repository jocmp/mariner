//! www.asahi.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.asahi.com")
        .title(Field::new().selectors(["main h1", ".ArticleTitle h1"]))
        .author(Field::new().attr(r#"meta[name="article:author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="pubdate"]"#, "value"))
        .excerpt(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("main").default_cleaner(false).clean([
            "div.AdMod",
            "div.LoginSelectArea",
            "time",
            "div.notPrint",
        ]))
}
