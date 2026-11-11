.PHONY: build-tui install-tui

# Builds the `mariner` terminal reader in release mode: target/release/mariner.
build-tui:
	cargo build --release --locked -p mariner-tui

# Installs `mariner` to ~/.cargo/bin, replacing an older install.
install-tui:
	cargo install --path crates/mariner-tui --locked --force
