//! An existing subscription, found before fetching anything.

use crate::database::Database;
use crate::model::Feed;
use crate::{Result, queries};

pub(super) struct ExistingFeed;

impl ExistingFeed {
    /// The feed already subscribed at `url`.
    pub async fn find(database: &Database, url: &str) -> Result<Option<Feed>> {
        queries::feed_by_url(database, url).await
    }
}
