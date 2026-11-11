//! The delegate of an account that lives only on this device: it finds and
//! fetches feeds itself, and has nothing to tell a service about changes.

use mariner_feeds::finder::{self, Fetch, FoundFeed};

use super::{Account, AccountDelegate, Status};
use crate::Result;
use crate::model::{AddFeedResult, Feed, FeedChoice};

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
