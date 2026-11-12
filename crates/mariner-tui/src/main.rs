//! Mariner's terminal UI: `mariner` opens the reader; subcommands add and
//! refresh feeds from the shell.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use mariner_core::{Account, Database, Kind};
use mariner_feeds::fetch::HttpFetcher;

mod app;
mod dialog;
mod halfblock;
mod images;
mod paths;
mod reader;
mod ui;

/// The account type the TUI uses: local, fetching over HTTP.
pub type LocalAccount = Account<HttpFetcher>;

#[derive(Parser)]
#[command(name = "mariner", about = "A feed reader", version)]
struct Cli {
    /// Where to keep data (default: the platform's data directory; or set
    /// MARINER_DATA_DIR)
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Subscribe to the feed a URL leads to
    Add {
        url: String,
        /// Use this title instead of the feed's
        #[arg(long)]
        title: Option<String>,
        /// Tag the feed (repeatable)
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// Fetch new articles for every feed
    Refresh,
    /// List subscribed feeds with their unread counts
    Feeds,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Runtime::new()?;
    let path = paths::local_account(&paths::data_dir(cli.data_dir));
    let account = runtime.block_on(async {
        let database = Database::open(&path, Kind::Account).await?;
        Ok::<_, Box<dyn std::error::Error>>(Account::local(database, HttpFetcher::new()?))
    })?;
    let account = Arc::new(account);

    match cli.command {
        None => app::run(&runtime, account),
        Some(Command::Add { url, title, tags }) => {
            let mut choices = runtime.block_on(account.find_feeds(&url))?;
            match choices.len() {
                0 => println!("No feed found at {url}"),
                1 => {
                    let choice = choices.remove(0);
                    let mut feed =
                        runtime.block_on(account.subscribe(&choice.url, title.as_deref()))?;
                    if !tags.is_empty() {
                        feed = runtime.block_on(account.update_feed(&feed, &feed.title, &tags))?;
                    }
                    println!("Added {} ({})", feed.title, feed.feed_url);
                }
                _ => {
                    println!("{url} has several feeds; add one of these:");
                    for choice in choices {
                        let title = choice.title.unwrap_or_default();
                        println!("  {}  {title}", choice.url);
                    }
                }
            }
            Ok(())
        }
        Some(Command::Refresh) => {
            runtime.block_on(account.refresh(None))?;
            let unread: usize = runtime.block_on(account.unread_counts())?.values().sum();
            println!("Refreshed; {unread} unread");
            Ok(())
        }
        Some(Command::Feeds) => {
            let feeds = runtime.block_on(account.feeds())?;
            let counts = runtime.block_on(account.unread_counts())?;
            for feed in feeds {
                let unread = counts.get(&feed.id).copied().unwrap_or(0);
                println!("{unread:>5}  {}  {}", feed.title, feed.feed_url);
            }
            Ok(())
        }
    }
}
