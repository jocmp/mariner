//! www.jnsa.org

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.jnsa.org")
        .title(Field::new().selector("#wgtitle h2"))
        .excerpt(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#main_area")
                .clean(["#pankuzu", "#side"]),
        )
}
