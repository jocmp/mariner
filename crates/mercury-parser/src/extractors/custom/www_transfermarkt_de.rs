//! www.transfermarkt.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.transfermarkt.de")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .date_published(
            Field::new()
                .selector(".news-header span:first-child")
                .format("DD.MM.YYYY - HH:mm")
                .timezone("Europe/Berlin"),
        )
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".news-content")
                .default_cleaner(false)
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    ".dachzeile",
                    ".news-header-social",
                    ".newsansicht-bildquelle",
                    ".news-widget--container",
                    ".pinpoll",
                    ".advertisment-button-container",
                    "tm-consent",
                ]),
        )
}
