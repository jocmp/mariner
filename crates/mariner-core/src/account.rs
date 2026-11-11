//! Accounts: where feeds come from and where changes go. Each account has
//! its own database; how it fetches and syncs depends on its kind.

use mariner_feeds::finder::Fetch;

use crate::database::Database;
use crate::model::{AddFeedResult, Feed};
use crate::{Result, queries};

mod local;

pub use local::LocalAccount;

/// The changes an account knows how to make. Each kind of account
/// implements them its own way: a local account in its database alone, a
/// sync account on its service too.
pub trait AccountDelegate {
    fn mark_read(&self, article_ids: &[String]) -> impl Future<Output = Result<()>> + Send;

    fn mark_unread(&self, article_ids: &[String]) -> impl Future<Output = Result<()>> + Send;

    fn add_star(&self, article_ids: &[String]) -> impl Future<Output = Result<()>> + Send;

    fn remove_star(&self, article_ids: &[String]) -> impl Future<Output = Result<()>> + Send;

    /// Subscribes to the feed `url` leads to, with an optional title and
    /// tags.
    fn add_feed(
        &self,
        url: &str,
        title: Option<&str>,
        tag_names: Option<&[String]>,
    ) -> impl Future<Output = Result<AddFeedResult>> + Send;

    /// Renames a feed and replaces its tags.
    fn update_feed(
        &self,
        feed: &Feed,
        title: &str,
        tag_names: &[String],
    ) -> impl Future<Output = Result<Feed>> + Send;

    fn update_tag(&self, old_name: &str, new_name: &str)
    -> impl Future<Output = Result<()>> + Send;

    /// Unsubscribes, deleting the feed's articles, starred ones included.
    fn remove_feed(&self, feed: &Feed) -> impl Future<Output = Result<()>> + Send;

    /// Removes the tag from every feed.
    fn remove_tag(&self, name: &str) -> impl Future<Output = Result<()>> + Send;
}

/// An account of any kind. (Traits with async methods can't be used as
/// `dyn`, so this enum dispatches to each kind.)
pub enum Account<F> {
    Local(LocalAccount<F>),
}

impl<F: Fetch + Send + Sync> Account<F> {
    /// Every subscribed feed, by title.
    pub async fn feeds(&self) -> Result<Vec<Feed>> {
        queries::feeds(self.database()).await
    }

    /// The account's database, for queries core doesn't offer yet.
    pub fn database(&self) -> &Database {
        match self {
            Account::Local(account) => account.database(),
        }
    }
}

impl<F: Fetch + Send + Sync> AccountDelegate for Account<F> {
    async fn mark_read(&self, article_ids: &[String]) -> Result<()> {
        match self {
            Account::Local(account) => account.mark_read(article_ids).await,
        }
    }

    async fn mark_unread(&self, article_ids: &[String]) -> Result<()> {
        match self {
            Account::Local(account) => account.mark_unread(article_ids).await,
        }
    }

    async fn add_star(&self, article_ids: &[String]) -> Result<()> {
        match self {
            Account::Local(account) => account.add_star(article_ids).await,
        }
    }

    async fn remove_star(&self, article_ids: &[String]) -> Result<()> {
        match self {
            Account::Local(account) => account.remove_star(article_ids).await,
        }
    }

    async fn add_feed(
        &self,
        url: &str,
        title: Option<&str>,
        tag_names: Option<&[String]>,
    ) -> Result<AddFeedResult> {
        match self {
            Account::Local(account) => account.add_feed(url, title, tag_names).await,
        }
    }

    async fn update_feed(&self, feed: &Feed, title: &str, tag_names: &[String]) -> Result<Feed> {
        match self {
            Account::Local(account) => account.update_feed(feed, title, tag_names).await,
        }
    }

    async fn update_tag(&self, old_name: &str, new_name: &str) -> Result<()> {
        match self {
            Account::Local(account) => account.update_tag(old_name, new_name).await,
        }
    }

    async fn remove_feed(&self, feed: &Feed) -> Result<()> {
        match self {
            Account::Local(account) => account.remove_feed(feed).await,
        }
    }

    async fn remove_tag(&self, name: &str) -> Result<()> {
        match self {
            Account::Local(account) => account.remove_tag(name).await,
        }
    }
}
