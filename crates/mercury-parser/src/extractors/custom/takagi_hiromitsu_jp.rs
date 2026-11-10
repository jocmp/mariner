//! takagi-hiromitsu.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("takagi-hiromitsu.jp")
        .title(Field::new().selector("h3"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[http-equiv="Last-Modified"]"#, "value"))
        .content(Field::new().selector("div.body").default_cleaner(false))
}
