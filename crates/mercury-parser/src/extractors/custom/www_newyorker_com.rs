//! www.newyorker.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.newyorker.com")
        .title(
            Field::new()
                .selectors([
                    r#"h1[class^="content-header"]"#,
                    r#"h1[class^="ArticleHeader__hed"]"#,
                    r#"h1[class*="ContentHeaderHed"]"#,
                ])
                .attr(r#"meta[name="og:title"]"#, "value"),
        )
        .author(
            Field::new()
                .selector(r#"article header div[class^="BylinesWrapper"]"#)
                .attr(r#"meta[name="article:author"]"#, "value")
                .selectors([
                    r#"div[class^="ArticleContributors"] a[rel="author"]"#,
                    r#"article header div[class*="Byline__multipleContributors"]"#,
                ]),
        )
        .content(
            Field::new()
                .selectors([
                    ".article__body",
                    "article.article.main-content",
                    r#"main[class^="Layout__content"]"#,
                ])
                .rename(".caption__text", "figcaption")
                .rename(".caption__credit", "figcaption")
                .clean([r#"footer[class^="ArticleFooter__footer"]"#, "aside"]),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .selector("time.content-header__publish-date")
                .attr(r#"meta[name="pubdate"]"#, "value")
                .timezone("America/New_York"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(
            Field::new()
                .selectors([
                    r#"div[class^="ContentHeaderDek"]"#,
                    "div.content-header__dek",
                    r#"h2[class^="ArticleHeader__dek"]"#,
                ])
                .attr(r#"meta[name="description"]"#, "value"),
        )
}
