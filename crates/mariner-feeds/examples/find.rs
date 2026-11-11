//! Prints the feeds FeedFinder finds for a URL.
//!
//! `cargo run -p mariner-feeds --example find -- <url>`

use mariner_feeds::fetch::HttpFetcher;
use mariner_feeds::finder::find;

#[tokio::main]
async fn main() {
    let Some(url) = std::env::args().nth(1) else {
        eprintln!("usage: find <url>");
        std::process::exit(64);
    };
    let fetcher = HttpFetcher::new().expect("HTTP client");
    match find(&fetcher, &url).await {
        Ok(feeds) if feeds.is_empty() => println!("no feeds found"),
        Ok(feeds) => {
            for found in feeds {
                let title = found.feed.title.as_deref().unwrap_or("(no title)");
                println!(
                    "{}  {title}  ({} entries)",
                    found.url,
                    found.feed.entries.len()
                );
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
