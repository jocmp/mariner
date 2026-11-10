//! www.blick.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.blick.de")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(".article-meta__author"))
        .date_published(Field::new().attr("time.article-meta__date[datetime]", "datetime"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("article")
                .default_cleaner(false)
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("figcaption details", |node| {
                    let text = node.text();
                    node.replace_with_html(format!("<span>{text}</span>"));
                    None
                })
                .rename("ul.gallery__item-wrapper", "div")
                .rename("li.gallery__item", "div")
                .clean([
                    ".section-header",
                    ".article__footer",
                    ".social-button-container",
                    ".gallery__button",
                    ".gallery__position-label",
                    ".detail-img__caption-toggle",
                    ".nativendo-mid-article",
                    ".taboola-mid-article",
                    "article > p",
                ]),
        )
}
