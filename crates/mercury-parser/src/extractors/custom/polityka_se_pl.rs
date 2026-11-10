//! polityka.se.pl

use crate::extractors::CustomExtractor;

pub fn extractor() -> CustomExtractor {
    CustomExtractor {
        domain: "polityka.se.pl".into(),
        ..super::www_se_pl::extractor()
    }
}
