#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Controls for scripts/check-mirror-config.sh. The checker reads repository
# variables and secrets from the environment (that is how GitHub Actions hands
# them to a workflow), so every control is a set of environment values.
set -euo pipefail

PROJECT="$(cd "$(dirname "$0")/../.." && pwd)"
CHECK="$PROJECT/scripts/check-mirror-config.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

mkdir -p "$scratch/.machine_readable/6a2"
cat > "$scratch/.machine_readable/6a2/META.a2ml" <<'META'
[development-practices]
test-coverage-target = "80%"
ci-pipelines = 17
multi-forge = true
forges = ["github", "gitlab", "bitbucket", "codeberg"]
META

# declared_and_enabled exports the three declared mirrors, keyed and hosted.
declared_and_enabled() {
  unset GITEA_MIRROR_ENABLED DISROOT_MIRROR_ENABLED SOURCEHUT_MIRROR_ENABLED RADICLE_MIRROR_ENABLED
  export GITLAB_MIRROR_ENABLED=true
  export BITBUCKET_MIRROR_ENABLED=true
  export CODEBERG_MIRROR_ENABLED=true
  unset GITEA_HOST
}

# Annotations (::error::/::warning::) go to stdout and the verdict to stderr,
# so every control reads the two streams together.
expect_pass() {
  if ! bash "$CHECK" "$scratch" > "$scratch/out" 2>&1; then
    echo "FAIL: $1 was rejected"; cat "$scratch/out"; exit 1
  fi
  echo "PASS: $1 accepted"
}

expect_failure() {
  if bash "$CHECK" "$scratch" > "$scratch/out" 2>&1; then
    echo "FAIL: $1 was accepted"; cat "$scratch/out"; exit 1
  fi
  grep -q "$2" "$scratch/out" || { echo "FAIL: $1 did not name the cause"; cat "$scratch/out"; exit 1; }
  echo "PASS: $1 fails closed"
}

# 1 — the declared set, enabled and keyed.
declared_and_enabled
expect_pass 'the declared mirror set, enabled and keyed'

# 2 — a forge enabled that the repository does not declare.
declared_and_enabled
export DISROOT_MIRROR_ENABLED=true
expect_failure 'an enabled but undeclared mirror' 'does not declare it'

# 3 — the preflight reads variables only. An empty key secret is the reusable's
# business (it prints a skip notice); no job here holds a forge key.
grep -q 'secrets\.' <(sed -n '/mirror-preflight:/,/^  mirror:/p' "$PROJECT/.github/workflows/mirror.yml") && {
  echo "FAIL: the preflight job references secrets"; exit 1; }
grep -q 'secrets\.' "$PROJECT/scripts/check-mirror-config.sh" && {
  echo "FAIL: the checker reads secrets"; exit 1; }
grep -q 'eval ' "$PROJECT/scripts/check-mirror-config.sh" && {
  echo "FAIL: the checker uses eval"; exit 1; }
echo "PASS: the preflight reads variables only, without eval"

# 4 — the gitea leg specifically: enabled, keyed, but no host variable.
declared_and_enabled
export GITEA_MIRROR_ENABLED=true
unset GITEA_HOST
expect_failure 'an enabled gitea mirror with no host variable' 'vars.GITEA_HOST is empty'

# 5 — declared but not enabled: a warning, not a failure.
declared_and_enabled
export GITLAB_MIRROR_ENABLED=false
expect_pass 'a declared mirror that is switched off'
grep -q 'declared in META.a2ml but not enabled' "$scratch/out" || {
  echo "FAIL: a switched-off declared mirror was silent"; cat "$scratch/out"; exit 1; }
echo "PASS: a switched-off declared mirror warns without failing"

# 6 — the configuration measured on this repository on 2026-10-14: gitea and
# disroot enabled alongside the three declared mirrors, sourcehut and radicle
# enabled but undeclared, and no GITEA_HOST. The checker must name each.
declared_and_enabled
export GITEA_MIRROR_ENABLED=true
export DISROOT_MIRROR_ENABLED=true
export SOURCEHUT_MIRROR_ENABLED=true
export RADICLE_MIRROR_ENABLED=true
unset GITEA_HOST
if bash "$CHECK" "$PROJECT" > "$scratch/repo.out" 2>&1; then
  echo "FAIL: the measured mirror configuration reported as consistent"
  cat "$scratch/repo.out"; exit 1
fi
grep -q 'gitea mirror is enabled for this repository but' "$scratch/repo.out" || {
  echo "FAIL: the undeclared gitea leg was not named"; cat "$scratch/repo.out"; exit 1; }
grep -q 'vars.GITEA_HOST is empty' "$scratch/repo.out" || {
  echo "FAIL: the missing gitea host was not named"; cat "$scratch/repo.out"; exit 1; }
grep -q 'disroot mirror is enabled for this repository but' "$scratch/repo.out" || {
  echo "FAIL: the undeclared disroot leg was not named"; cat "$scratch/repo.out"; exit 1; }
grep -q 'sourcehut mirror is enabled for this repository but' "$scratch/repo.out" || {
  echo "FAIL: the undeclared sourcehut leg was not named"; cat "$scratch/repo.out"; exit 1; }
echo "PASS: the repository's measured mirror drift is named in full"

# 7 — the workflow wires the checker into every push, before the fan-out.
WF="$PROJECT/.github/workflows/mirror.yml"
grep -q 'check-mirror-config.sh' "$WF" || { echo "FAIL: mirror.yml does not run the checker"; exit 1; }
grep -q 'mirror-preflight' "$WF" || { echo "FAIL: mirror.yml has no preflight job"; exit 1; }
echo "PASS: mirror.yml runs the preflight"

echo '7 mirror configuration controls passed.'
