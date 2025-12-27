# SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2024 Hyperpolymath <hyperpolymath@proton.me>
#
# rpa-elysium - Development Tasks

set shell := ["bash", "-uc"]
set dotenv-load := true

project := "rpa-elysium"

# Show all recipes
default:
    @just --list --unsorted

# Build all crates in release mode
build:
    cargo build --release

# Build in debug mode
build-debug:
    cargo build

# Run all tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Clean build artifacts
clean:
    cargo clean

# Format all code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run all checks (format, lint, test)
check: fmt-check lint test

# Build and run the filesystem workflow CLI
run-fs *ARGS:
    cargo run --release --bin rpa-fs -- {{ARGS}}

# Initialize an example workflow config
init-workflow:
    cargo run --release --bin rpa-fs -- init workflow.json

# Validate a workflow config
validate CONFIG:
    cargo run --release --bin rpa-fs -- validate {{CONFIG}}

# Watch and run tests on changes
watch-test:
    cargo watch -x "test --workspace"

# Generate documentation
doc:
    cargo doc --workspace --no-deps --open

# Install the rpa-fs binary locally
install:
    cargo install --path crates/rpa-fs-workflow
