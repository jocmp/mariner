# Parity testing

- Tools that generate reference output live in the sibling repo `../mariner-playground`, never here.
- They read this repo's fixtures and write neutral JSON to `outputs/<tool>/`, which is gitignored and never tracked.
- Parity tests skip when `outputs/` is missing, so a fresh clone still passes `cargo test`.

# Workflow

- Prefer running a single test (`cargo test -p <crate> <name>`) over the whole suite.
- Before committing: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
