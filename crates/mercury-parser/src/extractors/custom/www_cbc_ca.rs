//! www.cbc.ca

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.cbc.ca")
        .title(Field::new().selector("h1"))
        .author(Field::new().selectors([".authorText", ".bylineDetails"]))
        .date_published(Field::new().attr(".timeStamp[datetime]", "datetime"))
        .dek(Field::new().selector(".deck"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".story"))
}
