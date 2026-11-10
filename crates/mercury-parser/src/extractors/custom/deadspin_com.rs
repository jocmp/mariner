//! deadspin.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("deadspin.com")
        .supported_domains([
            "jezebel.com",
            "lifehacker.com",
            "kotaku.com",
            "gizmodo.com",
            "jalopnik.com",
            "kinja.com",
            "avclub.com",
            "clickhole.com",
            "splinternews.com",
            "theonion.com",
            "theroot.com",
            "thetakeout.com",
            "theinventory.com",
        ])
        .title(Field::new().selectors(["header h1", "h1.headline"]))
        .author(Field::new().selectors([r#"a[data-ga*="Author"]"#, ".author"]))
        .content(
            Field::new()
                .selectors([".js_post-content", ".post-content", ".entry-content"])
                .transform(
                    r#"iframe.lazyload[data-recommend-id^="youtube://"]"#,
                    |node| {
                        let id = node.attr("id")?;
                        // Upstream interpolates `undefined` when the id has no `youtube-` part.
                        let youtube_id = id.split("youtube-").nth(1).unwrap_or("undefined");
                        node.set_attr(
                            "src",
                            &format!("https://www.youtube.com/embed/{youtube_id}"),
                        );
                        None
                    },
                )
                .clean([".magnifier", ".lightbox"]),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr("time.updated[datetime]", "datetime"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new())
        .next_page_url(Field::new())
        .excerpt(Field::new())
}
