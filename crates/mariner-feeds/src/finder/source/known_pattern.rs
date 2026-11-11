use std::sync::LazyLock;

use regex::Regex;

use super::{Source, document};
use crate::finder::Response;

/// Feeds at known URLs: YouTube channels, users and playlists, subreddits,
/// Vimeo users and GitHub discussions; else a YouTube page's channel id;
/// else a Mastodon profile's `.rss`.
pub struct KnownPattern;

static PATTERNS: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
    [
        (
            r"https://www\.youtube\.com/channel/([^/#?]*)",
            "https://www.youtube.com/feeds/videos.xml?channel_id=%s",
        ),
        (
            r"https://www\.youtube\.com/user/([^/#?]*)",
            "https://www.youtube.com/feeds/videos.xml?user=%s",
        ),
        (
            r"https://www\.youtube\.com/playlist\?list=([^&]*)",
            "https://www.youtube.com/feeds/videos.xml?playlist_id=%s",
        ),
        (
            r"https?://(?:www\.)?reddit\.com/r/([^/#?]*)",
            "https://www.reddit.com/r/%s.rss",
        ),
        (
            r"https://vimeo\.com/([^/#?]*)",
            "https://vimeo.com/%s/videos/rss",
        ),
        (
            r"https://github\.com/(orgs/[^/#?]+/discussions)",
            "https://github.com/%s.atom",
        ),
        (
            r"https://github\.com/([^/#?]+/[^/#?]+/discussions)",
            "https://github.com/%s.atom",
        ),
    ]
    .into_iter()
    .map(|(pattern, template)| (Regex::new(pattern).unwrap(), template))
    .collect()
});

impl Source for KnownPattern {
    fn options(response: &Response) -> Vec<String> {
        let url = &response.url;
        if let Some(feed) = PATTERNS.iter().find_map(|(pattern, template)| {
            let id = pattern.captures(url)?.get(1)?.as_str();
            (!id.is_empty()).then(|| template.replace("%s", id))
        }) {
            return vec![feed];
        }
        if url.starts_with("https://www.youtube.com")
            && let Some(id) = channel_id(response)
        {
            return vec![format!(
                "https://www.youtube.com/feeds/videos.xml?channel_id={id}"
            )];
        }
        if mastodon_server(response) {
            return vec![format!("{url}.rss")];
        }
        Vec::new()
    }
}

/// From `<meta itemprop="channelId">`, or else `"channelId":"…"` in the
/// page's scripts.
fn channel_id(response: &Response) -> Option<String> {
    static CHANNEL_ID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""channelId":"(.*?)""#).unwrap());
    let meta = document(response)
        .select("meta[itemprop='channelId']")
        .attr("content")
        .map(|id| id.to_string());
    meta.or_else(|| {
        let body = String::from_utf8_lossy(&response.body);
        CHANNEL_ID.captures(&body).map(|c| c[1].to_string())
    })
}

/// Any response header mentioning Mastodon.
fn mastodon_server(response: &Response) -> bool {
    response
        .headers
        .iter()
        .any(|(_, value)| value.to_ascii_lowercase().contains("mastodon"))
}
