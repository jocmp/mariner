//! An account: where feeds come from and where changes go.
//!
//! `Account` owns the account's database and the work every kind of
//! account shares: storing feeds and articles, statuses, tags, and finding
//! an existing subscription. For what differs between kinds (finding
//! feeds, fetching them, telling a sync service about changes) it reaches
//! out to its delegate.

use chrono::{DateTime, Utc};
use mariner_feeds::fetch::Fetch;
use mariner_feeds::finder::FoundFeed;

use crate::database::Database;
use std::collections::HashMap;

use crate::model::{AddFeedResult, Article, ArticleContent, ArticleFilter, Feed};
use crate::{Result, queries, time};

mod articles;
mod existing_feed;
mod feeds;
mod local;
mod statuses;

use existing_feed::ExistingFeed;
use local::LocalAccountDelegate;
pub(crate) use statuses::Status;

pub struct Account<F> {
    database: Database,
    delegate: Delegate<F>,
}

impl<F> Account<F> {
    /// The account's database, for queries core doesn't offer yet.
    pub fn database(&self) -> &Database {
        &self.database
    }
}

impl<F: Fetch + Send + Sync> Account<F> {
    /// An account that lives only on this device, backed by `database`
    /// (opened as [`Kind::Account`]) and fetching with `fetcher`.
    ///
    /// [`Kind::Account`]: crate::Kind::Account
    pub fn local(database: Database, fetcher: F) -> Self {
        Account {
            database,
            delegate: Delegate::Local(LocalAccountDelegate::new(fetcher)),
        }
    }

    /// Every subscribed feed, by title.
    pub async fn feeds(&self) -> Result<Vec<Feed>> {
        queries::feeds(&self.database).await
    }

    /// Fetches new and changed articles, and deletes ones that left their
    /// feed: after 30 days, or sooner when read and stored before `cutoff`.
    /// Then forgets deleted articles' statuses once they're old enough not
    /// to be needed.
    pub async fn refresh(&self, cutoff: Option<DateTime<Utc>>) -> Result<()> {
        self.delegate.refresh(self, cutoff).await?;
        let before = time::now() - articles::TOMBSTONE_LIFETIME;
        articles::delete_old_tombstones(&self.database, before).await?;
        Ok(self.database.optimize().await?)
    }

    /// A page of articles, newest first, from one feed or all of them.
    pub async fn articles(
        &self,
        filter: ArticleFilter,
        feed_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Article>> {
        queries::articles(&self.database, filter, feed_id, limit, offset).await
    }

    /// One article with its content.
    pub async fn article(&self, id: &str) -> Result<Option<ArticleContent>> {
        queries::article(&self.database, id).await
    }

    /// Unread articles per feed id; feeds with none are absent.
    pub async fn unread_counts(&self) -> Result<HashMap<String, usize>> {
        queries::unread_counts(&self.database).await
    }

    /// Marks every unread article read, in one feed or all of them.
    pub async fn mark_all_read(&self, feed_id: Option<&str>) -> Result<()> {
        let ids = queries::unread_article_ids(&self.database, feed_id).await?;
        self.mark_read(&ids).await
    }

    pub async fn mark_read(&self, article_ids: &[String]) -> Result<()> {
        self.set_status(article_ids, Status::Read).await
    }

    pub async fn mark_unread(&self, article_ids: &[String]) -> Result<()> {
        self.set_status(article_ids, Status::Unread).await
    }

    pub async fn add_star(&self, article_ids: &[String]) -> Result<()> {
        self.set_status(article_ids, Status::Starred).await
    }

    pub async fn remove_star(&self, article_ids: &[String]) -> Result<()> {
        self.set_status(article_ids, Status::Unstarred).await
    }

    /// Subscribes to the feed `url` leads to, with an optional title and
    /// tags. A feed already subscribed at `url` is returned as is.
    pub async fn add_feed(
        &self,
        url: &str,
        title: Option<&str>,
        tag_names: Option<&[String]>,
    ) -> Result<AddFeedResult> {
        if let Some(feed) = ExistingFeed::find(&self.database, url).await? {
            return Ok(AddFeedResult::Added(feed));
        }
        let tag_names = tag_names.unwrap_or_default();
        self.delegate.add_feed(self, url, title, tag_names).await
    }

    /// Renames a feed and replaces its tags.
    pub async fn update_feed(
        &self,
        feed: &Feed,
        title: &str,
        tag_names: &[String],
    ) -> Result<Feed> {
        self.delegate
            .update_feed(self, feed, title, tag_names)
            .await?;
        feeds::update(&self.database, &feed.id, title, tag_names.to_vec()).await
    }

    /// Renames a tag on every feed, merging it into one of the same name.
    pub async fn update_tag(&self, old_name: &str, new_name: &str) -> Result<()> {
        self.delegate.update_tag(self, old_name, new_name).await?;
        feeds::rename_tag(&self.database, old_name, new_name).await
    }

    /// Unsubscribes, deleting the feed's articles, starred ones included.
    pub async fn remove_feed(&self, feed: &Feed) -> Result<()> {
        self.delegate.remove_feed(self, feed).await?;
        feeds::remove(&self.database, &feed.id).await
    }

    /// Removes the tag from every feed.
    pub async fn remove_tag(&self, name: &str) -> Result<()> {
        self.delegate.remove_tag(self, name).await?;
        feeds::remove_tag(&self.database, name).await
    }

    /// Stores a feed the delegate found, with its tags and current entries.
    pub(crate) async fn subscribe(
        &self,
        found: FoundFeed,
        title: Option<&str>,
        tag_names: &[String],
    ) -> Result<AddFeedResult> {
        if let Some(feed) = ExistingFeed::find(&self.database, &found.url).await? {
            return Ok(AddFeedResult::Added(feed));
        }
        let feed = feeds::subscribe(&self.database, found, title, tag_names.to_vec()).await?;
        Ok(AddFeedResult::Added(feed))
    }

    async fn set_status(&self, article_ids: &[String], status: Status) -> Result<()> {
        self.delegate.set_status(self, article_ids, status).await?;
        statuses::set(&self.database, article_ids, status).await
    }
}

/// Each kind of account's delegate. (Traits with async methods can't be
/// used as `dyn`, so this enum dispatches to each kind.)
enum Delegate<F> {
    Local(LocalAccountDelegate<F>),
}

impl<F: Fetch + Send + Sync> AccountDelegate<F> for Delegate<F> {
    async fn refresh(&self, account: &Account<F>, cutoff: Option<DateTime<Utc>>) -> Result<()> {
        match self {
            Delegate::Local(delegate) => delegate.refresh(account, cutoff).await,
        }
    }

