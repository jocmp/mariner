//! github.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("github.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new())
        .date_published(
            Field::new()
                .attr("relative-time[datetime]", "datetime")
                .attr(r#"span[itemprop="dateModified"] relative-time"#, "datetime"),
        )
        .dek(
            Field::new()
                .attr(r#"meta[name="description"]"#, "value")
                .selector(r#"span[itemprop="about"]"#),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().group(["#readme article"]))
}
