//! What an account stores, as callers see it.

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
}

/// What adding a feed by URL led to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddFeedResult {
    /// Subscribed, or already subscribed.
    Added(Feed),
    /// The URL leads to several feeds; add one of them by its URL.
    Choices(Vec<FeedChoice>),
    /// The URL leads to no feed.
    NotFound,
}

/// One of several feeds a URL leads to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedChoice {
    pub url: String,
    pub title: Option<String>,
}
