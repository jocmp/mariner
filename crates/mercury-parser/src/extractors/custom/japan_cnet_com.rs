//! japan.cnet.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("japan.cnet.com")
        .title(Field::new().selector(".leaf-headline-ttl"))
        .author(Field::new().selector(".writer"))
        .date_published(
            Field::new()
                .selector(".date")
                .format("YYYY年M月D日 HH時mm分")
                .timezone("Asia/Tokyo"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div.article_body"))
}
