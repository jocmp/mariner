# Code style

- When writing an extractor
  - Prefer meta tags, e.g. `.attr(r#"meta[name="og:title"]"#, "value")`, over tags found in the HTML body.
  - Prefer a single selector over adding several.
- Behavior must match the JS implementation (`../mercury-parser`, next to the mariner checkout), quirks included. Leave a short comment where the port preserves something surprising.

# Workflow

- Prefer running single tests over the whole suite, e.g. `MERCURY_ONLY=www.nytimes.com cargo test -p mercury-parser --test snapshot_parity -- --nocapture`.
- After changing an extractor upstream, regenerate its snapshot with `TZ=UTC node crates/mercury-parser/scripts/dump-snapshots.mjs --only <domain>` (from the repo root).
