# SPDX-License-Identifier: PMPL-1.0-or-later
# SPDX-FileCopyrightText: 2024-2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# rpa-elysium - Development Tasks

set shell := ["bash", "-uc"]
set dotenv-load := true

import? "contractile.just"

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

# [AUTO-GENERATED] Multi-arch / RISC-V target
build-riscv:
	@echo "Building for RISC-V..."
	cross build --target riscv64gc-unknown-linux-gnu

# Run panic-attacker pre-commit scan
assail:
    @command -v panic-attack >/dev/null 2>&1 && panic-attack assail . || echo "panic-attack not found — install from https://github.com/hyperpolymath/panic-attacker"

# Self-diagnostic — checks dependencies, permissions, paths
doctor:
    @echo "Running diagnostics for rpa-elysium..."
    @echo "Checking required tools..."
    @command -v just >/dev/null 2>&1 && echo "  [OK] just" || echo "  [FAIL] just not found"
    @command -v git >/dev/null 2>&1 && echo "  [OK] git" || echo "  [FAIL] git not found"
    @echo "Checking for hardcoded paths..."
    @grep -rn '$HOME\|$ECLIPSE_DIR' --include='*.rs' --include='*.ex' --include='*.res' --include='*.gleam' --include='*.sh' . 2>/dev/null | head -5 || echo "  [OK] No hardcoded paths"
    @echo "Diagnostics complete."

# Auto-repair common issues
heal:
    @echo "Attempting auto-repair for rpa-elysium..."
    @echo "Fixing permissions..."
    @find . -name "*.sh" -exec chmod +x {} \; 2>/dev/null || true
    @echo "Cleaning stale caches..."
    @rm -rf .cache/stale 2>/dev/null || true
    @echo "Repair complete."

# Guided tour of key features
tour:
    @echo "=== rpa-elysium Tour ==="
    @echo ""
    @echo "1. Project structure:"
    @ls -la
    @echo ""
    @echo "2. Available commands: just --list"
    @echo ""
    @echo "3. Read README.adoc for full overview"
    @echo "4. Read EXPLAINME.adoc for architecture decisions"
    @echo "5. Run 'just doctor' to check your setup"
    @echo ""
    @echo "Tour complete! Try 'just --list' to see all available commands."

# Open feedback channel with diagnostic context
help-me:
    @echo "=== rpa-elysium Help ==="
    @echo "Platform: $(uname -s) $(uname -m)"
    @echo "Shell: $SHELL"
    @echo ""
    @echo "To report an issue:"
    @echo "  https://github.com/hyperpolymath/rpa-elysium/issues/new"
    @echo ""
    @echo "Include the output of 'just doctor' in your report."


# Print the current CRG grade (reads from READINESS.md '**Current Grade:** X' line)
crg-grade:
    @grade=$$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1); \
    [ -z "$$grade" ] && grade="X"; \
    echo "$$grade"

# Generate a shields.io badge markdown for the current CRG grade
# Looks for '**Current Grade:** X' in READINESS.md; falls back to X
crg-badge:
    @grade=$$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1); \
    [ -z "$$grade" ] && grade="X"; \
    case "$$grade" in \
      A) color="brightgreen" ;; B) color="green" ;; C) color="yellow" ;; \
      D) color="orange" ;; E) color="red" ;; F) color="critical" ;; \
      *) color="lightgrey" ;; esac; \
    echo "[![CRG $$grade](https://img.shields.io/badge/CRG-$$grade-$$color?style=flat-square)](https://github.com/hyperpolymath/standards/tree/main/component-readiness-grades)"
