//! www.gizmodo.jp

use std::sync::LazyLock;

use regex::Regex;

use crate::extractors::{CustomExtractor, Field};

static LEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^.*=%27").unwrap());
static TRAILING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%27;$").unwrap());

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.gizmodo.jp")
        .title(Field::new().selector("h1.p-post-title"))
        .author(Field::new().selector("li.p-post-AssistAuthor"))
        .date_published(Field::new().attr("li.p-post-AssistTime time", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("article.p-post")
                .transform("img.p-post-thumbnailImage", |node| {
                    let src = node.attr("src")?;
                    let src = LEADING.replace(&src, "");
                    let src = TRAILING.replace(&src, "");
                    node.set_attr("src", &src);
                    None
                })
                .clean(["h1.p-post-title", "ul.p-post-Assist"]),
        )
}
