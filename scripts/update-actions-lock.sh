#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# Refresh actions.lock under one unambiguous policy boundary:
#
# - when actions.lock is present, gh actions-lock is authoritative for direct
#   and transitive commit resolution, so its symbolic workflow refs are kept;
# - when actions.lock is absent, the separate workflow gate requires inline
#   immutable SHAs.
#
# Verification is treated as observational even though released tool versions
# can rewrite workflow bytes. Updates keep tool-authored workflow changes, but
# any failed update rolls both workflows and the lock back atomically.
set -euo pipefail

MODE=update
if [ "${1:-}" = "--verify-local" ]; then
  MODE=verify
  shift
fi

WF_DIR="${1:-.github/workflows}"
# gh-actions-lock operates on the repository-root workflow directory. Do not
# snapshot one directory while asking the tool to verify a different one.
[ "$WF_DIR" = .github/workflows ] || {
  echo 'update-actions-lock: run from the repository root; only .github/workflows is supported' >&2
  exit 2
}
GH_BIN="${GH_BIN:-gh}"

[ -d "$WF_DIR" ] || { echo "update-actions-lock: no workflow directory: $WF_DIR" >&2; exit 1; }

SNAPSHOT="$(mktemp -d)"
COMPLETE=false

# Copy top-level regular .yml and .yaml files from WF_DIR into the directory
# given by $1, creating it if needed and overwriting files with matching names.
# Directory creation, discovery or copy failures return a non-zero status.
snapshot_workflows() {
  destination=$1
  mkdir -p "$destination"

  find "$WF_DIR" -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) -print0 |
    while IFS= read -r -d '' file; do
      cp "$file" "$destination/$(basename "$file")"
    done
}

# Copy top-level regular files from the snapshot directory given by $1 back
# into WF_DIR, overwriting matching names without removing additional files.
# Discovery or copy failures return a non-zero status.
restore_workflows() {
  source=$1

  find "$source" -maxdepth 1 -type f -print0 2>/dev/null |
    while IFS= read -r -d '' file; do
      cp "$file" "$WF_DIR/$(basename "$file")"
    done
}

# Succeeds when <workflow> calls a reusable workflow of <dependency>'s repo at
# exactly <dependency>'s ref (owner/repo/.github/workflows/<file>@<ref>).
workflow_references_reusable_dependency() {
  workflow=$1
  dependency=$2
  repo=${dependency%@*}
  ref=${dependency#*@}

  [[ -f "$workflow" ]] || return 1
  # Read `uses:` values with the YAML parser (YAML-POLICY Y-1): a line awk
  # never matched the quoted value in a KYAML workflow, so a KYAML caller of a
  # reusable had its stale finding rejected instead of accepted as coverage.
  # A parse failure returns 1 -- the finding stays a failure, never a pass.
  yq -r '.. | select(tag == "!!map") | select(has("uses")) | .uses | select(tag == "!!str")' "$workflow" 2>/dev/null |
    awk -v prefix="$repo/.github/workflows/" -v suffix="@$ref" '
      index($0, prefix) == 1 &&
      length($0) >= length(suffix) &&
      substr($0, length($0) - length(suffix) + 1) == suffix { found = 1 }
      END { exit(found ? 0 : 1) }
    '
}

# Succeeds when <workflow> has nothing for the lock to pin: every `uses:` is a
# same-repository path (./… or $/…), or there is none. gh-actions-lock v0.2.0
# reports such a workflow as `not-pinned` ("workflow has no lockfile entry")
# when a v0.1.6-written lock omits it. v0.1.6 never asked for that entry, and
# GitHub runs these workflows without one: measured 2026-10-14 on standards,
# where lock-sync-gate.yml (only `run:` steps) and scorecard.yml,
# secret-scanner.yml and self-ci.yml (only ./ reusable calls) all succeed with
# no entry. A parse failure returns 1 -- the finding stays a failure.
workflow_has_nothing_to_pin() {
  workflow=$1
  [[ -f "$workflow" ]] || return 1
  refs=$(yq -r '.. | select(tag == "!!map") | select(has("uses")) | .uses | select(tag == "!!str")' "$workflow" 2>/dev/null) ||
    return 1
  ! printf '%s\n' "$refs" | grep -v -e '^$' -e '^\./' -e '^\$/' | grep -q .
}

# Succeeds when <workflow> uses at least one same-repository `./…` action or
# reusable workflow and every one of them exists in this checkout (run from
# the repository root). gh-actions-lock v0.2.0 reports any `./` action as
# `local-action` ("not supported", rewrite to `$/…`). The estate measured the
# opposite: check-lock-sync.sh fails the `$/` spelling as an invalid rewrite,
# and v0.1.6 never flagged `./`. A missing path or a parse failure returns 1,
# so the finding stays a failure.
workflow_local_uses_resolve() {
  workflow=$1
  [[ -f "$workflow" ]] || return 1
  refs=$(yq -r '.. | select(tag == "!!map") | select(has("uses")) | .uses | select(tag == "!!str")' "$workflow" 2>/dev/null) ||
    return 1
  local_refs=$(printf '%s\n' "$refs" | grep '^\./') || return 1
  while IFS= read -r ref; do
    [[ -e "${ref%%@*}" ]] || return 1
  done <<< "$local_refs"
}

is_advisory_category() {
  # Advisory findings never affect the tool's own `valid` bit: a tree whose
  # only findings are `sha-as-ref` reports `"valid": true` (measured
  # 2026-09-22 on metadatastician/burble -- 7 sha-as-ref findings, valid true,
  # process exit 1). They must therefore not block on the `valid:false` path
  # either.
  #
  # Discriminate by CATEGORY, never by severity. gh-actions-lock v0.1.6 labels
  # every finding `"severity": "warning"`, `stale` included, so a severity test
  # would swallow real desyncs and turn this gate vacuous. v0.2.0 relabels
  # `stale` as `"error"`: the severity is not stable across releases either.
  case "$1" in
    sha-as-ref) return 0 ;;
    *) return 1 ;;
  esac
}

