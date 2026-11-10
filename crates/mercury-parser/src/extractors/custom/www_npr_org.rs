//! www.npr.org

use dom_query::Selection;

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.npr.org")
        .title(Field::new().selectors(["h1", ".storytitle"]))
        .author(Field::new().selector("p.byline__name.byline__name--block"))
        .date_published(
            Field::new()
                .attr(".dateblock time[datetime]", "datetime")
                .attr(r#"meta[name="date"]"#, "value"),
        )
        .lead_image_url(
            Field::new()
                .attr(r#"meta[name="og:image"]"#, "value")
                .attr(r#"meta[name="twitter:image:src"]"#, "value"),
        )
        .content(
            Field::new()
                .selector(".storytext")
                .rename(".bucketwrap.image", "figure")
                .rename(".bucketwrap.image .credit-caption", "figcaption")
                // Brightspot's data-template replaces src during lazy-image conversion,
                // and the CDN answers its unexpanded tokens with a placeholder image.
                // Absolute links arrive percent-encoded, so match the escaped braces too.
                .transform(
                    r#"img[src*="{width}"], img[src*="{quality}"], img[src*="{format}"], img[src*="%7Bwidth%7D"], img[src*="%7Bquality%7D"], img[src*="%7Bformat%7D"]"#,
                    |node| {
                        expand_image_template(node);
                        None
                    },
                )
                .clean(["div.enlarge_measure"]),
        )
}

fn expand_image_template(node: &Selection<'_>) {
    let src = node.attr("src").unwrap_or_default();
    let format = node
        .attr("data-format")
        .filter(|format| !format.trim().is_empty());
    let format = format.as_deref().unwrap_or("webp");
    let src = src
        .replace("%7Bwidth%7D", "{width}")
        .replace("%7Bquality%7D", "{quality}")
        .replace("%7Bformat%7D", "{format}")
        .replace("{width}", "1200")
        .replace("{quality}", "85")
        .replace("{format}", format);

    node.set_attr("src", &src);
}
