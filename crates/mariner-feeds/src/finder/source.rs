//! The ways of finding feeds from a fetched response. Each source can be
//! used on its own; [`find`](super::find) tries them in order.

use url::Url;

use super::{Fetch, FoundFeed, Response};

mod body_links;
mod guess;
mod known_pattern;
mod meta_links;
mod xml;

pub use body_links::BodyLinks;
pub use guess::Guess;
pub use known_pattern::KnownPattern;
pub use meta_links::MetaLinks;
pub use xml::Xml;

/// One way of finding feeds from a response.
pub trait Source {
    /// The feed URLs this source would try, without fetching them.
    fn options(response: &Response) -> Vec<String>;

    /// The feeds this source finds: by default, each option that fetches
    /// and parses as a feed.
    fn find(fetcher: &impl Fetch, response: &Response) -> impl Future<Output = Vec<FoundFeed>> {
        fetch_feeds(fetcher, Self::options(response))
    }
}

/// Fetches each URL once, keeping those that parse as feeds (one per final
/// URL). Failures are skipped.
async fn fetch_feeds(fetcher: &impl Fetch, urls: Vec<String>) -> Vec<FoundFeed> {
    let mut found: Vec<FoundFeed> = Vec::new();
    let mut tried: Vec<String> = Vec::new();
    for url in urls {
        if tried.contains(&url) {
            continue;
        }
        tried.push(url.clone());
        let Ok(response) = fetcher.fetch(&url).await else {
            continue;
        };
        if !(200..300).contains(&response.status) {
            continue;
        }
        if let Some(feed) = response.feed()
            && !found.iter().any(|existing| existing.url == response.url)
        {
            found.push(FoundFeed {
                url: response.url,
                feed,
            });
        }
    }
    found
}

/// The response's body as an HTML document.
fn document(response: &Response) -> dom_query::Document {
    dom_query::Document::from(String::from_utf8_lossy(&response.body).as_ref())
}

/// `href` resolved against the response's URL.
fn join_url(response: &Response, href: &str) -> Option<String> {
    let base = Url::parse(&response.url).ok()?;
    base.join(href.trim()).ok().map(String::from)
}
