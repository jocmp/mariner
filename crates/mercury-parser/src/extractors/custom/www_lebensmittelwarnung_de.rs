//! www.lebensmittelwarnung.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.lebensmittelwarnung.de")
        .title(Field::new().selectors([".lmw-intro__heading", "title"]))
        .date_published(
            Field::new()
                .attr(".lmw-intro__meta > time", "datetime")
                .format("DD.MM.YYYY")
                .timezone("Europe/Berlin"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("main")
                .transform("h2", |node| {
                    let button = node.select("button");

                    if button.exists() {
                        node.select(".lmw-section__toggle-icon").remove();
                        node.set_text(button.text().trim());
                    }

                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("ul", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform(".lmw-bodytext", |node| {
                    // Kontakt Information
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform(".lmw-description-list__item", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                }),
        )
}
