# Parity testing

- Reference output lives in `outputs/` (gitignored, never tracked). Tools that produce it never live in this repo.
- Parity tests skip when their reference output is missing, so a fresh clone still passes `cargo test`.

# Workflow

- Prefer running a single test (`cargo test -p <crate> <name>`) over the whole suite.
- Before committing: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.

# Databases

- Only `mariner-core` opens the SQLite files. Apps, widgets and extensions go through it (via `mariner-ffi`), never through the platform's SQLite (`android.database.sqlite`, `import SQLite3`, GRDB). Core bundles its own SQLite, so every process uses the same version; a second SQLite copy opening the same file can corrupt it.
- Tables are `STRICT`. All SQL lives in `crates/mariner-core/queries/<kind>/*.sql`, checked against the migrations by `build.rs`.
