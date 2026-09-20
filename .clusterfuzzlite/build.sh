#!/bin/bash -eu
# ClusterFuzzLite build script (language: rust).
#
# Notes:
# * No `+nightly` override: base-builder-rust pins a dated nightly via
#   RUSTUP_TOOLCHAIN (e.g. nightly-2025-09-05) and the bare `nightly` channel
#   is NOT installed, so `cargo +nightly ...` would fail with "toolchain
#   'nightly' is not installed".
# * $SANITIZER is exported by ClusterFuzzLite (address | undefined) and must
#   be forwarded so each matrix build instruments for its own sanitizer.
# * cargo-fuzz ships pre-installed in base-builder-rust (CFL rust docs).
cd "$SRC/project"
cargo fuzz build --release --sanitizer "$SANITIZER"
# Copy only the fuzz binary — the `fuzz_*` glob also matched .d files.
cp fuzz/target/*/release/fuzz_main "$OUT/"
