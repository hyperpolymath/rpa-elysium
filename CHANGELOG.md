<!--
SPDX-License-Identifier: MPL-2.0
Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
-->
# Changelog

All notable changes to `rpa-elysium` will be documented in this file.

This file is generated from conventional commits by the
[`changelog-reusable.yml`](https://github.com/hyperpolymath/standards/blob/main/.github/workflows/changelog-reusable.yml)
workflow (`hyperpolymath/standards#206`). Adopt the workflow in this repo's CI to keep this file in sync automatically — see
[`templates/cliff.toml`](https://github.com/hyperpolymath/standards/blob/main/templates/cliff.toml)
for the canonical config.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- feat(crg): add crg-grade and crg-badge justfile recipes
- feat: add stapeln.toml container definition
- feat: deploy UX Manifesto infrastructure
- feat: add CLADE.a2ml — clade taxonomy declaration
- feat(fs-workflow): add --dry-run flag, crate README, improved error messages
- feat: customize fuzz target with repo-specific logic
- feat: add ClusterFuzzLite fuzzing configuration

### Fixed

- fix(ci): bump a2ml/k9-validate-action pins to canonical (#42)
- fix(ci): sync hypatia-scan.yml to canonical (#41)
- fix(ci): rsr-antipattern.yml duplicate heredoc (#36)
- fix(ci): repair YAML block-scalar in workflow-linter Check Permissions step (#37)
- fix(ci): move secret-scanner Cargo.toml gate from job-level if: to step-level (#38)
- fix(ci): Resolve workflow-linter self-matching and metadata issues
- fix: correct email jonathan.jewell → j.d.a.jewell
- fix: global AGPL-3.0-or-later → PMPL-1.0-or-later replacement
- fix(license): SPDX AGPL-3.0 → PMPL-1.0-or-later in dotfiles
- fix: remove duplicate SCM files from root

### Changed

- refactor: migrate 6SCM → 6A2 (.scm → .a2ml format)

### Documentation

- docs: add post-audit status report for M5 sweep
- docs: substantive CRG C annotation (EXPLAINME.adoc)
- docs: add TEST-NEEDS.md and/or PROOF-NEEDS.md from audit
- docs: add EXPLAINME.adoc — prove-it file backing README claims
- docs: add HAR integration, proven-servers, Ephapax, PanLL cross-references
- docs: update SCM files with project information
- docs: add CONTRIBUTING.md
- docs: add SCM checkpoint files

### CI

- ci(rust): convert rust-ci.yml to thin wrapper (standards#174) (#48)
- ci: redistribute concurrency-cancel guard to read-only check workflows (#44)
- ci(secret-scanner): drop duplicate --fail from trufflehog extra_args (#35)
- ci: SHA-pin hyperpolymath validate-actions in dogfood-gate
- ci(antipattern): fix top-level dir + benchmark/lsp filename matching (#33)

## Pre-history

Prior commits to this file's introduction are recorded in git history but not formally classified into Keep-a-Changelog sections. To backfill, run `git cliff -o CHANGELOG.md` locally using the canonical [`cliff.toml`](https://github.com/hyperpolymath/standards/blob/main/templates/cliff.toml) — this is one-shot mechanical work.

---

<!-- This file was seeded by the 2026-05-26 estate tech-debt audit follow-up (Row-2 Phase 3); see [`hyperpolymath/standards/docs/audits/2026-05-26-estate-documentation-debt.md`](https://github.com/hyperpolymath/standards/blob/main/docs/audits/2026-05-26-estate-documentation-debt.md). -->
