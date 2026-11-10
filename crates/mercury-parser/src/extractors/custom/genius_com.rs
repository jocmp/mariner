//! genius.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("genius.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector("h2 a"))
        .date_published(
            Field::new().attr_with("meta[itemprop=page_data]", "value", |value| {
                song_data(value, &["song", "release_date"])
            }),
        )
        .dek(Field::new())
        .lead_image_url(
            Field::new().attr_with("meta[itemprop=page_data]", "value", |value| {
                song_data(value, &["song", "album", "cover_art_url"])
            }),
        )
        .content(Field::new().selector(".lyrics"))
}

/// Reads a string at `path` from the page's JSON data, or an empty string
/// where upstream would throw or return a non-string.
fn song_data(json: &str, path: &[&str]) -> String {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json) else {
        return String::new();
    };
    path.iter()
        .try_fold(&json, |value, key| value.get(key))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}
