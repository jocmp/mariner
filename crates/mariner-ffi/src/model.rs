//! Core's models as records and enums the apps can hold. Dates are
//! `SystemTime`, which arrives as `Date` in Swift and `Instant` in Kotlin.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mariner_core as core;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
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
    /// Whether its articles open in full text.
    pub sticky_full_content: bool,
}

/// A tag, shown as a folder of the feeds it's on.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Tag {
    pub name: String,
    pub expanded: bool,
}

/// An article as a list shows it, without its content.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Article {
    pub id: String,
    pub feed_id: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub published_at: SystemTime,
    pub read: bool,
    pub starred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ArticleContent {
    pub article: Article,
    pub content_html: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ArticleFilter {
    All,
    Unread,
    Starred,
}

/// Where articles are listed from.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ArticleScope {
    All,
    Feed { id: String },
    Tag { name: String },
}

/// A feed a URL leads to, to subscribe to by its URL.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FeedChoice {
    pub url: String,
    pub title: Option<String>,
}

impl From<core::Feed> for Feed {
    fn from(feed: core::Feed) -> Self {
        Feed {
            id: feed.id,
            subscription_id: feed.subscription_id,
            title: feed.title,
            feed_url: feed.feed_url,
            site_url: feed.site_url,
            favicon_url: feed.favicon_url,
            itunes_image_url: feed.itunes_image_url,
            tag_names: feed.tag_names,
            sticky_full_content: feed.sticky_full_content,
        }
    }
}

impl From<Feed> for core::Feed {
    fn from(feed: Feed) -> Self {
        core::Feed {
            id: feed.id,
            subscription_id: feed.subscription_id,
            title: feed.title,
            feed_url: feed.feed_url,
            site_url: feed.site_url,
            favicon_url: feed.favicon_url,
            itunes_image_url: feed.itunes_image_url,
            tag_names: feed.tag_names,
            sticky_full_content: feed.sticky_full_content,
        }
    }
}

impl From<core::Tag> for Tag {
    fn from(tag: core::Tag) -> Self {
        Tag {
            name: tag.name,
            expanded: tag.expanded,
        }
    }
}

impl From<core::Article> for Article {
    fn from(article: core::Article) -> Self {
        let seconds = u64::try_from(article.published_at.timestamp()).unwrap_or_default();
        Article {
            id: article.id,
            feed_id: article.feed_id,
            title: article.title,
            author: article.author,
            url: article.url,
            summary: article.summary,
            image_url: article.image_url,
            published_at: UNIX_EPOCH + Duration::from_secs(seconds),
            read: article.read,
            starred: article.starred,
        }
    }
}

impl From<core::ArticleContent> for ArticleContent {
    fn from(content: core::ArticleContent) -> Self {
        ArticleContent {
            article: content.article.into(),
            content_html: content.content_html,
        }
    }
}

impl From<ArticleFilter> for core::ArticleFilter {
    fn from(filter: ArticleFilter) -> Self {
        match filter {
            ArticleFilter::All => core::ArticleFilter::All,
            ArticleFilter::Unread => core::ArticleFilter::Unread,
            ArticleFilter::Starred => core::ArticleFilter::Starred,
        }
    }
}

impl From<ArticleScope> for core::ArticleScope {
    fn from(scope: ArticleScope) -> Self {
        match scope {
            ArticleScope::All => core::ArticleScope::All,
            ArticleScope::Feed { id } => core::ArticleScope::Feed(id),
            ArticleScope::Tag { name } => core::ArticleScope::Tag(name),
        }
    }
}

impl From<core::FeedChoice> for FeedChoice {
    fn from(choice: core::FeedChoice) -> Self {
        FeedChoice {
            url: choice.url,
            title: choice.title,
        }
    }
}
