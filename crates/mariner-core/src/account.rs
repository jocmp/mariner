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

use crate::model::{Article, ArticleContent, ArticleFilter, ArticleScope, Feed, FeedChoice, Tag};
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

    /// Every tag, by name.
    pub async fn tags(&self) -> Result<Vec<Tag>> {
        queries::tags(&self.database).await
    }

    /// A page of articles in `scope`, newest first.
    pub async fn articles(
        &self,
        filter: ArticleFilter,
        scope: &ArticleScope,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Article>> {
        queries::articles(&self.database, filter, scope, limit, offset).await
    }

    /// One article with its content.
    pub async fn article(&self, id: &str) -> Result<Option<ArticleContent>> {
        queries::article(&self.database, id).await
    }

    /// Unread articles per feed id; feeds with none are absent.
    pub async fn unread_counts(&self) -> Result<HashMap<String, usize>> {
        queries::unread_counts(&self.database).await
    }

    /// Marks every unread article in `scope` read.
    pub async fn mark_all_read(&self, scope: &ArticleScope) -> Result<()> {
        let ids = queries::unread_article_ids(&self.database, scope).await?;
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

    /// The feeds `url` leads to, to pick one to subscribe to. `url` can be
    /// a site or a feed, and may lack a scheme. Subscribes to nothing; an
    /// empty list means there's no feed there.
    pub async fn find_feeds(&self, url: &str) -> Result<Vec<FeedChoice>> {
        self.delegate.find_feeds(self, url).await
    }

    /// Subscribes to the feed at `url`, one of the choices from
    /// [`Self::find_feeds`], with an optional title. A feed already
    /// subscribed is returned as is. To put it in folders, follow with
    /// [`Self::update_feed`].
    pub async fn subscribe(&self, url: &str, title: Option<&str>) -> Result<Feed> {
        if let Some(feed) = ExistingFeed::find(&self.database, url).await? {
            return Ok(feed);
        }
        self.delegate.subscribe(self, url, title).await
    }

    /// Renames a feed and replaces its tags. A folder it leaves that no
    /// other feed is in is removed.
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

    /// Whether the feed's articles open in full text. This is a setting on
    /// this device, so the delegate isn't told.
    pub async fn set_sticky_full_content(&self, feed_id: &str, enabled: bool) -> Result<()> {
        feeds::set_sticky_full_content(&self.database, feed_id, enabled).await
    }

    /// Whether a tag's folder shows its feeds. This is a setting on this
    /// device, so the delegate isn't told.
    pub async fn set_tag_expanded(&self, name: &str, expanded: bool) -> Result<()> {
        feeds::set_tag_expanded(&self.database, name, expanded).await
    }

    /// Unsubscribes, deleting the feed's articles, starred ones included,
    /// and any of its folders that no other feed is in.
    pub async fn remove_feed(&self, feed: &Feed) -> Result<()> {
        self.delegate.remove_feed(self, feed).await?;
        feeds::remove(&self.database, &feed.id).await
    }

    /// Removes the tag from every feed.
    pub async fn remove_tag(&self, name: &str) -> Result<()> {
        self.delegate.remove_tag(self, name).await?;
        feeds::remove_tag(&self.database, name).await
    }

    /// Stores a feed the delegate subscribed to, with its current entries.
    /// One stored already (under the URL it redirected to) is returned as is.
    pub(crate) async fn store_feed(&self, found: FoundFeed, title: Option<&str>) -> Result<Feed> {
        if let Some(feed) = ExistingFeed::find(&self.database, &found.url).await? {
            return Ok(feed);
        }
        feeds::subscribe(&self.database, found, title).await
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

    async fn find_feeds(&self, account: &Account<F>, url: &str) -> Result<Vec<FeedChoice>> {
        match self {
            Delegate::Local(delegate) => delegate.find_feeds(account, url).await,
        }
    }

    async fn subscribe(
        &self,
        account: &Account<F>,
        url: &str,
        title: Option<&str>,
    ) -> Result<Feed> {
        match self {
            Delegate::Local(delegate) => delegate.subscribe(account, url, title).await,
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

    /// The feeds `url` leads to, without subscribing.
    fn find_feeds(
        &self,
        account: &Account<F>,
        url: &str,
    ) -> impl Future<Output = Result<Vec<FeedChoice>>> + Send;

    /// Subscribes to the feed at `url` (with the service, for a sync
    /// account), storing it through [`Account::store_feed`].
    fn subscribe(
        &self,
        account: &Account<F>,
        url: &str,
        title: Option<&str>,
    ) -> impl Future<Output = Result<Feed>> + Send;

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
