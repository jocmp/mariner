//! polskisamorzad.se.pl

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("polskisamorzad.se.pl")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(
            Field::new()
                .selectors([".article_author:first-of-type", ".article-author"])
                .attr(r#"meta[name="og:article:author"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".article-single")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    "#timezone",
                    ".author",
                    ".article__author__croppimg",
                    ".article_authors_with_thumbnail",
                    ".related_articles__elements",
                    ".gl_plugin.socials",
                    ".gl_plugin.player",
                    ".gl_plugin.video_player",
                    ".gl_plugin + video",
                ]),
        )
}
