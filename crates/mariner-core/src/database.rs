//! SQLite storage: one database per account, plus `app.db` for the account
//! registry and app-wide preferences.
//!
//! Each database has one writer connection and a few read-only ones, each
//! owned by its own thread. Callers send work to them and await the
//! result, so SQLite never blocks an async executor, whichever one it is.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::Connection;

mod connection;
mod pool;

use pool::Pool;

/// Read-only connections per database, for UI queries.
const READERS: usize = 3;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration failed: {0}")]
    Migration(#[from] refinery::Error),
    #[error("couldn't create the database's directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("the database is closed")]
    Closed,
    #[error("a database task panicked")]
    Panicked,
}

/// Which schema a database file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One account's feeds, articles and statuses.
    Account,
    /// The account registry and app-wide preferences (`app.db`).
    App,
}

/// An open database: a writer and a pool of readers.
pub struct Database {
    writer: Pool,
    readers: Pool,
    /// Bumped when the writer refreshes the planner's statistics.
    statistics: Arc<AtomicU64>,
}

impl Database {
    /// Opens (creating it and any missing directories if needed) and
    /// migrates the database at `path`.
    pub async fn open(path: impl AsRef<Path>, kind: Kind) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let statistics = Arc::new(AtomicU64::new(0));
        let writer_path = path.clone();
        let writer = Pool::open(1, Arc::default(), move || {
            connection::open_writer(&writer_path, kind)
        })
        .await?;
        let readers = Pool::open(READERS, Arc::clone(&statistics), move || {
            connection::open_reader(&path)
        })
        .await?;
        Ok(Database {
            writer,
            readers,
            statistics,
        })
    }

    /// Runs `work` on the writer connection.
    pub async fn write<T, F>(&self, work: F) -> Result<T, Error>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        self.writer.run(work).await
    }

    /// Runs `work` on one of the read-only connections.
    pub async fn read<T, F>(&self, work: F) -> Result<T, Error>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        self.readers.run(|connection| work(connection)).await
    }

    /// Refreshes SQLite's statistics for any table that changed a lot, so
    /// the query planner picks good plans (the unread list, for one, is
    /// about 50x slower without them). Cheap when little has changed; worth
    /// running after big writes, like a refresh.
    pub async fn optimize(&self) -> Result<(), Error> {
        self.write(|connection| connection.execute_batch("PRAGMA optimize = 0x10002;"))
            .await?;
        self.statistics.fetch_add(1, Ordering::Release);
        Ok(())
    }

    /// Closes every connection, running `PRAGMA optimize` on each first, and
    /// waits until they're closed. Dropping the database closes them too,
    /// without waiting.
    pub async fn close(self) {
        self.writer.close().await;
        self.readers.close().await;
    }
}
