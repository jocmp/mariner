//! Extractors, from `src/extractors/`.
//!
//! - [`custom`]: site-specific extractors, one module per domain.
//! - [`generic`]: heuristics used for every other site, and for any field a
//!   custom extractor doesn't define.
//! - [`root_extractor`]: combines the two.

#[cfg(feature = "fetch")]
pub mod collect_all_pages;
pub mod custom;
mod custom_extractor;
mod detect_by_html;
pub mod generic;
mod json;
pub mod root_extractor;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use dom_query::Document;
use url::Url;

pub use custom_extractor::{CustomExtractor, Field, Selector, Transform, TransformFn, ValueFn};
pub use detect_by_html::detect_by_html;
pub use json::ExtractorError;

/// Every built-in custom extractor, keyed by each domain it handles.
pub static BUILT_IN: LazyLock<HashMap<String, Arc<CustomExtractor>>> =
    LazyLock::new(|| index(custom::all()));

/// Map each extractor's domains to it. Later extractors win when two claim
/// the same domain.
pub fn index(
    extractors: impl IntoIterator<Item = CustomExtractor>,
) -> HashMap<String, Arc<CustomExtractor>> {
    let mut map = HashMap::new();
    for extractor in extractors {
        let extractor = Arc::new(extractor);
        for domain in extractor.domains() {
            map.insert(domain.to_string(), Arc::clone(&extractor));
        }
    }
    map
}

/// Choose the extractor for a page: by hostname, then by base domain (the
/// last two labels), then by markup. `None` means the generic extractor.
///
/// `added` extractors take precedence over the built-ins.
pub fn get_extractor(
    url: &Url,
    doc: &Document,
    added: &HashMap<String, Arc<CustomExtractor>>,
) -> Option<Arc<CustomExtractor>> {
    let hostname = url.host_str().unwrap_or_default();
    let labels: Vec<&str> = hostname.split('.').collect();
    let base_domain = labels[labels.len().saturating_sub(2)..].join(".");

    added
        .get(hostname)
        .or_else(|| added.get(&base_domain))
        .or_else(|| BUILT_IN.get(hostname))
        .or_else(|| BUILT_IN.get(&base_domain))
        .cloned()
        .or_else(|| detect_by_html(doc).map(Arc::new))
}
