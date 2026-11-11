.PHONY: build-tui

# Builds the `mariner` terminal reader in release mode: target/release/mariner.
build-tui:
	cargo build --release --locked -p mariner-tui
