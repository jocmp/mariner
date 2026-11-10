//! otrs.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("otrs.com")
        .title(Field::new().selector("#main article h1"))
        .author(Field::new().selector("div.dateplusauthor a"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#main article")
                .default_cleaner(false)
                .clean([
                    "div.dateplusauthor",
                    "div.gr-12.push-6.footershare",
                    "#atftbx",
                    "div.category-modul",
                ]),
        )
}
