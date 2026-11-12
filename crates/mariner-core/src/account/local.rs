//! The delegate of an account that lives only on this device: it finds and
//! fetches feeds itself, and has nothing to tell a service about changes.

use mariner_feeds::fetch::Fetch;
use mariner_feeds::finder::{self, FoundFeed};

use chrono::{DateTime, Utc};

use super::{Account, AccountDelegate, Status};
use crate::model::{Feed, FeedChoice};
use crate::{Error, Result};

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
    async fn find_feeds(&self, _: &Account<F>, url: &str) -> Result<Vec<FeedChoice>> {
        let found = finder::find(&self.fetcher, url).await?;
        Ok(found.into_iter().map(choice).collect())
    }

    /// Fetches the feed at `url` and stores it. A page that leads to one
    /// feed subscribes to that feed.
    async fn subscribe(
        &self,
        account: &Account<F>,
        url: &str,
        title: Option<&str>,
    ) -> Result<Feed> {
        let mut found = finder::find(&self.fetcher, url).await?;
        let index = found
            .iter()
            .position(|feed| feed.url == url)
            .or((found.len() == 1).then_some(0))
            .ok_or_else(|| Error::NoFeed {
                url: url.to_string(),
            })?;
        account.store_feed(found.swap_remove(index), title).await
    }

    /// Fetches every feed that isn't backing off. Read articles that left
    /// their feed are deleted when stored before `cutoff`.
    async fn refresh(&self, account: &Account<F>, cutoff: Option<DateTime<Utc>>) -> Result<()> {
        let cutoff = cutoff.map(|cutoff| cutoff.timestamp());
        LocalAccountRefresher::new(&self.fetcher)
            .refresh(account, cutoff)
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
