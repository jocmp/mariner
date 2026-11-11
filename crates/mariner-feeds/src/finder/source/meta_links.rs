use super::{Source, document, join_url};
use crate::fetch::Response;

/// `<link rel="alternate">` tags with a feed type.
pub struct MetaLinks;

const TYPES: [&str; 4] = [
    "application/rss+xml",
    "application/atom+xml",
    "application/feed+json",
    "application/json",
];

impl Source for MetaLinks {
    fn options(response: &Response) -> Vec<String> {
        document(response)
            .select("link[rel~=alternate]")
            .iter()
            .filter_map(|link| {
                let kind = link.attr("type")?.trim().to_ascii_lowercase();
                let href = link.attr("href")?;
                TYPES
                    .contains(&kind.as_str())
                    .then(|| join_url(response, &href))
                    .flatten()
            })
            .collect()
    }
}
