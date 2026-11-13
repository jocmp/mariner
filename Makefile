.PHONY: build-tui install-tui bindings ios-kit ios android

BINDINGS := target/bindings
IOS_TARGETS := aarch64-apple-ios aarch64-apple-ios-sim
IOS_KIT := ios/MarinerKit
# Matches ios/project.yml, so the core's C code (SQLite) targets it too.
IOS_DEPLOYMENT_TARGET := 18.0

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

# Builds the Rust core for iOS (device and simulator) into
# ios/MarinerKit: MarinerFFI.xcframework and the Swift bindings.
ios-kit: bindings
	$(foreach target,$(IOS_TARGETS),IPHONEOS_DEPLOYMENT_TARGET=$(IOS_DEPLOYMENT_TARGET) cargo build --profile mobile --locked -p mariner-ffi --target $(target) &&) true
	rm -rf target/headers $(IOS_KIT)/MarinerFFI.xcframework
	mkdir -p target/headers
	cp $(BINDINGS)/MarinerFFI.h target/headers/
	cp $(BINDINGS)/MarinerFFI.modulemap target/headers/module.modulemap
	xcodebuild -create-xcframework \
		$(foreach target,$(IOS_TARGETS),-library target/$(target)/mobile/libmariner_ffi.a -headers target/headers) \
		-output $(IOS_KIT)/MarinerFFI.xcframework
	cp $(BINDINGS)/Mariner.swift $(IOS_KIT)/Sources/Mariner/

# Builds the core and generates ios/Mariner.xcodeproj; open it in Xcode.
ios: ios-kit
	cd ios && xcodegen

# Builds the Android app (the Rust core and its bindings included) and
# installs it on the connected device or emulator.
android:
	cd android && ./gradlew installDebug
