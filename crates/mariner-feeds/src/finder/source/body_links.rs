use super::{Source, document, join_url};
use crate::finder::Response;

/// The first four `<a>` links whose URL mentions `feed`, `xml`, `rss` or
/// `atom`.
pub struct BodyLinks;

const HINTS: [&str; 4] = ["feed", "xml", "rss", "atom"];

impl Source for BodyLinks {
    fn options(response: &Response) -> Vec<String> {
        document(response)
            .select("a[href]")
            .iter()
            .filter_map(|anchor| {
                let href = anchor.attr("href")?;
                HINTS
                    .iter()
                    .any(|hint| href.contains(hint))
                    .then(|| join_url(response, &href))
                    .flatten()
            })
            .take(4)
            .collect()
    }
}
