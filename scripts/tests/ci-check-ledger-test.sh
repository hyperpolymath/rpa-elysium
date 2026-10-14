#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Controls for scripts/ci-check-ledger.sh. Every control must fail closed: a
# ledger that is missing an entry, an owner, an issue or a live review date is
# rejected, and a workflow recorded as green that has gone red must not pass
# verification.
set -euo pipefail

PROJECT="$(cd "$(dirname "$0")/../.." && pwd)"
CHECK="$PROJECT/scripts/ci-check-ledger.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

mkdir -p "$scratch/.github/workflows" "$scratch/docs/audits"
for wf in alpha beta gamma; do
  printf '# SPDX-License-Identifier: MPL-2.0\nname: %s\non:\n  push:\npermissions:\n  contents: read\njobs:\n  test:\n    runs-on: ubuntu-latest\n    timeout-minutes: 5\n    steps:\n      - run: "true"\n' \
    "${wf^}" > "$scratch/.github/workflows/$wf.yml"
done

ENTRIES="$scratch/entries.jsonl"
: > "$ENTRIES"
ISSUE_URL="https://github.com/hyperpolymath/rpa-elysium/issues/147"
RUN_URL="https://github.com/hyperpolymath/rpa-elysium/actions/runs/1"

run_date() { date -u -d "$1" +%F 2>/dev/null || date -u -v"$1" +%F; }
reset_entries() { : > "$ENTRIES"; }

# entry appends a ledger entry for the given workflow path.
entry() { # path state determination [extra-json]
  jq -cn --arg p "$1" --arg s "$2" --arg d "$3" --argjson extra "${4:-{\}}" '
    {path: $p, name: ($p | sub("^.*/"; "") | sub("\\.yml$"; "")),
     state: $s, conclusion: null, observed: "2026-10-14", workflow_state: "active",
     file_updated: null, determination: $d, reason: "", evidence: [], owner_action: null,
     issue: null, review_by: null} + $extra' >> "$ENTRIES"
}

green_entry() { # path
  entry "$1" green green "{\"evidence\":[\"$RUN_URL\"]}"
}

red_entry() { # path [review date expression]
  entry .github/workflows/alpha.yml red in-triage \
    "{\"reason\":\"a red check needs a full determination\",\"owner_action\":\"fix the generator or retire the workflow\",\"issue\":\"$ISSUE_URL\",\"review_by\":\"$(run_date "${1:-+30 days}")\"}"
}

# patch_last rewrites the most recent entry with a jq filter.
patch_last() {
  local n tmp="$ENTRIES.tmp"
  n=$(wc -l < "$ENTRIES" | tr -d ' ')
  [ "$n" -ge 1 ] || { echo "FAIL: no entry to patch"; exit 1; }
  head -n "$((n - 1))" "$ENTRIES" > "$tmp"
  sed -n "${n}p" "$ENTRIES" | jq -c "$1" >> "$tmp"
  mv "$tmp" "$ENTRIES"
}

ledger() {
  jq -cn --slurpfile w "$ENTRIES" '{schema_version: "1.0", repo: "hyperpolymath/rpa-elysium",
    default_branch: "main", updated: "2026-10-14", description: "fixture",
    workflows: $w}' > "$scratch/docs/audits/ci-check-ledger.json"
}

three_green() {
  reset_entries
  green_entry .github/workflows/alpha.yml
  green_entry .github/workflows/beta.yml
  green_entry .github/workflows/gamma.yml
  ledger
}

two_green() {
  reset_entries
  green_entry .github/workflows/beta.yml
  green_entry .github/workflows/gamma.yml
}

expect_pass() {
  if ! bash "$CHECK" "$scratch" --check --quiet > "$scratch/out" 2>&1; then
    echo "FAIL: $1 was rejected"; sed -n '1,8p' "$scratch/out"; exit 1
  fi
  echo "PASS: $1 accepted"
}

expect_failure() {
  if bash "$CHECK" "$scratch" --check --quiet > "$scratch/out" 2>&1; then
    echo "FAIL: $1 was accepted"; exit 1
  fi
  echo "PASS: $1 fails closed"
}

# ---------------------------------------------------------------- structure
# 1
three_green
expect_pass 'three green workflows with evidence'

# 2 — a workflow on disk with no ledger entry
two_green; ledger
expect_failure 'a workflow missing from the ledger'

# 3 — an entry whose workflow file does not exist
three_green
entry .github/workflows/deleted.yml green green "{\"evidence\":[\"$RUN_URL\"]}"
ledger
expect_failure 'a ledger entry with no workflow file'

# 4 — an unknown state name
three_green
patch_last '.state = "probably-fine"'
ledger
expect_failure 'an unknown state name'

# ------------------------------------------------------- the red strict tier
# 5
two_green; red_entry; ledger
expect_pass 'a red workflow with a complete determination'

# 6 — no owner
two_green; red_entry; patch_last '.owner_action = null'; ledger
expect_failure 'a red workflow with no owner_action'

# 7 — no issue
two_green; red_entry; patch_last '.issue = null'; ledger
expect_failure 'a red workflow with no issue'

# 8 — a review date that has already passed
two_green; red_entry '-1 day'; ledger
expect_failure 'a red workflow whose review date has passed'

# 9 — a review date so far out that nothing is reviewed
two_green; red_entry '+200 days'; ledger
expect_failure 'a red workflow whose review date is more than 90 days out'

# 10 — a reason of the "see above" variety is not a reason
two_green; red_entry; patch_last '.reason = "red"'; ledger
expect_failure 'a red workflow with a token reason'

# -------------------------------------------------------- green entries
# 11 — green with no evidence
reset_entries
entry .github/workflows/alpha.yml green green '{}'
green_entry .github/workflows/beta.yml; green_entry .github/workflows/gamma.yml; ledger
expect_failure 'a green workflow with no evidence'

