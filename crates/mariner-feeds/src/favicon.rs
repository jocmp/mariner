//! Finding a site's icon, to show beside its feed.

use url::Url;

use crate::fetch::{Fetch, Request, Response};

/// Icons smaller than this are passed over for a bigger one: icons show at
/// about 20 points, so 40 pixels and up on a 2x screen.
const PREFERRED_SIZE: u32 = 64;

/// The icon for the site at `site_url`: the best `<link rel="icon">` or
/// `apple-touch-icon` on its page, or else `/favicon.ico`. SVG icons are
/// skipped, as image views on phones don't draw them. `/favicon.ico` is on
/// the site the URL redirects to, so `http` sites that moved get `https`.
/// `None` only when `site_url` isn't a URL.
pub async fn find(fetcher: &impl Fetch, site_url: &str) -> Option<String> {
    let mut site = Url::parse(site_url).ok()?;
    if let Ok(response) = fetcher.fetch(&Request::get(site_url)).await {
        if response.is_success()
            && let Some(icon) = icon_on_page(&response)
        {
            return Some(icon);
        }
        // Where the site redirected to, often https.
        if let Ok(url) = Url::parse(&response.url) {
            site = url;
        }
    }
    site.join("/favicon.ico").ok().map(String::from)
}

/// The best icon a page links to, resolved against its URL: the smallest
/// one at least [`PREFERRED_SIZE`] wide, or else the largest.
pub fn icon_on_page(response: &Response) -> Option<String> {
    let base = Url::parse(&response.url).ok()?;
    let html = String::from_utf8_lossy(&response.body);
    let document = dom_query::Document::from(html.as_ref());
    let mut icons: Vec<(u32, String)> = document
        .select("link[rel][href]")
        .iter()
        .filter_map(|link| {
            let rel = link.attr("rel")?.to_ascii_lowercase();
            let touch = rel
                .split_whitespace()
                .any(|kind| kind.starts_with("apple-touch-icon"));
            if !touch && !rel.split_whitespace().any(|kind| kind == "icon") {
                return None;
            }
            let href = link.attr("href")?;
            let kind = link.attr("type").map(|kind| kind.to_ascii_lowercase());
            if kind.as_deref() == Some("image/svg+xml") || href.trim().ends_with(".svg") {
                return None;
            }
            let size = link
                .attr("sizes")
                .and_then(|sizes| largest_size(&sizes))
                .unwrap_or(if touch { 180 } else { 16 });
            Some((size, base.join(href.trim()).ok()?.to_string()))
        })
        .collect();
    icons.sort_by_key(|(size, _)| *size);
    let big_enough = icons.iter().find(|(size, _)| *size >= PREFERRED_SIZE);
    big_enough.or(icons.last()).map(|(_, url)| url.clone())
}

/// The widest of `sizes="16x16 32x32"`.
fn largest_size(sizes: &str) -> Option<u32> {
    sizes
        .split_whitespace()
        .filter_map(|size| size.to_ascii_lowercase().split('x').next()?.parse().ok())
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(html: &str) -> Response {
        Response {
            url: "https://example.com/blog/".into(),
            status: 200,
            body: html.as_bytes().to_vec(),
            ..Response::default()
        }
    }

    #[test]
    fn prefers_the_smallest_icon_big_enough() {
        let html = r#"<link rel="icon" href="/16.png" sizes="16x16">
            <link rel="icon" href="/96.png" sizes="96x96">
            <link rel="icon" href="/512.png" sizes="512x512">
            <link rel="icon" type="image/svg+xml" href="/icon.svg">"#;
        assert_eq!(
            icon_on_page(&page(html)).as_deref(),
            Some("https://example.com/96.png")
        );
    }

    #[test]
    fn takes_a_touch_icon_over_a_tiny_favicon() {
        let html = r#"<link rel="shortcut icon" href="favicon.png">
            <link rel="apple-touch-icon" href="touch.png">"#;
        assert_eq!(
            icon_on_page(&page(html)).as_deref(),
            Some("https://example.com/blog/touch.png")
        );
    }

    #[test]
    fn falls_back_to_the_largest_small_icon() {
        let html = r#"<link rel="icon" href="/16.png" sizes="16x16">
            <link rel="icon" href="/32.png" sizes="32x32">"#;
        assert_eq!(
            icon_on_page(&page(html)).as_deref(),
            Some("https://example.com/32.png")
        );
        assert_eq!(icon_on_page(&page("<p>No icons</p>")), None);
    }
}
