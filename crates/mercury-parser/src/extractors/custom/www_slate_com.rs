//! www.slate.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.slate.com")
        .title(Field::new().selectors([".hed", "h1"]))
        .author(Field::new().selector("a[rel=author]"))
        .date_published(
            Field::new()
                .selector(".pub-date")
                .timezone("America/New_York"),
        )
        .dek(Field::new().selector(".dek"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".body").clean([
            ".about-the-author",
            ".pullquote",
            ".newsletter-signup-component",
            ".top-comment",
        ]))
}
