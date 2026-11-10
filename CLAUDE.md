# Parity testing

- Reference output lives in `outputs/` (gitignored, never tracked). Tools that produce it never live in this repo.
- Parity tests skip when their reference output is missing, so a fresh clone still passes `cargo test`.

# Workflow

- Prefer running a single test (`cargo test -p <crate> <name>`) over the whole suite.
- Before committing: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
