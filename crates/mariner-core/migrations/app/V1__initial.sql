-- The account registry and app-wide preferences. Credentials are never
-- stored here. Tables are STRICT, so a value of the wrong type is an error.

CREATE TABLE accounts (
  id TEXT NOT NULL PRIMARY KEY,
  source TEXT NOT NULL,
  display_name TEXT,
  position INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE preferences (
  key TEXT NOT NULL PRIMARY KEY,
  -- JSON-encoded.
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL
) STRICT;
