<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- SPDX-FileCopyrightText: 2025-2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk> -->

# Contributing to RPA Elysium

We welcome contributions! Please read this guide before submitting.

## Development Setup

```bash
# Clone the repository
git clone https://github.com/hyperpolymath/rpa-elysium.git
cd rpa-elysium

# Build and test
just check   # fmt + lint + test
just build   # Release build

# Or individually
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Repository Structure

```
rpa-elysium/
├── crates/               # Rust workspace crates
│   ├── rpa-core/         # Core types, traits, abstractions
│   ├── rpa-plugin/       # WASM plugin system
│   └── rpa-fs-workflow/  # Filesystem automation CLI (MVP)
├── src/abi/              # Idris2 ABI definitions
├── ffi/zig/              # Zig FFI implementation
├── services/             # Backend services (Gleam)
├── examples/             # Example workflow configs
├── hooks/                # Validation scripts (pre-commit)
├── .github/workflows/    # CI/CD pipelines (18 workflows)
├── .machine_readable/    # A2ML project metadata
├── contractiles/         # Governance contracts (K9, must/trust/dust)
└── justfile              # Build automation
```

## Language Policy

This project follows the [Hyperpolymath Language Policy](.claude/CLAUDE.md):

| Allowed | Use Case |
|---------|----------|
| **Rust** | Core framework, WASM, CLI |
| **ReScript** | Management console UI |
| **Gleam** | Backend services (BEAM) |
| **Deno** | JS runtime (not Node.js) |
| **Idris2** | ABI definitions |
| **Zig** | FFI implementation |
| **Bash** | Scripts, automation |

**Banned**: TypeScript, Node.js, npm, Go, Python, Java, Kotlin, Swift.

## Branch Naming

```
feat/short-description       # New features
fix/issue-number-description # Bug fixes
docs/short-description       # Documentation
test/what-added              # Test additions
refactor/what-changed        # Code improvements
security/what-fixed          # Security fixes
```

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `security`, `chore`, `ci`

Scopes: `core`, `plugin`, `fs-workflow`, `abi`, `ffi`, `services`, `ci`

## Pull Request Process

1. Fork the repository and create a feature branch
2. Ensure `just check` passes (fmt, lint, test)
3. All SPDX headers must be `PMPL-1.0-or-later`
4. All GitHub Actions must be SHA-pinned
5. No banned language code introduced
6. Submit PR against `main` with descriptive title and body

## Code Standards

- All source files must have SPDX license headers
- All code must pass `cargo clippy -- -D warnings`
- All code must be formatted with `cargo fmt`
- Test coverage target: 80%+
- No hardcoded secrets, credentials, or API keys
- HTTPS only — no HTTP URLs

## Reporting Bugs

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md). Include:
- Environment details (OS, Rust version, etc.)
- Steps to reproduce
- Expected vs actual behaviour

## Suggesting Features

Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md). Include:
- Problem statement
- Proposed solution
- Which crate/component this affects

## License

All contributions are licensed under PMPL-1.0-or-later.
