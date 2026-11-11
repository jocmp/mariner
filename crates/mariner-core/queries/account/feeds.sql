-- name: select_feeds
-- Every feed, by title (case-insensitively).
SELECT id, subscription_id, title, feed_url, site_url, favicon_url, itunes_image_url
FROM feeds
ORDER BY title COLLATE NOCASE, id;

-- name: select_feed
SELECT id, subscription_id, title, feed_url, site_url, favicon_url, itunes_image_url
FROM feeds
WHERE id = ?1;

-- name: select_feed_id_by_url
SELECT id FROM feeds WHERE feed_url = ?1;

-- name: insert_feed
-- A local feed is its own subscription.
INSERT INTO feeds (id, subscription_id, title, feed_url, site_url, itunes_image_url)
VALUES (?1, ?1, ?2, ?3, ?4, ?5);

-- name: update_feed_title
UPDATE feeds SET title = ?2 WHERE id = ?1;

-- name: delete_feed_statuses
-- Statuses have no foreign key, so they're deleted before their articles.
DELETE FROM article_statuses
WHERE article_id IN (SELECT id FROM articles WHERE feed_id = ?1);

-- name: delete_feed
-- Cascades to the feed's articles, enclosures and taggings.
DELETE FROM feeds WHERE id = ?1;
