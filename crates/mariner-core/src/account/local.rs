//! The delegate of an account that lives only on this device: it finds and
//! fetches feeds itself, and has nothing to tell a service about changes.

use mariner_feeds::fetch::Fetch;
use mariner_feeds::finder::{self, FoundFeed};

use chrono::{DateTime, Utc};

use super::{Account, AccountDelegate, Status};
use crate::Result;
use crate::model::{AddFeedResult, Feed, FeedChoice};

mod crawl_data;
mod redirect_cache;
mod refresher;

use refresher::LocalAccountRefresher;

pub(super) struct LocalAccountDelegate<F> {
    fetcher: F,
}

impl<F> LocalAccountDelegate<F> {
    pub fn new(fetcher: F) -> Self {
        LocalAccountDelegate { fetcher }
    }
}

impl<F: Fetch + Send + Sync> AccountDelegate<F> for LocalAccountDelegate<F> {
    /// Finds the feeds `url` leads to: one is subscribed to, several are
    /// offered as choices.
    async fn add_feed(
        &self,
        account: &Account<F>,
        url: &str,
        title: Option<&str>,
        tag_names: &[String],
    ) -> Result<AddFeedResult> {
        let mut found = finder::find(&self.fetcher, url).await?;
        match found.len() {
            0 => Ok(AddFeedResult::NotFound),
            1 => account.subscribe(found.remove(0), title, tag_names).await,
            _ => Ok(AddFeedResult::Choices(
                found.into_iter().map(choice).collect(),
            )),
        }
    }

    /// Fetches every feed that isn't backing off. Local articles start
    /// unread, so none would be pruned right away: `cutoff` isn't needed.
    async fn refresh(&self, account: &Account<F>, _cutoff: Option<DateTime<Utc>>) -> Result<()> {
        LocalAccountRefresher::new(&self.fetcher)
            .refresh(account)
            .await
    }

    async fn set_status(&self, _: &Account<F>, _: &[String], _: Status) -> Result<()> {
        Ok(())
    }

    async fn update_feed(&self, _: &Account<F>, _: &Feed, _: &str, _: &[String]) -> Result<()> {
        Ok(())
    }

    async fn update_tag(&self, _: &Account<F>, _: &str, _: &str) -> Result<()> {
        Ok(())
    }

    async fn remove_feed(&self, _: &Account<F>, _: &Feed) -> Result<()> {
        Ok(())
    }

    async fn remove_tag(&self, _: &Account<F>, _: &str) -> Result<()> {
        Ok(())
    }
}

fn choice(found: FoundFeed) -> FeedChoice {
    FeedChoice {
        url: found.url,
        title: found.feed.title,
    }
}
