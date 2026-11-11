//! Opening and configuring connections.

use std::path::Path;

use rusqlite::{Connection, MAIN_DB, OpenFlags};

use super::{Error, Kind};

mod account {
    refinery::embed_migrations!("migrations/account");
}

mod app {
    refinery::embed_migrations!("migrations/app");
}

/// The writer: creates the file if needed, switches it to WAL (which
/// persists in the file), runs the kind's migrations and lets SQLite
/// refresh its query planner statistics.
pub(super) fn open_writer(path: &Path, kind: Kind) -> Result<Connection, Error> {
    let mut connection = Connection::open(path)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    configure(&connection)?;
    match kind {
        Kind::Account => account::migrations::runner().run(&mut connection)?,
        Kind::App => app::migrations::runner().run(&mut connection)?,
    };
    connection.execute_batch("PRAGMA optimize = 0x10002;")?;
    Ok(connection)
}

/// A read-only connection to a database the writer has already opened.
pub(super) fn open_reader(path: &Path) -> Result<Connection, Error> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags)?;
    configure(&connection)?;
    Ok(connection)
}

/// Runs `PRAGMA optimize` before the writer closes. (It may write, so read-only
/// connections skip it.)
pub(super) fn close(connection: Connection) {
    if !connection.is_readonly(MAIN_DB).unwrap_or(true) {
        let _ = connection.execute_batch("PRAGMA optimize;");
    }
}

/// Settings every connection needs.
fn configure(connection: &Connection) -> Result<(), Error> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -8000;",
    )?;
    Ok(())
}
