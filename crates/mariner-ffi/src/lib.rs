//! Thin bindings layer that exposes mariner-core to Swift and Kotlin
//! through UniFFI.
//!
//! The apps bring their own HTTP stack (URLSession, OkHttp) by
//! implementing [`HttpClient`]; nothing here makes network requests itself.
//! Every async method runs on the caller's executor (Swift concurrency,
//! Kotlin coroutines): core has no runtime of its own.

mod account;
mod error;
mod http;
mod model;

pub use account::Account;
pub use error::MarinerError;
pub use http::{HttpClient, HttpError, HttpHeader, HttpRequest, HttpResponse};
pub use model::{
    AddFeedResult, Article, ArticleContent, ArticleFilter, ArticleScope, Feed, FeedChoice, Tag,
};

uniffi::setup_scaffolding!();
