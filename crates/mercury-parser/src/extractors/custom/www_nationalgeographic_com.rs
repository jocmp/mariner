//! www.nationalgeographic.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.nationalgeographic.com")
        .title(Field::new().selectors(["h1", "h1.main-title"]))
        .author(Field::new().selector(".byline-component__contributors b span"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selectors([".Article__Headline__Desc", ".article__deck"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("section.Article__Content")
                .group([".parsys.content", ".__image-lead__"])
                .selector(".content")
                .transform(".parsys.content", |node| {
                    let image_parent = node.children().first();
                    if image_parent.has_class("imageGroup") {
                        let data_attr_container = image_parent
                            .select(".media--medium__container")
                            .children()
                            .first();
                        let img_path1 = data_attr_container
                            .attr("data-platform-image1-path")
                            .filter(|s| !s.is_empty());
                        let img_path2 = data_attr_container
                            .attr("data-platform-image2-path")
                            .filter(|s| !s.is_empty());
                        if let (Some(img_path1), Some(img_path2)) = (img_path1, img_path2) {
                            node.prepend_html(format!(
                                "<div class=\"__image-lead__\">\n                \
                                 <img src=\"{img_path1}\"/>\n                \
                                 <img src=\"{img_path2}\"/>\n              </div>"
                            ));
                        }
                    } else {
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
                    }
                    None
                })
                .clean([".pull-quote.pull-quote--small"]),
        )
}
