//! www.cnet.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.cnet.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selectors(["span.author", "a.author"]))
        .date_published(
            Field::new()
                .selector("time")
                .timezone("America/Los_Angeles"),
        )
        .dek(Field::new().selectors([".c-head_dek", ".article-dek"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group(["img.__image-lead__", ".article-main-body"])
                .selector(".article-main-body")
                .transform("figure.image", |node| {
                    let img = node.select("img");
                    img.set_attr("width", "100%");
                    img.set_attr("height", "100%");
                    img.add_class("__image-lead__");
                    // cheerio's `.remove(selector)` removes the matched node itself only when it
                    // matches the selector.
                    node.filter(".imgContainer").remove();
                    node.prepend_selection(&img);
                    None
                }),
        )
}
