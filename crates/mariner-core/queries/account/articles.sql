-- name: insert_article
-- Skips an article that's stored, or that was pruned: a status with no
-- article is a tombstone.
INSERT OR IGNORE INTO articles
  (id, feed_id, title, author, content_html, url, summary, image_url,
   enclosure_type, published_at, created_at)
SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
WHERE NOT EXISTS (SELECT 1 FROM article_statuses WHERE article_id = ?1);

-- name: insert_unread_status
INSERT OR IGNORE INTO article_statuses (article_id, updated_at) VALUES (?1, ?2);

-- name: insert_enclosure
INSERT OR IGNORE INTO enclosures
  (article_id, url, type, itunes_duration_seconds, itunes_image)
VALUES (?1, ?2, ?3, ?4, ?5);

-- name: delete_read_articles_before
-- Prunes read, unstarred articles stored before ?1. Their statuses stay as
-- tombstones, so a refresh doesn't add them again.
DELETE FROM articles
WHERE created_at < ?1
  AND id IN (SELECT article_id FROM article_statuses WHERE read = 1 AND starred = 0);
