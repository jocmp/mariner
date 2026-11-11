-- name: insert_article
-- Skips an article that's stored, or that was pruned: a status with no
-- article is a tombstone.
INSERT OR IGNORE INTO articles
  (id, feed_id, title, author, content_html, url, summary, image_url,
   enclosure_type, published_at, created_at)
SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
WHERE NOT EXISTS (SELECT 1 FROM article_statuses WHERE article_id = ?1);

-- name: insert_status
-- ?2 is whether it starts read.
INSERT OR IGNORE INTO article_statuses (article_id, read, updated_at) VALUES (?1, ?2, ?3);

-- name: insert_enclosure
INSERT OR IGNORE INTO enclosures
  (article_id, url, type, itunes_duration_seconds, itunes_image)
VALUES (?1, ?2, ?3, ?4, ?5);

-- name: select_feed_articles
-- The fields an update compares.
SELECT id, title, author, content_html, url, summary, image_url, enclosure_type
FROM articles
WHERE feed_id = ?1;

-- name: update_article
-- Leaves the dates and the status alone.
UPDATE articles
SET title = ?2, author = ?3, content_html = ?4, url = ?5, summary = ?6,
    image_url = ?7, enclosure_type = ?8
WHERE id = ?1;

-- name: delete_articles_gone_from_feed
-- Deletes the feed's unstarred articles that are no longer in it (?2 is a
-- JSON array of the ids it lists) and were stored before ?3, or are read
-- and were stored before the cutoff ?4 (NULL for none). Their statuses
-- stay as tombstones.
DELETE FROM articles
WHERE feed_id = ?1
  AND id NOT IN (SELECT value FROM json_each(?2))
  AND id NOT IN (SELECT article_id FROM article_statuses WHERE starred = 1)
  AND (
    created_at < ?3
    OR (?4 IS NOT NULL AND created_at < ?4
        AND id IN (SELECT article_id FROM article_statuses WHERE read = 1))
  );
