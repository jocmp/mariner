//! www.lifehacker.jp

use std::sync::LazyLock;

use regex::Regex;

use crate::extractors::{CustomExtractor, Field};

static LEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^.*=%27").unwrap());
static TRAILING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%27;$").unwrap());

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.lifehacker.jp")
        .title(Field::new().selectors([
            r#"h1[class^="article_pArticle_Title"]"#,
            "h1.lh-summary-title",
        ]))
        .author(
            Field::new()
                .attr(r#"meta[name="author"]"#, "value")
                .selector("p.lh-entryDetailInner--credit"),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr("div.lh-entryDetail-header time", "datetime"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([
                    r#"div[class^="article_pArticle_Body__"]"#,
                    "div.lh-entryDetail-body",
                ])
                .transform("img.lazyload", |node| {
                    let src = node.attr("src")?;
                    let src = LEADING.replace(&src, "");
                    let src = TRAILING.replace(&src, "");
                    node.set_attr("src", &src);
                    None
                })
                .clean(["p.lh-entryDetailInner--credit"]),
        )
}
