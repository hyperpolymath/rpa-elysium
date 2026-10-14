#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Controls for scripts/reconcile-audit-issue.sh. A stub `gh` records every call
# so the tests can assert what the reconciler did — and, more importantly, what
# it refused to do. No network, no repository writes.
set -euo pipefail

PROJECT="$(cd "$(dirname "$0")/../.." && pwd)"
SCRIPT="$PROJECT/scripts/reconcile-audit-issue.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

mkdir -p "$scratch/bin" "$scratch/state"
cat > "$scratch/bin/gh" <<'STUB'
#!/usr/bin/env bash
# Stub gh: logs every call, answers `issue list` from $GH_STUB_ISSUES, and
# copies any --body-file it is handed so the tests can inspect the report.
printf '%s\n' "$*" >> "$GH_STUB_LOG"
if [ "$1 $2" = "issue list" ]; then
  cat "$GH_STUB_ISSUES"
  exit 0
fi
prev=""
for arg in "$@"; do
  if [ "$prev" = "--body-file" ]; then cp "$arg" "$GH_STUB_BODY" 2>/dev/null || true; fi
  prev="$arg"
done
exit 0
STUB
chmod +x "$scratch/bin/gh"
export PATH="$scratch/bin:$PATH"
export GH_STUB_LOG="$scratch/state/calls.log"
export GH_STUB_ISSUES="$scratch/state/issues.json"
export GH_STUB_BODY="$scratch/state/body.md"

MARKER='<!-- rpa-elysium:cargo-audit -->'
RUN="https://github.com/hyperpolymath/rpa-elysium/actions/runs/123"
RUN2="https://github.com/hyperpolymath/rpa-elysium/actions/runs/456"

reset_log() { : > "$GH_STUB_LOG"; }

run_reconcile() { # status event
  bash "$SCRIPT" --status "$1" --event "$2" --run "$RUN" \
    --summary "$scratch/summary.txt" --repo hyperpolymath/rpa-elysium
}

expect_calls() { # description expected-substring...
  local what="$1"; shift
  local count
  count="$(wc -l < "$GH_STUB_LOG" | tr -d ' ')"
  [ "$count" -eq "$#" ] || {
    echo "FAIL: $what expected $# gh call(s), saw $count:"; cat "$GH_STUB_LOG"; exit 1; }
  local i=1
  for want in "$@"; do
    local got; got="$(sed -n "${i}p" "$GH_STUB_LOG")"
    case "$got" in *"$want"*) ;; *) echo "FAIL: $what call $i: expected '$want', got '$got'"; exit 1 ;; esac
    i=$((i + 1))
  done
  echo "PASS: $what"
}

printf 'error: 1 vulnerability found\nID: RUSTSEC-2026-0316\nPackage: wasmtime\nVersion: 36.0.15\nSolution: upgrade to >=36.0.16\n' \
  > "$scratch/summary.txt"

# 1. A passing audit with no open issue touches nothing.
printf '%s' '[]' > "$GH_STUB_ISSUES"
reset_log
run_reconcile 0 push > /dev/null
expect_calls 'a passing audit with no open issue lists but never writes' 'issue list'

# 2. A passing audit closes the marked issue with the green run linked.
jq -n --arg m "$MARKER" '[{number: 143, title: "Security: Dependency vulnerabilities detected", body: ("body " + $m)}]' \
  > "$GH_STUB_ISSUES"
reset_log
run_reconcile 0 push > /dev/null
expect_calls 'a passing audit records the resolution and closes the marked issue' \
  'issue list' 'issue edit 143' 'issue close 143'
grep -q 'Verified fixed' "$GH_STUB_LOG" || {
  echo "FAIL: the close carries no verification evidence"; cat "$GH_STUB_LOG"; exit 1; }
grep -q 'exited 0 on a run of the default branch' "$GH_STUB_BODY" || {
  echo "FAIL: the issue body does not record the green run"; exit 1; }

# 3. A passing audit adopts a markerless legacy issue, then closes it.
jq -n '[{number: 113, title: "Security: Dependency vulnerabilities detected", body: "cargo audit found vulnerabilities. Run `cargo audit` locally for details."}]' \
  > "$GH_STUB_ISSUES"
