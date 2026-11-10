//! www.apartmenttherapy.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.apartmenttherapy.com")
        .title(Field::new().selector("h1.headline"))
        .author(Field::new().selector(".PostByline__name"))
        .content(Field::new().selector("div.post__content").transform(
            r#"div[data-render-react-id="images/LazyPicture"]"#,
            |node| {
                let data: serde_json::Value =
                    serde_json::from_str(&node.attr("data-props")?).ok()?;
                let src = &data.get("sources")?.get(0)?["src"];
                let img = match src {
                    serde_json::Value::Null => "<img>".to_string(),
                    serde_json::Value::String(src) => {
                        format!(r#"<img src="{}">"#, escape_attr(src))
                    }
                    other => format!(r#"<img src="{}">"#, escape_attr(&other.to_string())),
                };
                node.replace_with_html(img);
                None
            },
        ))
        .date_published(Field::new().attr(".PostByline__timestamp[datetime]", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new())
        .next_page_url(Field::new())
        .excerpt(Field::new())
}

fn escape_attr(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}
