//! Opening, migrating and using databases, driven by a minimal executor
//! (no async runtime) to show core needs none.

use futures_executor::block_on;
use mariner_core::database::Error;
use mariner_core::{Database, Kind};
use rusqlite::params;

const ACCOUNT_TABLES: [&str; 12] = [
    "account_preferences",
    "article_notifications",
    "article_statuses",
    "article_sync_statuses",
    "articles",
    "enclosures",
    "excluded_statuses",
    "feeds",
    "saved_search_articles",
    "saved_searches",
    "taggings",
    "tags",
];

#[test]
fn creates_the_account_schema() {
    block_on(async {
        let (_dir, database) = account().await;
        let tables = database
            .read(|connection| {
                let mut statement = connection.prepare(
                    "SELECT name FROM sqlite_master
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                       AND name != 'refinery_schema_history'
                     ORDER BY name",
                )?;
                statement
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
            .unwrap();
        assert_eq!(tables, ACCOUNT_TABLES);
    });
}

#[test]
fn creates_the_app_schema() {
    block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::open(dir.path().join("app.db"), Kind::App)
            .await
            .unwrap();
        let count: i64 = database
            .read(|connection| {
                connection.query_row(
                    "SELECT count(*) FROM sqlite_master
                     WHERE type = 'table' AND name IN ('accounts', 'preferences')",
                    [],
                    |row| row.get(0),
                )
            })
            .await
            .unwrap();
        assert_eq!(count, 2);
    });
}

#[test]
fn configures_every_connection() {
    block_on(async {
        let (_dir, database) = account().await;
        let writer = database
            .write(|connection| settings(connection))
            .await
            .unwrap();
        let reader = database.read(settings).await.unwrap();
        let expected = Settings {
            journal_mode: "wal".into(),
            foreign_keys: 1,
            synchronous: 1,
            busy_timeout: 5000,
            temp_store: 2,
            cache_size: -8000,
        };
        assert_eq!(writer, expected);
        assert_eq!(reader, expected);
    });
}

#[test]
fn readers_cannot_write() {
    block_on(async {
        let (_dir, database) = account().await;
        let result = database
            .read(|connection| connection.execute("INSERT INTO tags (name) VALUES ('nope')", []))
            .await;
        assert!(matches!(result, Err(Error::Sqlite(_))));
    });
}

#[test]
fn deleting_a_feed_cascades_but_keeps_statuses() {
    block_on(async {
        let (_dir, database) = account().await;
        database
            .write(|connection| {
                let transaction = connection.transaction()?;
                transaction.execute(
                    "INSERT INTO feeds (id, subscription_id, title, feed_url)
                     VALUES ('f1', 's1', 'Feed', 'https://example.com/feed.xml')",
                    [],
                )?;
                transaction.execute(
                    "INSERT INTO articles (id, feed_id, published_at, created_at)
                     VALUES ('a1', 'f1', 1, 1)",
                    [],
                )?;
                transaction.execute(
                    "INSERT INTO enclosures (article_id, url, type)
                     VALUES ('a1', 'https://example.com/a.mp3', 'audio/mpeg')",
                    [],
                )?;
                transaction.execute(
                    "INSERT INTO taggings (id, feed_id, name) VALUES ('t1', 'f1', 'news')",
                    [],
                )?;
                transaction.execute(
                    "INSERT INTO article_statuses (article_id, starred, updated_at)
                     VALUES ('a1', 1, 1)",
                    [],
                )?;
                transaction.execute("DELETE FROM feeds WHERE id = 'f1'", [])?;
                transaction.commit()
            })
            .await
            .unwrap();

        let counts = database
            .read(|connection| {
                ["articles", "enclosures", "taggings", "article_statuses"]
                    .iter()
                    .map(|table| {
                        connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                            row.get::<_, i64>(0)
                        })
                    })
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
            .unwrap();
        assert_eq!(counts, [0, 0, 0, 1]);
    });
}

#[test]
fn enforces_foreign_keys() {
    block_on(async {
        let (_dir, database) = account().await;
        let result = database
            .write(|connection| {
                connection.execute(
                    "INSERT INTO articles (id, feed_id, published_at, created_at)
                     VALUES ('a1', 'missing', 1, 1)",
                    [],
                )
            })
            .await;
        assert!(matches!(result, Err(Error::Sqlite(_))));
    });
}

#[test]
fn reopening_keeps_data_and_migrates_once() {
    block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts").join("local.db");
        let database = Database::open(&path, Kind::Account).await.unwrap();
        database
            .write(|connection| {
                connection.execute("INSERT INTO tags (name) VALUES (?1)", params!["news"])
            })
            .await
            .unwrap();
        database.close().await;

        let database = Database::open(&path, Kind::Account).await.unwrap();
        let (tags, migrations): (i64, i64) = database
            .read(|connection| {
                Ok((
                    connection.query_row("SELECT count(*) FROM tags", [], |row| row.get(0))?,
                    connection.query_row(
                        "SELECT count(*) FROM refinery_schema_history",
                        [],
                        |row| row.get(0),
                    )?,
                ))
            })
            .await
            .unwrap();
        assert_eq!((tags, migrations), (1, 1));
    });
}

#[test]
fn survives_a_panicking_task() {
    block_on(async {
        let (_dir, database) = account().await;
        let result = database
            .write(|_| -> rusqlite::Result<()> { panic!("bad task") })
            .await;
        assert!(matches!(result, Err(Error::Panicked)));
        let one: i64 = database
            .write(|connection| connection.query_row("SELECT 1", [], |row| row.get(0)))
            .await
            .unwrap();
        assert_eq!(one, 1);
    });
}

#[test]
fn futures_can_move_between_threads() {
    fn assert_send<T: Send>(_: T) {}
    let dir = tempfile::tempdir().unwrap();
    assert_send(Database::open(dir.path().join("a.db"), Kind::Account));
    let database = block_on(Database::open(dir.path().join("b.db"), Kind::Account)).unwrap();
    assert_send(database.read(|_| Ok(())));
    assert_send(database.write(|_| Ok(())));
}

async fn account() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let database = Database::open(dir.path().join("local.db"), Kind::Account)
        .await
        .unwrap();
    (dir, database)
}

#[derive(Debug, PartialEq)]
struct Settings {
    journal_mode: String,
    foreign_keys: i64,
    synchronous: i64,
    busy_timeout: i64,
    temp_store: i64,
    cache_size: i64,
}

fn settings(connection: &rusqlite::Connection) -> rusqlite::Result<Settings> {
    let pragma = |name: &str| {
        connection.query_row(&format!("PRAGMA {name}"), [], |row| row.get::<_, i64>(0))
    };
    Ok(Settings {
        journal_mode: connection.query_row("PRAGMA journal_mode", [], |row| row.get(0))?,
        foreign_keys: pragma("foreign_keys")?,
        synchronous: pragma("synchronous")?,
        busy_timeout: pragma("busy_timeout")?,
        temp_store: pragma("temp_store")?,
        cache_size: pragma("cache_size")?,
    })
}
