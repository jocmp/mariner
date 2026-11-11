//! Mariner's core logic, with no UI or FFI concerns: models, accounts (each
//! with its own SQLite database), refresh and sync.
//!
//! Core doesn't depend on an async runtime. Its async functions can be
//! awaited from any executor: blocking work runs on core's own threads,
//! and network access goes through a fetcher the caller provides.

pub mod database;

pub use database::{Database, Error, Kind};
