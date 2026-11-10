//! gr.euronews.com

use crate::extractors::CustomExtractor;

pub fn extractor() -> CustomExtractor {
    CustomExtractor {
        domain: "gr.euronews.com".into(),
        ..super::www_euronews_com::extractor()
    }
}
