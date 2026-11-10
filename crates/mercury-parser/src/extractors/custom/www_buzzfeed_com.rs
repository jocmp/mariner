//! www.buzzfeed.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.buzzfeed.com")
        .supported_domains(["www.buzzfeednews.com"])
        .title(Field::new().selector("h1.embed-headline-title"))
        .author(
            Field::new()
                .selectors([r#"a[data-action="user/username"]"#, "byline__author"])
                .attr(r#"meta[name="author"]"#, "value"),
        )
        .content(
            Field::new()
                .group([
                    r#"div[class^="featureimage_featureImageWrapper"]"#,
                    ".js-subbuzz-wrapper",
                ])
                .group([".js-subbuzz-wrapper"])
                .default_cleaner(false)
                .rename("h2", "b")
                .transform("div.longform_custom_header_media", |_| {
                    // Upstream checks `$node.has(...)`, which returns an always-truthy selection,
                    // so every match is converted.
                    Some("figure".to_string())
                })
                .rename(
                    "figure.longform_custom_header_media .longform_header_image_source",
                    "figcaption",
                )
                .clean([
                    ".instapaper_ignore",
                    ".suplist_list_hide .buzz_superlist_item .buzz_superlist_number_inline",
                    ".share-box",
                    ".print",
                    ".js-inline-share-bar",
                    ".js-ad-placement",
                ]),
        )
        .date_published(Field::new().attr("time[datetime]", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new().selector(".embed-headline-description"))
}
