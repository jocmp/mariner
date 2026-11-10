//! www.videogameschronicle.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.videogameschronicle.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(r#".author-byline a[rel="author"]"#))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .content(
            Field::new()
                .selectors([".post__content-body", "article"])
                .transform("figure a", |node| {
                    let href = node.attr("href").filter(|s| !s.is_empty());
                    let img = node.select("img");
                    if let Some(href) = href
                        && img.exists()
                        && img.attr("src").is_none_or(|s| s.is_empty())
                    {
                        img.set_attr("src", &href);
                        node.replace_with_selection(&img);
                    }
                    None
                })
                .clean([".adcontainer"]),
        )
}
