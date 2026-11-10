//! sect.iij.ad.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("sect.iij.ad.jp")
        .title(Field::new().selectors(["div.title-box-inner h1", "h3"]))
        .author(Field::new().selectors(["p.post-author a", "dl.entrydate dd"]))
        .date_published(
            Field::new()
                .selector("time")
                .format("YYYY年M月D日")
                .timezone("Asia/Tokyo"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".entry-inner", "#article"])
                .clean(["dl.entrydate"]),
        )
}
