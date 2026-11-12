.PHONY: build-tui install-tui bindings xcframework

BINDINGS := target/bindings
IOS_TARGETS := aarch64-apple-ios aarch64-apple-ios-sim

# Builds the `mariner` terminal reader in release mode: target/release/mariner.
build-tui:
	cargo build --release --locked -p mariner-tui

# Installs `mariner` to ~/.cargo/bin, replacing an older install.
install-tui:
	cargo install --path crates/mariner-tui --locked --force

# Generates the Swift and Kotlin bindings into target/bindings.
bindings:
	cargo build --locked -p mariner-ffi
	cargo run --locked -p mariner-bindgen -- generate \
		--library target/debug/libmariner_ffi.$(if $(filter Darwin,$(shell uname)),dylib,so) \
		--language swift --language kotlin --out-dir $(BINDINGS) --no-format

# Builds target/Mariner.xcframework (device and simulator) for the iOS app,
# with Mariner.swift beside it in target/bindings.
xcframework: bindings
	$(foreach target,$(IOS_TARGETS),cargo build --release --locked -p mariner-ffi --target $(target) &&) true
	rm -rf target/headers target/Mariner.xcframework
	mkdir -p target/headers
	cp $(BINDINGS)/MarinerFFI.h target/headers/
	cp $(BINDINGS)/MarinerFFI.modulemap target/headers/module.modulemap
	xcodebuild -create-xcframework \
		$(foreach target,$(IOS_TARGETS),-library target/$(target)/release/libmariner_ffi.a -headers target/headers) \
		-output target/Mariner.xcframework
