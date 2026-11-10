//! obamawhitehouse.archives.gov

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("obamawhitehouse.archives.gov")
        .supported_domains(["whitehouse.gov"])
        .title(Field::new().selectors(["h1", ".pane-node-title"]))
        .author(Field::new().selectors([".blog-author-link", ".node-person-name-link"]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selector(".field-name-field-forall-summary"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .default_cleaner(false)
                .selectors(["div#content-start", ".pane-node-field-forall-body"])
                .clean([".pane-node-title", ".pane-custom.pane-1"]),
        )
}
