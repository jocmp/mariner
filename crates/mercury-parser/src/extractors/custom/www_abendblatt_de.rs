//! www.abendblatt.de

use dom_query::Selection;

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.abendblatt.de")
        .title(Field::new().selector("h2.article__header__headline"))
        .author(Field::new().selector("span.author-info__name-text"))
        .date_published(
            Field::new()
                .attr("time.teaser-stream-time", "datetime")
                .attr("time.article__header__date", "datetime"),
        )
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.article__body")
                .transform("p", deobfuscate)
                .transform("div", deobfuscate),
        )
}

fn deobfuscate(node: &Selection<'_>) -> Option<String> {
    if !node.has_class("obfuscated") {
        return None;
    }
    let mut o: Vec<u16> = Vec::new();
    for r in node.text().encode_utf16() {
        match r {
            177 => o.push(u16::from(b'%')),
            178 => o.push(u16::from(b'!')),
            180 => o.push(u16::from(b';')),
            181 => o.push(u16::from(b'=')),
            32 => o.push(u16::from(b' ')),
            10 => o.push(u16::from(b'\n')),
            r if r > 33 => o.push(r - 1),
            _ => {}
        }
    }

    node.set_html(String::from_utf16_lossy(&o));
    node.remove_class("obfuscated");
    node.add_class("deobfuscated");
    None
}
