-- The account registry and app-wide preferences. Credentials are never
-- stored here.

CREATE TABLE accounts (
  id TEXT NOT NULL PRIMARY KEY,
  source TEXT NOT NULL,
  display_name TEXT,
  position INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);

CREATE TABLE preferences (
  key TEXT NOT NULL PRIMARY KEY,
  -- JSON-encoded.
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
