#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# check-workflow-headers.sh — the ONE canonical workflow-header predicate.
#
# Checks, per workflow file:
#   1. an SPDX-License-Identifier line exists in the file's leading comment
#      block;
#   2. a top-level `permissions:` declaration exists.
#
# ── Why the header BLOCK and not line 1 ────────────────────────────────────
# REUSE places the identifier anywhere in a file's leading comment block, and
# `gh actions-lock` INSERTS `# This workflow is managed by gh actions-lock.`
# at line 1 whenever it mints or refreshes a lockfile — so a line-1 test
# fights the estate's own tool and re-fails every time a lockfile is
# refreshed. This is a contradiction in the pons-asinorum sense: two
# governors asserting incompatible invariants over the same byte.
#
# Provenance: this predicate is byte-faithful to the canonical check in
# hyperpolymath/standards `governance-reusable.yml` ("Check SPDX headers +
# permissions", since 2026-08-07), which measured the line-1 form falsely
# reporting 27 hypatia workflows and 13 more elsewhere as missing headers
# they all had — and "fixing" that by prepending defaults MIS-LICENSED three
# files before it was caught. The local line-1 copy this repo carried was a
# stale divergent mutant of that check; it is what issue #138 recorded red
# on `lint-workflows` while `governance / Workflow security linter` (the
# canonical copy) was green on the same files, same commit.
#
# ── Self-test (expected-rejection controls) ────────────────────────────────
# `--self-test` runs the predicate against a built-in fixture corpus with
# known verdicts, BOTH directions: fixtures that must pass and fixtures that
# must be rejected. A predicate that cannot kill its own mutant is a
# Certified Null Operation (absolute-zero) and must not be trusted in CI;
# the self-test runs first in the linter job so silent rot in either
# direction (header loss accepted, banner re-criminalised) fails the gate
# rather than passing vacuously.
#
# Usage:
#   check-workflow-headers.sh [WORKFLOW_DIR]   check every *.yml/*.yaml
#   check-workflow-headers.sh --self-test      run the fixture corpus

set -euo pipefail

check_file() {
  local file=$1
  local failed=0

  # The leading run of comment lines is read, tolerating a YAML document
  # marker. A licence declared there is declared.
  if ! awk '/^---[[:space:]]*$/ { next } /^#/ { print; next } { exit }' "$file" \
       | grep -q "^# SPDX-License-Identifier:"; then
    echo "ERROR: $file has no SPDX-License-Identifier in its header comment block"
    failed=1
  fi
  if ! grep -q "^permissions:" "$file"; then
    echo "ERROR: $file missing top-level 'permissions:' declaration"
    failed=1
  fi
  return "$failed"
}

self_test() {
  # Not `local`: the EXIT trap must still see it when the script ends.
  dir=$(mktemp -d)
  trap 'rm -rf "${dir:-}"' EXIT
  local failures=0

  # ---- fixtures that MUST pass --------------------------------------------
  cat > "$dir/pass-banner.yml" <<'EOF'
# This workflow is managed by gh actions-lock.
# SPDX-License-Identifier: MPL-2.0
# This workflow is managed by gh actions-lock.
name: Banner Fixture
on: push
permissions: read-all
jobs: {}
EOF
  cat > "$dir/pass-line1.yml" <<'EOF'
# SPDX-License-Identifier: MPL-2.0
name: Line1 Fixture
on: push
permissions: read-all
jobs: {}
EOF
  cat > "$dir/pass-docmarker.yml" <<'EOF'
---
# This workflow is managed by gh actions-lock.
# SPDX-License-Identifier: MPL-2.0
name: Docmarker Fixture
on: push
permissions:
  contents: read
jobs: {}
EOF

  # ---- fixtures that MUST be rejected -------------------------------------
  cat > "$dir/fail-no-spdx.yml" <<'EOF'
# This workflow is managed by gh actions-lock.
# Copyright (c) 2026 Someone
name: No SPDX Fixture
on: push
permissions: read-all
jobs: {}
EOF
  cat > "$dir/fail-no-permissions.yml" <<'EOF'
# SPDX-License-Identifier: MPL-2.0
name: No Permissions Fixture
on: push
jobs: {}
EOF

  for f in "$dir"/pass-*.yml; do
    if check_file "$f"; then
      echo "self-test: ok   (accepted) $(basename "$f")"
    else
      echo "self-test: FAIL (should have been accepted) $(basename "$f")"
      failures=$((failures + 1))
    fi
  done
  for f in "$dir"/fail-*.yml; do
    if check_file "$f" >/dev/null 2>&1; then
      echo "self-test: FAIL (should have been rejected) $(basename "$f")"
      failures=$((failures + 1))
    else
      echo "self-test: ok   (rejected) $(basename "$f")"
    fi
  done

  if [ "$failures" -ne 0 ]; then
    echo "check-workflow-headers: self-test FAILED ($failures fixture(s)) — the predicate itself has drifted" >&2
    return 1
  fi
  echo "check-workflow-headers: self-test passed (3 accepted, 2 rejected)"
}

main() {
  local arg=${1:-.github/workflows}
  if [ "$arg" = "--self-test" ]; then
    self_test
    return
  fi

  local failed=0
  shopt -s nullglob
  for f in "$arg"/*.yml "$arg"/*.yaml; do
    [ -f "$f" ] || continue
    check_file "$f" || failed=1
  done
  if [ "$failed" -eq 1 ]; then
    echo "Add the SPDX header to the leading comment block and a top-level 'permissions:' declaration." >&2
    return 1
  fi
  echo "All workflows have SPDX headers + permissions"
}

main "$@"