# Run GH_BIN's local verifier, which may modify workflows and actions.lock;
# the caller restores those inputs. With jq, return 0 for valid:true or for
# valid:false explained solely by the three exceptions below and optional
# sha-as-ref advisories; advisories alone cannot explain valid:false.
# Missing or malformed output returns the tool's non-zero status, or 1 if it exited 0;
# unaccepted findings return 1. Without jq, propagate the plain verifier status.
verify_lock_coverage() {
  # gh-actions-lock (v0.1.6, and still v0.2.0) does not recognise
  # reusable-workflow `uses:` paths. GitHub's startup enforcement nevertheless
  # requires callers to carry the reusable repository and its transitive
  # actions in actions.lock. Accept exactly three tool false positives:
  #   - `stale`, when the named workflow contains the exact
  #     owner/repo/.github/workflows/file@ref dependency;
  #   - `not-pinned` with no dependency ("workflow has no lockfile entry",
  #     v0.2.0), when the workflow has nothing to pin;
  #   - `local-action` with no dependency (v0.2.0), when every `./` action
  #     the workflow uses exists in this checkout.
  # Every other finding, including a wrong ref, remains blocking.
  command -v jq >/dev/null 2>&1 || {
    "$GH_BIN" actions-lock --verify-local
    return
  }

  set +e
  result=$("$GH_BIN" actions-lock --verify-local --json=valid,findings)
  status=$?
  set -e

  if [ -z "$result" ] || ! printf '%s' "$result" | jq -e -s 'length == 1 and (.[0] | type == "object" and (.valid | type == "boolean") and (.findings | type == "array"))' >/dev/null 2>&1; then
    printf '%s\n' "$result"
    [[ "$status" -ne 0 ]] && return "$status"
    return 1
  fi
  # The authoritative validity bit and the process exit are not equivalent.
  # gh-actions-lock v0.1.6 exits 1 for advisory findings such as `sha-as-ref`
  # even while reporting `"valid": true`. Preserve advisory findings in the
  # log, but accept the lock exactly when the authoritative tool says it is
  # valid.
  if printf '%s' "$result" | jq -e '.valid == true' >/dev/null; then
    if printf '%s' "$result" | jq -e '.findings | length > 0' >/dev/null; then
      printf '%s\n' "$result"
      echo "actions-lock: valid with advisory finding(s)"
    fi
    return 0
  fi

  remaining=0
  accepted=0
  advisory=0
  while IFS=$'\t' read -r category workflow dependency; do
    if is_advisory_category "$category"; then
      advisory=$((advisory + 1))
    elif [[ "$category" = stale ]] &&
       workflow_references_reusable_dependency "$workflow" "$dependency"; then
      echo "Accepted reusable-workflow lock coverage: $workflow -> $dependency"
      accepted=$((accepted + 1))
    elif [[ "$category" = not-pinned && -z "$dependency" ]] &&
       workflow_has_nothing_to_pin "$workflow"; then
      echo "Accepted workflow with nothing to pin: $workflow"
      accepted=$((accepted + 1))
    elif [[ "$category" = local-action && -z "$dependency" ]] &&
       workflow_local_uses_resolve "$workflow"; then
      echo "Accepted same-repository ./ action(s): $workflow"
      accepted=$((accepted + 1))
    else
      remaining=$((remaining + 1))
    fi
  done < <(printf '%s' "$result" | jq -r '.findings[] | [.category, .workflow, .dependency] | @tsv')

  if [[ "$advisory" -gt 0 ]]; then
    echo "actions-lock: $advisory advisory finding(s) recorded, not blocking"
  fi

  # `valid:false` with no findings is contradictory and cannot be explained by
  # the one known reusable-workflow false positive. Fail closed rather than
  # turning an empty/malformed diagnostic into approval. Advisory findings do
  # not explain it either, so they never satisfy this guard on their own.
  if [[ "$remaining" -ne 0 || "$accepted" -eq 0 ]]; then
    printf '%s\n' "$result"
    return 1
  fi
}

