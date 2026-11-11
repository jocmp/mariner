-- name: insert_article
INSERT OR IGNORE INTO articles
  (id, feed_id, title, author, content_html, url, summary, image_url,
   enclosure_type, published_at, created_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);

-- name: insert_unread_status
INSERT OR IGNORE INTO article_statuses (article_id, updated_at) VALUES (?1, ?2);

-- name: insert_enclosure
INSERT OR IGNORE INTO enclosures
  (article_id, url, type, itunes_duration_seconds, itunes_image)
VALUES (?1, ?2, ?3, ?4, ?5);
