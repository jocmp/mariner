//! An account that lives only on this device: it fetches feeds itself and
//! keeps everything in its own database.

use mariner_feeds::finder::{self, Fetch, FoundFeed};

use super::AccountDelegate;
use crate::Result;
use crate::database::Database;
use crate::model::{AddFeedResult, Feed, FeedChoice};

mod articles;
mod existing_feed;
mod feeds;
mod statuses;

use existing_feed::ExistingFeed;
use statuses::Status;

pub struct LocalAccount<F> {
    database: Database,
    fetcher: F,
}

impl<F> LocalAccount<F> {
    /// An account backed by `database` (opened as [`Kind::Account`]),
    /// fetching with `fetcher`.
    ///
    /// [`Kind::Account`]: crate::Kind::Account
    pub fn new(database: Database, fetcher: F) -> Self {
        LocalAccount { database, fetcher }
    }

    /// The account's database, for queries core doesn't offer yet.
    pub fn database(&self) -> &Database {
        &self.database
    }
}

impl<F: Fetch + Send + Sync> AccountDelegate for LocalAccount<F> {
    async fn mark_read(&self, article_ids: &[String]) -> Result<()> {
        statuses::set(&self.database, article_ids, Status::Read).await
    }

    async fn mark_unread(&self, article_ids: &[String]) -> Result<()> {
        statuses::set(&self.database, article_ids, Status::Unread).await
    }

    async fn add_star(&self, article_ids: &[String]) -> Result<()> {
        statuses::set(&self.database, article_ids, Status::Starred).await
    }

    async fn remove_star(&self, article_ids: &[String]) -> Result<()> {
        statuses::set(&self.database, article_ids, Status::Unstarred).await
    }

    /// Checks for an existing subscription first, then finds the feeds `url`
    /// leads to. One feed is subscribed to with its first entries; several
    /// are offered as choices.
    async fn add_feed(
        &self,
        url: &str,
        title: Option<&str>,
        tag_names: Option<&[String]>,
    ) -> Result<AddFeedResult> {
        if let Some(feed) = ExistingFeed::find(&self.database, url).await? {
            return Ok(AddFeedResult::Added(feed));
        }
        let mut found = finder::find(&self.fetcher, url).await?;
        match found.len() {
            0 => Ok(AddFeedResult::NotFound),
            1 => {
                let found = found.remove(0);
                if let Some(feed) = ExistingFeed::find(&self.database, &found.url).await? {
                    return Ok(AddFeedResult::Added(feed));
                }
                let tag_names = tag_names.unwrap_or_default().to_vec();
                let feed = feeds::subscribe(&self.database, found, title, tag_names).await?;
                Ok(AddFeedResult::Added(feed))
            }
            _ => Ok(AddFeedResult::Choices(
                found.into_iter().map(choice).collect(),
            )),
        }
    }

    async fn update_feed(&self, feed: &Feed, title: &str, tag_names: &[String]) -> Result<Feed> {
        feeds::update(&self.database, &feed.id, title, tag_names.to_vec()).await
    }

    async fn update_tag(&self, old_name: &str, new_name: &str) -> Result<()> {
        let (old_name, new_name) = (old_name.to_string(), new_name.to_string());
        self.database
            .write(move |connection| {
                let transaction = connection.transaction()?;
                // A feed that already has the new tag keeps one tagging.
                transaction.execute(
                    "UPDATE OR IGNORE taggings SET name = ?2, id = feed_id || ':' || ?2
                     WHERE name = ?1",
                    [&old_name, &new_name],
                )?;
                transaction.execute("DELETE FROM taggings WHERE name = ?1", [&old_name])?;
                transaction.execute(
                    "INSERT OR IGNORE INTO tags (name, expanded)
                     SELECT ?2, expanded FROM tags WHERE name = ?1",
                    [&old_name, &new_name],
                )?;
                transaction.execute("DELETE FROM tags WHERE name = ?1", [&old_name])?;
                transaction.commit()
            })
            .await?;
        Ok(())
    }

    async fn remove_feed(&self, feed: &Feed) -> Result<()> {
        let id = feed.id.clone();
        self.database
            .write(move |connection| {
                let transaction = connection.transaction()?;
                // Unsubscribing isn't pruning: statuses go too, so the
                // articles come back if the feed is added again.
                transaction.execute(
                    "DELETE FROM article_statuses
                     WHERE article_id IN (SELECT id FROM articles WHERE feed_id = ?1)",
                    [&id],
                )?;
                transaction.execute("DELETE FROM feeds WHERE id = ?1", [&id])?;
                transaction.commit()
            })
            .await?;
        Ok(())
    }

    async fn remove_tag(&self, name: &str) -> Result<()> {
        let name = name.to_string();
        self.database
            .write(move |connection| {
                let transaction = connection.transaction()?;
                transaction.execute("DELETE FROM taggings WHERE name = ?1", [&name])?;
                transaction.execute("DELETE FROM tags WHERE name = ?1", [&name])?;
                transaction.commit()
            })
            .await?;
        Ok(())
    }
}

fn choice(found: FoundFeed) -> FeedChoice {
    FeedChoice {
        url: found.url,
        title: found.feed.title,
    }
}
