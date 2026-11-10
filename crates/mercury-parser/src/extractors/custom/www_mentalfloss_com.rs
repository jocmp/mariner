//! www.mentalfloss.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.mentalfloss.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selectors(["h1.title", ".title-group", ".inner"]),
        )
        .author(Field::new().selectors([
            r#"a[data-vars-label*="authors"]"#,
            ".field-name-field-enhanced-authors",
        ]))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .selector(".date-display-single")
                .timezone("America/New_York"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["article main", "div.field.field-name-body"])
                .clean(["small"]),
        )
}
