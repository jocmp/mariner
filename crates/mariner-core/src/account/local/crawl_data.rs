//! A feed's fetch state: conditional GET headers, the last body's
//! fingerprint, errors with back-off, and redirects.

use chrono::DateTime;
use mariner_feeds::fetch::{Redirect, Request};

use super::redirect_cache::RedirectCache;

/// The latest a `Retry-After` header can push a retry: 8 hours.
const RETRY_AFTER_LIMIT: i64 = 8 * 60 * 60;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CrawlData {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub last_fetched_at: Option<i64>,
    pub fetch_fingerprint: Option<String>,
    pub error_count: i64,
    pub failed_at: Option<i64>,
    pub last_error_kind: Option<String>,
    pub last_error_status: Option<i64>,
    pub retry_after: Option<i64>,
    pub redirect: RedirectCache,
}

/// Why a fetch failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ErrorKind {
    NotFound,
    ClientError,
    ServerError,
    Timeout,
    Network,
    NotAFeed,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::NotFound => "not_found",
            ErrorKind::ClientError => "client_error",
            ErrorKind::ServerError => "server_error",
            ErrorKind::Timeout => "timeout",
            ErrorKind::Network => "network",
            ErrorKind::NotAFeed => "not_a_feed",
        }
    }

    /// The kind for an unsuccessful HTTP status.
    pub fn for_status(status: u16) -> Self {
        match status {
            404 | 410 => ErrorKind::NotFound,
            500.. => ErrorKind::ServerError,
            _ => ErrorKind::ClientError,
        }
    }
}

impl CrawlData {
    /// A conditional GET for the feed subscribed at `feed_url`, from its
    /// trusted redirect if it has one.
    pub fn request(&self, feed_url: &str) -> Request {
        Request::get(self.redirect.url(feed_url))
            .header("If-None-Match", self.etag.as_deref())
            .header("If-Modified-Since", self.last_modified.as_deref())
    }

    pub fn log_download(&mut self, now: i64) {
        self.last_fetched_at = Some(now);
    }

    /// A fetch that got a response. Errors clear, except "not a feed": that
    /// only clears once a body parses.
    pub fn download_success(&mut self, redirects: &[Redirect]) {
        if self.last_error_kind.as_deref() != Some(ErrorKind::NotAFeed.as_str()) {
            self.clear();
        }
        self.redirect.save(redirects);
    }

    /// Records a failure and when to try again: after the server's
    /// `Retry-After` (at most 8 hours away) or the back-off, whichever is
    /// later.
    pub fn download_error(
        &mut self,
        kind: ErrorKind,
        status: Option<u16>,
        retry_after_header: Option<&str>,
        now: i64,
    ) {
        self.error_count += 1;
        self.failed_at = Some(now);
        self.last_error_kind = Some(kind.as_str().to_string());
        self.last_error_status = status.map(i64::from);
        let header = retry_after_header
            .and_then(|header| retry_after_time(header, now))
            .map(|time| time.min(now + RETRY_AFTER_LIMIT));
        let backoff = now + self.backoff();
        self.retry_after = Some(header.map_or(backoff, |header| header.max(backoff)));
    }

    /// Keeps a 200 response's validators and fingerprint for next time.
    pub fn save(&mut self, etag: Option<&str>, last_modified: Option<&str>, fingerprint: String) {
        self.etag = etag.map(String::from);
        self.last_modified = last_modified.map(String::from);
        self.fetch_fingerprint = Some(fingerprint);
    }

    /// Clears errors after a body parses.
    pub fn clear(&mut self) {
        self.error_count = 0;
        self.failed_at = None;
        self.last_error_kind = None;
        self.last_error_status = None;
        self.retry_after = None;
    }

    /// `clamp(errors, 8, 23)^4` seconds: about 68 minutes, growing to about
    /// 3.2 days.
    fn backoff(&self) -> i64 {
        self.error_count.clamp(8, 23).pow(4)
    }
}

/// A `Retry-After` header as a time: seconds from now, or an HTTP date.
fn retry_after_time(header: &str, now: i64) -> Option<i64> {
    let header = header.trim();
    if let Ok(seconds) = header.parse::<i64>() {
        return Some(now + seconds);
    }
    DateTime::parse_from_rfc2822(header)
        .ok()
        .map(|time| time.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_000;

    #[test]
    fn backs_off_with_each_error() {
        let mut crawl = CrawlData::default();
        crawl.download_error(ErrorKind::ServerError, Some(503), None, NOW);
        assert_eq!(crawl.retry_after, Some(NOW + 4096), "about 68 minutes");
        assert_eq!(crawl.last_error_kind.as_deref(), Some("server_error"));
        assert_eq!(crawl.last_error_status, Some(503));

        for _ in 0..30 {
            crawl.download_error(ErrorKind::Timeout, None, None, NOW);
        }
        assert_eq!(
            crawl.retry_after,
            Some(NOW + 23_i64.pow(4)),
            "about 3.2 days at most"
        );
        assert_eq!(crawl.last_error_status, None);
    }

    #[test]
    fn honors_retry_after_up_to_8_hours() {
        let mut crawl = CrawlData::default();
        crawl.download_error(ErrorKind::ClientError, Some(429), Some("7200"), NOW);
        assert_eq!(crawl.retry_after, Some(NOW + 7200));

        crawl.download_error(ErrorKind::ClientError, Some(429), Some("999999"), NOW);
        assert_eq!(crawl.retry_after, Some(NOW + RETRY_AFTER_LIMIT));

        crawl.download_error(
            ErrorKind::ClientError,
            Some(429),
            Some("Tue, 14 Nov 2023 22:43:20 GMT"),
            NOW,
        );
        assert_eq!(
            crawl.retry_after,
            Some(NOW + 4096),
            "the back-off when it's later"
        );

        crawl.download_error(ErrorKind::ClientError, Some(429), Some("soon"), NOW);
        assert_eq!(crawl.retry_after, Some(NOW + 4096));
    }

    #[test]
    fn success_clears_errors_except_not_a_feed() {
        let mut crawl = CrawlData::default();
        crawl.download_error(ErrorKind::Timeout, None, None, NOW);
        crawl.download_success(&[]);
        assert_eq!((crawl.error_count, crawl.retry_after), (0, None));

        crawl.download_error(ErrorKind::NotAFeed, Some(200), None, NOW);
        crawl.download_success(&[]);
        assert_eq!(crawl.last_error_kind.as_deref(), Some("not_a_feed"));
        crawl.clear();
        assert_eq!(crawl.last_error_kind, None);
    }

    #[test]
    fn requests_conditionally_from_a_trusted_redirect() {
        let crawl = CrawlData {
            etag: Some("\"abc\"".into()),
            last_modified: Some("Tue, 14 Nov 2023 22:13:20 GMT".into()),
            redirect: RedirectCache {
                redirected_to: Some("https://example.com/new".into()),
                redirect_count: super::super::redirect_cache::TRUST_AFTER,
            },
            ..CrawlData::default()
        };
        let request = crawl.request("https://example.com/old");
        assert_eq!(request.url, "https://example.com/new");
        assert_eq!(
            request.headers,
            [
                ("If-None-Match".to_string(), "\"abc\"".to_string()),
                (
                    "If-Modified-Since".to_string(),
                    "Tue, 14 Nov 2023 22:13:20 GMT".to_string()
                ),
            ]
        );
    }
}
