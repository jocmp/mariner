//! szczecin.se.pl

use crate::extractors::CustomExtractor;

pub fn extractor() -> CustomExtractor {
    CustomExtractor {
        domain: "szczecin.se.pl".into(),
        ..super::www_se_pl::extractor()
    }
}
