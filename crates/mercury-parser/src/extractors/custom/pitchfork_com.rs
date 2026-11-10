//! pitchfork.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("pitchfork.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector("title"),
        )
        .author(
            Field::new()
                .attr(r#"meta[name="article:author"]"#, "value")
                .selector(".authors-detail__display-name"),
        )
        .date_published(
            Field::new()
                .selector(r#"div[class^="InfoSliceWrapper-"]"#)
                .attr(".pub-date", "datetime"),
        )
        .dek(
            Field::new()
                .attr(r#"meta[name="og:description"]"#, "value")
                .selector(".review-detail__abstract"),
        )
        .lead_image_url(
            Field::new()
                .attr(r#"meta[name="og:image"]"#, "value")
                .attr(".single-album-tombstone__art img", "src"),
        )
        .content(Field::new().selectors(["div.body__inner-container", ".review-detail__text"]))
        .extend(
            "score",
            Field::new().selectors([r#"p[class*="Rating"]"#, ".score"]),
        )
}
