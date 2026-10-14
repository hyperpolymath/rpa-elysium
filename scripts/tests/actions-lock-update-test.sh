#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail
SCRIPTS="$(cd "$(dirname "$0")/.." && pwd)"
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT
mkdir -p "$ROOT/.github/workflows"
cd "$ROOT"
cat > .github/workflows/example.yml <<'YAML'
# SPDX-License-Identifier: MPL-2.0
jobs:
  example:
    uses: owner/shared/.github/workflows/ci.yml@v1
YAML
cat > .github/workflows/actions.lock <<'YAML'
version: v0.0.3
workflows:
  '.github/workflows/example.yml': ['owner/shared@v1']
dependencies:
  'owner/shared@v1':
    ref: v1
    commit: sha1-1111111111111111111111111111111111111111
    owner_id: 1
    repo_id: 2
    uses: ['owner/transitive@v1']
  'owner/transitive@v1':
    ref: v1
    commit: sha1-2222222222222222222222222222222222222222
    owner_id: 1
    repo_id: 3
YAML
cp .github/workflows/example.yml original.yml
cp .github/workflows/actions.lock original.lock
cat > fake-gh <<'SH'
#!/usr/bin/env bash
set -eu
if [[ "$*" == *--verify-local* ]]; then
  # The released verifier may mutate either input; neither mutation is ours.
  echo '# verifier-only mutation' >> .github/workflows/example.yml
  echo '# verifier-only lock mutation' >> .github/workflows/actions.lock
  printf '{"valid":%s,"findings":[]}\n' "${FAKE_VERDICT:-true}"
else
  # Model the released generator forgetting a job-level reusable dependency.
  printf '%s\n' '{"version":"v0.0.3","workflows":{".github/workflows/example.yml":[]},"dependencies":{}}' > .github/workflows/actions.lock
  sed -i '1i# This workflow is managed by gh actions-lock.' .github/workflows/example.yml
fi
SH
chmod +x fake-gh
export GH_BIN="$ROOT/fake-gh"
bash "$SCRIPTS/update-actions-lock.sh" --verify-local
cmp original.yml .github/workflows/example.yml
cmp original.lock .github/workflows/actions.lock
echo 'PASS: successful verify is observational for YAML and lock'
export FAKE_VERDICT=false
if bash "$SCRIPTS/update-actions-lock.sh"; then echo 'FAIL: invalid lock accepted'; exit 1; fi
cmp original.yml .github/workflows/example.yml
cmp original.lock .github/workflows/actions.lock
echo 'PASS: failed update rolls back every original byte'
export FAKE_VERDICT='"unexpected-string"'
if bash "$SCRIPTS/update-actions-lock.sh" --verify-local; then echo 'FAIL: malformed verdict accepted'; exit 1; fi
cmp original.yml .github/workflows/example.yml
cmp original.lock .github/workflows/actions.lock
echo 'PASS: malformed verdict fails closed and restores inputs'
# Seed obsolete data only for the update test; the verifier must prune it,
# while retaining the real transitive node above.
cat >> .github/workflows/actions.lock <<'YAML'
  'owner/dead@v1':
    ref: v1
    commit: sha1-3333333333333333333333333333333333333333
    owner_id: 1
    repo_id: 4
YAML
export FAKE_VERDICT=true
bash "$SCRIPTS/update-actions-lock.sh"
[ "$(head -1 .github/workflows/example.yml)" = '# SPDX-License-Identifier: MPL-2.0' ]
if grep -q verifier-only .github/workflows/example.yml .github/workflows/actions.lock; then
  echo 'FAIL: verifier-only mutations survived'; exit 1
fi
bash "$SCRIPTS/check-lock-sync.sh"
echo 'PASS: update preserves reusable closure, SPDX ordering, and discards verifier mutations'
yq -o=json '.' .github/workflows/actions.lock | jq -e '.dependencies["owner/transitive@v1"] != null and .dependencies["owner/dead@v1"] == null' >/dev/null
echo 'PASS: restore retains reachable transitive nodes and prunes stale old records'
