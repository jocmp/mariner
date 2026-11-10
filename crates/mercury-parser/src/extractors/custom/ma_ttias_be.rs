//! ma.ttias.be

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("ma.ttias.be")
        .title(Field::new().attr(r#"meta[name="twitter:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .content(
            Field::new()
                .group([".content"])
                .transform("h2", |node| {
                    // The "id" attribute values would result in low scores and the element being
                    // removed.
                    node.remove_attr("id");

                    // h1 elements will be demoted to h2, so demote h2 elements to h3.
                    Some("h3".to_string())
                })
                .transform("h1", |node| {
                    // The "id" attribute values would result in low scores and the element being
                    // removed.
                    node.remove_attr("id");

                    // A subsequent h2 will be removed if there is not a paragraph before it, so
                    // add a paragraph here. It will be removed anyway because it is empty.
                    node.after_html("<p></p>");
                    None
                })
                .transform("ul", |node| {
                    // Articles contain lists of links which look like, but are not, navigation
                    // elements. Adding this class attribute avoids them being incorrectly removed.
                    node.set_attr("class", "entry-content-asset");
                    None
                }),
        )
}