# 12 — evidence pointing at another repository
reset_entries
entry .github/workflows/alpha.yml green green '{"evidence":["https://github.com/elsewhere/other/actions/runs/1"]}'
green_entry .github/workflows/beta.yml; green_entry .github/workflows/gamma.yml; ledger
expect_failure 'evidence pointing at another repository'

# 13 — an exemption on a workflow that is green
reset_entries
entry .github/workflows/alpha.yml green exempt \
  "{\"evidence\":[\"$RUN_URL\"],\"reason\":\"exemptions belong on non-green states\",\"review_by\":\"$(run_date '+30 days')\"}"
green_entry .github/workflows/beta.yml; green_entry .github/workflows/gamma.yml; ledger
expect_failure 'an exemption attached to a green workflow'

# --------------------------------------------------------- the light tier
# 14 — dormant: reason, issue and review date are enough
reset_entries
entry .github/workflows/alpha.yml dormant exempt \
  "{\"reason\":\"the owner disabled this workflow deliberately\",\"review_by\":\"$(run_date '+30 days')\",\"issue\":\"$ISSUE_URL\"}"
green_entry .github/workflows/beta.yml; green_entry .github/workflows/gamma.yml; ledger
expect_pass 'a dormant workflow with reason, issue and review date'

# 15 — the light tier still needs a pointer
reset_entries
entry .github/workflows/alpha.yml stale in-triage \
  "{\"reason\":\"the last run predates the repair now on main\",\"review_by\":\"$(run_date '+30 days')\"}"
green_entry .github/workflows/beta.yml; green_entry .github/workflows/gamma.yml; ledger
expect_failure 'a stale workflow with no owner_action and no issue'

# ------------------------------------------------------------- verification
OBS="$scratch/obs.json"
cat > "$OBS" <<JSON
{".github/workflows/alpha.yml": {"workflow_state": "active", "conclusion": "failure", "run": "$RUN_URL", "observed": "2026-10-14", "file_updated": null},
 ".github/workflows/beta.yml": {"workflow_state": "active", "conclusion": "success", "run": "$RUN_URL", "observed": "2026-10-14", "file_updated": null},
 ".github/workflows/gamma.yml": {"workflow_state": "active", "conclusion": "success", "run": "$RUN_URL", "observed": "2026-10-14", "file_updated": null}}
JSON

# 16 — a green workflow that has gone red fails verification
three_green
if bash "$CHECK" "$scratch" --verify --observations "$OBS" --quiet > "$scratch/out" 2>&1; then
  echo "FAIL: an unrecorded red was accepted"; exit 1
fi
grep -q 'observed red' "$scratch/out" || { echo "FAIL: the drift message did not name the red"; cat "$scratch/out"; exit 1; }
echo "PASS: an unrecorded red fails verification"

# 17 — the same red, recorded, passes
two_green; red_entry; ledger
if ! bash "$CHECK" "$scratch" --verify --observations "$OBS" --quiet > "$scratch/out" 2>&1; then
  echo "FAIL: a recorded red was rejected by verification"; cat "$scratch/out"; exit 1
fi
echo "PASS: a recorded red passes verification"

# 18 — a benign transition is announced without failing. alpha is recorded red
# and matches the observation; gamma is recorded superseded while the
# observation says green, which is a transition to record rather than a hazard.
reset_entries
red_entry
green_entry .github/workflows/beta.yml
entry .github/workflows/gamma.yml superseded in-triage \
  "{\"reason\":\"the deploy was superseded by a newer push\",\"review_by\":\"$(run_date '+30 days')\",\"issue\":\"$ISSUE_URL\"}"
ledger
if ! bash "$CHECK" "$scratch" --verify --observations "$OBS" --quiet > "$scratch/out" 2>&1; then
  echo "FAIL: a benign transition failed verification"; cat "$scratch/out"; exit 1
fi
grep -q 'state drift (record it)' "$scratch/out" || { echo "FAIL: the benign transition was silent"; cat "$scratch/out"; exit 1; }
echo "PASS: a benign transition warns without failing"

# 19 — an observation set that cannot be read is not treated as drift-free
three_green
printf 'not json at all' > "$scratch/bad.json"
if ! bash "$CHECK" "$scratch" --verify --observations "$scratch/bad.json" --quiet > "$scratch/out" 2>&1; then
  echo "FAIL: an unreadable observation set failed verification instead of warning"; cat "$scratch/out"; exit 1
fi
grep -q 'NOT verified' "$scratch/out" || { echo "FAIL: an unreadable observation set was silent"; cat "$scratch/out"; exit 1; }
echo "PASS: an unreadable observation set warns and is not counted as verified"

# 20 — --write records the observation and preserves the human determination
two_green; red_entry; ledger
bash "$CHECK" "$scratch" --write --observations "$OBS" --quiet > "$scratch/out" 2>&1 || true
jq -e '.workflows[] | select(.path == ".github/workflows/alpha.yml")
       | .determination == "in-triage" and .state == "red" and .conclusion == "failure"
         and (.evidence | length) == 1 and (.evidence[0] | endswith("/1"))' \
  "$scratch/docs/audits/ci-check-ledger.json" > /dev/null || {
  echo "FAIL: --write did not record the observation while preserving the determination"; cat "$scratch/out"; exit 1; }
echo "PASS: --write records observations and preserves determinations"

# 21 — the committed ledger in this repository satisfies the same rules
bash "$CHECK" "$PROJECT" --check --quiet > /dev/null || { echo "FAIL: the committed ledger is invalid"; exit 1; }
echo "PASS: the committed ledger validates"

echo '21 CI check ledger controls passed.'
