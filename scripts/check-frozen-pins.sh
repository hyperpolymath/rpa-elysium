#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# check-frozen-pins.sh — gate-enforced action-pin freezes.
#
# ── Why a gate and not a Dependabot ignore rule ────────────────────────────
# An ignore rule is a REQUEST to a third-party robot; a freeze check is an
# INVARIANT this repository enforces itself. The distinction is not
# theoretical. Measured, on github/codeql-action:
#
#   2026-09-21  nexia-list#100  estate freeze: v4.38.1 refused at workflow
#               startup (tag AND SHA form); roll back to v4.38.0 SHA.
#   2026-09-21  nexia-list#101  Dependabot re-raised the blocked bump WITHIN
#               1 HOUR of #100 landing — SHA-form re-bump bypassed the
#               `versions: ["4.38.1"]` ignore — while copying the inline
#               "4.38.1 blocked" warning comment verbatim.
#   2026-09-21  nexia-list#104  upgraded to an UNCONDITIONAL hold (no
#               versions key = all updates ignored).
#   2026-09-29  rpa-elysium#140 the unconditional hold was bypassed ANYWAY:
#               Dependabot swapped b96794f0 (v4.38.0) for 2892aa5e (a
#               2026-09-24 releases/v4 merge, "update-v4.38.2") in SHA form,
#               again leaving the "v4.38.0 blocked" comment a lie. The bump
#               also desynced actions.lock (startup_failure for CodeQL and
#               GitHub Pages; issue #138 triage family).
#
# Two documented bypasses of one policy is the point at which a promise must
# become an assertion (epistemic-types: warrant is not knowledge; a claim
# becomes knowledge only when a check on YOUR side of the boundary can fail).
# This script is that check. Had it existed, #140 would have been red at PR
# time with the reason and the held SHA named.
#
# ── The table ──────────────────────────────────────────────────────────────
# One row per frozen action. A `uses:` ref outside the allowed set fails.
# Editing the table IS the deliberate act of lifting a freeze — a one-line,
# reviewable, revertable change (januskey: every operation carries its
# inverse; `git revert` of the table edit restores the freeze).
#
# ── Self-test ──────────────────────────────────────────────────────────────
# `--self-test` runs the table logic against fixtures that must pass and
# fixtures that must be killed, so the gate cannot decay into a Certified
# Null Operation (absolute-zero): it demonstrably rejects the mutant.
#
# Usage:
#   check-frozen-pins.sh [WORKFLOW_DIR]   check every *.yml/*.yaml
#   check-frozen-pins.sh --self-test      run the fixture corpus

set -euo pipefail

# ── the freeze table ───────────────────────────────────────────────────────
# action                      allowed refs (space-separated)
FROZEN_ACTION_1="github/codeql-action"
FROZEN_REFS_1="b96794f015dfd88f77b49b1c93e0fa7110f94c63"
FROZEN_REASON_1="v4.38.1+ refused at workflow startup estate-wide (nexia-list#100); unconditional hold (nexia-list#104); SHA-form re-bump bypassed that hold on 2026-09-29 (rpa-elysium#140)"

N_FROZEN=1

check_file() {
  local file=$1
  local failed=0
  local i action refs reason

  for i in $(seq 1 "$N_FROZEN"); do
    action=$(eval "echo \$FROZEN_ACTION_$i")
    refs=$(eval "echo \$FROZEN_REFS_$i")
    reason=$(eval "echo \$FROZEN_REASON_$i")

    # every uses: line naming this action (owner/repo or owner/repo/subpath)
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      local ref
      ref=${line##*@}
      ref=${ref%%[[:space:]#]*}
      local ok=0 r
      for r in $refs; do
        [ "$ref" = "$r" ] && ok=1 && break
      done
      if [ "$ok" -ne 1 ]; then
        echo "ERROR: $file pins $action at '$ref', which is outside the frozen set."
        echo "       Hold: $reason"
        echo "       Allowed ref(s): $refs"
        echo "       If and only if the freeze is being lifted deliberately, edit the"
        echo "       table at the top of scripts/check-frozen-pins.sh in its own commit."
        failed=1
      fi
    done < <(grep -E "uses:[[:space:]]*${action}(/[^@[:space:]]*)?@" "$file" \
             | sed -E "s/^[[:space:]-]*uses:[[:space:]]*//" | sed -E "s/[[:space:]]+#.*$//" || true)
  done
  return "$failed"
}

self_test() {
  # Not `local`: the EXIT trap must still see it when the script ends.
  dir=$(mktemp -d)
  trap 'rm -rf "${dir:-}"' EXIT
  local failures=0

  cat > "$dir/pass-frozen.yml" <<'EOF'
# SPDX-License-Identifier: MPL-2.0
name: Frozen Fixture
on: push
permissions: read-all
jobs:
  j:
    runs-on: ubuntu-latest
    steps:
      - uses: github/codeql-action/init@b96794f015dfd88f77b49b1c93e0fa7110f94c63 # v4.38.0
      - uses: github/codeql-action/upload-sarif@b96794f015dfd88f77b49b1c93e0fa7110f94c63
      - uses: actions/checkout@v7.0.1
EOF
  cat > "$dir/fail-sha-bump.yml" <<'EOF'
# SPDX-License-Identifier: MPL-2.0
name: SHA Bump Fixture
on: push
permissions: read-all
jobs:
  j:
    runs-on: ubuntu-latest
    steps:
      - uses: github/codeql-action/analyze@2892aa5e19bbd11bc0cff5427e3b750a04d9e3c2 # v4.38.0 (lie)
EOF
  cat > "$dir/fail-tag-bump.yml" <<'EOF'
# SPDX-License-Identifier: MPL-2.0
name: Tag Bump Fixture
on: push
permissions: read-all
jobs:
  j:
    runs-on: ubuntu-latest
    steps:
      - uses: github/codeql-action/init@v4.38.1
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
    echo "check-frozen-pins: self-test FAILED ($failures fixture(s)) — the gate has drifted" >&2
    return 1
  fi
  echo "check-frozen-pins: self-test passed (1 accepted, 2 rejected)"
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
    return 1
  fi
  echo "All frozen action pins are inside their allowed sets"
}

main "$@"
