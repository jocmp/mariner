//! What an account stores, as callers see it.

use chrono::{DateTime, Utc};

/// A subscribed feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    pub id: String,
    pub subscription_id: String,
    pub title: String,
    /// The URL as subscribed.
    pub feed_url: String,
    pub site_url: Option<String>,
    pub favicon_url: Option<String>,
    pub itunes_image_url: Option<String>,
    /// Sorted by name.
    pub tag_names: Vec<String>,
    /// Whether its articles open in full text, extracted from their pages.
    pub sticky_full_content: bool,
}

/// A tag, which apps show as a folder of the feeds it's on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    /// Whether its folder shows its feeds.
    pub expanded: bool,
}

/// An article as a list shows it, without its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub id: String,
    pub feed_id: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub published_at: DateTime<Utc>,
    pub read: bool,
    pub starred: bool,
}

/// An article with its content, for reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleContent {
    pub article: Article,
    pub content_html: Option<String>,
}

/// Which articles to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArticleFilter {
    All,
    Unread,
    Starred,
}

/// Where articles are listed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArticleScope {
    /// Every feed.
    All,
    /// One feed, by id.
    Feed(String),
    /// Every feed with a tag, by name.
    Tag(String),
}

impl ArticleScope {
    /// The feed id and tag name to limit a query to.
    pub(crate) fn feed_and_tag(&self) -> (Option<String>, Option<String>) {
        match self {
            ArticleScope::All => (None, None),
            ArticleScope::Feed(id) => (Some(id.clone()), None),
            ArticleScope::Tag(name) => (None, Some(name.clone())),
        }
    }
}

/// What importing a subscription list did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpmlImport {
    /// Feeds subscribed to.
    pub added: usize,
    /// Feeds subscribed to already, which got the list's folders.
    pub existing: usize,
}

/// A feed a URL leads to, to subscribe to by its URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedChoice {
    pub url: String,
    pub title: Option<String>,
}
