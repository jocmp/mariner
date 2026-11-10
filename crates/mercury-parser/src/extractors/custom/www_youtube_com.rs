//! www.youtube.com

use dom_query::Selection;

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.youtube.com")
        .title(
            Field::new()
                .attr(r#"meta[name="title"]"#, "value")
                .selectors([".watch-title", "h1.watch-title-container"]),
        )
        .author(
            Field::new()
                .attr(r#"link[itemprop="name"]"#, "content")
                .selector(".yt-user-info"),
        )
        .date_published(Field::new().attr(r#"meta[itemProp="datePublished"]"#, "value"))
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .default_cleaner(false)
                .selectors([
                    "#player-container-outer",
                    "ytd-expandable-video-description-body-renderer #description",
                ])
                .group(["#player-api", "#description"])
                .transform("#player-api", |node| {
                    let root = Selection::from(node.nodes().first()?.tree.root());
                    let video_id = root.select(r#"meta[itemProp="videoId"]"#).attr("value");
                    let video_id = video_id.as_deref().unwrap_or("undefined");
                    node.set_html(format!(
                        "\n          <iframe src=\"https://www.youtube.com/embed/{video_id}\" \
                         frameborder=\"0\" allowfullscreen></iframe>"
                    ));
                    None
                })
                .transform("#player-container-outer", |node| {
                    let root = Selection::from(node.nodes().first()?.tree.root());
                    let video_id = root.select(r#"meta[itemProp="videoId"]"#).attr("value");
                    let video_id = video_id.as_deref().unwrap_or("undefined");
                    let description = root.select(r#"meta[itemProp="description"]"#).attr("value");
                    let description = description.as_deref().unwrap_or("undefined");
                    node.set_html(format!(
                        "\n        <iframe src=\"https://www.youtube.com/embed/{video_id}\" \
                         frameborder=\"0\" allowfullscreen></iframe>\n        \
                         <div><span>{description}</span></div>"
                    ));
                    None
                }),
        )
}
