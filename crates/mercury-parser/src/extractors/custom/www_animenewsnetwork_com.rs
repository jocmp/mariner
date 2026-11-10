//! www.animenewsnetwork.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.animenewsnetwork.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .date_published(Field::new().attr("small time", "datetime"))
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".KonaBody")
                .transform("img", |node| {
                    if let Some(data_src) = node.attr("data-src").filter(|s| !s.is_empty()) {
                        let src = if data_src.starts_with('/') {
                            format!("https://www.animenewsnetwork.com{data_src}")
                        } else {
                            data_src.to_string()
                        };
                        node.set_attr("src", &src);
                        node.remove_attr("data-src");
                    }
                    None
                })
                .clean([".intro", "instaread-player"]),
        )
}