    async fn add_feed(
        &self,
        account: &Account<F>,
        url: &str,
        title: Option<&str>,
        tag_names: &[String],
    ) -> Result<AddFeedResult> {
        match self {
            Delegate::Local(delegate) => delegate.add_feed(account, url, title, tag_names).await,
        }
    }

    async fn set_status(
        &self,
        account: &Account<F>,
        article_ids: &[String],
        status: Status,
    ) -> Result<()> {
        match self {
            Delegate::Local(delegate) => delegate.set_status(account, article_ids, status).await,
        }
    }

    async fn update_feed(
        &self,
        account: &Account<F>,
        feed: &Feed,
        title: &str,
        tag_names: &[String],
    ) -> Result<()> {
        match self {
            Delegate::Local(delegate) => {
                delegate.update_feed(account, feed, title, tag_names).await
            }
        }
    }

    async fn update_tag(&self, account: &Account<F>, old_name: &str, new_name: &str) -> Result<()> {
        match self {
            Delegate::Local(delegate) => delegate.update_tag(account, old_name, new_name).await,
        }
    }

    async fn remove_feed(&self, account: &Account<F>, feed: &Feed) -> Result<()> {
        match self {
            Delegate::Local(delegate) => delegate.remove_feed(account, feed).await,
        }
    }

    async fn remove_tag(&self, account: &Account<F>, name: &str) -> Result<()> {
        match self {
            Delegate::Local(delegate) => delegate.remove_tag(account, name).await,
        }
    }
}

/// What each kind of account does its own way. Every method gets the
/// account, for the work all accounts share. Changes run before the
/// account applies them locally, so a sync account can refuse one.
///
/// Status changes must work offline: a sync account queues them in its
/// outbox (`article_sync_statuses`) here and pushes the outbox at the
/// start of its next refresh, before pulling, so local changes win.
pub(crate) trait AccountDelegate<F> {
    /// Brings in new articles. `cutoff` is when the account will prune
    /// read articles to, so a sync account can skip storing ones it would
    /// prune right away.
    fn refresh(
        &self,
        account: &Account<F>,
        cutoff: Option<DateTime<Utc>>,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Finds what `url` leads to, subscribing through
    /// [`Account::subscribe`] when it's one feed.
    fn add_feed(
        &self,
        account: &Account<F>,
        url: &str,
        title: Option<&str>,
        tag_names: &[String],
    ) -> impl Future<Output = Result<AddFeedResult>> + Send;

    fn set_status(
        &self,
        account: &Account<F>,
        article_ids: &[String],
        status: Status,
    ) -> impl Future<Output = Result<()>> + Send;

    fn update_feed(
        &self,
        account: &Account<F>,
        feed: &Feed,
        title: &str,
        tag_names: &[String],
    ) -> impl Future<Output = Result<()>> + Send;

    fn update_tag(
        &self,
        account: &Account<F>,
        old_name: &str,
        new_name: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    fn remove_feed(
        &self,
        account: &Account<F>,
        feed: &Feed,
    ) -> impl Future<Output = Result<()>> + Send;

    fn remove_tag(
        &self,
        account: &Account<F>,
        name: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}
