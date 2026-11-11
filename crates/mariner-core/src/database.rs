//! SQLite storage: one database per account, plus `app.db` for the account
//! registry and app-wide preferences.
//!
//! Each database has one writer connection and a few read-only ones, each
//! owned by its own thread. Callers send work to them and await the
//! result, so SQLite never blocks an async executor, whichever one it is.

use std::path::Path;

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
        let writer_path = path.clone();
        let writer = Pool::open(1, move || connection::open_writer(&writer_path, kind)).await?;
        let readers = Pool::open(READERS, move || connection::open_reader(&path)).await?;
        Ok(Database { writer, readers })
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

    /// Closes every connection, running `PRAGMA optimize` on each first, and
    /// waits until they're closed. Dropping the database closes them too,
    /// without waiting.
    pub async fn close(self) {
        self.writer.close().await;
        self.readers.close().await;
    }
}
