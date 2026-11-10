//! URL helpers: `parse-url.js`, `resolve-url.js`, and `validate-url.js`.

use url::Url;

/// `new URL(url)`, or `None` when it doesn't parse.
pub fn parse_url(url: &str) -> Option<Url> {
    Url::parse(url).ok()
}

/// `new URL(url, base).href`, or `url` unchanged when resolution fails.
pub fn resolve_url(url: &str, base: &str) -> String {
    resolve_url_with(url, Url::parse(base).ok().as_ref())
}

/// [`resolve_url`] against an already-parsed base, for resolving many URLs
/// against the same one.
pub fn resolve_url_with(url: &str, base: Option<&Url>) -> String {
    base.map_or_else(|| Url::parse(url), |base| base.join(url))
        .or_else(|_| Url::parse(url))
        .map_or_else(|_| url.to_string(), String::from)
}

/// Extremely simple URL validation: it parsed and has a hostname.
pub fn validate_url(parsed: Option<&Url>) -> bool {
    parsed
        .and_then(Url::host_str)
        .is_some_and(|host| !host.is_empty())
}

/// The `host` of a URL as JS reports it: hostname plus any non-default port.
pub fn host(url: &Url) -> String {
    match (url.host_str(), url.port()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_urls() {
        assert_eq!(
            resolve_url("/a/b", "https://example.com/c/d"),
            "https://example.com/a/b"
        );
        assert_eq!(
            resolve_url("https://other.com/x", "not a url"),
            "https://other.com/x"
        );
        assert_eq!(resolve_url("foo", "not a url"), "foo");
    }

    #[test]
    fn validates_urls() {
        assert!(validate_url(parse_url("https://example.com").as_ref()));
        assert!(!validate_url(parse_url("mailto:me@example.com").as_ref()));
        assert!(!validate_url(None));
    }
}
