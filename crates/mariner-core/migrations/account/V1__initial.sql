-- One database per account. Timestamps are Unix seconds; booleans are
-- INTEGER 0/1. Tables are STRICT, so a value of the wrong type is an error. article_statuses, article_sync_statuses and
-- excluded_statuses have no foreign keys: their rows outlive local deletes.

CREATE TABLE feeds (
  id TEXT NOT NULL PRIMARY KEY,
  subscription_id TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  feed_url TEXT NOT NULL,
  site_url TEXT,
  favicon_url TEXT,
  itunes_image_url TEXT,
  enable_sticky_full_content INTEGER NOT NULL DEFAULT 0,
  enable_notifications INTEGER NOT NULL DEFAULT 0,
  open_articles_in_browser INTEGER NOT NULL DEFAULT 0,
  show_unread_badge INTEGER NOT NULL DEFAULT 1,
  read_later INTEGER NOT NULL DEFAULT 0,
  -- Fetch state, used only by local accounts.
  etag TEXT,
  last_modified TEXT,
  last_fetched_at INTEGER,
  fetch_fingerprint TEXT,
  error_count INTEGER NOT NULL DEFAULT 0,
  failed_at INTEGER,
  last_error_kind TEXT,
  last_error_status INTEGER,
  retry_after INTEGER,
  redirected_to TEXT,
  redirect_count INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE articles (
  id TEXT NOT NULL PRIMARY KEY,
  feed_id TEXT NOT NULL REFERENCES feeds (id) ON DELETE CASCADE,
  title TEXT,
  author TEXT,
  content_html TEXT,
  extracted_content_url TEXT,
  url TEXT,
  summary TEXT,
  image_url TEXT,
  enclosure_type TEXT,
  published_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE article_statuses (
  article_id TEXT NOT NULL PRIMARY KEY,
  read INTEGER NOT NULL DEFAULT 0,
  starred INTEGER NOT NULL DEFAULT 0,
  last_read_at INTEGER,
  last_unstarred_at INTEGER,
  updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE taggings (
  id TEXT NOT NULL PRIMARY KEY,
  feed_id TEXT NOT NULL REFERENCES feeds (id) ON DELETE CASCADE,
  name TEXT NOT NULL
) STRICT;

CREATE TABLE tags (
  name TEXT NOT NULL PRIMARY KEY,
  expanded INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE enclosures (
  article_id TEXT NOT NULL REFERENCES articles (id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  type TEXT NOT NULL,
  itunes_duration_seconds INTEGER,
  itunes_image TEXT,
  PRIMARY KEY (article_id, url)
) STRICT;

CREATE TABLE saved_searches (
  id TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  query_text TEXT,
  show_unread_badge INTEGER NOT NULL DEFAULT 1
) STRICT;

CREATE TABLE saved_search_articles (
  saved_search_id TEXT NOT NULL REFERENCES saved_searches (id) ON DELETE CASCADE,
  article_id TEXT NOT NULL REFERENCES articles (id) ON DELETE CASCADE,
  PRIMARY KEY (saved_search_id, article_id)
) STRICT;

CREATE TABLE article_notifications (
  article_id TEXT NOT NULL PRIMARY KEY REFERENCES articles (id) ON DELETE CASCADE,
  dismissed_at INTEGER
) STRICT;

CREATE TABLE excluded_statuses (
  article_id TEXT NOT NULL,
  type TEXT NOT NULL,
  PRIMARY KEY (article_id, type)
) STRICT;

CREATE TABLE article_sync_statuses (
  article_id TEXT NOT NULL,
  key TEXT NOT NULL,
  flag INTEGER NOT NULL DEFAULT 0,
  selected INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (article_id, key)
) STRICT;

CREATE TABLE account_preferences (
  key TEXT NOT NULL PRIMARY KEY,
  value TEXT
) STRICT;

CREATE INDEX articles_feed_id_index ON articles (feed_id);
CREATE INDEX articles_published_at_index ON articles (published_at);
CREATE INDEX articles_created_at_index ON articles (created_at);
CREATE INDEX article_statuses_read_index ON article_statuses (read);
CREATE INDEX article_statuses_starred_index ON article_statuses (starred);
CREATE INDEX article_statuses_updated_at_index ON article_statuses (updated_at);
CREATE INDEX article_statuses_covering_index ON article_statuses (article_id, read, starred);
CREATE UNIQUE INDEX taggings_feed_id_name_unique ON taggings (feed_id, name);
CREATE INDEX taggings_name_index ON taggings (name);
CREATE INDEX article_notifications_dismissed_at_index ON article_notifications (dismissed_at);
CREATE INDEX saved_search_articles_article_id_index ON saved_search_articles (article_id);
CREATE INDEX excluded_statuses_type_article_id_index ON excluded_statuses (type, article_id);
CREATE UNIQUE INDEX feeds_read_later_unique ON feeds (read_later) WHERE read_later = 1;
CREATE UNIQUE INDEX feeds_feed_url_unique ON feeds (feed_url);
