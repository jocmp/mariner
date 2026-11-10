//! www.spektrum.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.spektrum.de")
        .title(Field::new().selector(".content__title"))
        .author(Field::new().selector(".content__author__info__name"))
        .date_published(
            Field::new()
                .selector(".content__meta__date")
                .format("D.M.YYYY")
                .timezone("Europe/Berlin"),
        )
        .dek(Field::new().selector(".content__intro"))
        .lead_image_url(
            Field::new()
                .attr(r#"meta[name="og:image"]"#, "value")
                .attr(r#"meta[property="og:image"]"#, "content")
                .selector(".image__article__top img"),
        )
        .content(Field::new().selector("article.content").clean([
            ".breadcrumbs",
            ".hide-for-print",
            "aside",
            "header h2",
            ".image__article__top",
            ".content__author",
            ".copyright",
            ".callout-box",
        ]))
}