# EXIT handler: unless COMPLETE is true, restore saved workflows and the lock
# (or remove the lock if none was saved), then remove SNAPSHOT and exit with
# the entry status. A restoration or removal failure can terminate this early.
cleanup() {
  status=$?
  if [ "$COMPLETE" != true ]; then
    restore_workflows "$SNAPSHOT/original-workflows"
    if [ -f "$SNAPSHOT/actions.lock" ]; then
      cp "$SNAPSHOT/actions.lock" "$WF_DIR/actions.lock"
    else
      rm -f "$WF_DIR/actions.lock"
    fi
  fi
  rm -rf "$SNAPSHOT"
  exit "$status"
}
trap cleanup EXIT

snapshot_workflows "$SNAPSHOT/original-workflows"
[ ! -f "$WF_DIR/actions.lock" ] || cp "$WF_DIR/actions.lock" "$SNAPSHOT/actions.lock"

if [ "$MODE" = update ]; then
  "$GH_BIN" actions-lock
  bash "$(dirname "${BASH_SOURCE[0]}")/restore-reusable-lock-coverage.sh" \
    "$SNAPSHOT/actions.lock" "$WF_DIR"
  # The generator prepends a management marker. Keep SPDX first so a valid
  # lock refresh cannot reintroduce the header failures repaired in PR #145.
  for file in "$WF_DIR"/*.yml "$WF_DIR"/*.yaml; do
    [ -f "$file" ] || continue
    spdx=$(grep -m1 '^# SPDX-License-Identifier:' "$file") || {
      echo "::error::$file has no SPDX header; choose its correct licence first"
      exit 1
    }
    { printf '%s\n' "$spdx"; grep -v '^# SPDX-License-Identifier:' "$file"; } \
      > "$SNAPSHOT/normalised.yml"
    cp "$SNAPSHOT/normalised.yml" "$file"
  done
fi

# Despite its name, --verify-local can mutate workflow bytes (including
# migrating local `./` action paths to an invalid `$/` spelling). Preserve the
# post-update state, run verification, then discard verifier-only mutations.
snapshot_workflows "$SNAPSHOT/preverify-workflows"
cp "$WF_DIR/actions.lock" "$SNAPSHOT/preverify.lock"
verify_lock_coverage
restore_workflows "$SNAPSHOT/preverify-workflows"
# --verify-local is observational for the lockfile too, not just YAML.
cp "$SNAPSHOT/preverify.lock" "$WF_DIR/actions.lock"
# Neither upstream advisory handling nor its reusable-workflow exception can
# waive direct-ref drift, missing dependency records, or a corrupt commit.
bash "$(dirname "${BASH_SOURCE[0]}")/check-lock-sync.sh" "$WF_DIR"
COMPLETE=true
