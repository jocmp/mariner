//! Extracting an article's full text from its page.

use common::Network;
use futures_executor::block_on;
use mariner_core::extract::full_content;

mod common;

const PAGE: &str = r#"<!doctype html>
<html><head><title>A Long Read</title></head>
<body>
  <nav><a href="/">Home</a> · <a href="/about">About</a></nav>
  <article>
    <h1>A Long Read</h1>
    <p>The first paragraph of the article has enough words in it to look like real prose to an extractor, which wants sentences, commas, and some length.</p>
    <p>A second paragraph continues the story, adding more detail so that the article body clearly outweighs the navigation and the footer around it.</p>
    <p>Finally, a third paragraph wraps things up with a conclusion, a few more clauses, and a closing thought for the reader to take away.</p>
  </article>
  <footer>Copyright Example. All rights reserved.</footer>
</body></html>"#;

#[test]
fn extracts_the_article_from_its_page() {
    block_on(async {
        let network = Network::default().with("https://example.com/long-read", PAGE);
        let content = full_content(&network, "https://example.com/long-read")
            .await
            .unwrap()
            .unwrap();
        assert!(
            content.contains("The first paragraph of the article"),
            "{content}"
        );
        assert!(content.contains("a closing thought"), "{content}");
        assert!(!content.contains("All rights reserved"), "{content}");
    });
}

#[test]
fn reports_a_page_that_fails() {
    block_on(async {
        let network = Network::default();
        let error = full_content(&network, "https://example.com/missing")
            .await
            .unwrap_err();
        assert!(
            matches!(error, mariner_core::Error::Status { status: 404, .. }),
            "{error}"
        );
    });
}
