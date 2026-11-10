//! superseriale.se.pl

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("superseriale.se.pl")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(".article_author:first-of-type"))
        .date_published(Field::new().selector("#timezone").timezone("Europe/Warsaw"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("article")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    "#timezone",
                    ".author",
                    ".article__author__croppimg",
                    ".related_articles__elements",
                    ".gl_plugin.socials",
                    ".gl_plugin.player",
                    ".gl_plugin.video_player",
                    ".gl_plugin + video",
                ]),
        )
}
