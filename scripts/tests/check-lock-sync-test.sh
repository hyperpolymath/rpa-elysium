#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail
CHECK="$(cd "$(dirname "$0")/.." && pwd)/check-lock-sync.sh"
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT
mkdir -p "$ROOT/.github/workflows"
WF="$ROOT/.github/workflows"
# Remove the files in WF and recreate a workflow and transitively closed lock
# with reusable-workflow and action refs as the baseline for each test case.
fixture() {
  rm -f "$WF"/*
  cat > "$WF/example.yml" <<'YAML'
# SPDX-License-Identifier: MPL-2.0
name: Example
on: pull_request
permissions: {contents: read}
jobs:
  reusable:
    uses: Owner/Shared/.github/workflows/ci.yml@v1
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: owner/action/subpath@v1
      - run: |
          echo 'uses: not/an-action@v9'
YAML
  cat > "$WF/actions.lock" <<'YAML'
version: 'v0.0.3'
workflows:
  '.github/workflows/example.yml': ['owner/shared@v1', 'owner/action@v1']
dependencies:
  'owner/shared@v1':
    ref: v1
    commit: sha1-1111111111111111111111111111111111111111
    owner_id: 1
    repo_id: 2
    uses: ['owner/action@v1']
  'owner/action@v1':
    ref: v1
    commit: sha1-2222222222222222222222222222222222222222
    owner_id: 1
    repo_id: 3
YAML
}
# Run CHECK against WF and assert the expected pass/fail outcome in $1, using $2
# as the test label. Print captured diagnostics and exit 1 on an unexpected result.
assert_status() {
  expected=$1; label=$2
  rc=0
  bash "$CHECK" "$WF" > "$ROOT/output" 2>&1 || rc=$?
  if { [ "$expected" = pass ] && [ "$rc" -ne 0 ]; } || { [ "$expected" = fail ] && [ "$rc" -eq 0 ]; }; then
    cat "$ROOT/output"; echo "FAIL: $label (exit $rc)"; exit 1
  fi
  echo "PASS: $label"
}
fixture
before=$(sha256sum "$WF"/*)
assert_status pass 'block YAML, subpaths, case folding and run-body decoys'
[ "$before" = "$(sha256sum "$WF"/*)" ] || { echo 'Verifier modified inputs'; exit 1; }
fixture
cat > "$WF/example.yml" <<'YAML'
{jobs: {reusable: {uses: "Owner/Shared/.github/workflows/ci.yml@v1"}, build: {runs-on: "ubuntu-latest", steps: [{uses: "owner/action/subpath@v1"}, {run: "echo uses: decoy/action@v9"}]}}}
YAML
assert_status pass 'flow/quoted YAML is structurally equivalent'
fixture; sed -i 's/subpath@v1/subpath@v2/' "$WF/example.yml"
assert_status fail 'changed action ref must be locked in the same PR'
fixture; sed -i '/uses: Owner\/Shared/d' "$WF/example.yml"
assert_status fail 'orphan reusable lock ref blocks'
fixture; sed -i 's/owner\/action@v1/owner\/missing@v1/' "$WF/actions.lock"
assert_status fail 'dangling direct and transitive edges block'
fixture; sed -i '/uses:/s/owner\/action@v1/owner\/missing@v1/' "$WF/actions.lock"
assert_status fail 'dangling transitive-only edge blocks'
fixture; sed -i 's/sha1-2222222222222222222222222222222222222222/main/' "$WF/actions.lock"
assert_status fail 'mutable dependency commit blocks'
fixture; sed -i 's/owner\/action\/subpath@v1/owner\/action/' "$WF/example.yml"
assert_status fail 'malformed uses never silently disappears'
fixture; sed -i "s/version: 'v0.0.3'/version: 'future-version'/" "$WF/actions.lock"
assert_status fail 'unknown lock schema fails closed'
fixture; sed -i 's/owner_id: 1/owner_id: bogus/' "$WF/actions.lock"
assert_status fail 'invalid dependency identity blocks'
fixture; echo '[' > "$WF/example.yml"
assert_status fail 'unparseable YAML fails closed'
fixture; rm "$WF/example.yml"
assert_status fail 'empty workflow directory fails closed'
fixture; cat > "$WF/local.yml" <<'YAML'
jobs:
  local:
    runs-on: ubuntu-latest
    steps:
      - run: echo ok
YAML
assert_status pass 'zero-uses workflow does not require a fabricated dependency'
fixture; sed -i "s/owner\/shared@v1/owner\/shared@V1/" "$WF/actions.lock"
assert_status fail 'refs remain case-sensitive'
fixture; sed -i "s@.github/workflows/example.yml@.github/workflows/deleted.yml@" "$WF/actions.lock"
assert_status fail 'deleted workflow entry blocks'
fixture
cat >> "$WF/actions.lock" <<'YAML'
  'owner/dead@v1':
    ref: v1
    commit: sha1-3333333333333333333333333333333333333333
    owner_id: 1
    repo_id: 4
YAML
assert_status fail 'unreferenced dependency records block'
fixture
# A dependency back-edge must terminate, not hang the validator.
sed -i '/repo_id: 3/a\    uses: ['owner/shared@v1']' "$WF/actions.lock"
assert_status pass 'reachable cyclic dependency graph terminates'
echo '17 lock-sync regression controls passed (plus observational byte comparison).'
