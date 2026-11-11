//! Mariner's core logic, with no UI or FFI concerns: models, accounts (each
//! with its own SQLite database), refresh and sync.
//!
//! Core doesn't depend on an async runtime. Its async functions can be
//! awaited from any executor: blocking work runs on core's own threads,
//! and network access goes through a fetcher the caller provides.

pub mod account;
pub mod database;
mod error;
mod model;
mod queries;
mod sql;
mod time;

pub use account::Account;
pub use database::{Database, Kind};
pub use error::{Error, Result};
pub use model::{AddFeedResult, Article, ArticleContent, ArticleFilter, Feed, FeedChoice};
