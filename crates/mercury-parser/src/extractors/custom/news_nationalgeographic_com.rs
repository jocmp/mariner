//! news.nationalgeographic.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("news.nationalgeographic.com")
        .title(Field::new().selectors(["h1", "h1.main-title"]))
        .author(Field::new().selector(".byline-component__contributors b span"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .format("ddd MMM D HH:mm:ss zz YYYY")
                .timezone("EST"),
        )
        .dek(Field::new().selector(".article__deck"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([".parsys.content", ".__image-lead__"])
                .selector(".content")
                .transform(".parsys.content", |node| {
                    let img_src = node
                        .select(".image.parbase.section")
                        .select(".picturefill")
                        .first()
                        .attr("data-platform-src");
                    if let Some(img_src) = img_src.filter(|s| !s.is_empty()) {
                        node.prepend_html(format!(
                            r#"<img class="__image-lead__" src="{img_src}"/>"#
                        ));
                    }
                    None
                })
                .clean([".pull-quote.pull-quote--large"]),
        )
}
