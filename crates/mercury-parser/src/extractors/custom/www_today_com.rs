//! www.today.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.today.com")
        .title(Field::new().selectors(["h1.article-hero-headline__htag", "h1.entry-headline"]))
        .author(
            Field::new()
                .selector("span.byline-name")
                .attr(r#"meta[name="author"]"#, "value"),
        )
        .date_published(
            Field::new()
                .selector("time[datetime]")
                .attr(r#"meta[name="DC.date.issued"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["div.article-body__content", ".entry-container"])
                .clean([".label-comment"]),
        )
}
