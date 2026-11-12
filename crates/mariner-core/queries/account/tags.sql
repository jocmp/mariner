-- name: select_tags
SELECT name, expanded FROM tags ORDER BY name COLLATE NOCASE, name;

-- name: update_tag_expanded
UPDATE tags SET expanded = ?2 WHERE name = ?1;

-- name: select_feed_tag_names
SELECT name FROM taggings WHERE feed_id = ?1 ORDER BY name;

-- name: delete_feed_taggings
DELETE FROM taggings WHERE feed_id = ?1;

-- name: insert_tagging
-- A local tagging's id is `<feed id>:<tag name>`.
INSERT OR IGNORE INTO taggings (id, feed_id, name) VALUES (?1 || ':' || ?2, ?1, ?2);

-- name: insert_tag
INSERT OR IGNORE INTO tags (name) VALUES (?1);

-- name: rename_taggings
-- A feed that already has the new name keeps that tagging; the old one is
-- left for `delete_taggings_named`.
UPDATE OR IGNORE taggings SET name = ?2, id = feed_id || ':' || ?2 WHERE name = ?1;

-- name: delete_taggings_named
DELETE FROM taggings WHERE name = ?1;

-- name: copy_tag
-- The new name keeps the old one's settings, unless it has its own.
INSERT OR IGNORE INTO tags (name, expanded) SELECT ?2, expanded FROM tags WHERE name = ?1;

-- name: delete_tag
DELETE FROM tags WHERE name = ?1;

-- name: delete_tags_without_feeds
-- Of the tags named in ?1 (a JSON array), deletes those no feed has.
DELETE FROM tags
WHERE name IN (SELECT value FROM json_each(?1))
  AND NOT EXISTS (SELECT 1 FROM taggings WHERE taggings.name = tags.name);
