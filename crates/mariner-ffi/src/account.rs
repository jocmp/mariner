//! An account, for the apps: core's [`mariner_core::Account`] fetching
//! through the app's [`HttpClient`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use mariner_core::{Database, Kind};

use crate::error::MarinerError;
use crate::http::{HttpClient, PlatformFetcher};
use crate::model::{
    AddFeedResult, Article, ArticleContent, ArticleFilter, ArticleScope, Feed, Tag,
};

type Result<T> = std::result::Result<T, MarinerError>;

#[derive(uniffi::Object)]
pub struct Account {
    account: mariner_core::Account<PlatformFetcher>,
    /// For full texts, which aren't the account's to fetch.
    fetcher: PlatformFetcher,
}

#[uniffi::export]
impl Account {
    /// A local account stored at `path`, which is created when missing,
    /// fetching with `http`.
    #[uniffi::constructor]
    pub async fn local(path: String, http: Arc<dyn HttpClient>) -> Result<Arc<Self>> {
        let database = Database::open(path, Kind::Account).await?;
        let fetcher = PlatformFetcher::new(http);
        Ok(Arc::new(Account {
            account: mariner_core::Account::local(database, fetcher.clone()),
            fetcher,
        }))
    }

    /// Every subscribed feed, by title.
    pub async fn feeds(&self) -> Result<Vec<Feed>> {
        Ok(into_all(self.account.feeds().await?))
    }

    /// Every tag, by name.
    pub async fn tags(&self) -> Result<Vec<Tag>> {
        Ok(into_all(self.account.tags().await?))
    }

    /// A page of articles in `scope`, newest first.
    pub async fn articles(
        &self,
        filter: ArticleFilter,
        scope: ArticleScope,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Article>> {
        let articles = self
            .account
            .articles(
                filter.into(),
                &scope.into(),
                limit as usize,
                offset as usize,
            )
            .await?;
        Ok(into_all(articles))
    }

    /// One article with its content.
    pub async fn article(&self, id: String) -> Result<Option<ArticleContent>> {
        Ok(self.account.article(&id).await?.map(Into::into))
    }

    /// Unread articles per feed id; feeds with none are absent.
    pub async fn unread_counts(&self) -> Result<HashMap<String, u64>> {
        let counts = self.account.unread_counts().await?;
        Ok(counts
            .into_iter()
            .map(|(id, count)| (id, count as u64))
            .collect())
    }

    /// Fetches new and changed articles, and deletes ones that left their
    /// feed: after 30 days, or sooner when read and stored before `cutoff`.
    pub async fn refresh(&self, cutoff: Option<SystemTime>) -> Result<()> {
        Ok(self.account.refresh(cutoff.map(Into::into)).await?)
    }

    /// Subscribes to the feed `url` leads to, with an optional title and
    /// tags.
    pub async fn add_feed(
        &self,
        url: String,
        title: Option<String>,
        tag_names: Option<Vec<String>>,
    ) -> Result<AddFeedResult> {
        let result = self
            .account
            .add_feed(&url, title.as_deref(), tag_names.as_deref())
            .await?;
        Ok(result.into())
    }

    /// Renames a feed and replaces its tags.
    pub async fn update_feed(
        &self,
        feed: Feed,
        title: String,
        tag_names: Vec<String>,
    ) -> Result<Feed> {
        let feed = self
            .account
            .update_feed(&feed.into(), &title, &tag_names)
            .await?;
        Ok(feed.into())
    }

    /// Unsubscribes, deleting the feed's articles, starred ones included.
    pub async fn remove_feed(&self, feed: Feed) -> Result<()> {
        Ok(self.account.remove_feed(&feed.into()).await?)
    }

    /// Renames a tag on every feed, merging it into one of the same name.
    pub async fn update_tag(&self, old_name: String, new_name: String) -> Result<()> {
        Ok(self.account.update_tag(&old_name, &new_name).await?)
    }

    /// Removes the tag from every feed.
    pub async fn remove_tag(&self, name: String) -> Result<()> {
        Ok(self.account.remove_tag(&name).await?)
    }

    pub async fn mark_read(&self, article_ids: Vec<String>) -> Result<()> {
        Ok(self.account.mark_read(&article_ids).await?)
    }

    pub async fn mark_unread(&self, article_ids: Vec<String>) -> Result<()> {
        Ok(self.account.mark_unread(&article_ids).await?)
    }

    pub async fn add_star(&self, article_ids: Vec<String>) -> Result<()> {
        Ok(self.account.add_star(&article_ids).await?)
    }

    pub async fn remove_star(&self, article_ids: Vec<String>) -> Result<()> {
        Ok(self.account.remove_star(&article_ids).await?)
    }

    /// Marks every unread article in `scope` read.
    pub async fn mark_all_read(&self, scope: ArticleScope) -> Result<()> {
        Ok(self.account.mark_all_read(&scope.into()).await?)
    }

    /// Whether the feed's articles open in full text.
    pub async fn set_sticky_full_content(&self, feed_id: String, enabled: bool) -> Result<()> {
        Ok(self
            .account
            .set_sticky_full_content(&feed_id, enabled)
            .await?)
    }

    /// Whether a tag's folder shows its feeds.
    pub async fn set_tag_expanded(&self, name: String, expanded: bool) -> Result<()> {
        Ok(self.account.set_tag_expanded(&name, expanded).await?)
    }

    /// The article at `url`, extracted from its page as HTML, or `None` when
    /// the page has no article to find.
    pub async fn full_content(&self, url: String) -> Result<Option<String>> {
        Ok(mariner_core::extract::full_content(&self.fetcher, &url).await?)
    }
}

fn into_all<T, U: From<T>>(items: Vec<T>) -> Vec<U> {
    items.into_iter().map(Into::into).collect()
}
