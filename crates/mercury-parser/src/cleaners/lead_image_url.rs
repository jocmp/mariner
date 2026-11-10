//! `src/cleaners/lead-image-url.js`.

use url::Url;

use crate::utils::js;

/// Normalize an image URL, or `None` if it isn't a valid absolute URL.
pub fn clean_image(lead_image_url: &str) -> Option<String> {
    Url::parse(js::trim(lead_image_url)).ok().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_image_urls() {
        assert_eq!(
            clean_image("  https://example.com/a b.jpg "),
            Some("https://example.com/a%20b.jpg".to_string())
        );
        assert_eq!(clean_image("/relative.jpg"), None);
    }
}
