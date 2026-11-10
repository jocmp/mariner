//! weekly.ascii.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("weekly.ascii.jp")
        .title(Field::new().selectors(["article h1", r#"h1[itemprop="headline"]"#]))
        .author(Field::new().selector("p.author"))
        .date_published(
            Field::new()
                .selector("p.date")
                .attr(r#"meta[name="odate"]"#, "value")
                .format("YYYY年M月D日 HH:mm")
                .timezone("Asia/Tokyo"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors(["div#contents_detail", "div.article"]))
}
