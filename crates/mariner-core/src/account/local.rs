//! The delegate of an account that lives only on this device: it finds and
//! fetches feeds itself, and has nothing to tell a service about changes.

use futures_util::stream::{self, StreamExt};
use mariner_feeds::favicon;
use mariner_feeds::fetch::Fetch;
use mariner_feeds::finder::{self, FoundFeed};
use mariner_feeds::opml::Subscription;

use chrono::{DateTime, Utc};

use super::{Account, AccountDelegate, Status};
use crate::model::{Feed, FeedChoice, OpmlImport};
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

    /// Fetches the feed at `url` and stores it with its icon. A page that
    /// leads to one feed subscribes to that feed.
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
        let found = found.swap_remove(index);
        let own_icon = found
            .feed
            .icon
            .clone()
            .filter(|icon| icon.starts_with("http"));
        let mut feed = account.store_feed(found, title).await?;
        if feed.favicon_url.is_none() {
            let icon = match own_icon {
                Some(icon) => Some(icon),
                None => self.find_favicon(feed.site_url.as_deref()).await,
            };
            if let Some(icon) = icon {
                account.set_favicon(&feed.id, &icon).await?;
                feed.favicon_url = Some(icon);
            }
        }
        Ok(feed)
    }

    /// Stores the list's feeds as they are, NetNewsWire's way: nothing is
    /// fetched, so even a long list takes a moment.
    async fn import_opml(
        &self,
        account: &Account<F>,
        subscriptions: Vec<Subscription>,
    ) -> Result<OpmlImport> {
        account.store_subscriptions(subscriptions).await
    }

    /// Fetches every feed that isn't backing off. Read articles that left
    /// their feed are deleted when stored before `cutoff`.
    async fn refresh(&self, account: &Account<F>, cutoff: Option<DateTime<Utc>>) -> Result<()> {
        let cutoff = cutoff.map(|cutoff| cutoff.timestamp());
        LocalAccountRefresher::new(&self.fetcher)
            .refresh(account, cutoff)
            .await?;
        self.find_missing_favicons(account).await
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

/// Icons looked for at once.
const FAVICON_CONCURRENCY: usize = 6;

impl<F: Fetch + Send + Sync> LocalAccountDelegate<F> {
    async fn find_favicon(&self, site_url: Option<&str>) -> Option<String> {
        favicon::find(&self.fetcher, site_url?).await
    }

    /// Looks for the icons of feeds that have none yet, such as those
    /// subscribed before icons were looked for.
    async fn find_missing_favicons(&self, account: &Account<F>) -> Result<()> {
        let missing = account.feeds_without_favicons().await?;
        let found: Vec<(String, Option<String>)> = stream::iter(missing)
            .map(
                |(id, site_url)| async move { (id, favicon::find(&self.fetcher, &site_url).await) },
            )
            .buffer_unordered(FAVICON_CONCURRENCY)
            .collect()
            .await;
        for (id, icon) in found {
            if let Some(icon) = icon {
                account.set_favicon(&id, &icon).await?;
            }
        }
        Ok(())
    }
}

fn choice(found: FoundFeed) -> FeedChoice {
    FeedChoice {
        url: found.url,
        title: found.feed.title,
    }
}
