//! clinicaltrials.gov

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("clinicaltrials.gov")
        .title(Field::new().selector("h1.tr-solo_record"))
        .author(Field::new().selector("div#sponsor.tr-info-text"))
        .date_published(
            Field::new().selector(r#"div:has(> span.term[data-term="Last Update Posted"])"#),
        )
        .content(
            Field::new()
                .selector("div#tab-body")
                .clean([".usa-alert> img"]),
        )
}
