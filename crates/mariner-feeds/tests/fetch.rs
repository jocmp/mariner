//! Following redirects for clients that make one request at a time.

use std::cell::RefCell;

use mariner_feeds::fetch::{FetchError, Redirect, Request, Response, follow_redirects};

fn response(url: &str, status: u16, location: Option<&str>, body: &str) -> Response {
    Response {
        url: url.to_string(),
        status,
        headers: location
            .map(|location| vec![("Location".to_string(), location.to_string())])
            .unwrap_or_default(),
        body: body.as_bytes().to_vec(),
        redirects: Vec::new(),
    }
}

#[tokio::test]
async fn follows_and_records_each_redirect() {
    let sent = RefCell::new(Vec::new());
    let request = Request::get("http://example.com/feed").header("If-None-Match", Some("\"v1\""));
    let result = follow_redirects(&request, |hop| {
        sent.borrow_mut().push(hop.clone());
        let response = match hop.url.as_str() {
            "http://example.com/feed" => {
                response(&hop.url, 301, Some("https://example.com/feed"), "")
            }
            "https://example.com/feed" => response(&hop.url, 302, Some("/feed.xml"), ""),
            _ => response(&hop.url, 200, None, "<rss/>"),
        };
        async move { Ok(response) }
    })
    .await
    .unwrap();

    assert_eq!(result.url, "https://example.com/feed.xml");
    assert_eq!(result.body, b"<rss/>");
    assert_eq!(
        result.redirects,
        [
            Redirect {
                status: 301,
                from: "http://example.com/feed".into(),
                to: "https://example.com/feed".into(),
            },
            Redirect {
                status: 302,
                from: "https://example.com/feed".into(),
                to: "https://example.com/feed.xml".into(),
            },
        ]
    );
    assert!(
        sent.borrow()
            .iter()
            .all(|hop| hop.headers == [("If-None-Match".to_string(), "\"v1\"".to_string())]),
        "every hop keeps the headers"
    );
}

#[tokio::test]
async fn gives_up_on_a_redirect_loop() {
    let request = Request::get("https://example.com/a");
    let result = follow_redirects(&request, |hop| {
        let next = if hop.url.ends_with("/a") { "/b" } else { "/a" };
        let response = response(&hop.url, 302, Some(next), "");
        async move { Ok(response) }
    })
    .await;
    assert!(matches!(result, Err(FetchError::Request(message)) if message.contains("redirects")));
}

#[tokio::test]
async fn a_redirect_without_a_location_is_the_response() {
    let request = Request::get("https://example.com/");
    let result = follow_redirects(&request, |hop| {
        let response = response(&hop.url, 304, None, "");
        async move { Ok(response) }
    })
    .await
    .unwrap();
    assert_eq!(result.status, 304);
    assert!(result.redirects.is_empty());
}
