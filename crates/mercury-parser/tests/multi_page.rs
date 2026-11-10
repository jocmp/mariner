//! Multi-page collection against a local server.

#![cfg(feature = "fetch")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use mercury_parser::{CustomExtractor, Field, ParseOptions, Parser};

const PARAGRAPH: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";

fn page(body: &str, next: Option<&str>) -> String {
    let next = next.map_or_else(String::new, |href| {
        format!(r#"<a class="next" href="{href}">Next</a>"#)
    });
    format!(
        "<html><head><title>Paged</title></head><body><h1>Paged Article</h1><article><p>{body} {PARAGRAPH}</p></article>{next}</body></html>"
    )
}

// Serve `/2` and `/3` for as many requests as arrive; returns the base URL.
fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let next = format!("{base}/3");

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = [0; 2048];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            let body = if request.starts_with("GET /2 ") {
                page("Second page.", Some(&next))
            } else {
                page("Third page.", None)
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });

    base
}

#[tokio::test]
async fn collects_every_page() {
    let base = serve();
    let host = base
        .trim_start_matches("http://")
        .split(':')
        .next()
        .unwrap()
        .to_string();

    let mut parser = Parser::new();
    parser.add_extractor(
        CustomExtractor::new(host)
            .title(Field::new().selector("h1"))
            .content(Field::new().selector("article"))
            .next_page_url(Field::new().attr("a.next", "href")),
    );

    let html = page("First page.", Some(&format!("{base}/2")));
    let result = parser
        .parse(&format!("{base}/1"), ParseOptions::new().html(html))
        .await
        .unwrap();

    assert_eq!(result.title.as_deref(), Some("Paged Article"));
    assert_eq!(result.total_pages, 3);
    assert_eq!(result.rendered_pages, 3);
    let content = result.content.unwrap();
    assert!(content.contains("First page."));
    assert!(content.contains("<hr><h4>Page 2</h4>"));
    assert!(content.contains("Second page."));
    assert!(content.contains("<hr><h4>Page 3</h4>"));
    assert!(content.contains("Third page."));
}

#[tokio::test]
async fn stops_when_asked_not_to_fetch() {
    let result = Parser::new()
        .parse(
            "https://example.com/1",
            ParseOptions::new()
                .html(page("Only page.", Some("https://example.com/2")))
                .fetch_all_pages(false),
        )
        .await
        .unwrap();

    assert_eq!(result.total_pages, 1);
}

#[tokio::test]
async fn parse_futures_are_send() {
    let task = tokio::spawn(async {
        Parser::new()
            .parse(
                "https://example.com/1",
                ParseOptions::new().html(page("Spawned.", None)),
            )
            .await
    });
    assert!(task.await.unwrap().is_ok());
}
