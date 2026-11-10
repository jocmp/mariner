# Mariner

An RSS reader focused on discovery. A Rust core shared by a terminal UI and, later, iOS and Android apps.

## Layout

| Path | What |
|---|---|
| `crates/mariner-core` | Models, accounts, SQLite storage |
| `crates/mariner-condense` | Article HTML -> native content blocks |
| `crates/mariner-feeds` | Feed parsing and feed discovery |
| `crates/mariner-ffi` | UniFFI bindings for the mobile apps |
| `crates/mariner-tui` | Terminal UI |
| `crates/mercury-parser` | Article extraction (Rust port of Mercury Parser) |
| `crates/mercury-cli` | Command-line interface for mercury-parser |

## Build and test

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Parity tests compare against reference JSON in `outputs/` (never tracked). When it's missing they are skipped.

## License

MIT. See [LICENSE](LICENSE).
