//! UniFFI's binding generator, kept in its own crate so the CLI's
//! dependencies stay out of the library the apps link. Run from the
//! workspace root:
//!
//! ```sh
//! cargo build -p mariner-ffi
//! cargo run -p mariner-bindgen -- generate --library target/debug/libmariner_ffi.dylib \
//!     --language swift --language kotlin --out-dir target/bindings
//! ```

fn main() {
    uniffi::uniffi_bindgen_main()
}