reset_log
run_reconcile 0 push > /dev/null
expect_calls 'a passing audit adopts and closes a legacy markerless issue' \
  'issue list' 'issue edit 113' 'issue close 113'

# 4. A failing audit with no issue opens exactly one, with the findings and criteria.
printf '%s' '[]' > "$GH_STUB_ISSUES"
reset_log
run_reconcile 1 push > /dev/null
expect_calls 'a failing audit opens one issue' 'issue list' 'issue create'
grep -q 'RUSTSEC-2026-0316' "$GH_STUB_BODY" || {
  echo "FAIL: the created issue does not carry the findings"; exit 1; }
grep -q 'Acceptance criteria' "$GH_STUB_BODY" || {
  echo "FAIL: the created issue has no acceptance criteria"; exit 1; }
grep -q 'rpa-elysium:cargo-audit' "$GH_STUB_BODY" || {
  echo "FAIL: the created issue has no marker for the next run to find"; exit 1; }

# 5. A failing audit on a new run comments the finding and updates the issue.
jq -n --arg m "$MARKER" --arg r "$RUN2" \
  '[{number: 143, title: "Security: Dependency vulnerabilities detected", body: ("body " + $m + " <!-- last-run: " + $r + " -->")}]' \
  > "$GH_STUB_ISSUES"
reset_log
run_reconcile 1 push > /dev/null
expect_calls 'a new failing run updates and comments' \
  'issue list' 'issue edit 143' 'issue comment 143'

# 6. The same failing run again updates the report without a duplicate comment.
jq -n --arg m "$MARKER" --arg r "$RUN" \
  '[{number: 143, title: "Security: Dependency vulnerabilities detected", body: ("body " + $m + " <!-- last-run: " + $r + " -->")}]' \
  > "$GH_STUB_ISSUES"
reset_log
run_reconcile 1 push > /dev/null
expect_calls 'the same failing run does not duplicate the comment' 'issue list' 'issue edit 143'

# 7. Pull-request events never touch issues.
printf '%s' '[]' > "$GH_STUB_ISSUES"
reset_log
run_reconcile 1 pull_request > /dev/null
expect_calls 'a pull request never reconciles issues'

# 8. --dry-run performs no mutating call.
printf '%s' '[]' > "$GH_STUB_ISSUES"
reset_log
bash "$SCRIPT" --status 1 --event push --run "$RUN" --summary "$scratch/summary.txt" \
  --repo hyperpolymath/rpa-elysium --dry-run > "$scratch/dry.out" 2>&1
[ ! -s "$GH_STUB_LOG" ] || { echo "FAIL: --dry-run called gh"; cat "$GH_STUB_LOG"; exit 1; }
grep -q 'would run: gh issue create' "$scratch/dry.out" || {
  echo "FAIL: --dry-run did not report the action it would take"; cat "$scratch/dry.out"; exit 1; }
echo "PASS: --dry-run reports without writing"

# 9. A missing summary is reported in the body rather than silently dropped.
printf '%s' '[]' > "$GH_STUB_ISSUES"
bash "$SCRIPT" --status 1 --event push --run "$RUN" --summary "$scratch/absent.txt" \
  --repo hyperpolymath/rpa-elysium > "$scratch/nosummary.out" 2>&1
grep -q 'no cargo-audit output was captured' "$GH_STUB_BODY" || {
  echo "FAIL: a missing summary was not recorded"; exit 1; }
echo "PASS: a missing summary is recorded in the issue body"

# 10. The workflow really calls this script, and keeps the gate blocking.
WF="$PROJECT/.github/workflows/cargo-audit.yml"
grep -q 'reconcile-audit-issue.sh' "$WF" || { echo "FAIL: the workflow does not call the reconciler"; exit 1; }
grep -q 'cargo audit --deny warnings' "$WF" || { echo "FAIL: the workflow lost the blocking audit"; exit 1; }
grep -q 'exit "\$status"' "$WF" || { echo "FAIL: the audit step no longer propagates its failure"; exit 1; }
echo "PASS: the workflow calls the reconciler and stays blocking"

echo '10 audit-issue reconciliation controls passed.'
