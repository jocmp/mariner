//! balloon-juice.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("balloon-juice.com")
        .title(Field::new().selector("h1.entry-title"))
        .author(Field::new().selector(".entry-author-name"))
        .date_published(
            Field::new()
                .attr(r#"meta[property="article:published_time"]"#, "content")
                .attr(r#"meta[name="article:published_time"]"#, "value"),
        )
        .lead_image_url(
            Field::new()
                .attr(r#"meta[property="og:image"]"#, "content")
                .attr(r#"meta[name="og:image"]"#, "value"),
        )
        .content(
            Field::new()
                .selectors([".entry-content", "article"])
                .transform(r#"iframe[src*="embed.bsky.app"]"#, |node| {
                    node.add_class("mercury-parser-keep iframe-embed-bsky");
                    let parent = node.parent();
                    if parent.is(".bluesky-embed") {
                        parent.add_class("mercury-parser-keep");
                    }
                    None
                })
                .transform("blockquote.bluesky-embed[data-bluesky-uri]", |node| {
                    if let Some(uri) = node.attr("data-bluesky-uri").filter(|u| !u.is_empty()) {
                        // Convert at://did:plc:.../app.bsky.feed.post/... to embed URL
                        let embed_path = uri.replacen("at://", "", 1);
                        node.replace_with_html(format!(
                            r#"<iframe src="https://embed.bsky.app/embed/{embed_path}" class="mercury-parser-keep iframe-embed-bsky" width="100%" frameborder="0"></iframe>"#
                        ));
                    }
                    None
                })
                .clean([".shared-counts-wrap", ".entry-meta"]),
        )
}
