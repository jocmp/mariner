use url::Url;

use super::Source;
use crate::fetch::Response;

/// `/rss` and `/feed` on the response's host.
pub struct Guess;

impl Source for Guess {
    fn options(response: &Response) -> Vec<String> {
        let Ok(url) = Url::parse(&response.url) else {
            return Vec::new();
        };
        let Some(host) = url.host_str() else {
            return Vec::new();
        };
        ["/rss", "/feed"]
            .iter()
            .map(|path| format!("{}://{host}{path}", url.scheme()))
            .collect()
    }
}
