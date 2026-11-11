//! Read and starred states, changed in one transaction per call.

use rusqlite::params;

use crate::database::Database;
use crate::sql::account::statuses;
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
    fn sql(self) -> &'static str {
        match self {
            Status::Read => statuses::MARK_READ,
            Status::Unread => statuses::MARK_UNREAD,
            Status::Starred => statuses::ADD_STAR,
            Status::Unstarred => statuses::REMOVE_STAR,
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
            let transaction = connection.transaction()?;
            {
                let mut statement = transaction.prepare_cached(status.sql())?;
                for id in &article_ids {
                    statement.execute(params![id, now])?;
                }
            }
            transaction.commit()
        })
        .await?;
    Ok(())
}
