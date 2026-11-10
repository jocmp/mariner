//! www.androidauthority.com

use dom_query::Selection;

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.androidauthority.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector("h1"),
        )
        .author(Field::new().selector("button.d_ic"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("main")
                .transform("div", |node| {
                    remove_affiliate_link(node);
                    None
                })
                .transform("p", |node| {
                    if node.text().starts_with("Published on") {
                        node.remove();
                    }

                    remove_affiliate_link(node);
                    None
                })
                .transform("ol", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h3", |node| {
                    if !remove_polls(node) {
                        node.set_attr("class", "mercury-parser-keep");
                    }
                    None
                })
                .clean([
                    "h1 + div",
                    "picture + div",
                    r#"div:has(> div > a[href*="andauth.co/AAGoogle"])"#,
                ]),
        )
}

fn remove_affiliate_link(node: &Selection<'_>) {
    if node
        .text()
        .starts_with("Affiliate links on Android Authority may earn us a commission.")
    {
        node.remove();
    }
}

fn remove_polls(node: &Selection<'_>) -> bool {
    let siblings = node.parent().children();

    if siblings.select("button:not(:has(picture))").exists() {
        node.parent().remove();

        return true;
    }

    false
}
