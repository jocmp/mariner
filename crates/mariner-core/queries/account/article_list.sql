-- Listing every article in a scope, newest first. ?1 is a feed id and ?2 a
-- tag name, either NULL for no limit. ?3 is an article to list even when
-- the filter would leave it out (the open one), or NULL; listing them all
-- needs none.

-- name: select_articles
SELECT a.id, a.feed_id, a.title, a.author, a.url, a.summary, a.image_url,
       a.published_at, COALESCE(s.read, 0), COALESCE(s.starred, 0)
FROM articles a
LEFT JOIN article_statuses s ON s.article_id = a.id
WHERE (?1 IS NULL OR a.feed_id = ?1)
  AND (?2 IS NULL OR a.feed_id IN (SELECT feed_id FROM taggings WHERE name = ?2))
ORDER BY a.published_at DESC, a.id;

-- name: select_unread_articles
SELECT a.id, a.feed_id, a.title, a.author, a.url, a.summary, a.image_url,
       a.published_at, s.read, s.starred
FROM articles a
JOIN article_statuses s ON s.article_id = a.id
WHERE (s.read = 0 OR a.id = ?3) AND (?1 IS NULL OR a.feed_id = ?1)
  AND (?2 IS NULL OR a.feed_id IN (SELECT feed_id FROM taggings WHERE name = ?2))
ORDER BY a.published_at DESC, a.id;

-- name: select_starred_articles
SELECT a.id, a.feed_id, a.title, a.author, a.url, a.summary, a.image_url,
       a.published_at, s.read, s.starred
FROM articles a
JOIN article_statuses s ON s.article_id = a.id
WHERE (s.starred = 1 OR a.id = ?3) AND (?1 IS NULL OR a.feed_id = ?1)
  AND (?2 IS NULL OR a.feed_id IN (SELECT feed_id FROM taggings WHERE name = ?2))
ORDER BY a.published_at DESC, a.id;

-- name: select_unread_article_ids
-- Every unread article's id, for marking them all read. ?1 is a feed id and
-- ?2 a tag name, either NULL for no limit.
SELECT s.article_id
FROM article_statuses s
JOIN articles a ON a.id = s.article_id
WHERE s.read = 0 AND (?1 IS NULL OR a.feed_id = ?1)
  AND (?2 IS NULL OR a.feed_id IN (SELECT feed_id FROM taggings WHERE name = ?2));

-- name: select_article
-- One article with its content, for reading.
SELECT a.id, a.feed_id, a.title, a.author, a.url, a.summary, a.image_url,
       a.published_at, COALESCE(s.read, 0), COALESCE(s.starred, 0),
       a.content_html
FROM articles a
LEFT JOIN article_statuses s ON s.article_id = a.id
WHERE a.id = ?1;

-- name: select_unread_counts
SELECT a.feed_id, count(*)
FROM articles a
JOIN article_statuses s ON s.article_id = a.id
WHERE s.read = 0
GROUP BY a.feed_id;
