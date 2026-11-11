use super::Source;
use crate::fetch::{Fetch, Response};
use crate::finder::FoundFeed;

/// The response itself, when it's a feed.
pub struct Xml;

impl Source for Xml {
    fn options(_response: &Response) -> Vec<String> {
        Vec::new()
    }

    fn find(_fetcher: &impl Fetch, response: &Response) -> impl Future<Output = Vec<FoundFeed>> {
        let found = response
            .feed()
            .map(|feed| FoundFeed {
                url: response.url.clone(),
                feed,
            })
            .into_iter()
            .collect();
        std::future::ready(found)
    }
}
