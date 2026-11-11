-- Each sets one state on an article that exists, creating its status row
-- if needed. ?1 is the article id, ?2 the current time.

-- name: mark_read
INSERT INTO article_statuses (article_id, read, last_read_at, updated_at)
SELECT id, 1, ?2, ?2 FROM articles WHERE id = ?1
ON CONFLICT (article_id) DO UPDATE SET read = 1, last_read_at = ?2, updated_at = ?2;

-- name: mark_unread
INSERT INTO article_statuses (article_id, read, updated_at)
SELECT id, 0, ?2 FROM articles WHERE id = ?1
ON CONFLICT (article_id) DO UPDATE SET read = 0, updated_at = ?2;

-- name: add_star
INSERT INTO article_statuses (article_id, starred, updated_at)
SELECT id, 1, ?2 FROM articles WHERE id = ?1
ON CONFLICT (article_id) DO UPDATE SET starred = 1, updated_at = ?2;

-- name: remove_star
INSERT INTO article_statuses (article_id, starred, last_unstarred_at, updated_at)
SELECT id, 0, ?2, ?2 FROM articles WHERE id = ?1
ON CONFLICT (article_id) DO UPDATE SET starred = 0, last_unstarred_at = ?2, updated_at = ?2;

-- name: delete_old_tombstones
-- Statuses whose article is gone, unstarred and unchanged since ?1. By then
-- the article has left its feed, so it won't be added back.
DELETE FROM article_statuses
WHERE starred = 0
  AND updated_at < ?1
  AND article_id NOT IN (SELECT id FROM articles);
