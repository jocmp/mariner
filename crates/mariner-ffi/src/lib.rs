//! Thin bindings layer that exposes mariner-core and mariner-condense to
//! Swift and Kotlin through UniFFI.
//!
//! The apps bring their own HTTP stack (URLSession, OkHttp) by
//! implementing [`HttpClient`]; nothing here makes network requests itself.
//! Every async method runs on the caller's executor (Swift concurrency,
//! Kotlin coroutines): core has no runtime of its own.

mod account;
mod condense;
mod error;
mod http;
mod model;

pub use account::Account;
pub use condense::{
    ArticleDocument, AudioBlock, AudioSource, Block, CellKind, ImageBlock, ImageSource,
    ListItemBlock, QuoteBlock, TableBlock, TableCell, TextBlock, TextKind, TextSpan, TextStyle,
    Truncation, VideoBlock, VideoSource, condense,
};
pub use error::MarinerError;
pub use http::{HttpClient, HttpError, HttpHeader, HttpRequest, HttpResponse};
pub use model::{
    Article, ArticleContent, ArticleFilter, ArticleScope, Feed, FeedChoice, OpmlImport, Tag,
};

uniffi::setup_scaffolding!();
