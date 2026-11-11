-- How a local account fetches each feed: conditional GET headers, the
-- last body's fingerprint, errors and back-off, and redirects.

-- name: select_due_feeds
-- Feeds not backing off. ?1 is the current time.
SELECT id, feed_url, etag, last_modified, last_fetched_at, fetch_fingerprint,
       error_count, failed_at, last_error_kind, last_error_status, retry_after,
       redirected_to, redirect_count
FROM feeds
WHERE retry_after IS NULL OR retry_after < ?1;

-- name: update_crawl_data
UPDATE feeds
SET etag = ?2,
    last_modified = ?3,
    last_fetched_at = ?4,
    fetch_fingerprint = ?5,
    error_count = ?6,
    failed_at = ?7,
    last_error_kind = ?8,
    last_error_status = ?9,
    retry_after = ?10,
    redirected_to = ?11,
    redirect_count = ?12
WHERE id = ?1;
