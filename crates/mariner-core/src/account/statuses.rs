//! Read and starred states, changed in one transaction per call.

use rusqlite::params;

use crate::database::Database;
use crate::{Result, time};

/// A change to an article's read or starred state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Read,
    Unread,
    Starred,
    Unstarred,
}

impl Status {
    /// The columns to set; `?2` is the current time.
    fn assignments(self) -> &'static str {
        match self {
            Status::Read => "read = 1, last_read_at = ?2",
            Status::Unread => "read = 0",
            Status::Starred => "starred = 1",
            Status::Unstarred => "starred = 0, last_unstarred_at = ?2",
        }
    }

    /// The same columns as values for a new row.
    fn columns(self) -> (&'static str, &'static str) {
        match self {
            Status::Read => ("read, last_read_at", "1, ?2"),
            Status::Unread => ("read", "0"),
            Status::Starred => ("starred", "1"),
            Status::Unstarred => ("starred, last_unstarred_at", "0, ?2"),
        }
    }
}

/// Sets `status` on each article that exists, creating its status row if
/// needed.
pub(super) async fn set(database: &Database, article_ids: &[String], status: Status) -> Result<()> {
    let article_ids = article_ids.to_vec();
    database
        .write(move |connection| {
            let now = time::now();
            let (columns, values) = status.columns();
            let sql = format!(
                "INSERT INTO article_statuses (article_id, {columns}, updated_at)
                 SELECT id, {values}, ?2 FROM articles WHERE id = ?1
                 ON CONFLICT (article_id) DO UPDATE SET {}, updated_at = ?2",
                status.assignments()
            );
            let transaction = connection.transaction()?;
            {
                let mut statement = transaction.prepare_cached(&sql)?;
                for id in &article_ids {
                    statement.execute(params![id, now])?;
                }
            }
            transaction.commit()
        })
        .await?;
    Ok(())
}
