//! bookwalker.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("bookwalker.jp")
        .title(Field::new().selectors(["h1.p-main__title", "h1.main-heading"]))
        .author(Field::new().selectors(["div.p-author__list", "div.authors"]))
        .date_published(
            Field::new()
                .selectors([
                    "dl.p-information__data dd:nth-of-type(7)",
                    ".work-info .work-detail:first-of-type .work-detail-contents:last-of-type",
                ])
                .timezone("Asia/Tokyo"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.p-main__information")
                .group(["div.main-info", "div.main-cover-inner"])
                .default_cleaner(false)
                .clean([
                    "span.label.label--trial",
                    "dt.info-head.info-head--coin",
                    "dd.info-contents.info-contents--coin",
                    "div.info-notice.fn-toggleClass",
                ]),
        )
}
