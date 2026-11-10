//! techlog.iij.ad.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("techlog.iij.ad.jp")
        .title(Field::new().selector("h1.entry-title"))
        .author(Field::new().selector(r#"a[rel="author"]"#))
        .date_published(Field::new().attr("time.entry-date", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.entry-content")
                .default_cleaner(false)
                .clean([".wp_social_bookmarking_light"]),
        )
}
